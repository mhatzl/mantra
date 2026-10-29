use anyhow::Context;
use mantra_schema::product::ProductId;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn remove_outdated_products(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        todo!();

        Ok(())
    }
}
