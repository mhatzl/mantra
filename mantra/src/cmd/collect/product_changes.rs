use mantra_schema::product::ProductId;

use crate::cmd::collect::{Collection, cfg::CollectMode};

impl<'db> Collection<'db> {
    pub(super) async fn update_existing_products(
        &mut self,
        existing_product_ids: impl Iterator<Item = &ProductId>,
        collect_mode: &CollectMode,
    ) -> Result<(), anyhow::Error> {
        // Newly added products

        // Removed products

        // optional: changed products metadata maybe?

        // existing products

        todo!()
    }
}
