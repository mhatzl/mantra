use mantra_schema::FmtHash;
use mantra_schema::Line;
use mantra_schema::Properties;
use mantra_schema::path::RelativePath;
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
    pub(super) async fn update_per_review_schema(
        &mut self,
        filepath: &RelativePath,
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
            self.update_review(
                filepath,
                review,
                &base_origin_hash,
                &review_schema.properties,
            )
            .await?;
        }

        Ok(())
    }

    pub(crate) async fn delete_outdated_reviews(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        let updated_records = sqlx::query!(
            "
                select name, utc_date from Reviews
                where product_id = $1 and last_collect_nr = $2
            ",
            product_id,
            collect_nr
        )
        .fetch_all(self.connection_mut())
        .await?;

        // Note: always deleting outdated data for collected reviews,
        // because this means that the data got removed in the original source.
        for record in updated_records {
            sqlx::query!(
                "
                delete from ReviewAuthors
                where product_id = $1 and last_collect_nr < $2
                and review_name = $3 and review_date = $4

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
                delete from ReviewProperties
                where product_id = $1 and last_collect_nr < $2
                and review_name = $3 and review_date = $4

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
                delete from ReviewRevisions
                where product_id = $1 and last_collect_nr < $2
                and review_name = $3 and review_date = $4

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
                delete from ReviewRevisionAuthors
                where product_id = $1 and last_collect_nr < $2
                and review_name = $3 and review_date = $4

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
                delete from ManuallyVerifiedRequirements
                where product_id = $1 and last_collect_nr < $2
                and review_name = $3 and review_date = $4

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
                delete from TestCaseOverrides
                where product_id = $1 and last_collect_nr < $2
                and review_name = $3 and review_date = $4

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
                delete from TestRunLineCoverageOverrides
                where product_id = $1 and last_collect_nr < $2
                and review_name = $3 and review_date = $4

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
                delete from TestCaseLineCoverageOverrides
                where product_id = $1 and last_collect_nr < $2
                and review_name = $3 and review_date = $4

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
                delete from IgnoredReviewEntries
                where product_id = $1 and last_collect_nr < $2
                and review_name = $3 and review_date = $4

            ",
                product_id,
                collect_nr,
                record.name,
                record.utc_date
            )
            .execute(self.connection_mut())
            .await?;
        }

        // Note: due to cascade rules, deletions in the base Reviews table
        // cascade to the other tables
        sqlx::query!(
            "
                delete from Reviews
                where product_id = $1 and last_collect_nr < $2
            ",
            product_id,
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_review(
        &mut self,
        filepath: &RelativePath,
        review: Review,
        base_origin_hash: &Option<FmtHash>,
        base_props: &Option<Properties>,
    ) -> Result<(), anyhow::Error> {
        // TODO: optimize by checking src-hash first and skip if unchanged

        let collect_nr = self.collect_nr();
        let product_id = &self.product_id();

        if let Some(record) = sqlx::query!(
            "
            select name, utc_date, data_filepath
            from Reviews
            where last_collect_nr = $1 and product_id = $2
            and name = $3 and utc_date = $4
            ",
            collect_nr,
            product_id,
            review.name,
            review.utc_date
        )
        .fetch_optional(self.connection_mut())
        .await?
        {
            anyhow::bail!(
                "Duplicate review with name='{}' date='{}' found in the same collection! Duplicate definition in '{}'; Previous definition in '{}'.",
                review.name,
                review.utc_date,
                filepath,
                record.data_filepath
            );
        }

        let data_hash = FmtHash::from(&serde_json::json!({
            "base_origin_hash": base_origin_hash,
            "base_props": base_props,
            "review": &review
        }));
        let data_filepath = filepath.as_str();

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
            self.insert_general_text(hash, description, None).await?;
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
                data_hash,
                data_filepath
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
            on conflict (product_id, name, utc_date)
            do update set
                last_collect_nr = excluded.last_collect_nr,
                base_origin_hash = excluded.base_origin_hash,
                origin_hash = excluded.origin_hash,
                description_hash = excluded.description_hash,
                data_hash = excluded.data_hash,
                data_filepath = excluded.data_filepath
            ",
            collect_nr,
            product_id,
            review.name,
            review.utc_date,
            base_origin_hash,
            origin_hash,
            description_hash,
            data_hash,
            data_filepath
        )
        .execute(self.connection_mut())
        .await?;

        for author in review.authors {
            sqlx::query!(
                "
                insert into ReviewAuthors (
                    last_collect_nr,
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
                on conflict (product_id, review_name, review_date, author)
                do update set
                    last_collect_nr = excluded.last_collect_nr
                ",
                collect_nr,
                product_id,
                review.name,
                review.utc_date,
                author
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
                    insert into ReviewRevisions (
                        last_collect_nr,
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
                    on conflict (product_id, review_name, review_date, revision)
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
            self.insert_general_text(&comment_hash, verified_req.comment, None)
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

        for test_run_override in review.test_run_overrides {
            for test_case_override in test_run_override.test_cases {
                if let Some(override_state) = test_case_override.state {
                    let comment_hash = FmtHash::from(&override_state.comment);
                    self.insert_general_text(&comment_hash, override_state.comment, None)
                        .await?;
                    let state = override_state.new.as_nr();

                    let test_case_exists = sqlx::query!(
                        "
                        select * from TestCases
                        where
                            last_collect_nr = $1
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
                            test_run_override.name,
                            test_run_override.utc_date,
                            test_case_override.name,
                            review.name,
                            review.utc_date,
                            state,
                            comment_hash
                        )
                        .execute(self.connection_mut())
                        .await?;
                    } else {
                        let entry = DbIgnoredEntry::from_test_case_state(
                            test_run_override.name.clone(),
                            test_run_override.utc_date.clone(),
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
                        self.insert_general_text(&comment_hash, line_info.comment, None)
                            .await?;

                        for line_nr in line_info.nrs {
                            let covered_line_exists = sqlx::query!(
                                "
                                select * from TestCaseLineCoverage
                                where
                                    product_id = $1 and test_run_name = $2
                                    and test_run_date = $3 and test_case_name = $4
                                    and cov_filepath = $5 and cov_line = $6
                                ",
                                product_id,
                                test_run_override.name,
                                test_run_override.utc_date,
                                test_case_override.name,
                                filepath,
                                line_nr
                            )
                            .fetch_optional(self.connection_mut())
                            .await?
                            .is_some();

                            if covered_line_exists {
                                sqlx::query!(
                                    "
                                    insert into TestCaseLineCoverageOverrides (
                                        last_collect_nr,
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
                                    on conflict (
                                        product_id,
                                        test_run_name,
                                        test_run_date,
                                        test_case_name,
                                        review_name,
                                        review_date,
                                        cov_filepath,
                                        cov_line
                                    )
                                    do update set
                                        last_collect_nr = excluded.last_collect_nr,
                                        hits = excluded.hits,
                                        comment_hash = excluded.comment_hash
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
                                .await?;
                            } else {
                                let entry = DbIgnoredEntry::from_test_case_line_coverage(
                                    test_run_override.name.clone(),
                                    test_run_override.utc_date.clone(),
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
                    self.insert_general_text(&comment_hash, line_info.comment, None)
                        .await?;

                    for line_nr in line_info.nrs {
                        let covered_line_exists = sqlx::query!(
                            "
                            select * from TestRunLineCoverage
                            where
                                product_id = $1 and test_run_name = $2
                                and test_run_date = $3 and cov_filepath = $4
                                and cov_line = $5
                            ",
                            product_id,
                            test_run_override.name,
                            test_run_override.utc_date,
                            filepath,
                            line_nr
                        )
                        .fetch_optional(self.connection_mut())
                        .await?
                        .is_some();

                        if covered_line_exists {
                            sqlx::query!(
                                "
                                insert into TestRunLineCoverageOverrides (
                                    last_collect_nr,
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
                                on conflict (
                                    product_id,
                                    test_run_name,
                                    test_run_date,
                                    review_name,
                                    review_date,
                                    cov_filepath,
                                    cov_line
                                )
                                do update set
                                    last_collect_nr = excluded.last_collect_nr,
                                    hits = excluded.hits,
                                    comment_hash = excluded.comment_hash
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
                            .await?;
                        } else {
                            let entry = DbIgnoredEntry::from_test_run_line_coverage(
                                test_run_override.name.clone(),
                                test_run_override.utc_date.clone(),
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
                DbIgnoredEntry::from_verified_req(req_id.clone(), comment_hash.clone());
            self.insert_ignored_entry(review_name, review_date, ignored_entry)
                .await?;
        }

        Ok(())
    }

    async fn insert_ignored_entry(
        &mut self,
        review_name: &str,
        review_date: &OffsetDateTime,
        entry: DbIgnoredEntry,
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
#[serde(rename_all = "snake_case")]
pub(crate) enum DbIgnoredEntry {
    Requirement {
        id: ReqId,
        comment_hash: FmtHash,
    },
    TestCaseStateOverride {
        test_run_name: String,
        test_run_date: OffsetDateTime,
        test_case_name: String,
        state: TestCaseState,
        comment_hash: FmtHash,
    },
    TestCaseLineCoverageOverride {
        test_run_name: String,
        test_run_date: OffsetDateTime,
        test_case_name: String,
        cov_filepath: RelativePathBuf,
        cov_line: Line,
        hits: Option<i64>,
        comment_hash: FmtHash,
    },
    TestRunLineCoverageOverride {
        test_run_name: String,
        test_run_date: OffsetDateTime,
        cov_filepath: RelativePathBuf,
        cov_line: Line,
        hits: Option<i64>,
        comment_hash: FmtHash,
    },
}

impl DbIgnoredEntry {
    fn from_verified_req(req_id: ReqId, comment_hash: FmtHash) -> Self {
        Self::Requirement {
            id: req_id,
            comment_hash,
        }
    }

    fn from_test_case_state(
        test_run_name: String,
        test_run_date: OffsetDateTime,
        test_case_name: String,
        state: TestCaseState,
        comment_hash: FmtHash,
    ) -> Self {
        Self::TestCaseStateOverride {
            test_run_name,
            test_run_date,
            test_case_name,
            state,
            comment_hash,
        }
    }

    fn from_test_case_line_coverage(
        test_run_name: String,
        test_run_date: OffsetDateTime,
        test_case_name: String,
        cov_filepath: RelativePathBuf,
        cov_line: Line,
        hits: Option<i64>,
        comment_hash: FmtHash,
    ) -> Self {
        Self::TestCaseLineCoverageOverride {
            test_run_name,
            test_run_date,
            test_case_name,
            cov_filepath,
            cov_line,
            hits,
            comment_hash,
        }
    }

    fn from_test_run_line_coverage(
        test_run_name: String,
        test_run_date: OffsetDateTime,
        cov_filepath: RelativePathBuf,
        cov_line: Line,
        hits: Option<i64>,
        comment_hash: FmtHash,
    ) -> Self {
        Self::TestRunLineCoverageOverride {
            test_run_name,
            test_run_date,
            cov_filepath,
            cov_line,
            hits,
            comment_hash,
        }
    }
}
