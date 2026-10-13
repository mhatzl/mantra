use anyhow::Context;
use mantra_schema::report::requirement::RequirementState;

use crate::cmd::collect::Collection;

pub mod direct;
pub mod indirect;

impl<'db> Collection<'db> {
    pub(crate) async fn aggregate_verification_data(&mut self) -> Result<(), anyhow::Error> {
        self.update_direct_req_verification_states()
            .await
            .context("Failed to update direct requirement states")?;
        self.update_indirect_req_verification_states()
            .await
            .context("Failed to update indirect requirement states")?;
        self.update_req_verification_states()
            .await
            .context("Failed to update aggregated requirement states")?;

        self.update_verified_reqs()
            .await
            .context("Failed to update verified requirements")?;
        self.update_failed_reqs()
            .await
            .context("Failed to update failed requirements")?;
        self.update_skipped_reqs()
            .await
            .context("Failed to update skipped requirements")?;
        self.update_satisfied_reqs()
            .await
            .context("Failed to update satisfied requirements")?;
        self.update_untraced_reqs()
            .await
            .context("Failed to update untraced requirements")?;

        Ok(())
    }

    async fn update_req_verification_states(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let req_verified_nr = RequirementState::Verified.as_nr();
        let req_failed_nr = RequirementState::Failed.as_nr();
        let req_skipped_nr = RequirementState::Skipped.as_nr();
        let req_satisfied_nr = RequirementState::Satisfied.as_nr();
        let req_untraced_nr = RequirementState::Untraced.as_nr();
        let req_deprecated_nr = RequirementState::Deprecated.as_nr();
        let req_excluded_nr = RequirementState::Excluded.as_nr();

        sqlx::query!(
            "
            insert or replace into RequirementVerificationStates (
                last_collect_nr,
                product_id,
                id,
                state
            )
            select
                r.last_collect_nr,
                r.product_id,
                r.id,
                case
                    when exists (
                        select dr.id
                        from DeprecatedRequirements dr
                        where r.last_collect_nr = dr.last_collect_nr
                        and r.product_id = dr.product_id
                        and r.id = dr.id
                    ) then $7
                    when exists (
                        select ir.id
                        from ExcludedRequirements ir
                        where r.last_collect_nr = ir.last_collect_nr
                        and r.product_id = ir.product_id
                        and r.id = ir.id
                    ) then $8
                    -- failed
                    when ((ids.state is not null and ids.state = $4) or (ds.state is not null and ds.state = $4))
                    then $4
                    -- skipped
                    when ds.state is not null and (ds.state = $5
                        or (ds.state = $6 and (ids.state is not null and ids.state = $5))
                    ) then $5
                    -- verified
                    when ds.state is not null
                    and ((ds.state = $3 and (
                            ids.state is null
                            -- only optional children with failed and skipped already handled above
                            or not exists (
                                select rh.parent_req_id
                                from RequirementHierarchies rh
                                where rh.parent_req_id = r.id and rh.parent_product_id = r.product_id
                                and not exists (
                                    select op.id
                                    from OptionalRequirements op
                                    where op.id = rh.child_req_id and op.product_id = rh.child_product_id
                                )
                            )
                        ))
                        or ((ds.state = $3 or (
                            -- manual requirements must always be directly verified
                            ds.state = $6 and not exists (
                                select mr.id
                                from ManualRequirements mr
                                where mr.id = r.id and mr.product_id = r.product_id
                            )))
                            and (ids.state is not null and ids.state = $3))
                    ) then $3
                    -- unverified
                    else $6
                end
            from Requirements r
                left join DirectRequirementVerificationStates ds on
                r.last_collect_nr = ds.last_collect_nr
                and r.product_id = ds.product_id
                and r.id = ds.id
                left join IndirectRequirementVerificationStates ids on
                r.last_collect_nr = ids.last_collect_nr
                and r.product_id = ids.product_id
                and r.id = ids.id
            where r.last_collect_nr = $1 and r.product_id = $2
            ",
            collect_nr,
            product_id,
            req_verified_nr,
            req_failed_nr,
            req_skipped_nr,
            req_unverified_nr,
            req_deprecated_nr,
            req_excluded_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_verified_reqs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let req_verified_nr = RequirementState::Verified.as_nr();

        sqlx::query!(
            "
            insert or replace into VerifiedRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select
                rs.last_collect_nr,
                rs.product_id,
                rs.id
            from RequirementVerificationStates rs
            where rs.last_collect_nr = $1 and rs.product_id = $2
            and rs.state == $3
            ",
            collect_nr,
            product_id,
            req_verified_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_skipped_reqs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let req_skipped_nr = RequirementState::Skipped.as_nr();

        sqlx::query!(
            "
            insert or replace into SkippedRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select
                rs.last_collect_nr,
                rs.product_id,
                rs.id
            from RequirementVerificationStates rs
            where rs.last_collect_nr = $1 and rs.product_id = $2
            and rs.state == $3
            ",
            collect_nr,
            product_id,
            req_skipped_nr,
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_failed_reqs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let req_failed_nr = RequirementState::Failed.as_nr();

        sqlx::query!(
            "
            insert or replace into FailedRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select
                rs.last_collect_nr,
                rs.product_id,
                rs.id
            from RequirementVerificationStates rs
            where rs.last_collect_nr = $1 and rs.product_id = $2
            and rs.state == $3
            ",
            collect_nr,
            product_id,
            req_failed_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_satisfied_reqs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let req_satisfied_nr = RequirementState::Satisfied.as_nr();

        sqlx::query!(
            "
            insert or replace into SatisfiedRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select
                rs.last_collect_nr,
                rs.product_id,
                rs.id
            from RequirementVerificationStates rs
            where rs.last_collect_nr = $1 and rs.product_id = $2
            and rs.state == $3
            ",
            collect_nr,
            product_id,
            req_satisfied_nr,
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_untraced_reqs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let req_untraced_nr = RequirementState::Untraced.as_nr();

        sqlx::query!(
            "
            insert or replace into UntracedRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select
                rs.last_collect_nr,
                rs.product_id,
                rs.id
            from RequirementVerificationStates rs
            where rs.last_collect_nr = $1 and rs.product_id = $2
            and rs.state == $3
            ",
            collect_nr,
            product_id,
            req_untraced_nr,
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
