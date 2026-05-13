use mantra_schema::{
    FmtHash, Properties,
    path::RelativePath,
    test_runs::{TestRun, TestRunSchema},
    time::OffsetDateTime,
};

use crate::{
    cmd::collect::{Collection, merge_local_and_base_properties},
    db::FilepathExt,
};

pub mod aggregate;

impl<'db> Collection<'db> {
    pub(super) async fn update_per_test_run_schema(
        &mut self,
        filepath: Option<&RelativePath>,
        test_run_schema: TestRunSchema,
    ) -> Result<(), anyhow::Error> {
        let base_origin_hash = test_run_schema.origin.as_ref().map(FmtHash::from);

        if let Some(hash) = &base_origin_hash
            && let Some(origin) = test_run_schema.origin
        {
            self.insert_general_json(&hash, origin.clone()).await?;
        }

        // TODO: do not stop at first collect error

        for test_run in test_run_schema.test_runs {
            self.update_per_test_run(
                filepath,
                test_run,
                &base_origin_hash,
                &test_run_schema.test_run_properties,
                &test_run_schema.test_case_properties,
            )
            .await?;
        }

        Ok(())
    }

    pub(crate) async fn delete_outdated_test_runs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        let updated_records = sqlx::query!(
            "
                select name, utc_date from TestRuns
                where product_id = $1 and last_collect_nr = $2
            ",
            product_id,
            collect_nr
        )
        .fetch_all(self.connection_mut())
        .await?;

        // Note: always deleting outdated data for collected test runs,
        // because this means that the data got removed in the original source.
        for record in updated_records {
            sqlx::query!(
                "
                delete from TestRunProperties
                where product_id = $1 and last_collect_nr < $2
                and test_run_name = $3 and test_run_date = $4
            ",
                product_id,
                collect_nr,
                record.name,
                record.utc_date
            )
            .execute(self.connection_mut())
            .await?;

            sqlx::query!(
                "
                delete from TestRunRevisions
                where product_id = $1 and last_collect_nr < $2
                and test_run_name = $3 and test_run_date = $4
            ",
                product_id,
                collect_nr,
                record.name,
                record.utc_date
            )
            .execute(self.connection_mut())
            .await?;

            sqlx::query!(
                "
                delete from TestRunRevisionAuthors
                where product_id = $1 and last_collect_nr < $2
                and test_run_name = $3 and test_run_date = $4
            ",
                product_id,
                collect_nr,
                record.name,
                record.utc_date
            )
            .execute(self.connection_mut())
            .await?;

            sqlx::query!(
                "
                delete from TestRunHierarchies
                where last_collect_nr < $2 and product_id = $1
                and ((parent_name = $3 and parent_date = $4)
                or (child_name = $3 and child_date = $4))
            ",
                product_id,
                collect_nr,
                record.name,
                record.utc_date
            )
            .execute(self.connection_mut())
            .await?;

            sqlx::query!(
                "
                delete from TestRunLogs
                where product_id = $1 and last_collect_nr < $2
                and test_run_name = $3 and test_run_date = $4
            ",
                product_id,
                collect_nr,
                record.name,
                record.utc_date
            )
            .execute(self.connection_mut())
            .await?;

            sqlx::query!(
                "
                delete from TestRunLineCoverage
                where product_id = $1 and last_collect_nr < $2
                and test_run_name = $3 and test_run_date = $4
            ",
                product_id,
                collect_nr,
                record.name,
                record.utc_date
            )
            .execute(self.connection_mut())
            .await?;

            sqlx::query!(
                "
                delete from TestCases
                where product_id = $1 and last_collect_nr < $2
                and test_run_name = $3 and test_run_date = $4
            ",
                product_id,
                collect_nr,
                record.name,
                record.utc_date
            )
            .execute(self.connection_mut())
            .await?;

            sqlx::query!(
                "
                delete from TestCaseVerifiedRequirements
                where product_id = $1 and last_collect_nr < $2
                and test_run_name = $3 and test_run_date = $4
            ",
                product_id,
                collect_nr,
                record.name,
                record.utc_date
            )
            .execute(self.connection_mut())
            .await?;

            sqlx::query!(
                "
                delete from TestCaseProperties
                where product_id = $1 and last_collect_nr < $2
                and test_run_name = $3 and test_run_date = $4
            ",
                product_id,
                collect_nr,
                record.name,
                record.utc_date
            )
            .execute(self.connection_mut())
            .await?;

            sqlx::query!(
                "
                delete from TestCaseLogs
                where product_id = $1 and last_collect_nr < $2
                and test_run_name = $3 and test_run_date = $4
            ",
                product_id,
                collect_nr,
                record.name,
                record.utc_date
            )
            .execute(self.connection_mut())
            .await?;

            sqlx::query!(
                "
                delete from TestCaseLocations
                where product_id = $1 and last_collect_nr < $2
                and test_run_name = $3 and test_run_date = $4
            ",
                product_id,
                collect_nr,
                record.name,
                record.utc_date
            )
            .execute(self.connection_mut())
            .await?;

            sqlx::query!(
                "
                delete from TestCaseStateProperties
                where product_id = $1 and last_collect_nr < $2
                and test_run_name = $3 and test_run_date = $4
            ",
                product_id,
                collect_nr,
                record.name,
                record.utc_date
            )
            .execute(self.connection_mut())
            .await?;

            sqlx::query!(
                "
                delete from TestCaseLineCoverage
                where product_id = $1 and last_collect_nr < $2
                and test_run_name = $3 and test_run_date = $4
            ",
                product_id,
                collect_nr,
                record.name,
                record.utc_date
            )
            .execute(self.connection_mut())
            .await?;
        }

        // Note: due to cascade rules, deletions in the base TestRuns table
        // cascade to the other tables
        sqlx::query!(
            "
                delete from TestRuns
                where product_id = $1 and last_collect_nr < $2
            ",
            product_id,
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    pub(super) async fn insert_test_run_data_filepaths(
        &mut self,
        test_run_name: &str,
        test_run_date: &OffsetDateTime,
        filepath: &RelativePath,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        // Note: Checking for duplicate test run definitions here,
        // because data filepaths are inserted before the actual test run.
        // So if a test run already exists, it must be a duplicate.
        if sqlx::query!(
            "
            select name, utc_date from TestRuns
            where last_collect_nr = $1 and product_id = $2
            and name = $3 and utc_date = $4
            ",
            collect_nr,
            product_id,
            test_run_name,
            test_run_date
        )
        .fetch_optional(self.connection_mut())
        .await?
        .is_some()
        {
            let records = sqlx::query!(
                "
                select filepath from TestRunDataFilepaths
                where last_collect_nr = $1 and product_id = $2
                and test_run_name = $3 and test_run_date = $4
                ",
                collect_nr,
                product_id,
                test_run_name,
                test_run_date
            )
            .fetch_all(self.connection_mut())
            .await?;

            let prev_filepaths = records.iter().fold(String::new(), |mut s, item| {
                if !s.is_empty() {
                    s.push_str(", ");
                }
                s.push_str(&format!("'{}'", item.filepath));
                s
            });

            anyhow::bail!(
                "Duplicate test run definition for name='{}' date='{}' found in the same collection! Duplicate definition in '{}';  Previous related filepaths: {}",
                test_run_name,
                test_run_date,
                filepath,
                prev_filepaths
            );
        }

        let filepath = filepath.as_str();
        sqlx::query!(
            "
            insert into TestRunDataFilepaths (
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                filepath
            )
            values ($1, $2, $3, $4, $5)
            on conflict
            do update set
                last_collect_nr = excluded.last_collect_nr
            ",
            collect_nr,
            product_id,
            test_run_name,
            test_run_date,
            filepath
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_per_test_run(
        &mut self,
        filepath: Option<&RelativePath>,
        test_run: TestRun,
        base_origin_hash: &Option<FmtHash>,
        base_test_run_props: &Option<Properties>,
        base_test_case_props: &Option<Properties>,
    ) -> Result<(), anyhow::Error> {
        // TODO: optimize by checking data-hash first and skip if unchanged

        let collect_nr = self.collect_nr();
        let product_id = &self.product_id();

        // Note: Important to insert data filepath **before** inserting the test run, because this internally checks for duplicate test run entries!
        // If it would be inserted after the test run, then it would not be possible to detect which data filepaths
        // are part of the previous test run definition, because well-known formats may have multiple data filepaths added
        // while building up the test run.
        if let Some(data_filepath) = filepath {
            self.insert_test_run_data_filepaths(&test_run.name, &test_run.utc_date, data_filepath)
                .await?;
        }

        let data_hash = FmtHash::from(&serde_json::json!({
            "base_origin_hash": base_origin_hash,
            "base_test_run_props": base_test_run_props,
            "base_test_case_props": base_test_case_props,
            "test_run": &test_run
        }));

        let origin_hash = &test_run.origin.as_ref().map(FmtHash::from);
        if let Some(hash) = &origin_hash
            && let Some(origin) = test_run.origin
        {
            self.insert_general_json(hash, origin).await?;
        }
        let description_hash = test_run.description.as_ref().map(FmtHash::from);
        if let Some(hash) = &description_hash
            && let Some(description) = test_run.description
        {
            self.insert_general_text(hash, description, None).await?;
        }
        let duration = test_run.duration_sec.map(|d| d.as_seconds_f64());

        sqlx::query!(
            "
            insert into TestRuns (
                last_collect_nr,
                product_id,
                name,
                utc_date,
                description_hash,
                duration_sec,
                nr_of_test_cases,
                base_origin_hash,
                origin_hash,
                data_hash
            )
            values (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9,
                $10
            )
            on conflict (product_id, name, utc_date)
            do update set
                last_collect_nr = excluded.last_collect_nr,
                description_hash = excluded.description_hash,
                duration_sec = excluded.duration_sec,
                nr_of_test_cases = excluded.nr_of_test_cases,
                base_origin_hash = excluded.base_origin_hash,
                origin_hash = excluded.origin_hash,
                data_hash = excluded.data_hash
            ",
            collect_nr,
            product_id,
            test_run.name,
            test_run.utc_date,
            description_hash,
            duration,
            test_run.nr_of_test_cases,
            base_origin_hash,
            origin_hash,
            data_hash
        )
        .execute(self.connection_mut())
        .await?;

        if let Some(props) =
            merge_local_and_base_properties(test_run.properties, base_test_run_props)
        {
            for prop in props {
                let value_hash = FmtHash::from(&prop.1);
                self.insert_general_json(&value_hash, prop.1).await?;

                sqlx::query!(
                    "
                    insert into TestRunProperties (
                        last_collect_nr,
                        product_id,
                        test_run_name,
                        test_run_date,
                        property_key,
                        value_hash
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4,
                        $5,
                        $6
                    )
                    on conflict (product_id, test_run_name, test_run_date, property_key)
                    do update set
                        last_collect_nr = excluded.last_collect_nr,
                        value_hash = excluded.value_hash
                    ",
                    collect_nr,
                    product_id,
                    test_run.name,
                    test_run.utc_date,
                    prop.0,
                    value_hash
                )
                .execute(self.connection_mut())
                .await?;
            }
        }

        if let Some(revisions) = test_run.revisions {
            for revision in revisions {
                sqlx::query!(
                    "
                    insert into TestRunRevisions (
                        last_collect_nr,
                        product_id,
                        test_run_name,
                        test_run_date,
                        revision,
                        comment
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4,
                        $5,
                        $6
                    )
                    on conflict (product_id, test_run_name, test_run_date, revision)
                    do update set
                        last_collect_nr = excluded.last_collect_nr,
                        comment = excluded.comment
                    ",
                    collect_nr,
                    product_id,
                    test_run.name,
                    test_run.utc_date,
                    revision.nr,
                    revision.comment
                )
                .execute(self.connection_mut())
                .await?;

                for author in revision.authors {
                    sqlx::query!(
                        "
                        insert into TestRunRevisionAuthors (
                            last_collect_nr,
                            product_id,
                            test_run_name,
                            test_run_date,
                            revision,
                            author
                        )
                        values (
                            $1,
                            $2,
                            $3,
                            $4,
                            $5,
                            $6
                        )
                        on conflict (product_id, test_run_name, test_run_date, revision, author)
                        do update set
                            last_collect_nr = excluded.last_collect_nr
                        ",
                        collect_nr,
                        product_id,
                        test_run.name,
                        test_run.utc_date,
                        revision.nr,
                        author
                    )
                    .execute(self.connection_mut())
                    .await?;
                }
            }
        }

        for child_test_run in test_run.test_runs {
            sqlx::query!(
                "
                    insert into TestRunHierarchies (
                        last_collect_nr,
                        product_id,
                        parent_name,
                        parent_date,
                        child_name,
                        child_date
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4,
                        $5,
                        $6
                    )
                    on conflict (
                        product_id,
                        parent_name,
                        parent_date,
                        child_name,
                        child_date
                    )
                    do update set
                        last_collect_nr = excluded.last_collect_nr
                    ",
                collect_nr,
                product_id,
                test_run.name,
                test_run.utc_date,
                child_test_run.name,
                child_test_run.utc_date
            )
            .execute(self.connection_mut())
            .await?;

            // foreign key constraints are deferred for test run hierarchies
            // => safe to add child test run after hierarchy inside the same transaction
            Box::pin(self.update_per_test_run(
                filepath,
                child_test_run,
                base_origin_hash,
                base_test_run_props,
                base_test_case_props,
            ))
            .await?;
        }

        if let Some(logs) = test_run.logs {
            // TODO: ensure that log srcs only appear once
            for log in logs {
                let log_src = log.source.as_nr();
                let log_hash = FmtHash::from(&log.content);
                self.insert_general_text(&log_hash, log.content, None)
                    .await?;

                sqlx::query!(
                    "
                        insert into TestRunLogs (
                            last_collect_nr,
                            product_id,
                            test_run_name,
                            test_run_date,
                            log_src,
                            log_hash
                        )
                        values (
                            $1,
                            $2,
                            $3,
                            $4,
                            $5,
                            $6
                        )
                        on conflict (
                            product_id,
                            test_run_name,
                            test_run_date,
                            log_src
                        )
                        do update set
                            last_collect_nr = excluded.last_collect_nr,
                            log_hash = excluded.log_hash
                        ",
                    collect_nr,
                    product_id,
                    test_run.name,
                    test_run.utc_date,
                    log_src,
                    log_hash
                )
                .execute(self.connection_mut())
                .await?;
            }
        }

        for covered_file in test_run.covered_files {
            let filepath = covered_file.filepath.to_filepath();

            for line in covered_file.lines {
                sqlx::query!(
                    "
                    insert into TestRunLineCoverage (
                        last_collect_nr,
                        product_id,
                        test_run_name,
                        test_run_date,
                        cov_filepath,
                        cov_file_hash,
                        cov_line,
                        hits
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
                    on conflict (
                        product_id,
                        test_run_name,
                        test_run_date,
                        cov_filepath,
                        cov_line
                    )
                    do update set
                        last_collect_nr = excluded.last_collect_nr,
                        hits = excluded.hits
                    ",
                    collect_nr,
                    product_id,
                    test_run.name,
                    test_run.utc_date,
                    filepath,
                    covered_file.file_hash,
                    line.nr,
                    line.hits
                )
                .execute(self.connection_mut())
                .await?;
            }
        }

        for test_case in test_run.test_cases {
            let state = test_case.state.as_nr();
            let description_hash = test_case.description.as_ref().map(FmtHash::from);
            if let Some(hash) = &description_hash
                && let Some(description) = test_case.description
            {
                self.insert_general_text(hash, description, None).await?;
            }
            let duration = test_case.duration_sec.map(|d| d.as_seconds_f64());

            sqlx::query!(
                "
                insert into TestCases (
                    last_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date,
                    name,
                    state,
                    description_hash,
                    utc_date,
                    duration_sec
                )
                values (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6,
                    $7,
                    $8,
                    $9
                )
                on conflict (
                    product_id,
                    test_run_name,
                    test_run_date,
                    name
                )
                do update set
                    last_collect_nr = excluded.last_collect_nr,
                    state = excluded.state,
                    description_hash = excluded.description_hash,
                    utc_date = excluded.utc_date,
                    duration_sec = excluded.duration_sec
                ",
                collect_nr,
                product_id,
                test_run.name,
                test_run.utc_date,
                test_case.name,
                state,
                description_hash,
                test_case.utc_date,
                duration
            )
            .execute(self.connection_mut())
            .await?;

            for verified_req in test_case.verified_reqs {
                sqlx::query!(
                    "
                    insert into TestCaseVerifiedRequirements (
                        last_collect_nr,
                        product_id,
                        test_run_name,
                        test_run_date,
                        test_case_name,
                        req_id
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4,
                        $5,
                        $6
                    )
                    on conflict (product_id, test_run_name, test_run_date, test_case_name, req_id)
                    do update set
                        last_collect_nr = excluded.last_collect_nr
                    ",
                    collect_nr,
                    product_id,
                    test_run.name,
                    test_run.utc_date,
                    test_case.name,
                    verified_req
                )
                .execute(self.connection_mut())
                .await?;
            }

            if let Some(properties) =
                merge_local_and_base_properties(test_case.properties, base_test_case_props)
            {
                for property in properties {
                    let value_hash = FmtHash::from(&property.1);
                    self.insert_general_json(&value_hash, property.1).await?;

                    sqlx::query!(
                        "
                        insert into TestCaseProperties (
                            last_collect_nr,
                            product_id,
                            test_run_name,
                            test_run_date,
                            test_case_name,
                            property_key,
                            value_hash
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
                        on conflict (product_id, test_run_name, test_run_date, test_case_name, property_key)
                        do update set
                            last_collect_nr = excluded.last_collect_nr,
                            value_hash = excluded.value_hash
                        ",
                        collect_nr,
                        product_id,
                        test_run.name,
                        test_run.utc_date,
                        test_case.name,
                        property.0,
                        value_hash
                    )
                    .execute(self.connection_mut())
                    .await?;
                }
            }

            if let Some(logs) = test_case.logs {
                // TODO: ensure that log srcs only appear once
                for log in logs {
                    let log_src = log.source.as_nr();
                    let log_hash = FmtHash::from(&log.content);
                    self.insert_general_text(&log_hash, log.content, None)
                        .await?;

                    sqlx::query!(
                        "
                            insert into TestCaseLogs (
                                last_collect_nr,
                                product_id,
                                test_run_name,
                                test_run_date,
                                test_case_name,
                                log_src,
                                log_hash
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
                            on conflict (
                                product_id,
                                test_run_name,
                                test_run_date,
                                test_case_name,
                                log_src
                            )
                            do update set
                                last_collect_nr = excluded.last_collect_nr,
                                log_hash = excluded.log_hash
                            ",
                        collect_nr,
                        product_id,
                        test_run.name,
                        test_run.utc_date,
                        test_case.name,
                        log_src,
                        log_hash
                    )
                    .execute(self.connection_mut())
                    .await?;
                }
            }

            if let Some(location) = test_case.location {
                let filepath = location.filepath.to_filepath();

                sqlx::query!(
                    "
                        insert into TestCaseLocations (
                            last_collect_nr,
                            product_id,
                            test_run_name,
                            test_run_date,
                            test_case_name,
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
                            $6,
                            $7,
                            $8
                        )
                        on conflict (
                            product_id,
                            test_run_name,
                            test_run_date,
                            test_case_name
                        )
                        do update set
                            last_collect_nr = excluded.last_collect_nr,
                            filepath = excluded.filepath,
                            file_hash = excluded.file_hash,
                            line = excluded.line
                        ",
                    collect_nr,
                    product_id,
                    test_run.name,
                    test_run.utc_date,
                    test_case.name,
                    filepath,
                    location.file_hash,
                    location.line
                )
                .execute(self.connection_mut())
                .await?;
            }

            if let Some(state_props) = test_case.state_properties {
                for property in state_props {
                    let value_hash = FmtHash::from(&property.1);
                    self.insert_general_json(&value_hash, property.1).await?;

                    sqlx::query!(
                        "
                        insert into TestCaseStateProperties (
                            last_collect_nr,
                            product_id,
                            test_run_name,
                            test_run_date,
                            test_case_name,
                            property_key,
                            value_hash
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
                        on conflict (product_id, test_run_name, test_run_date, test_case_name, property_key)
                        do update set
                            last_collect_nr = excluded.last_collect_nr,
                            value_hash = excluded.value_hash
                        ",
                        collect_nr,
                        product_id,
                        test_run.name,
                        test_run.utc_date,
                        test_case.name,
                        property.0,
                        value_hash
                    )
                    .execute(self.connection_mut())
                    .await?;
                }
            }

            for covered_file in test_case.covered_files {
                let filepath = covered_file.filepath.to_filepath();

                for line in covered_file.lines {
                    sqlx::query!(
                        "
                        insert into TestCaseLineCoverage (
                            last_collect_nr,
                            product_id,
                            test_run_name,
                            test_run_date,
                            test_case_name,
                            cov_filepath,
                            cov_file_hash,
                            cov_line,
                            hits
                        )
                        values (
                            $1,
                            $2,
                            $3,
                            $4,
                            $5,
                            $6,
                            $7,
                            $8,
                            $9
                        )
                        on conflict (
                            product_id,
                            test_run_name,
                            test_run_date,
                            test_case_name,
                            cov_filepath,
                            cov_line
                        )
                        do update set
                            last_collect_nr = excluded.last_collect_nr,
                            hits = excluded.hits
                        ",
                        collect_nr,
                        product_id,
                        test_run.name,
                        test_run.utc_date,
                        test_case.name,
                        filepath,
                        covered_file.file_hash,
                        line.nr,
                        line.hits
                    )
                    .execute(self.connection_mut())
                    .await?;
                }
            }
        }

        Ok(())
    }
}
