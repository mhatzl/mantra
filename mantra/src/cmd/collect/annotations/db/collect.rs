use anyhow::Context;
use mantra_schema::{
    FmtHash,
    annotations::{
        AnnotationSchema, CoverageExclude, CoverageExcludeKind, Element, ElementIdentSource,
        FileAnnotations, Trace, TraceRelatedCodeVariant,
    },
};

use crate::cmd::collect::product_collection::ProductCollection;

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn collect_per_annotation_schema(
        &mut self,
        annotation_schema: &AnnotationSchema,
        schema_hash: &FmtHash,
    ) -> Result<(), anyhow::Error> {
        let schema_collected = self.schema_collected(schema_hash).await?;

        if !schema_collected && let Some(props) = &annotation_schema.trace_properties {
            for (key, value) in props {
                let value_hash = FmtHash::from(&value);

                self.insert_general_json(&value_hash, &value)
                    .await
                    .with_context(|| format!("Inserting value for trace property '{key}'"))?;

                sqlx::query!(
                    "
                    insert into SchemaTraceProperties (
                        schema_hash,
                        property_key,
                        value_hash
                    )
                    values (
                        $1,
                        $2,
                        $3
                    )
                    ",
                    schema_hash,
                    key,
                    value_hash
                )
                .execute(self.connection_mut())
                .await
                .with_context(|| format!("Inserting trace property '{key}'"))?;
            }
        }

        let collect_nr = self.collect_nr();
        let product_id = self.product_id().clone();

        let schema_collected_for_product = sqlx::query!(
            "
            select filepath
            from ProductAnnotationSources
            where collect_nr = $1 and product_id = $2
            and schema_hash = $3
            ",
            collect_nr,
            product_id,
            schema_hash,
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Failed to get collected annotation sources")?
        .is_some();

        if !schema_collected_for_product {
            let element_source = ElementIdentSource::Schema(schema_hash.clone());
            let element_source_hash = FmtHash::from(&element_source);
            let source_value = serde_json::to_value(&element_source)?;

            // Note: This may insert an unused JSON blob in case no element ident is defined in this schema,
            // but this is acceptable
            self.insert_general_json(&element_source_hash, &source_value)
                .await?;

            // TODO: do not stop at first collect error

            for file_annotations in &annotation_schema.files {
                let filepath = file_annotations.filepath.as_str();

                self.collect_per_annotation_file(file_annotations, &element_source_hash)
                    .await
                    .with_context(|| format!("Updating annotations for file: '{filepath}'"))?;

                // Note: must be added after collecting file data,
                // to check if file hash has already been collected in this collection
                sqlx::query!(
                    "
                    insert into ProductAnnotationSources (
                        collect_nr,
                        product_id,
                        schema_hash,
                        filepath,
                        file_hash
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4,
                        $5
                    )
                    ",
                    collect_nr,
                    product_id,
                    schema_hash,
                    filepath,
                    file_annotations.file_hash
                )
                .execute(self.connection_mut())
                .await
                .with_context(|| format!("Inserting annotation source '{filepath}'"))?;
            }
        }

        Ok(())
    }

    async fn collect_per_annotation_file(
        &mut self,
        file_annotations: &FileAnnotations,
        element_source_hash: &FmtHash,
    ) -> Result<(), anyhow::Error> {
        // TODO: don't return on first error

        let collect_nr = self.collect_nr();
        let product_id = self.product_id().clone();
        let filepath = file_annotations.filepath.as_str();
        let file_hash = &file_annotations.file_hash;

        let file_hash_collected = sqlx::query!(
            "
            select product_id, filepath
            from ProductAnnotationSources
            where collect_nr = $1 and file_hash = $2
            ",
            collect_nr,
            file_hash
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Failed to get collected file hashes")?
        .is_some();

        // **Note:** Adding elements first to be able to map traces to elements later
        for element in &file_annotations.annotations.elements {
            if !file_hash_collected {
                self.update_element(&file_annotations.file_hash, element)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to update the element '{}' defined at line '{}'",
                            element.name, element.definition_line
                        )
                    })?;
            }

            if let Some(ident) = &element.ident {
                sqlx::query!(
                    "
                    insert into ElementIdents (
                        collect_nr,
                        product_id,
                        filepath,
                        file_hash,
                        definition_line,
                        ident,
                        source_hash
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
                    ",
                    collect_nr,
                    product_id,
                    filepath,
                    file_hash,
                    element.definition_line,
                    ident,
                    element_source_hash
                )
                .execute(self.connection_mut())
                .await
                .context("Failed inserting the element identifier")?;
            }
        }

        if !file_hash_collected {
            for trace in &file_annotations.annotations.traces {
                let line = trace.line;

                self.update_trace(filepath, &file_annotations.file_hash, trace)
                    .await
                    .with_context(|| {
                        format!("Failed to update the trace found at line '{}'", line)
                    })?;
            }

            for coverage_exclude in &file_annotations.annotations.coverage_excludes {
                let line = coverage_exclude.start_line();

                self.update_coverage_exclude(&file_annotations.file_hash, coverage_exclude)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to update the coverage exclusion starting at line '{}'",
                            line
                        )
                    })?;
            }
        }

        // Update direct product traces even if file hash has been collected already.
        // Needed because a file hash may have been collected by another product.
        // Only traces to requirements defined for the product are considered.
        // Since PK is over all fields, "insert or ignore" is ok to use
        sqlx::query!(
            "
            insert or ignore into DirectProductReqTraces (
                collect_nr,
                product_id,
                req_id,
                filepath,
                file_hash,
                line
            )
            select rt.last_collect_nr, $2 as product_id, rt.req_id, $3 as filepath, rt.file_hash, rt.line
            from DetectedReqTraces rt, Requirements r
            where rt.last_collect_nr = $1
            and rt.file_hash = $4
            and r.collect_nr = $1 and r.product_id = $2
            and r.id = rt.req_id

            union

            select rt.last_collect_nr, rt.product_id, rt.req_id, $3 as filepath, rt.file_hash, rt.line
            from DetectedProductReqTraces rt, Requirements r
            where rt.last_collect_nr = $1
            and rt.product_id = $2
            and rt.file_hash = $4
            and r.collect_nr = $1 and r.product_id = $2
            and r.id = rt.req_id
            ",
            collect_nr,
            product_id,
            filepath,
            file_hash
        )
        .execute(self.connection_mut())
        .await
        .context("Updating product traces")?;

        Ok(())
    }

    async fn update_element(
        &mut self,
        file_hash: &FmtHash,
        element: &Element,
    ) -> Result<(), anyhow::Error> {
        let kind = element.kind.as_nr();
        let collect_nr = self.collect_nr();

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
        .await
        .context("Failed inserting element base data")?;

        Ok(())
    }

    async fn update_trace(
        &mut self,
        filepath: &str,
        file_hash: &FmtHash,
        trace: &Trace,
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
        .await
        .context("Failed to get collected traces")?
        .is_some()
        {
            // If the trace has already been collected in this run,
            // it indicates either that the same file hash is collected more than once,
            // or two traces are defined at the same line.
            // Since we skip collection of traces if the file hash was already collected in this collection,
            // it only leaves the case of two traces being defined at the same line.
            //
            // Two traces must not be defined at the same line, because it interferes
            // with the mapping to line coverage from test results.
            // e.g. two traces could be set for different statements or conditions at the same line,
            // but line coverage would treat both traces as covered.
            anyhow::bail!("Duplicate entry for trace. Only one trace may be set per line.");
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
        .await
        .context("Failed to insert trace base data")?;

        if let Some(props) = &trace.properties {
            for prop in props {
                let value_hash = FmtHash::from(&prop.1);
                self.insert_general_json(&value_hash, prop.1)
                    .await
                    .with_context(|| {
                        format!("Failed to insert trace content for property '{}'", &prop.0)
                    })?;

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
                .await
                .with_context(|| format!("Failed to insert trace property '{}'", prop.0))?;
            }
        }

        for traced_req in &trace.ids {
            if let Some(pid) = &traced_req.product_id {
                sqlx::query!(
                    "
                    insert into DetectedProductReqTraces (
                        last_collect_nr,
                        product_id,
                        req_id,
                        file_hash,
                        line
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4,
                        $5
                    )
                    on conflict (product_id, req_id, file_hash, line)
                    do update set
                        last_collect_nr = excluded.last_collect_nr
                    ",
                    collect_nr,
                    pid,
                    traced_req.id,
                    file_hash,
                    trace.line
                )
                .execute(self.connection_mut())
                .await
                .with_context(|| {
                    format!("Failed to insert product trace to req '{}'", traced_req.id)
                })?;
            } else {
                sqlx::query!(
                    "
                    insert into DetectedReqTraces (
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
                    traced_req.id,
                    file_hash,
                    trace.line
                )
                .execute(self.connection_mut())
                .await
                .with_context(|| format!("Failed to insert trace to req '{}'", traced_req.id))?;
            };
        }

        if let Some(related_code) = &trace.related_code {
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
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to insert traced code block starting at line '{}'",
                            code_block.span.start
                        )
                    })?;
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
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to insert trace relation to element defined at line '{}'",
                            def_line
                        )
                    })?;
                }
            }
        }

        Ok(())
    }

    async fn update_coverage_exclude(
        &mut self,
        file_hash: &FmtHash,
        coverage_exclude: &CoverageExclude,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let comment_hash = FmtHash::from(&coverage_exclude.comment);
        self.insert_general_text(&comment_hash, &coverage_exclude.comment)
            .await
            .context("Failed to insert coverage exclusion comment")?;

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
                .await
                .context("Failed to insert the coverage exclusion block")?;
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
                .await
                .context("Failed to insert the coverage exclusion line")?;
            }
        }

        Ok(())
    }
}
