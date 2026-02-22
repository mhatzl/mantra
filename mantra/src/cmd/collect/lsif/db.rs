use mantra_schema::{FmtHash, Line, path::RelativePathBuf};

use crate::cmd::collect::Collection;
use crate::db::{Filepath, FilepathExt};

pub(super) struct IdentMissingElement {
    pub(super) filepath: RelativePathBuf,
    pub(super) file_hash: FmtHash,
    pub(super) definition_line: Line,
}

pub(super) struct ElementIdent {
    pub(super) filepath: RelativePathBuf,
    pub(super) file_hash: FmtHash,
    pub(super) definition_line: Line,
    pub(super) ident: String,
}

impl<'db> Collection<'db> {
    pub(super) async fn elements_missing_idents(
        &mut self,
    ) -> Result<Vec<IdentMissingElement>, anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        let records = sqlx::query!(
            r#"
            select pf.filepath as "filepath: Filepath", pf.file_hash, el.definition_line
            from Elements el, ProductRelatedFiles pf
            where pf.last_collect_nr = $1 and pf.product_id = $2
            and pf.file_hash = el.file_hash
            and not exists (
                select *
                from ElementIdents ei
                where ei.last_collect_nr = $1 and ei.product_id = $2
                and ei.filepath = pf.filepath
                and ei.file_hash = pf.file_hash
                and ei.definition_line = el.definition_line
            )
            "#,
            collect_nr,
            product_id
        )
        .fetch_all(self.connection_mut())
        .await?;

        Ok(records
            .into_iter()
            .map(|r| IdentMissingElement {
                filepath: RelativePathBuf::from_filepath(r.filepath),
                file_hash: FmtHash::with_inner(r.file_hash),
                definition_line: r.definition_line,
            })
            .collect())
    }

    pub(super) async fn update_element_idents(
        &mut self,
        elements: Vec<ElementIdent>,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        Ok(())
    }
}
