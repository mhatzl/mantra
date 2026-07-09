use mantra_schema::{product::ProductId, requirements::RequirementSchema};

use crate::cmd::collect::collector::CollectableFile;

mod markdown;

pub fn collect_requirements(
    product_id: &ProductId,
    content: &CollectableFile,
) -> Result<Option<RequirementSchema>, anyhow::Error> {
    todo!()
}
