use mantra_schema::annotations::TraceKind;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn aggregate_annotations_data(&mut self) -> Result<(), anyhow::Error> {
        // Note: order is important, because later queries build on updated tables

        Ok(())
    }
}
