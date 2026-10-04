use anyhow::Context;
use mantra_schema::{
    FmtHash,
    requirements::ReqId,
    reviews::{OneOrMultRequirementIds, Review, ReviewSchema},
    time::OffsetDateTime,
};

use crate::cmd::collect::{product_collection::ProductCollection, reviews::db::DbIgnoredEntry};

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn update_per_review_schema(
        &mut self,
        review_schema: &ReviewSchema,
        schema_hash: &FmtHash,
    ) -> Result<(), anyhow::Error> {
        let schema_collected = self.schema_collected(schema_hash).await?;

        if !schema_collected && let Some(props) = &review_schema.properties {
            for (key, value) in props {
                let value_hash = FmtHash::from(&value);

                self.insert_general_json(&value_hash, &value)
                    .await
                    .with_context(|| {
                        format!("Inserting value for review schema property '{key}'")
                    })?;

                sqlx::query!(
                    "
                    insert into SchemaReviewProperties (
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
                .with_context(|| format!("Inserting review schema property '{key}'"))?;
            }
        }

        // TODO: do not stop at first collect error

        for review in &review_schema.reviews {
            let name = &review.name;

            self.update_review(review, schema_hash)
                .await
                .with_context(|| format!("Failed to update review '{}'", name))?;
        }

        Ok(())
    }

    async fn update_review(
        &mut self,
        review: &Review,
        schema_hash: &FmtHash,
    ) -> Result<(), anyhow::Error> {
        // TODO: optimize by checking src-hash first and skip if unchanged

        let collect_nr = self.collect_nr();
        let product_id = self.product_id().clone();

        if let Some(_record) = sqlx::query!(
            "
            select name, utc_date
            from Reviews
            where collect_nr = $1 and product_id = $2
            and name = $3 and utc_date = $4
            ",
            collect_nr,
            product_id,
            review.name,
            review.utc_date
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Failed to get collected reviews")?
        {
            anyhow::bail!("Duplicate review entry found in the same collection!");
        }

        let data_hash = FmtHash::from(&review);

        let origin_hash = review.origin.as_ref().map(FmtHash::from);
        if let Some(hash) = &origin_hash
            && let Some(origin) = &review.origin
        {
            self.insert_general_json(hash, origin)
                .await
                .context("Failed to insert the review origin")?;
        }
        let description_hash = review.description.as_ref().map(FmtHash::from);
        if let Some(hash) = &description_hash
            && let Some(description) = &review.description
        {
            self.insert_general_text(hash, description)
                .await
                .context("Failed to insert the review description")?;
        }

        sqlx::query!(
            "
            insert into Reviews (
                collect_nr,
                product_id,
                name,
                utc_date,
                origin_hash,
                description_hash,
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
                $9
            )
            ",
            collect_nr,
            product_id,
            review.name,
            review.utc_date,
            origin_hash,
            description_hash,
            data_hash,
            schema_hash,
            review.media_type
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update the review base data")?;

        for author in &review.authors {
            sqlx::query!(
                "
                insert into ReviewAuthors (
                    collect_nr,
                    product_id,
                    review_name,
                    review_date,
                    author
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
                review.name,
                review.utc_date,
                author
            )
            .execute(self.connection_mut())
            .await
            .with_context(|| format!("Failed to insert author '{}'", author))?;
        }

        if let Some(props) = &review.properties {
            for prop in props {
                let key = &prop.0;
                let value_hash = FmtHash::from(&prop.1);
                self.insert_general_json(&value_hash, prop.1)
                    .await
                    .with_context(|| format!("Failed to insert content for property '{}'", key))?;

                sqlx::query!(
                    "
                    insert into ReviewProperties (
                        collect_nr,
                        product_id,
                        review_name,
                        review_date,
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
                    review.name,
                    review.utc_date,
                    key,
                    value_hash
                )
                .execute(self.connection_mut())
                .await
                .with_context(|| format!("Failed to insert property '{}'", key))?;
            }
        }

        if let Some(revisions) = &review.revisions {
            for revision in revisions {
                sqlx::query!(
                    "
                    insert into ReviewRevisions (
                        collect_nr,
                        product_id,
                        review_name,
                        review_date,
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
                    review.name,
                    review.utc_date,
                    revision.nr,
                    revision.comment
                )
                .execute(self.connection_mut())
                .await
                .with_context(|| format!("Failed to insert revision '{}'", revision.nr))?;

                for author in &revision.authors {
                    sqlx::query!(
                        "
                        insert into ReviewRevisionAuthors (
                            collect_nr,
                            product_id,
                            review_name,
                            review_date,
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
                        review.name,
                        review.utc_date,
                        revision.nr,
                        author
                    )
                    .execute(self.connection_mut())
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to insert revision author '{}' for revision '{}'",
                            author, revision.nr
                        )
                    })?;
                }
            }
        }

        for verified_req in &review.requirements {
            let comment_hash = FmtHash::from(&verified_req.comment);
            self.insert_general_text(&comment_hash, &verified_req.comment)
                .await
                .with_context(|| {
                    format!(
                        "Failed to insert verification comment for requirement '{}'",
                        verified_req.id
                    )
                })?;

            match &verified_req.id {
                OneOrMultRequirementIds::One(id) => {
                    self.update_verified_req(&review.name, &review.utc_date, id, &comment_hash)
                        .await
                        .with_context(|| {
                            format!(
                                "Failed to insert review verification for requirement '{}'",
                                id,
                            )
                        })?;
                }
                OneOrMultRequirementIds::Mult(ids) => {
                    for id in ids {
                        self.update_verified_req(&review.name, &review.utc_date, id, &comment_hash)
                            .await
                            .with_context(|| {
                                format!(
                                    "Failed to insert review verification for requirement '{}'",
                                    id,
                                )
                            })?;
                    }
                }
            }
        }

        for test_run_override in &review.test_run_overrides {
            for test_case_override in &test_run_override.test_cases {
                if let Some(override_state) = &test_case_override.state {
                    let comment_hash = FmtHash::from(&override_state.comment);
                    self.insert_general_text(&comment_hash, &override_state.comment)
                    .await
                    .with_context(||
                        format!(
                            "Failed to insert state override comment for test case '{}' from test run '{}'",
                            test_case_override.name,
                            test_run_override.name
                        )
                    )?;
                    let state = override_state.new.as_nr();

                    let test_case_exists = sqlx::query!(
                        "
                        select * from TestCases
                        where
                            collect_nr = $1
                            and product_id = $2
                            and test_run_name = $3
                            and test_run_date = $4
                            and name = $5
                        ",
                        collect_nr,
                        product_id,
                        test_run_override.name,
                        test_run_override.utc_date,
                        test_case_override.name
                    )
                    .fetch_optional(self.connection_mut())
                    .await
                    .context("Failed to get collected test cases")?
                    .is_some();

                    if test_case_exists {
                        sqlx::query!(
                        "
                        insert into TestCaseOverrides (
                            collect_nr,
                            product_id,
                            test_run_name,
                            test_run_date,
                            test_case_name,
                            review_name,
                            review_date,
                            state,
                            comment_hash
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
                        test_run_override.name,
                        test_run_override.utc_date,
                        test_case_override.name,
                        review.name,
                        review.utc_date,
                        state,
                        comment_hash
                    )
                    .execute(self.connection_mut())
                    .await
                    .with_context(||
                        format!(
                            "Failed to insert test case state override for test case '{}' from test run '{}'",
                            test_case_override.name,
                            test_run_override.name
                        )
                    )?;
                    } else {
                        log::warn!(
                            "Ignoring state override for unknown test case '{}' from test run '{}'",
                            test_case_override.name,
                            test_run_override.name
                        );

                        let entry = DbIgnoredEntry::from_test_case_state(
                            test_run_override.name.clone(),
                            test_run_override.utc_date,
                            test_case_override.name.clone(),
                            override_state.new,
                            comment_hash,
                        );

                        self.insert_ignored_entry(&review.name, &review.utc_date, entry)
                        .await
                        .with_context(||
                            format!(
                                "Failed to insert ignore entry for state override of unknown test case '{}' from test run '{}'",
                                test_case_override.name,
                                test_run_override.name
                            )
                        )?;
                    }
                }

                for coverage_override in &test_case_override.coverage {
                    let filepath = coverage_override.filepath.as_str();

                    for line_info in &coverage_override.lines {
                        let comment_hash = FmtHash::from(&line_info.comment);
                        self.insert_general_text(&comment_hash, &line_info.comment)
                        .await
                        .with_context(||
                            format!(
                                "Failed to insert line coverage override comment for lines '{}' in file '{}' for test case '{}' from test run '{}'",
                                line_info.nrs.iter().map(|l| l.to_string()).collect::<Vec<String>>().join(","),
                                coverage_override.filepath,
                                test_case_override.name,
                                test_run_override.name
                            )
                        )?;

                        for line_nr in &line_info.nrs {
                            let covered_line_exists = sqlx::query!(
                                "
                                select * from TestCaseLineCoverage
                                where
                                    collect_nr = $1 and product_id = $2
                                    and test_run_name = $3
                                    and test_run_date = $4 and test_case_name = $5
                                    and cov_filepath = $6 and cov_line = $7
                                ",
                                collect_nr,
                                product_id,
                                test_run_override.name,
                                test_run_override.utc_date,
                                test_case_override.name,
                                filepath,
                                line_nr
                            )
                            .fetch_optional(self.connection_mut())
                            .await
                            .context("Failed to get collected test case line coverage")?
                            .is_some();

                            if covered_line_exists {
                                sqlx::query!(
                                "
                                insert into TestCaseLineCoverageOverrides (
                                    collect_nr,
                                    product_id,
                                    test_run_name,
                                    test_run_date,
                                    test_case_name,
                                    review_name,
                                    review_date,
                                    cov_filepath,
                                    cov_line,
                                    hits,
                                    comment_hash
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
                                test_run_override.name,
                                test_run_override.utc_date,
                                test_case_override.name,
                                review.name,
                                review.utc_date,
                                filepath,
                                line_nr,
                                line_info.hits,
                                comment_hash
                            )
                            .execute(self.connection_mut())
                            .await
                            .with_context(||
                                format!(
                                    "Failed to insert line coverage override for line '{}' in file '{}' for test case '{}' from test run '{}'",
                                    line_nr,
                                    coverage_override.filepath,
                                    test_case_override.name,
                                    test_run_override.name
                                )
                            )?;
                            } else {
                                log::warn!(
                                    "Ignoring line coverage override for unknown line '{}' in file '{}' for test case '{}' from test run '{}'",
                                    line_nr,
                                    coverage_override.filepath,
                                    test_case_override.name,
                                    test_run_override.name
                                );

                                let entry = DbIgnoredEntry::from_test_case_line_coverage(
                                    test_run_override.name.clone(),
                                    test_run_override.utc_date,
                                    test_case_override.name.clone(),
                                    coverage_override.filepath.clone(),
                                    *line_nr,
                                    line_info.hits,
                                    comment_hash.clone(),
                                );

                                self.insert_ignored_entry(&review.name, &review.utc_date, entry)
                                .await
                                .with_context(||
                                    format!(
                                        "Failed to insert ignore entry for line coverage override for line '{}' in file '{}' for test case '{}' from test run '{}'",
                                        line_nr,
                                        coverage_override.filepath,
                                        test_case_override.name,
                                        test_run_override.name
                                    )
                                )?;
                            }
                        }
                    }
                }
            }

            for coverage_override in &test_run_override.coverage {
                let filepath = coverage_override.filepath.as_str();

                for line_info in &coverage_override.lines {
                    let comment_hash = FmtHash::from(&line_info.comment);
                    self.insert_general_text(&comment_hash, &line_info.comment)
                    .await
                    .with_context(||
                        format!(
                            "Failed to insert line coverage override comment for lines '{}' in file '{}' for test run '{}'",
                            line_info.nrs.iter().map(|l| l.to_string()).collect::<Vec<String>>().join(","),
                            coverage_override.filepath,
                            test_run_override.name
                        )
                    )?;

                    for line_nr in &line_info.nrs {
                        let covered_line_exists = sqlx::query!(
                            "
                            select * from TestRunLineCoverage
                            where
                                collect_nr = $1 and product_id = $2
                                and test_run_name = $3
                                and test_run_date = $4 and cov_filepath = $5
                                and cov_line = $6
                            ",
                            collect_nr,
                            product_id,
                            test_run_override.name,
                            test_run_override.utc_date,
                            filepath,
                            line_nr
                        )
                        .fetch_optional(self.connection_mut())
                        .await
                        .context("Failed to get collected test run line coverage")?
                        .is_some();

                        if covered_line_exists {
                            sqlx::query!(
                            "
                            insert into TestRunLineCoverageOverrides (
                                collect_nr,
                                product_id,
                                test_run_name,
                                test_run_date,
                                review_name,
                                review_date,
                                cov_filepath,
                                cov_line,
                                hits,
                                comment_hash
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
                            ",
                            collect_nr,
                            product_id,
                            test_run_override.name,
                            test_run_override.utc_date,
                            review.name,
                            review.utc_date,
                            filepath,
                            line_nr,
                            line_info.hits,
                            comment_hash
                        )
                        .execute(self.connection_mut())
                        .await
                        .with_context(||
                            format!(
                                "Failed to insert line coverage override for line '{}' in file '{}' for test run '{}'",
                                line_nr,
                                coverage_override.filepath,
                                test_run_override.name
                            )
                        )?;
                        } else {
                            log::warn!(
                                "Ignoring line coverage override for unknown line '{}' in file '{}' for test run '{}'",
                                line_nr,
                                coverage_override.filepath,
                                test_run_override.name
                            );

                            let entry = DbIgnoredEntry::from_test_run_line_coverage(
                                test_run_override.name.clone(),
                                test_run_override.utc_date,
                                coverage_override.filepath.clone(),
                                *line_nr,
                                line_info.hits,
                                comment_hash.clone(),
                            );

                            self.insert_ignored_entry(&review.name, &review.utc_date, entry)
                            .await
                            .with_context(||
                                format!(
                                    "Failed to insert ignore entry for line coverage override for line '{}' in file '{}' for test run '{}'",
                                    line_nr,
                                    coverage_override.filepath,
                                    test_run_override.name
                                )
                            )?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn update_verified_req(
        &mut self,
        review_name: &str,
        review_date: &OffsetDateTime,
        req_id: &ReqId,
        comment_hash: &FmtHash,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id().clone();

        let req_available = sqlx::query!(
            "
            select id from Requirements
            where collect_nr = $1 and product_id = $2 and id = $3
            ",
            collect_nr,
            product_id,
            req_id
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Failed to get collected requirements")?
        .is_some();

        if req_available {
            sqlx::query!(
                "
            insert into ManuallyVerifiedRequirements (
                collect_nr,
                req_id,
                product_id,
                review_name,
                review_date,
                comment_hash
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
                req_id,
                product_id,
                review_name,
                review_date,
                comment_hash
            )
            .execute(self.connection_mut())
            .await?;
        } else {
            log::warn!(
                "Ignoring manual verification for unknown requirement '{}'",
                req_id
            );

            let ignored_entry =
                DbIgnoredEntry::from_verified_req(req_id.clone(), comment_hash.clone());

            self.insert_ignored_entry(review_name, review_date, ignored_entry)
                .await
                .context(
                    "Failed to insert ignore entry for manual verification of unknown requirement",
                )?;
        }

        Ok(())
    }

    async fn insert_ignored_entry(
        &mut self,
        review_name: &str,
        review_date: &OffsetDateTime,
        entry: DbIgnoredEntry,
    ) -> Result<(), anyhow::Error> {
        let entry_hash = FmtHash::from(&entry);

        self.insert_general_json(&entry_hash, &serde_json::to_value(&entry)?)
            .await
            .context("Failed to insert ignore content")?;

        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert into IgnoredReviewEntries (
                collect_nr,
                product_id,
                review_name,
                review_date,
                entry_hash
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
            review_name,
            review_date,
            entry_hash
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
