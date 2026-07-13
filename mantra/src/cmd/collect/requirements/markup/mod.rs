use mantra_schema::{product::ProductId, requirements::RequirementSchema};

use crate::cmd::collect::collector::CollectableFile;

mod markdown;

pub(super) fn collect_requirements(
    product_id: &ProductId,
    file: &CollectableFile,
) -> Result<Option<RequirementSchema>, anyhow::Error> {
    let media_type =
        mime_guess::from_ext(file.filepath.extension().unwrap_or_default()).first_raw();

    match media_type {
        Some("text/markdown") => markdown::collect_requirements(product_id, file),
        _ => {
            log::error!(
                "Requirement definition collection is not supported from file '{}'",
                file.filepath
            );

            Ok(None)
        }
    }
}
