use anyhow::Context;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn aggregate_annotations_data(&mut self) -> Result<(), anyhow::Error> {
        self.update_trace_spans()
            .await
            .context("Failed to update trace spans")?;
        self.update_coverage_exclude_lines()
            .await
            .context("Failed to update coverage exclusions")?;

        Ok(())
    }

    async fn update_trace_spans(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            insert or replace into TraceSpans (
                file_hash,
                traced_line,
                start_line,
                end_line
            )
            with SingleLineTraces as (
                select file_hash, line as traced_line, line as start_line, line as end_line
                from Traces
            ),
            ElementTraces as (
                select e.file_hash, de.traced_line, e.start_line, e.end_line
                from Traces t, DirectTracedElements de, Elements e
                where t.file_hash = de.file_hash and t.file_hash = e.file_hash
                and t.line = de.traced_line and de.element_definition_line = e.definition_line
            ),
            CodeBlockTraces as (
                select file_hash, traced_line, start_line, end_line
                from TracedCodeBlocks
            )
            select file_hash, traced_line, min(start_line), max(end_line)
            from (
                select file_hash, traced_line, start_line, end_line
                from SingleLineTraces
                union all
                select file_hash, traced_line, start_line, end_line
                from ElementTraces
                union all
                select file_hash, traced_line, start_line, end_line
                from CodeBlockTraces
            )
            group by file_hash, traced_line
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_coverage_exclude_lines(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            with recursive block_line as (
                select
                    file_hash,
                    start_line as line,
                    end_line
                from CoverageBlockExcludes
                union all
                select
                    bl.file_hash,
                    bl.line + 1 as line,
                    bl.end_line
                from block_line bl
                where bl.line <= bl.end_line
            )
            insert or ignore into ExcludedCoverageLines (file_hash, line)
            select file_hash, line
            from block_line
            union all
            select file_hash, line
            from CoverageLineExcludes
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
