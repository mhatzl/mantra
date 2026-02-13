use mantra_schema::FmtHash;
use mantra_schema::Line;
use mantra_schema::Properties;
use mantra_schema::path::RelativePathBuf;
use mantra_schema::requirements::ReqId;
use mantra_schema::reviews::OneOrMultRequirementIds;
use mantra_schema::reviews::Review;
use mantra_schema::reviews::ReviewSchema;
use mantra_schema::test_runs::TestCaseState;
use mantra_schema::time::OffsetDateTime;

use crate::cmd::collect::Collection;
use crate::cmd::collect::merge_local_and_base_properties;
use crate::db::FilepathExt;

impl<'db> Collection<'db> {
    pub(super) async fn update_reviews(
        &mut self,
        review_schemas: Vec<ReviewSchema>,
    ) -> Result<(), anyhow::Error> {
        for reviews in review_schemas {
            self.update_per_review_schema(reviews).await?;
        }

        Ok(())
    }

    pub(super) async fn update_per_review_schema(
        &mut self,
        review_schema: ReviewSchema,
    ) -> Result<(), anyhow::Error> {
        let base_origin_hash = review_schema.origin.as_ref().map(FmtHash::from);

        if let Some(hash) = &base_origin_hash
            && let Some(origin) = review_schema.origin
        {
            self.insert_general_json(&hash, origin.clone()).await?;
        }

        // TODO: do not stop at first collect error

        for review in review_schema.reviews {
            self.update_review(review, &base_origin_hash, &review_schema.properties)
                .await?;
        }

        todo!()
    }

    async fn update_review(
        &mut self,
        review: Review,
        base_origin_hash: &Option<FmtHash>,
        base_props: &Option<Properties>,
    ) -> Result<(), anyhow::Error> {
        // TODO: optimize by checking src-hash first and skip if unchanged

        let collect_nr = self.collect_nr();
        let product_id = &self.product_id();
        let src_hash = FmtHash::from(&serde_json::json!({
            "base_origin_hash": base_origin_hash,
            "base_props": base_props,
            "review": &review
        }));

        let origin_hash = review.origin.as_ref().map(FmtHash::from);
        if let Some(hash) = &origin_hash
            && let Some(origin) = review.origin
        {
            self.insert_general_json(&hash, origin).await?;
        }
        let description_hash = review.description.as_ref().map(FmtHash::from);
        if let Some(hash) = &description_hash
            && let Some(description) = review.description
        {
            self.insert_general_text(hash, description).await?;
        }

        sqlx::query!(
            "
            insert into Reviews (
                last_collect_nr,
                product_id,
                name,
                utc_date,
                base_origin_hash,
                origin_hash,
                description_hash,
                src_hash
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
            on conflict (product_id, name, utc_date)
            do update set
                last_collect_nr = excluded.last_collect_nr,
                base_origin_hash = excluded.base_origin_hash,
                origin_hash = excluded.origin_hash,
                description_hash = excluded.description_hash,
                src_hash = excluded.src_hash
            ",
            collect_nr,
            product_id,
            review.name,
            review.utc_date,
            base_origin_hash,
            origin_hash,
            description_hash,
            src_hash
        )
        .execute(self.connection_mut())
        .await?;

        for reviewer in review.reviewer {
            sqlx::query!(
                "
                insert into ReviewReviewers (
                    last_collect_nr,
                    product_id,
                    review_name,
                    review_date,
                    reviewer
                )
                values (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5
                )
                on conflict (product_id, review_name, review_date, reviewer)
                do update set
                    last_collect_nr = excluded.last_collect_nr
                ",
                collect_nr,
                product_id,
                review.name,
                review.utc_date,
                reviewer
            )
            .execute(self.connection_mut())
            .await?;
        }

        if let Some(props) = merge_local_and_base_properties(review.properties, base_props) {
            for prop in props {
                let value_hash = FmtHash::from(&prop.1);
                self.insert_general_json(&value_hash, prop.1).await?;

                sqlx::query!(
                    "
                    insert into ReviewProperties (
                        last_collect_nr,
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
                    on conflict (product_id, review_name, review_date, property_key)
                    do update set
                        last_collect_nr = excluded.last_collect_nr,
                        value_hash = excluded.value_hash
                    ",
                    collect_nr,
                    product_id,
                    review.name,
                    review.utc_date,
                    prop.0,
                    value_hash
                )
                .execute(self.connection_mut())
                .await?;
            }
        }

        if let Some(revisions) = review.revisions {
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
                    review.name,
                    review.utc_date,
                    revision.nr,
                    revision.comment
                )
                .execute(self.connection_mut())
                .await?;

                for author in revision.authors {
                    sqlx::query!(
                        "
                        insert into ReviewRevisionAuthors (
                            last_collect_nr,
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
                        on conflict (product_id, review_name, review_date, revision, author)
                        do update set
                            last_collect_nr = excluded.last_collect_nr
                        ",
                        collect_nr,
                        product_id,
                        review.name,
                        review.utc_date,
                        revision.nr,
                        author
                    )
                    .execute(self.connection_mut())
                    .await?;
                }
            }
        }

        for verified_req in review.requirements {
            let comment_hash = FmtHash::from(&verified_req.comment);
            self.insert_general_text(&comment_hash, verified_req.comment)
                .await?;

            match verified_req.id {
                OneOrMultRequirementIds::One(id) => {
                    self.update_verified_req(&review.name, &review.utc_date, &id, &comment_hash)
                        .await?;
                }
                OneOrMultRequirementIds::Mult(ids) => {
                    for id in ids {
                        self.update_verified_req(
                            &review.name,
                            &review.utc_date,
                            &id,
                            &comment_hash,
                        )
                        .await?;
                    }
                }
            }
        }

        for test_run_override in review.overrides {
            for test_case_override in test_run_override.test_cases {
                if let Some(override_state) = test_case_override.state {
                    let comment_hash = FmtHash::from(&override_state.comment);
                    self.insert_general_text(&comment_hash, override_state.comment)
                        .await?;
                    let state = override_state.new.as_nr();

                    let test_case_exists = sqlx::query!(
                        "
                        select * from TestCases
                        where
                            product_id = $1 and test_run_name = $2
                            and test_run_date = $3 and name = $4
                        ",
                        product_id,
                        test_run_override.test_run.name,
                        test_run_override.test_run.utc_date,
                        test_case_override.name
                    )
                    .fetch_optional(self.connection_mut())
                    .await?
                    .is_some();

                    if test_case_exists {
                        sqlx::query!(
                            "
                            insert into TestCaseOverrides (
                                last_collect_nr,
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
                            on conflict (
                                product_id,
                                test_run_name,
                                test_run_date,
                                test_case_name,
                                review_name,
                                review_date
                            )
                            do update set
                                last_collect_nr = excluded.last_collect_nr,
                                state = excluded.state,
                                comment_hash = excluded.comment_hash
                            ",
                            collect_nr,
                            product_id,
                            test_run_override.test_run.name,
                            test_run_override.test_run.utc_date,
                            test_case_override.name,
                            review.name,
                            review.utc_date,
                            state,
                            comment_hash
                        )
                        .execute(self.connection_mut())
                        .await?;
                    } else {
                        let entry = IgnoredEntry::from_test_case_state(
                            test_run_override.test_run.name.clone(),
                            test_run_override.test_run.utc_date.clone(),
                            test_case_override.name.clone(),
                            override_state.new,
                            comment_hash,
                        );
                        self.insert_ignored_entry(&review.name, &review.utc_date, entry)
                            .await?;
                    }
                }

                for coverage_override in test_case_override.coverage {
                    let filepath = coverage_override.filepath.clone().to_filepath();

                    for line_info in coverage_override.lines {
                        let comment_hash = FmtHash::from(&line_info.comment);
                        self.insert_general_text(&comment_hash, line_info.comment)
                            .await?;

                        for line_nr in line_info.nrs {
                            let statement_exists = sqlx::query!(
                                "
                                select * from TestCaseStatementCoverage
                                where
                                    product_id = $1 and test_run_name = $2
                                    and test_run_date = $3 and test_case_name = $4
                                    and stmnt_filepath = $5 and stmnt_line = $6
                                ",
                                product_id,
                                test_run_override.test_run.name,
                                test_run_override.test_run.utc_date,
                                test_case_override.name,
                                filepath,
                                line_nr
                            )
                            .fetch_optional(self.connection_mut())
                            .await?
                            .is_some();

                            if statement_exists {
                                sqlx::query!(
                                    "
                                    insert into TestCaseStatementCoverageOverrides (
                                        last_collect_nr,
                                        product_id,
                                        test_run_name,
                                        test_run_date,
                                        test_case_name,
                                        review_name,
                                        review_date,
                                        stmnt_filepath,
                                        stmnt_line,
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
                                    on conflict (
                                        product_id,
                                        test_run_name,
                                        test_run_date,
                                        test_case_name,
                                        review_name,
                                        review_date,
                                        stmnt_filepath,
                                        stmnt_line
                                    )
                                    do update set
                                        last_collect_nr = excluded.last_collect_nr,
                                        hits = excluded.hits,
                                        comment_hash = excluded.comment_hash
                                    ",
                                    collect_nr,
                                    product_id,
                                    test_run_override.test_run.name,
                                    test_run_override.test_run.utc_date,
                                    test_case_override.name,
                                    review.name,
                                    review.utc_date,
                                    filepath,
                                    line_nr,
                                    line_info.hits,
                                    comment_hash
                                )
                                .execute(self.connection_mut())
                                .await?;
                            } else {
                                let entry = IgnoredEntry::from_test_case_statement_coverage(
                                    test_run_override.test_run.name.clone(),
                                    test_run_override.test_run.utc_date.clone(),
                                    test_case_override.name.clone(),
                                    coverage_override.filepath.clone(),
                                    line_nr,
                                    line_info.hits,
                                    comment_hash.clone(),
                                );
                                self.insert_ignored_entry(&review.name, &review.utc_date, entry)
                                    .await?;
                            }
                        }
                    }
                }
            }

            for coverage_override in test_run_override.coverage {
                let filepath = coverage_override.filepath.clone().to_filepath();

                for line_info in coverage_override.lines {
                    let comment_hash = FmtHash::from(&line_info.comment);
                    self.insert_general_text(&comment_hash, line_info.comment)
                        .await?;

                    for line_nr in line_info.nrs {
                        let statement_exists = sqlx::query!(
                            "
                            select * from TestRunStatementCoverage
                            where
                                product_id = $1 and test_run_name = $2
                                and test_run_date = $3 and stmnt_filepath = $4
                                and stmnt_line = $5
                            ",
                            product_id,
                            test_run_override.test_run.name,
                            test_run_override.test_run.utc_date,
                            filepath,
                            line_nr
                        )
                        .fetch_optional(self.connection_mut())
                        .await?
                        .is_some();

                        if statement_exists {
                            sqlx::query!(
                                "
                                insert into TestRunStatementCoverageOverrides (
                                    last_collect_nr,
                                    product_id,
                                    test_run_name,
                                    test_run_date,
                                    review_name,
                                    review_date,
                                    stmnt_filepath,
                                    stmnt_line,
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
                                on conflict (
                                    product_id,
                                    test_run_name,
                                    test_run_date,
                                    review_name,
                                    review_date,
                                    stmnt_filepath,
                                    stmnt_line
                                )
                                do update set
                                    last_collect_nr = excluded.last_collect_nr,
                                    hits = excluded.hits,
                                    comment_hash = excluded.comment_hash
                                ",
                                collect_nr,
                                product_id,
                                test_run_override.test_run.name,
                                test_run_override.test_run.utc_date,
                                review.name,
                                review.utc_date,
                                filepath,
                                line_nr,
                                line_info.hits,
                                comment_hash
                            )
                            .execute(self.connection_mut())
                            .await?;
                        } else {
                            let entry = IgnoredEntry::from_test_run_statement_coverage(
                                test_run_override.test_run.name.clone(),
                                test_run_override.test_run.utc_date.clone(),
                                coverage_override.filepath.clone(),
                                line_nr,
                                line_info.hits,
                                comment_hash.clone(),
                            );
                            self.insert_ignored_entry(&review.name, &review.utc_date, entry)
                                .await?;
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
        let product_id = &self.product_id();

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
                insert into ManuallyVerifiedRequirements (
                    last_collect_nr,
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
                on conflict (product_id, req_id, review_name, review_date)
                do update set
                    last_collect_nr = excluded.last_collect_nr,
                    comment_hash = excluded.comment_hash
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
            let ignored_entry =
                IgnoredEntry::from_verified_req(req_id.clone(), comment_hash.clone());
            self.insert_ignored_entry(review_name, review_date, ignored_entry)
                .await?;
        }

        Ok(())
    }

    async fn insert_ignored_entry(
        &mut self,
        review_name: &str,
        review_date: &OffsetDateTime,
        entry: IgnoredEntry,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = &self.product_id();
        let entry_hash = FmtHash::from(&entry);
        self.insert_general_json(&entry_hash, serde_json::json!(entry))
            .await?;

        sqlx::query!(
            "
                insert into IgnoredReviewEntries (
                    last_collect_nr,
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
                on conflict (product_id, review_name, review_date, entry_hash)
                do update set
                    last_collect_nr = excluded.last_collect_nr
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum IgnoredEntry {
    Requirement {
        id: ReqId,
        comment_hash: FmtHash,
    },
    TestCaseStateOverride {
        test_run_name: String,
        test_run_utc_date: OffsetDateTime,
        test_case_name: String,
        state: TestCaseState,
        comment_hash: FmtHash,
    },
    TestCaseStatementCoverageOverride {
        test_run_name: String,
        test_run_utc_date: OffsetDateTime,
        test_case_name: String,
        stmnt_filepath: RelativePathBuf,
        stmnt_line: Line,
        hits: Option<i64>,
        comment_hash: FmtHash,
    },
    TestRunStatementCoverageOverride {
        test_run_name: String,
        test_run_utc_date: OffsetDateTime,
        stmnt_filepath: RelativePathBuf,
        stmnt_line: Line,
        hits: Option<i64>,
        comment_hash: FmtHash,
    },
}

impl IgnoredEntry {
    fn from_verified_req(req_id: ReqId, comment_hash: FmtHash) -> Self {
        Self::Requirement {
            id: req_id,
            comment_hash,
        }
    }

    fn from_test_case_state(
        test_run_name: String,
        test_run_utc_date: OffsetDateTime,
        test_case_name: String,
        state: TestCaseState,
        comment_hash: FmtHash,
    ) -> Self {
        Self::TestCaseStateOverride {
            test_run_name,
            test_run_utc_date,
            test_case_name,
            state,
            comment_hash,
        }
    }

    fn from_test_case_statement_coverage(
        test_run_name: String,
        test_run_utc_date: OffsetDateTime,
        test_case_name: String,
        stmnt_filepath: RelativePathBuf,
        stmnt_line: Line,
        hits: Option<i64>,
        comment_hash: FmtHash,
    ) -> Self {
        Self::TestCaseStatementCoverageOverride {
            test_run_name,
            test_run_utc_date,
            test_case_name,
            stmnt_filepath,
            stmnt_line,
            hits,
            comment_hash,
        }
    }

    fn from_test_run_statement_coverage(
        test_run_name: String,
        test_run_utc_date: OffsetDateTime,
        stmnt_filepath: RelativePathBuf,
        stmnt_line: Line,
        hits: Option<i64>,
        comment_hash: FmtHash,
    ) -> Self {
        Self::TestRunStatementCoverageOverride {
            test_run_name,
            test_run_utc_date,
            stmnt_filepath,
            stmnt_line,
            hits,
            comment_hash,
        }
    }
}
