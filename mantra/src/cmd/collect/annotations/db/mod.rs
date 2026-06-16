use anyhow::bail;
use mantra_schema::{
    FmtHash, Properties,
    annotations::{
        AnnotationSchema, CoverageExclude, CoverageExcludeKind, Element, FileAnnotations, Trace,
        TraceRelatedCodeVariant,
    },
    path::RelativePath,
};

use crate::cmd::collect::{Collection, merge_local_and_base_properties};

pub mod aggregate;

impl<'db> Collection<'db> {
    pub(super) async fn update_per_annotation_schema(
        &mut self,
        filepath: &RelativePath,
        annotaiton_schema: AnnotationSchema,
    ) -> Result<(), anyhow::Error> {
        let base_origin_hash = annotaiton_schema.origin.as_ref().map(FmtHash::from);

        if let Some(hash) = &base_origin_hash
            && let Some(origin) = annotaiton_schema.origin
        {
            self.insert_general_json(hash, origin.clone()).await?;
        }

        let collect_nr = self.collect_nr();
        let product_id = self.product_id();
        let data_filepath = filepath.as_str();

        // TODO: do not stop at first collect error

        for file_annotations in annotaiton_schema.files {
            self.update_per_annotation_file(
                file_annotations,
                &base_origin_hash,
                &annotaiton_schema.trace_properties,
            )
            .await?;
        }

        // Note: Must be added after file annotations, because for the annotation variant from content
        // the filepath for the schema and file annotation is the same and would prevent content being added.
        sqlx::query!(
            "
            insert into AnnotatedDataSources (
                last_collect_nr,
                product_id,
                filepath
            )
            values ($1, $2, $3)
            on conflict (product_id, filepath)
            do update set
                last_collect_nr = excluded.last_collect_nr
            ",
            collect_nr,
            product_id,
            data_filepath
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    pub(crate) async fn delete_outdated_annotations(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
                delete from AnnotatedFileOrigins
                where product_id = $1 and last_collect_nr < $2
            ",
            product_id,
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
                delete from DirectProductReqTraces
                where product_id = $1 and last_collect_nr < $2
            ",
            product_id,
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
                delete from ElementIdents
                where product_id = $1 and last_collect_nr < $2
            ",
            product_id,
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_per_annotation_file(
        &mut self,
        file_annotations: FileAnnotations,
        base_origin_hash: &Option<FmtHash>,
        base_trace_props: &Option<Properties>,
    ) -> Result<(), anyhow::Error> {
        // TODO: don't return on first error

        let collect_nr = self.collect_nr();
        let product_id = &self.product_id();
        let filepath = file_annotations.filepath.as_str();

        self.insert_file_hash(&file_annotations.filepath, &file_annotations.file_hash)
            .await?;

        // Filepath has been added in same collection already.
        // Ensure that values are matching to allow file skipping.
        //
        // Note: Base trace props currently cannot be checked, because traces may have their own props as well
        // and they get merged before insertion.
        if sqlx::query!(
            "
            select filepath
            from AnnotatedDataSources
            where last_collect_nr = $1 and product_id = $2
            and filepath = $3
            ",
            collect_nr,
            product_id,
            filepath
        )
        .fetch_optional(self.connection_mut())
        .await?
        .is_some()
        {
            let opt_prev_base_origin_hash = sqlx::query!(
                "
                select base_origin_hash
                from AnnotatedFileOrigins
                where last_collect_nr = $1 and product_id = $2
                and filepath = $3
                ",
                collect_nr,
                product_id,
                filepath
            )
            .fetch_optional(self.connection_mut())
            .await?
            .map(|r| FmtHash::with_inner(r.base_origin_hash));

            if &opt_prev_base_origin_hash != base_origin_hash {
                bail!(
                    "Duplicate entry in same collection for filepath '{}' with different base origins.",
                    filepath
                );
            }

            log::info!(
                "Skipping already collected annotations for filepath '{}'",
                filepath
            );
            return Ok(());
        }

        // TODO: update last-collect-nr only if file hash has been collected before

        if let Some(content) = file_annotations.content {
            self.insert_file_content(
                &file_annotations.filepath,
                &file_annotations.file_hash,
                &content,
            )
            .await?;
        }

        sqlx::query!(
            "
            insert into AnnotatedDataSources (
                last_collect_nr,
                product_id,
                filepath
            )
            values ($1, $2, $3)
            on conflict (product_id, filepath)
            do update set
                last_collect_nr = excluded.last_collect_nr
            ",
            collect_nr,
            product_id,
            filepath
        )
        .execute(self.connection_mut())
        .await?;

        if let Some(base_origin_hash) = base_origin_hash {
            sqlx::query!(
                "
                insert into AnnotatedFileOrigins (
                    last_collect_nr,
                    product_id,
                    filepath,
                    base_origin_hash
                )
                values (
                    $1,
                    $2,
                    $3,
                    $4
                )
                on conflict (product_id, filepath)
                do update set
                    last_collect_nr = excluded.last_collect_nr,
                    base_origin_hash = excluded.base_origin_hash
                ",
                collect_nr,
                product_id,
                filepath,
                base_origin_hash
            )
            .execute(self.connection_mut())
            .await?;
        }

        // **Note:** Adding elements first to be able to map traces to elements later
        for element in file_annotations.annotations.elements {
            self.update_element(filepath, &file_annotations.file_hash, element)
                .await?;
        }

        for trace in file_annotations.annotations.traces {
            self.update_trace(
                filepath,
                &file_annotations.file_hash,
                trace,
                base_trace_props,
            )
            .await?;
        }

        for coverage_exclude in file_annotations.annotations.coverage_excludes {
            self.update_coverage_exclude(&file_annotations.file_hash, coverage_exclude)
                .await?;
        }

        Ok(())
    }

    async fn update_element(
        &mut self,
        filepath: &str,
        file_hash: &FmtHash,
        element: Element,
    ) -> Result<(), anyhow::Error> {
        let kind = element.kind.as_nr();
        let collect_nr = self.collect_nr();
        let product_id = &self.product_id();

        sqlx::query!(
            "
            insert into Elements (
                last_collect_nr,
                name,
                file_hash,
                definition_line,
                start_line,
                end_line,
                kind,
                content_hash
            )
            values (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                $8
            )
            on conflict (file_hash, definition_line)
            do update set
                last_collect_nr = excluded.last_collect_nr,
                start_line = excluded.start_line,
                end_line = excluded.end_line,
                kind = excluded.kind,
                content_hash = excluded.content_hash
            ",
            collect_nr,
            element.name,
            file_hash,
            element.definition_line,
            element.span.start,
            element.span.end,
            kind,
            element.content_hash
        )
        .execute(self.connection_mut())
        .await?;

        if let Some(ident) = element.ident {
            sqlx::query!(
                "
                insert into ElementIdents (
                    last_collect_nr,
                    product_id,
                    filepath,
                    file_hash,
                    definition_line,
                    ident
                )
                values (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6
                )
                on conflict (product_id, filepath, file_hash, definition_line)
                do update set
                    ident = excluded.ident
                ",
                collect_nr,
                product_id,
                filepath,
                file_hash,
                element.definition_line,
                ident
            )
            .execute(self.connection_mut())
            .await?;
        }

        Ok(())
    }

    async fn update_trace(
        &mut self,
        filepath: &str,
        file_hash: &FmtHash,
        trace: Trace,
        base_trace_props: &Option<Properties>,
    ) -> Result<(), anyhow::Error> {
        let kind = trace.kind.as_nr();
        let collect_nr = self.collect_nr();
        let product_id = &self.product_id();

        if sqlx::query!(
            "
            select line
            from Traces
            where last_collect_nr = $1
            and file_hash = $2 and line = $3
            ",
            collect_nr,
            file_hash,
            trace.line
        )
        .fetch_optional(self.connection_mut())
        .await?
        .is_some()
        {
            // If the trace and filepath have already been collected in this run,
            // it indicates either that two annotation schemas contain the same filepath
            // and file hash, or two traces are defined at the same line.
            // Since collection is skipped for annotations already collected from the same filepaths,
            // this leaves the case of two traces being defined at the same line.
            // Two traces must not be defined at the same line, because it interferes
            // with the mapping to line coverage from test results.
            // e.g. two traces could be set for different statements or conditions at the same line,
            // but line coverage would treat both traces as covered.
            if sqlx::query!(
                "
                select filepath
                from AnnotatedDataSources
                where last_collect_nr = $1 and product_id = $2
                and filepath = $3
                ",
                collect_nr,
                product_id,
                filepath
            )
            .fetch_optional(self.connection_mut())
            .await?
            .is_some()
            {
                bail!(
                    "Duplicate entry for trace at line '{}' in file '{}'. Only one trace may be set per line.",
                    trace.line,
                    filepath
                );
            }
        }

        sqlx::query!(
            "
            insert into Traces (
                last_collect_nr,
                file_hash,
                line,
                kind
            )
            values (
                $1,
                $2,
                $3,
                $4
            )
            on conflict (file_hash, line)
            do update set
                last_collect_nr = excluded.last_collect_nr,
                kind = excluded.kind
            ",
            collect_nr,
            file_hash,
            trace.line,
            kind
        )
        .execute(self.connection_mut())
        .await?;

        if let Some(props) = merge_local_and_base_properties(trace.properties, base_trace_props) {
            for prop in props {
                let value_hash = FmtHash::from(&prop.1);
                self.insert_general_json(&value_hash, prop.1).await?;

                sqlx::query!(
                    "
                    insert into TraceProperties (
                        last_collect_nr,
                        file_hash,
                        line,
                        property_key,
                        value_hash
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4,
                        $5
                    )
                    on conflict (file_hash, line, property_key)
                    do update set
                        last_collect_nr = excluded.last_collect_nr,
                        value_hash = excluded.value_hash
                    ",
                    collect_nr,
                    file_hash,
                    trace.line,
                    prop.0,
                    value_hash
                )
                .execute(self.connection_mut())
                .await?;
            }
        }

        for req_id in trace.ids {
            sqlx::query!(
                "
                insert into DirectReqTraces (
                    last_collect_nr,
                    req_id,
                    file_hash,
                    line
                )
                values (
                    $1,
                    $2,
                    $3,
                    $4
                )
                on conflict (req_id, file_hash, line)
                do update set
                    last_collect_nr = excluded.last_collect_nr
                ",
                collect_nr,
                req_id,
                file_hash,
                trace.line
            )
            .execute(self.connection_mut())
            .await?;

            let req_available = sqlx::query!(
                "
                select id from Requirements
                where id = $1 and product_id = $2
                ",
                req_id,
                product_id
            )
            .fetch_optional(self.connection_mut())
            .await?
            .is_some();

            if req_available {
                sqlx::query!(
                    "
                    insert into DirectProductReqTraces (
                        last_collect_nr,
                        product_id,
                        req_id,
                        filepath,
                        file_hash,
                        line
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4,
                        $5,
                        $6
                    )
                    on conflict (product_id, req_id, filepath, file_hash, line)
                    do update set
                        last_collect_nr = excluded.last_collect_nr
                    ",
                    collect_nr,
                    product_id,
                    req_id,
                    filepath,
                    file_hash,
                    trace.line
                )
                .execute(self.connection_mut())
                .await?;
            }
        }

        if let Some(related_code) = trace.related_code {
            match related_code {
                TraceRelatedCodeVariant::CodeBlock(code_block) => {
                    let kind = code_block.kind.as_nr();

                    sqlx::query!(
                        "
                        insert into TracedCodeBlocks (
                            last_collect_nr,
                            file_hash,
                            traced_line,
                            start_line,
                            end_line,
                            kind,
                            content_hash
                        )
                        values (
                            $1,
                            $2,
                            $3,
                            $4,
                            $5,
                            $6,
                            $7
                        )
                        on conflict (file_hash, traced_line)
                        do update set
                            last_collect_nr = excluded.last_collect_nr,
                            start_line = excluded.start_line,
                            end_line = excluded.end_line,
                            kind = excluded.kind,
                            content_hash = excluded.content_hash
                        ",
                        collect_nr,
                        file_hash,
                        trace.line,
                        code_block.span.start,
                        code_block.span.end,
                        kind,
                        code_block.content_hash
                    )
                    .execute(self.connection_mut())
                    .await?;
                }
                TraceRelatedCodeVariant::ElementAtLine(def_line) => {
                    sqlx::query!(
                        "
                        insert into DirectTracedElements (
                            last_collect_nr,
                            file_hash,
                            traced_line,
                            element_definition_line
                        )
                        values (
                            $1,
                            $2,
                            $3,
                            $4
                        )
                        on conflict (file_hash, traced_line, element_definition_line)
                        do update set
                            last_collect_nr = excluded.last_collect_nr
                        ",
                        collect_nr,
                        file_hash,
                        trace.line,
                        def_line
                    )
                    .execute(self.connection_mut())
                    .await?;
                }
            }
        }

        Ok(())
    }

    async fn update_coverage_exclude(
        &mut self,
        file_hash: &FmtHash,
        coverage_exclude: CoverageExclude,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let comment_hash = FmtHash::from(&coverage_exclude.comment);
        self.insert_general_text(&comment_hash, coverage_exclude.comment, None)
            .await?;

        match coverage_exclude.kind {
            CoverageExcludeKind::Block { start, end } => {
                sqlx::query!(
                    "
                    insert into CoverageBlockExcludes (
                        last_collect_nr,
                        file_hash,
                        start_line,
                        end_line,
                        comment_hash
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4,
                        $5
                    )
                    on conflict (file_hash, start_line)
                    do update set
                        last_collect_nr = excluded.last_collect_nr,
                        end_line = excluded.end_line,
                        comment_hash = excluded.comment_hash
                    ",
                    collect_nr,
                    file_hash,
                    start,
                    end,
                    comment_hash
                )
                .execute(self.connection_mut())
                .await?;
            }
            CoverageExcludeKind::Line(line) => {
                sqlx::query!(
                    "
                    insert into CoverageLineExcludes (
                        last_collect_nr,
                        file_hash,
                        line,
                        comment_hash
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4
                    )
                    on conflict (file_hash, line)
                    do update set
                        last_collect_nr = excluded.last_collect_nr,
                        comment_hash = excluded.comment_hash
                    ",
                    collect_nr,
                    file_hash,
                    line,
                    comment_hash
                )
                .execute(self.connection_mut())
                .await?;
            }
        }

        Ok(())
    }
}
