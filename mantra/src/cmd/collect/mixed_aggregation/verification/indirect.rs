use anyhow::Context;
use mantra_schema::report::requirement::RequirementState;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(super) async fn update_indirect_req_verification_states(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let req_verified_nr = RequirementState::Verified.as_nr();
        let req_failed_nr = RequirementState::Failed.as_nr();
        let req_skipped_nr = RequirementState::Skipped.as_nr();
        let req_satisfied_nr = RequirementState::Satisfied.as_nr();
        let req_unknown_nr = RequirementState::Unknown.as_nr();
        let req_partially_verified_nr = RequirementState::PartiallyVerified.as_nr();

        sqlx::query!(
            "
            insert into UsableLeafRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            select distinct
                ur.agg_collect_nr,
                ur.req_collect_nr,
                ur.product_id,
                ur.req_id
            from UsableRequirements ur, LeafRequirements lr
            where ur.agg_collect_nr = $1
            and ur.agg_collect_nr = lr.agg_collect_nr
            and ur.req_collect_nr = lr.req_collect_nr
            and ur.product_id = lr.product_id
            and ur.req_id = lr.req_id
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update UsableLeafRequirements")?;

        sqlx::query!(
            "
            insert into UsableNonLeafRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            select distinct
                ur.agg_collect_nr,
                ur.req_collect_nr,
                ur.product_id,
                ur.req_id
            from UsableRequirements ur
            where ur.agg_collect_nr = $1

            except

            select distinct
                ur.agg_collect_nr,
                ur.req_collect_nr,
                ur.product_id,
                ur.req_id
            from UsableLeafRequirements ur
            where ur.agg_collect_nr = $1
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update UsableNonLeafRequirements")?;

        sqlx::query!(
            "
            -- Note: Includes leaf requirements needed for indirect check later
            insert into RequirementsWithOnlyOptionalChildren (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            with RequirementsWithNonOptionalChild (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            ) as (
                select
                    ur.agg_collect_nr,
                    ur.req_collect_nr,
                    ur.product_id,
                    ur.req_id
                from UsableNonLeafRequirements ur
                where ur.agg_collect_nr = $1
                and exists (
                    select rh.child_req_id
                    from
                        RequirementHierarchies rh,
                        UsableRequirements uri
                    where uri.agg_collect_nr = ur.agg_collect_nr
                    and rh.agg_collect_nr = (
                        select max(crh.agg_collect_nr)
                        from RequirementHierarchies crh
                        where crh.child_product_id = rh.child_product_id
                        and crh.parent_product_id = rh.parent_product_id
                    )
                    and ur.req_collect_nr = rh.parent_collect_nr
                    and ur.product_id = rh.parent_product_id
                    and ur.req_id = rh.parent_req_id
                    and uri.req_collect_nr = rh.child_collect_nr
                    and uri.product_id = rh.child_product_id
                    and uri.req_id = rh.child_req_id
                    and rh.optional = false
                    and not exists (
                        select op.req_id
                        from OptionalRequirements op
                        where op.agg_collect_nr = (
                            select max(cop.agg_collect_nr)
                            from OptionalRequirements cop
                            where cop.product_id = op.product_id
                        )
                        and op.req_collect_nr = rh.child_collect_nr
                        and op.product_id = rh.child_product_id
                        and op.req_id = rh.child_req_id
                    )
                )
            )
            select
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            from UsableRequirements ur
            where agg_collect_nr = $1

            except

            select
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            from RequirementsWithNonOptionalChild
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update RequirementsWithOnlyOptionalChildren")?;

        // Note: The table may contain multiple state entries per requirement
        // This will be considered when updating IndirectRequirementVerificationStates below
        sqlx::query!(
            "
            insert into StatesOfRequirementsWithOnlyOptionalChildren (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id,
                child_collect_nr,
                child_product_id,
                child_req_id,
                state
            )
            with recursive StatesOfReqWithOnlyOptionalChildren (
                req_collect_nr,
                product_id,
                req_id,
                child_collect_nr,
                child_product_id,
                child_req_id,
                state
            ) as (
                -- Leaf requirements are their own child.
                -- Also needed to have leafs added here to resolve indirect requirement states in the end.
                select
                    lr.req_collect_nr,
                    lr.product_id,
                    lr.req_id,
                    lr.req_collect_nr as child_collect_nr,
                    lr.product_id as child_product_id,
                    lr.req_id as child_req_id,
                    ds.state
                from
                    UsableLeafRequirements lr,
                    DirectRequirementVerificationStates ds
                -- TODO: check if max() should be checked here
                where lr.agg_collect_nr = $1
                and ds.agg_collect_nr = $1
                and lr.req_collect_nr = ds.req_collect_nr
                and lr.product_id = ds.product_id
                and lr.req_id = ds.req_id

                union all

                select
                    r.req_collect_nr,
                    r.product_id,
                    r.req_id,
                    rh.child_collect_nr,
                    rh.child_product_id,
                    rh.child_req_id,
                    case
                        when ds.state = $3 or sor.state = $3 then $3
                        when ds.state = $2 or (ds.state = $4 and sor.state = $2) then $2
                        else ds.state
                    end
                from
                    RequirementsWithOnlyOptionalChildren r,
                    DirectRequirementVerificationStates ds,
                    RequirementHierarchies rh,
                    StatesOfReqWithOnlyOptionalChildren sor
                where r.agg_collect_nr = $1
                and ds.agg_collect_nr = $1
                and rh.agg_collect_nr = (
                select max(crh.agg_collect_nr)
                    from RequirementHierarchies crh
                    where crh.child_product_id = rh.child_product_id
                    and crh.parent_product_id = rh.parent_product_id
                )
                and r.req_collect_nr = ds.req_collect_nr
                and r.product_id = ds.product_id
                and r.req_id = ds.req_id
                and r.req_collect_nr = rh.parent_collect_nr
                and r.product_id = rh.parent_product_id
                and r.req_id = rh.parent_req_id
                and sor.req_collect_nr = rh.child_collect_nr
                and sor.product_id = rh.child_product_id
                and sor.req_id = rh.child_req_id
            )
            select distinct
                $1 as agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id,
                child_collect_nr,
                child_product_id,
                child_req_id,
                state
            from StatesOfReqWithOnlyOptionalChildren
            ",
            collect_nr,
            req_verified_nr,
            req_failed_nr,
            req_unknown_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update StatesOfRequirementsWithOnlyOptionalChildren")?;

        sqlx::query!(
            "
            insert into VerifiedRequirementsWithOnlyOptionalChildren (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            -- If requirement itself and no child failed, having at least one verifying state
            -- is sufficient for requirements with only optional children,
            -- because either it is verified directly or by at least one child
            select sr.agg_collect_nr, sr.req_collect_nr, sr.product_id, sr.req_id
            from StatesOfRequirementsWithOnlyOptionalChildren sr
            where sr.agg_collect_nr = $1
            -- at least one verified entry
            and exists (
                select isr.req_id
                from StatesOfRequirementsWithOnlyOptionalChildren isr
                where isr.agg_collect_nr = sr.agg_collect_nr
                and isr.req_collect_nr = sr.req_collect_nr
                and isr.product_id = sr.product_id
                and isr.req_id = sr.req_id
                and isr.state = $2
            )

            except

            select agg_collect_nr, req_collect_nr, product_id, req_id
            from FailedRequirements
            where agg_collect_nr = $1
            ",
            collect_nr,
            req_verified_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update VerifiedRequirementsWithOnlyOptionalChildren")?;

        sqlx::query!(
            "
            insert into RequirementsWithNonOptionalChildrenHavingSkippedState (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            with recursive ReqsWithNonOptionalChildrenHavingSkippedState (
                req_collect_nr,
                product_id,
                req_id,
                child_collect_nr,
                child_product_id,
                child_req_id
            ) as (
                select
                    req_collect_nr,
                    product_id,
                    req_id,
                    child_collect_nr,
                    child_product_id,
                    child_req_id
                from (
                    select
                        r.req_collect_nr,
                        r.product_id,
                        r.req_id,
                        rh.child_collect_nr,
                        rh.child_product_id,
                        rh.child_req_id
                    from UsableNonLeafRequirements r, RequirementHierarchies rh
                    where r.agg_collect_nr = $1
                    and rh.agg_collect_nr = (
                        select max(crh.agg_collect_nr)
                        from RequirementHierarchies crh
                        where crh.child_product_id = rh.child_product_id
                        and crh.parent_product_id = rh.parent_product_id
                    )
                    and r.req_collect_nr = rh.parent_collect_nr
                    and r.product_id = rh.parent_product_id
                    and r.req_id = rh.parent_req_id
                    and exists (
                        select rh.child_req_id
                        from
                            DirectRequirementVerificationStates ds,
                            UsableLeafRequirements lr
                        where ds.agg_collect_nr = r.agg_collect_nr
                        and lr.agg_collect_nr = r.agg_collect_nr
                        and rh.child_collect_nr = lr.req_collect_nr
                        and rh.child_product_id = lr.product_id
                        and rh.child_req_id = lr.req_id
                        and lr.req_collect_nr = ds.req_collect_nr
                        and lr.product_id = ds.product_id
                        and lr.req_id = ds.req_id
                        and ds.state = $2
                        and rh.optional = false
                        and not exists (
                            select op.req_id
                            from OptionalRequirements op
                            where op.agg_collect_nr = (
                                select max(cop.agg_collect_nr)
                                from OptionalRequirements cop
                                where cop.product_id = op.product_id
                            )
                            and op.req_collect_nr = rh.child_collect_nr
                            and op.product_id = rh.child_product_id
                            and op.req_id = rh.child_req_id
                        )
                    )

                    union all

                    -- skipped req that only has optional children
                    -- may serve as skipped child for its parents
                    select
                        ds.req_collect_nr,
                        ds.product_id,
                        ds.req_id,
                        ds.req_collect_nr,
                        ds.product_id,
                        ds.req_id
                    from
                        DirectRequirementVerificationStates ds,
                        RequirementsWithOnlyOptionalChildren ooc
                    where ds.agg_collect_nr = $1
                    and ooc.agg_collect_nr = ds.agg_collect_nr
                    and ooc.req_collect_nr = ds.req_collect_nr
                    and ooc.product_id = ds.product_id
                    and ooc.req_id = ds.req_id
                    and ds.state = $2
                )

                union all

                select
                    r.req_collect_nr,
                    r.product_id,
                    r.req_id,
                    rh.child_collect_nr,
                    rh.child_product_id,
                    rh.child_req_id
                from
                    UsableNonLeafRequirements r,
                    ReqsWithNonOptionalChildrenHavingSkippedState sc,
                    RequirementHierarchies rh
                where rh.agg_collect_nr = (
                    select max(crh.agg_collect_nr)
                    from RequirementHierarchies crh
                    where crh.child_product_id = rh.child_product_id
                    and crh.parent_product_id = rh.parent_product_id
                )
                and r.agg_collect_nr = $1
                and r.req_collect_nr = rh.parent_collect_nr
                and r.product_id = rh.parent_product_id
                and r.req_id = rh.parent_req_id
                and sc.req_collect_nr = rh.child_collect_nr
                and sc.product_id = rh.child_product_id
                and sc.req_id = rh.child_req_id
                and rh.optional = false
                and not exists (
                    select op.req_id
                    from OptionalRequirements op
                    where op.agg_collect_nr = (
                        select max(cop.agg_collect_nr)
                        from OptionalRequirements cop
                        where cop.product_id = op.product_id
                    )
                    and op.req_collect_nr = rh.child_collect_nr
                    and op.product_id = rh.child_product_id
                    and op.req_id = rh.child_req_id
                )
            )
            select distinct $1 as agg_collect_nr, req_collect_nr, product_id, req_id
            from ReqsWithNonOptionalChildrenHavingSkippedState
            -- filter base requirements since their state did not depend on children
            where req_collect_nr != child_collect_nr
            or product_id != child_product_id
            or req_id != child_req_id
            ",
            collect_nr,
            req_skipped_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update RequirementsWithNonOptionalChildrenHavingSkippedState")?;

        sqlx::query!(
            "
            insert into RequirementsWithNonOptionalChildrenHavingStatisfiedOrUnknownState (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            with recursive ReqsWithNonOptionalChildrenHavingStatisfiedOrUnknownState (
                req_collect_nr,
                product_id,
                req_id,
                child_collect_nr,
                child_product_id,
                child_req_id
            ) as (
                select
                    req_collect_nr,
                    product_id,
                    req_id,
                    child_collect_nr,
                    child_product_id,
                    child_req_id
                from (
                    -- at least one non-optional leaf either has satisfied or unknown
                    select
                        r.req_collect_nr,
                        r.product_id,
                        r.req_id,
                        rh.child_collect_nr,
                        rh.child_product_id,
                        rh.child_req_id
                    from UsableNonLeafRequirements r, RequirementHierarchies rh
                    where r.agg_collect_nr = $1
                    and rh.agg_collect_nr = (
                        select max(crh.agg_collect_nr)
                        from RequirementHierarchies crh
                        where crh.child_product_id = rh.child_product_id
                        and crh.parent_product_id = rh.parent_product_id
                    )
                    and r.req_collect_nr = rh.parent_collect_nr
                    and r.product_id = rh.parent_product_id
                    and r.req_id = rh.parent_req_id
                    and exists (
                        select lr.req_id
                        from
                            DirectRequirementVerificationStates ds,
                            UsableLeafRequirements lr
                        where ds.agg_collect_nr = r.agg_collect_nr
                        and lr.agg_collect_nr = r.agg_collect_nr
                        and rh.child_collect_nr = lr.req_collect_nr
                        and rh.child_product_id = lr.product_id
                        and rh.child_req_id = lr.req_id
                        and lr.req_collect_nr = ds.req_collect_nr
                        and lr.product_id = ds.product_id
                        and lr.req_id = ds.req_id
                        and (ds.state = $2 or ds.state = $3)
                        and rh.optional = false
                        and not exists (
                            select op.req_id
                            from OptionalRequirements op
                            where op.agg_collect_nr = (
                                select max(cop.agg_collect_nr)
                                from OptionalRequirements cop
                                where cop.product_id = op.product_id
                            )
                            and op.req_collect_nr = rh.child_collect_nr
                            and op.product_id = rh.child_product_id
                            and op.req_id = rh.child_req_id
                        )
                    )

                    union all

                    select
                        r.req_collect_nr,
                        r.product_id,
                        r.req_id,
                        r.req_collect_nr as child_collect_nr,
                        r.product_id as child_product_id,
                        r.req_id as child_req_id
                    from UsableRequirements r
                    -- reqs with only optional children added as base in case they are
                    -- only of satisfied or unknown state, because they are likely children
                    -- of requirements that may have non-optional children as well.
                    where r.agg_collect_nr = $1
                    and exists (
                        select ooc.req_id
                        from
                            DirectRequirementVerificationStates ds,
                            RequirementsWithOnlyOptionalChildren ooc
                        where ds.agg_collect_nr = r.agg_collect_nr
                        and ds.req_collect_nr = r.req_collect_nr
                        and ds.product_id = r.product_id
                        and ds.req_id = r.req_id
                        and ooc.agg_collect_nr = r.agg_collect_nr
                        and ooc.req_collect_nr = r.req_collect_nr
                        and ooc.product_id = r.product_id
                        and ooc.req_id = r.req_id
                        and (ds.state = $2 or ds.state = $3)
                        and not exists (
                            select vr.req_id
                            from VerifiedRequirementsWithOnlyOptionalChildren vr
                            where r.agg_collect_nr = vr.agg_collect_nr
                            and r.req_collect_nr = vr.req_collect_nr
                            and r.product_id = vr.product_id
                            and r.req_id = vr.req_id
                        )
                    )
                    -- manual requirements must be reviewed directly independent of the state of children
                    or exists (
    					select mr.req_id
                        from ManualRequirements mr, DirectRequirementVerificationStates ds
                        where mr.agg_collect_nr = (
                            select max(cmr.agg_collect_nr)
                            from ManualRequirements cmr
                            where cmr.product_id = mr.product_id
                        )
                        and mr.req_collect_nr = r.req_collect_nr
                        and mr.product_id = r.product_id
                        and mr.req_id = r.req_id
                        and ds.agg_collect_nr = r.agg_collect_nr
                        and ds.req_collect_nr = r.req_collect_nr
                        and ds.product_id = r.product_id
                        and ds.req_id = r.req_id
                        and (ds.state = $2 or ds.state = $3)
    				)
				)

                union all

                select
                    r.req_collect_nr,
                    r.product_id,
                    r.req_id,
                    rh.child_collect_nr,
                    rh.child_product_id,
                    rh.child_req_id
                from
                    UsableNonLeafRequirements r,
                    ReqsWithNonOptionalChildrenHavingStatisfiedOrUnknownState uc,
                    RequirementHierarchies rh
                where rh.agg_collect_nr = (
                    select max(crh.agg_collect_nr)
                    from RequirementHierarchies crh
                    where crh.child_product_id = rh.child_product_id
                    and crh.parent_product_id = rh.parent_product_id
                )
                and r.agg_collect_nr = $1
                and r.req_collect_nr = rh.parent_collect_nr
                and r.product_id = rh.parent_product_id
                and r.req_id = rh.parent_req_id
                and uc.req_collect_nr = rh.child_collect_nr
                and uc.product_id = rh.child_product_id
                and uc.req_id = rh.child_req_id
                and rh.optional = false
                and not exists (
                    select op.req_id
                    from OptionalRequirements op
                    where op.agg_collect_nr = (
                        select max(cop.agg_collect_nr)
                        from OptionalRequirements cop
                        where cop.product_id = op.product_id
                    )
                    and op.req_collect_nr = rh.child_collect_nr
                    and op.product_id = rh.child_product_id
                    and op.req_id = rh.child_req_id
                )
            )
            select distinct $1 as agg_collect_nr, req_collect_nr, product_id, req_id
            from ReqsWithNonOptionalChildrenHavingStatisfiedOrUnknownState
            -- filter base requirements since their state did not depend on children
            where req_collect_nr != child_collect_nr
            or product_id != child_product_id
            or req_id != child_req_id
            ",
            collect_nr,
            req_satisfied_nr,
            req_unknown_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update RequirementsWithNonOptionalChildrenHavingStatisfiedOrUnknownState")?;

        sqlx::query!(
            "
            insert into RequirementsWithChildrenHavingPartiallyVerifiedState (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            with recursive ReqsWithChildrenHavingPartiallyVerifiedState (
                req_collect_nr,
                product_id,
                req_id
            ) as (
                -- start at any partially verified requirement
                select r.req_collect_nr, r.product_id, r.req_id
                from UsableRequirements r, DirectRequirementVerificationStates ds
                where r.agg_collect_nr = $1
                and ds.agg_collect_nr = r.agg_collect_nr
                and ds.req_collect_nr = r.req_collect_nr
                and ds.product_id = r.product_id
                and ds.req_id = r.req_id
                and ds.state = $2

                union all

                select r.req_collect_nr, r.product_id, r.req_id
                from
                    UsableNonLeafRequirements r,
                    ReqsWithChildrenHavingPartiallyVerifiedState sc,
                    RequirementHierarchies rh
                where rh.agg_collect_nr = (
                    select max(crh.agg_collect_nr)
                    from RequirementHierarchies crh
                    where crh.child_product_id = rh.child_product_id
                    and crh.parent_product_id = rh.parent_product_id
                )
                and r.agg_collect_nr = $1
                and r.req_collect_nr = rh.parent_collect_nr
                and r.product_id = rh.parent_product_id
                and r.req_id = rh.parent_req_id
                and sc.req_collect_nr = rh.child_collect_nr
                and sc.product_id = rh.child_product_id
                and sc.req_id = rh.child_req_id
                -- ignore verified requirements with only-optional children,
                -- because even if a child of it is only partially verified
                -- the requirement is considered verified for its parents
                and not exists (
                    select vr.req_id
                    from VerifiedRequirementsWithOnlyOptionalChildren vr
                    where vr.agg_collect_nr = r.agg_collect_nr
                    and vr.req_collect_nr = rh.child_collect_nr
                    and vr.product_id = rh.child_product_id
                    and vr.req_id = rh.child_req_id
                )
            )
            select distinct $1 as agg_collect_nr, req_collect_nr, product_id, req_id
            from ReqsWithChildrenHavingPartiallyVerifiedState
            ",
            collect_nr,
            req_skipped_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update RequirementsWithChildrenHavingPartiallyVerifiedState")?;

        sqlx::query!(
            "
            insert into IndirectRequirementVerificationStates (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id,
                state
            )
            with ReqsWithVerifiedDescendants (
                req_collect_nr,
                product_id,
                req_id
            ) as (
                select r.req_collect_nr, r.product_id, r.req_id
                from
                    UsableNonLeafRequirements r,
                    RequirementDescendants rd,
                    DirectRequirementVerificationStates ds
                where r.agg_collect_nr = $1
                and ds.agg_collect_nr = r.agg_collect_nr
                and rd.agg_collect_nr = (
                    select max(crd.agg_collect_nr)
                    from RequirementDescendants crd
                    where crd.product_id = rd.product_id
                )
                and rd.req_collect_nr = r.req_collect_nr
                and rd.product_id = r.product_id
                and rd.req_id = r.req_id
                and rd.descendant_collect_nr = ds.req_collect_nr
                and rd.descendant_product_id = ds.product_id
                and rd.descendant_req_id = ds.req_id
                and ds.state = $2
            )
            select
                r.agg_collect_nr,
                r.req_collect_nr,
                r.product_id,
                r.req_id,
                case
                    -- failed
                    when exists (
                        select f.req_id
                        from FailedRequirements f
                        where f.agg_collect_nr = r.agg_collect_nr
                        and f.req_collect_nr = r.req_collect_nr
                        and f.product_id = r.product_id
                        and f.req_id = r.req_id
                    ) then $3
                    -- skipped
                    when exists (
                        select s.req_id
                        from RequirementsWithNonOptionalChildrenHavingSkippedState s
                        where s.agg_collect_nr = r.agg_collect_nr
                        and s.req_collect_nr = r.req_collect_nr
                        and s.product_id = r.product_id
                        and s.req_id = r.req_id
                    ) then $4
                    -- verified requirements with only-optional non-failed children
                    when exists (
                        select os.req_id
                        from VerifiedRequirementsWithOnlyOptionalChildren os
                        where os.agg_collect_nr = r.agg_collect_nr
                        and os.req_collect_nr = r.req_collect_nr
                        and os.product_id = r.product_id
                        and os.req_id = r.req_id
                        -- filter requirements that are only directly verified
                        -- this requires that leaf requirements are part of VerifiedRequirementsWithOnlyOptionalChildren
                        and exists (
                            select ocs.req_id
                            from
                                VerifiedRequirementsWithOnlyOptionalChildren ocs,
                                RequirementHierarchies rh
                            where rh.agg_collect_nr = (
                                select max(crh.agg_collect_nr)
                                from RequirementHierarchies crh
                                where crh.child_product_id = rh.child_product_id
                                and crh.parent_product_id = rh.parent_product_id
                            )
                            and r.req_collect_nr = rh.parent_collect_nr
                            and r.product_id = rh.parent_product_id
                            and r.req_id = rh.parent_req_id
                            and ocs.agg_collect_nr = r.agg_collect_nr
                            and ocs.req_collect_nr = rh.child_collect_nr
                            and ocs.product_id = rh.child_product_id
                            and ocs.req_id = rh.child_req_id
                        )
                    ) then $2
                    -- partially verified beats unknown, because it provides more information
                    when exists (
                        select u.req_id
                        from RequirementsWithChildrenHavingPartiallyVerifiedState u
                        where u.agg_collect_nr = r.agg_collect_nr
                        and u.req_collect_nr = r.req_collect_nr
                        and u.product_id = r.product_id
                        and u.req_id = r.req_id
                    ) then $6
                    -- unknown, because satisfied can only be achieved directly
                    when exists (
                        select u.req_id
                        from RequirementsWithNonOptionalChildrenHavingStatisfiedOrUnknownState u
                        where u.agg_collect_nr = r.agg_collect_nr
                        and u.req_collect_nr = r.req_collect_nr
                        and u.product_id = r.product_id
                        and u.req_id = r.req_id
                    ) then $5
                    -- failed, skipped, partially verified, satisfied, or unknown handled above.
                    -- If at least one descendant is verified and the requirement was not matched so far,
                    -- verification must have propagated up.
                    -- => verified
                    when exists (
                        select v.req_id
                        from ReqsWithVerifiedDescendants v
                        where v.req_collect_nr = r.req_collect_nr
                        and v.product_id = r.product_id
                        and v.req_id = r.req_id
                    ) then $2
                    -- unknown
                    else $5
                end
            from UsableNonLeafRequirements r
            where r.agg_collect_nr = $1
            ",
            collect_nr,
            req_verified_nr,
            req_failed_nr,
            req_skipped_nr,
            req_unknown_nr,
            req_partially_verified_nr
        )
        .execute(self.connection_mut())
        .await.context("Failed to update IndirectRequirementVerificationStates")?;

        Ok(())
    }
}
