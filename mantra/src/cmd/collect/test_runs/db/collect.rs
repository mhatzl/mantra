use anyhow::Context;
use mantra_schema::{
    FmtHash,
    media_type::MediaType,
    test_runs::{TestRun, TestRunSchema},
};

use crate::cmd::collect::product_collection::ProductCollection;

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn collect_test_run_schema(
        &mut self,
        test_run_schema: &TestRunSchema,
        schema_hash: &FmtHash,
    ) -> Result<(), anyhow::Error> {
        let schema_collected = self.schema_collected(schema_hash).await?;

        if !schema_collected {
            if let Some(props) = &test_run_schema.test_run_properties {
                for (key, value) in props {
                    let value_hash = FmtHash::from(&value);

                    self.insert_general_json(&value_hash, &value)
                        .await
                        .with_context(|| {
                            format!("Inserting value for test run schema property '{key}'")
                        })?;

                    sqlx::query!(
                        "
                        insert into SchemaTestRunProperties (
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
                    .with_context(|| format!("Inserting test run schema property '{key}'"))?;
                }
            }

            if let Some(props) = &test_run_schema.test_case_properties {
                for (key, value) in props {
                    let value_hash = FmtHash::from(&value);

                    self.insert_general_json(&value_hash, &value)
                        .await
                        .with_context(|| {
                            format!("Inserting value for test case schema property '{key}'")
                        })?;

                    sqlx::query!(
                        "
                        insert into SchemaTestCaseProperties (
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
                    .with_context(|| format!("Inserting test case schema property '{key}'"))?;
                }
            }
        }

        // TODO: do not stop at first collect error

        for test_run in &test_run_schema.test_runs {
            let name = &test_run.name;

            // Note: Passing the media type of the top-level test run,
            // so nested ones can use it as fallback
            self.update_per_test_run(test_run, schema_hash, test_run.media_type.as_ref())
                .await
                .with_context(|| format!("Failed to update test run '{}'", name))?;
        }

        Ok(())
    }

    async fn update_per_test_run(
        &mut self,
        test_run: &TestRun,
        schema_hash: &FmtHash,
        parent_media_type: Option<&MediaType>,
    ) -> Result<(), anyhow::Error> {
        // TODO: optimize by checking data-hash first and skip if unchanged

        let collect_nr = self.collect_nr();
        let product_id = self.product_id().clone();

        let collected_test_run_hash = sqlx::query!(
            "
            select data_hash
            from TestRuns
            where collect_nr = $1 and product_id = $2
            and name = $3 and utc_date = $4
            ",
            collect_nr,
            product_id,
            test_run.name,
            test_run.utc_date,
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Querying collected test runs")?
        .map(|r| r.data_hash);

        let data_hash = FmtHash::from(&serde_json::to_value(&test_run)?);

        if let Some(collected_data) = collected_test_run_hash {
            if FmtHash::with_inner(collected_data) != data_hash {
                anyhow::bail!("Duplicate test run entry in same collection!");
            } else {
                log::info!("Skipping already collected test run '{}'", test_run.name);
                return Ok(());
            }
        }

        let origin_hash = test_run.origin.as_ref().map(FmtHash::from);
        if let Some(hash) = &origin_hash
            && let Some(origin) = &test_run.origin
        {
            self.insert_general_json(hash, origin)
                .await
                .context("Failed to insert the test run origin")?;
        }

        let description_hash = test_run.description.as_ref().map(FmtHash::from);
        if let Some(hash) = &description_hash
            && let Some(description) = &test_run.description
        {
            self.insert_general_text(hash, description)
                .await
                .context("Failed to insert the test run description")?;
        }

        let duration = test_run.duration_sec.map(|d| d.as_seconds_f64());
        let media_type = test_run.media_type.as_ref().or(parent_media_type);

        sqlx::query!(
            "
            insert into TestRuns (
                collect_nr,
                product_id,
                name,
                utc_date,
                description_hash,
                duration_sec,
                nr_of_test_cases,
                origin_hash,
                data_hash,
                schema_hash,
                media_type
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
                $10,
                $11
            )
            ",
            collect_nr,
            product_id,
            test_run.name,
            test_run.utc_date,
            description_hash,
            duration,
            test_run.nr_of_test_cases,
            origin_hash,
            data_hash,
            schema_hash,
            media_type
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to insert data into TestRuns")?;

        if let Some(props) = &test_run.properties {
            for prop in props {
                let value_hash = FmtHash::from(&prop.1);
                self.insert_general_json(&value_hash, prop.1).await?;

                sqlx::query!(
                    "
                    insert into TestRunProperties (
                        collect_nr,
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
                    ",
                    collect_nr,
                    product_id,
                    test_run.name,
                    test_run.utc_date,
                    prop.0,
                    value_hash
                )
                .execute(self.connection_mut())
                .await
                .context("Failed to insert data into TestRunProperties")?;
            }
        }

        if let Some(revisions) = &test_run.revisions {
            for revision in revisions {
                sqlx::query!(
                    "
                    insert into TestRunRevisions (
                        collect_nr,
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
                    ",
                    collect_nr,
                    product_id,
                    test_run.name,
                    test_run.utc_date,
                    revision.nr,
                    revision.comment
                )
                .execute(self.connection_mut())
                .await
                .context("Failed to insert data into TestRunRevisions")?;

                for author in &revision.authors {
                    sqlx::query!(
                        "
                        insert into TestRunRevisionAuthors (
                            collect_nr,
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
                        ",
                        collect_nr,
                        product_id,
                        test_run.name,
                        test_run.utc_date,
                        revision.nr,
                        author
                    )
                    .execute(self.connection_mut())
                    .await
                    .context("Failed to insert data into TestRunRevisionAuthors")?;
                }
            }
        }

        for child_test_run in &test_run.test_runs {
            sqlx::query!(
                "
                insert into TestRunHierarchies (
                    collect_nr,
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
                ",
                collect_nr,
                product_id,
                test_run.name,
                test_run.utc_date,
                child_test_run.name,
                child_test_run.utc_date
            )
            .execute(self.connection_mut())
            .await
            .context("Failed to insert data into TestRunHierarchies")?;

            // foreign key constraints are deferred for test run hierarchies
            // => safe to add child test run after hierarchy inside the same transaction
            let name = &child_test_run.name;

            Box::pin(self.update_per_test_run(child_test_run, schema_hash, media_type))
                .await
                .with_context(|| format!("Failed to update child test run '{}'", name))?;
        }

        if let Some(logs) = &test_run.logs {
            // TODO: ensure that log srcs only appear once
            for log in logs {
                let log_src = log.source.as_nr();
                let log_hash = FmtHash::from(&log.content);

                self.insert_general_text(&log_hash, &log.content)
                    .await
                    .context("Failed to insert the log content")?;

                sqlx::query!(
                    "
                    insert into TestRunLogs (
                        collect_nr,
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
                    ",
                    collect_nr,
                    product_id,
                    test_run.name,
                    test_run.utc_date,
                    log_src,
                    log_hash
                )
                .execute(self.connection_mut())
                .await
                .with_context(|| format!("Failed to insert logs from source '{}'", log.source))?;
            }
        }

        for covered_file in &test_run.covered_files {
            let filepath = covered_file.filepath.as_str();

            for line in &covered_file.lines {
                sqlx::query!(
                    "
                    insert into TestRunLineCoverage (
                        collect_nr,
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
                .await
                .with_context(|| {
                    format!(
                        "Failed to update test run line coverage for line '{}' in file '{}'",
                        line.nr, filepath
                    )
                })?;
            }
        }

        for test_case in &test_run.test_cases {
            let state = test_case.state.as_nr();
            let description_hash = test_case.description.as_ref().map(FmtHash::from);
            if let Some(hash) = &description_hash
                && let Some(description) = &test_case.description
            {
                self.insert_general_text(hash, description)
                    .await
                    .context("Failed to insert the test case description")?;
            }
            let duration = test_case.duration_sec.map(|d| d.as_seconds_f64());

            sqlx::query!(
                "
                insert into TestCases (
                    collect_nr,
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
            .await
            .with_context(|| {
                format!(
                    "Failed to insert base data for test case '{}'",
                    test_case.name
                )
            })?;

            for verified_req in &test_case.verified_reqs {
                sqlx::query!(
                    "
                    insert into TestCaseVerifiedRequirements (
                        collect_nr,
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
                    ",
                    collect_nr,
                    product_id,
                    test_run.name,
                    test_run.utc_date,
                    test_case.name,
                    verified_req
                )
                .execute(self.connection_mut())
                .await
                .with_context(|| {
                    format!(
                        "Failed to insert req '{}' for test case '{}'",
                        verified_req, test_case.name
                    )
                })?;
            }

            if let Some(properties) = &test_case.properties {
                for property in properties {
                    let key = &property.0;
                    let value_hash = FmtHash::from(&property.1);

                    self.insert_general_json(&value_hash, property.1)
                        .await
                        .with_context(|| {
                            format!(
                                "Failed to insert content for property '{}' of test case '{}'",
                                key, test_case.name
                            )
                        })?;

                    sqlx::query!(
                        "
                        insert into TestCaseProperties (
                            collect_nr,
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
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to update property '{}' of test case '{}'",
                            key, test_case.name
                        )
                    })?;
                }
            }

            if let Some(logs) = &test_case.logs {
                // TODO: ensure that log srcs only appear once
                for log in logs {
                    let log_src = log.source.as_nr();
                    let log_hash = FmtHash::from(&log.content);
                    self.insert_general_text(&log_hash, &log.content)
                        .await
                        .with_context(|| {
                            format!(
                                "Failed to insert content for '{}' log of test case '{}'",
                                log.source, test_case.name
                            )
                        })?;

                    sqlx::query!(
                        "
                        insert into TestCaseLogs (
                            collect_nr,
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
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to update '{}' log of test case '{}'",
                            log.source, test_case.name
                        )
                    })?;
                }
            }

            if let Some(location) = &test_case.location {
                let filepath = location.filepath.as_str();

                sqlx::query!(
                    "
                    insert into TestCaseLocations (
                        collect_nr,
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
                .await
                .with_context(|| {
                    format!(
                        "Failed to update the location of test case '{}'",
                        test_case.name
                    )
                })?;
            }

            if let Some(state_props) = &test_case.state_properties {
                for property in state_props {
                    let key = &property.0;
                    let value_hash = FmtHash::from(&property.1);

                    self.insert_general_json(&value_hash, property.1)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to insert content for the state property '{}' of test case '{}'",
                            key, test_case.name
                        )
                    })?;

                    sqlx::query!(
                        "
                        insert into TestCaseStateProperties (
                            collect_nr,
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
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to update the state property '{}' of test case '{}'",
                            key, test_case.name
                        )
                    })?;
                }
            }

            for covered_file in &test_case.covered_files {
                let filepath = covered_file.filepath.as_str();

                for line in &covered_file.lines {
                    sqlx::query!(
                    "
                    insert into TestCaseLineCoverage (
                        collect_nr,
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
                .await
                .with_context(|| {
                    format!(
                        "Failed to update coverage for line '{}' in file '{}' for test case '{}'",
                        line.nr, filepath, test_case.name
                    )
                })?;
                }
            }
        }

        Ok(())
    }
}
