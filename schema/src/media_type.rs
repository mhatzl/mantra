use std::path::Path;

use crate::path::RelativePath;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    sqlx::Type,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct MediaType(Option<String>);

impl MediaType {
    pub fn new(filepath: &RelativePath) -> Self {
        Self(
            mime_guess::from_ext(filepath.extension().unwrap_or_default())
                .first_raw()
                .map(|m| m.to_string()),
        )
    }

    pub fn from_path(filepath: &Path) -> Self {
        Self(
            mime_guess::from_ext(
                filepath
                    .extension()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default(),
            )
            .first_raw()
            .map(|m| m.to_string()),
        )
    }

    pub fn as_str(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

impl From<&RelativePath> for MediaType {
    fn from(value: &RelativePath) -> Self {
        Self::new(value)
    }
}

impl From<&Path> for MediaType {
    fn from(value: &Path) -> Self {
        Self::from_path(value)
    }
}
