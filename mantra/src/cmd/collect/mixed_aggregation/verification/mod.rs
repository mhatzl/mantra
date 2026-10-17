use anyhow::Context;
use mantra_schema::report::requirement::RequirementState;

use crate::cmd::collect::Collection;

pub mod direct;
pub mod indirect;

impl<'db> Collection<'db> {
    pub(crate) async fn update_verification_states(&mut self) -> Result<(), anyhow::Error> {
        self.update_direct_req_verification_states()
            .await
            .context("Failed to update direct requirement states")?;

        self.update_failed_reqs()
            .await
            .context("Failed to update failed requirements")?;

        self.update_indirect_req_verification_states()
            .await
            .context("Failed to update indirect requirement states")?;
        self.update_req_verification_states()
            .await
            .context("Failed to update aggregated requirement states")?;

        self.update_verified_reqs()
            .await
            .context("Failed to update verified requirements")?;

        self.update_skipped_reqs()
            .await
            .context("Failed to update skipped requirements")?;
        self.update_satisfied_reqs()
            .await
            .context("Failed to update satisfied requirements")?;
        self.update_unknown_reqs()
            .await
            .context("Failed to update untraced requirements")?;

        Ok(())
    }

    async fn update_failed_reqs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let req_failed_nr = RequirementState::Failed.as_nr();

        sqlx::query!(
            "
            insert or replace into FailedRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            select
                ur.agg_collect_nr,
                ur.req_collect_nr,
                ur.product_id,
                ur.req_id
            from UsableRequirements ur
                left join DirectRequirementVerificationStates ds
                    on ur.agg_collect_nr = ds.agg_collect_nr
                    and ur.req_collect_nr = ds.req_collect_nr
                    and ur.product_id = ds.product_id
                    and ur.req_id = ds.req_id
            where ur.agg_collect_nr = $1
            and (
                -- directly failed
                (ds.state not null and ds.state = $2)
                -- at least one descendant failed
                or exists (
                    select rd.req_id
                    from RequirementDescendants rd, DirectRequirementVerificationStates dsi
                    where rd.agg_collect_nr = (
                        select max(crd.agg_collect_nr)
                        from RequirementDescendants crd
                        where crd.product_id = rd.product_id
                    )
                    and ur.req_collect_nr = rd.req_collect_nr
                    and ur.product_id = rd.product_id
                    and ur.req_id = rd.req_id
                    and ur.agg_collect_nr = dsi.agg_collect_nr
                    and rd.descendant_collect_nr = dsi.req_collect_nr
                    and rd.descendant_product_id = dsi.product_id
                    and rd.descendant_req_id = dsi.req_id
                    and dsi.state = $2
                )
            )
            ",
            collect_nr,
            req_failed_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_req_verification_states(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let req_verified_nr = RequirementState::Verified.as_nr();
        let req_failed_nr = RequirementState::Failed.as_nr();
        let req_skipped_nr = RequirementState::Skipped.as_nr();
        let req_satisfied_nr = RequirementState::Satisfied.as_nr();
        let req_unknown_nr = RequirementState::Unknown.as_nr();
        let req_deprecated_nr = RequirementState::Deprecated.as_nr();
        let req_excluded_nr = RequirementState::Excluded.as_nr();
        let req_partially_verified_nr = RequirementState::PartiallyVerified.as_nr();

        sqlx::query!(
            "
            insert into RequirementVerificationStates (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id,
                state
            )
            select
                $1 as agg_collect_nr,
                r.collect_nr,
                r.product_id,
                r.id,
                case
                    -- deprecated
                    when exists (
                        select dr.req_id
                        from DeprecatedRequirements dr
                        where dr.agg_collect_nr = (
                            select max(cdr.agg_collect_nr)
                            from DeprecatedRequirements cdr
                            where cdr.product_id = dr.product_id
                        )
                        and r.collect_nr = dr.req_collect_nr
                        and r.product_id = dr.product_id
                        and r.id = dr.req_id
                    ) then $6
                    -- excluded
                    when exists (
                        select er.req_id
                        from ExcludedRequirements er
                        where er.agg_collect_nr = (
                            select max(cer.agg_collect_nr)
                            from ExcludedRequirements cer
                            where cer.product_id = er.product_id
                        )
                        and r.collect_nr = er.req_collect_nr
                        and r.product_id = er.product_id
                        and r.id = er.req_id
                    ) then $7
                    -- failed
                    when exists (
                        select fr.req_id
                        from FailedRequirements fr
                        where fr.agg_collect_nr = $1
                        and r.collect_nr = fr.req_collect_nr
                        and r.product_id = fr.product_id
                        and r.id = fr.req_id
                    ) then $3
                    -- skipped
                    when ds.state is not null and (ds.state = $4
                        or (
                            (ds.state = $5 or ds.state = $8) -- unknown or satisfied
                            and (ids.state is not null and ids.state = $4)
                        )
                    ) then $4
                    -- leaves: unknown, satisfied, partially verified, verified
                    --
                    -- directly verified reqs with only optional children
                    when exists (
                        select voc.req_id
                        from VerifiedRequirementsWithOnlyOptionalChildren voc
                        where voc.agg_collect_nr = $1
                        and r.collect_nr = voc.req_collect_nr
                        and r.product_id = voc.product_id
                        and r.id = voc.req_id
                    ) then $2
                    -- manual requirement
                    when exists(
                        select mr.req_id
                        from ManualRequirements mr
                        where mr.agg_collect_nr = (
                            select max(cmr.agg_collect_nr)
                            from ManualRequirements cmr
                            where cmr.product_id = mr.product_id
                        )
                        and mr.req_collect_nr = r.collect_nr
                        and mr.product_id = r.product_id
                        and mr.req_id = r.id
                    ) then
                        case
                            -- manual reqs must always be directly verified
                            when ds.state is not null and ds.state = $2
                            then
                                case
                                    -- no children
                                    when ids.state is null or ids.state = $2
                                    then $2
                                    -- directly verified, but children not verified => partially verified
                                    -- at least one child either partially verified, satisfied, or unknown
                                    else $9
                                end
                            -- missing direct verification, but (partially) verified through children
                            when ids.state is not null and (ids.state = $9 or ids.state = $2)
                            then $9
                            -- satisfied
                            when exists (
                                select sr.req_id
                                from DirectlySatisfiedRequirements sr
                                where sr.agg_collect_nr = (
                                    select max(csr.agg_collect_nr)
                                    from DirectlySatisfiedRequirements csr
                                    where csr.product_id = sr.product_id
                                )
                                and sr.req_collect_nr = r.collect_nr
                                and sr.product_id = r.product_id
                                and sr.req_id = r.id
                            ) then $8
                            -- unknown
                            else $5
                        end
                    -- partially verified
                    when ds.state is not null and (ds.state = $9
                        or (ids.state is not null
                            and (ids.state = $9
                                -- indirectly either satisfied or unknown for non-manual is left besides verified
                                or (ds.state = $2 and ids.state != $2)
                            )
                        )
                    ) then $9
                    -- verified
                    when ds.state is not null
                    and (
                        -- verified leaf
                        (ds.state = $2 and ids.state is null)
                        -- indirectly verified, direct verification is optional in this case
                        -- manual reqs handled above
                        or ids.state = $2
                    ) then $2
                    -- satisfied
                    when ds.state is not null and ds.state = $8
                    then $8
                    -- unknown
                    else $5
                end
            from Requirements r
                left join DirectRequirementVerificationStates ds
                    on r.collect_nr = ds.req_collect_nr
                    and r.product_id = ds.product_id
                    and r.id = ds.req_id
                    and ds.agg_collect_nr = $1
                left join IndirectRequirementVerificationStates ids
                    on r.collect_nr = ids.req_collect_nr
                    and r.product_id = ids.product_id
                    and r.id = ids.req_id
                    and ids.agg_collect_nr = $1
            where r.collect_nr = (
                select max(cr.collect_nr)
                from Requirements cr
                where cr.product_id = r.product_id
            )
            ",
            collect_nr,
            req_verified_nr,
            req_failed_nr,
            req_skipped_nr,
            req_unknown_nr,
            req_deprecated_nr,
            req_excluded_nr,
            req_satisfied_nr,
            req_partially_verified_nr
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
            insert into VerifiedRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            select
                rs.agg_collect_nr,
                rs.req_collect_nr,
                rs.product_id,
                rs.req_id
            from RequirementVerificationStates rs
            where rs.agg_collect_nr = $1
            and rs.state == $2
            ",
            collect_nr,
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
            insert into SkippedRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            select
                rs.agg_collect_nr,
                rs.req_collect_nr,
                rs.product_id,
                rs.req_id
            from RequirementVerificationStates rs
            where rs.agg_collect_nr = $1
            and rs.state == $2
            ",
            collect_nr,
            req_skipped_nr,
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
            insert into SatisfiedRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            select
                rs.agg_collect_nr,
                rs.req_collect_nr,
                rs.product_id,
                rs.req_id
            from RequirementVerificationStates rs
            where rs.agg_collect_nr = $1
            and rs.state == $2
            ",
            collect_nr,
            req_satisfied_nr,
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_unknown_reqs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let req_unknown_nr = RequirementState::Unknown.as_nr();

        sqlx::query!(
            "
            insert into UnknownRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            select
                rs.agg_collect_nr,
                rs.req_collect_nr,
                rs.product_id,
                rs.req_id
            from RequirementVerificationStates rs
            where rs.agg_collect_nr = $1
            and rs.state == $2
            ",
            collect_nr,
            req_unknown_nr,
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
