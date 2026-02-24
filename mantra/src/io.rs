use std::{
    io::Read,
    path::{Path, PathBuf},
};

pub fn sync_read_encoding_independent(
    path: impl AsRef<std::path::Path>,
) -> Result<String, anyhow::Error> {
    let raw_content = std::fs::read(path)?;
    let mut decoder = encoding_rs_io::DecodeReaderBytes::new(raw_content.as_slice());
    let mut content = String::with_capacity(raw_content.len());
    decoder.read_to_string(&mut content)?;
    Ok(content)
}

pub async fn async_read_encoding_independent(
    path: impl AsRef<std::path::Path>,
) -> Result<String, anyhow::Error> {
    let raw_content = tokio::fs::read(path).await?;
    let mut decoder = encoding_rs_io::DecodeReaderBytes::new(raw_content.as_slice());
    let mut content = String::with_capacity(raw_content.len());
    decoder.read_to_string(&mut content)?;
    Ok(content)
}

pub fn deserialize_serde_content<T: serde::de::DeserializeOwned>(
    extension: Option<&str>,
    content: &str,
) -> Result<T, anyhow::Error> {
    match extension {
        Some("toml") => Ok(toml::from_str::<T>(content)?),
        // JSON5 is a superset of JSON, so JSON files are also accepted by JSON5
        Some("json") | Some("json5") => Ok(json5::from_str::<T>(content)?),
        Some(extension) => Ok(json5::from_str::<T>(content).inspect_err(|_| {
            eprintln!(
                "Tried to read content from unsupported extension '{}'",
                extension
            )
        })?),
        None => anyhow::bail!("No extension to determine collector."),
    }
}

pub fn sync_deserialize_from_path<T: serde::de::DeserializeOwned>(
    path: impl AsRef<std::path::Path>,
) -> Result<T, anyhow::Error> {
    let content = sync_read_encoding_independent(path.as_ref())?;
    let extension = path.as_ref().extension().and_then(|ext| ext.to_str());
    deserialize_serde_content(extension, &content)
}

pub async fn async_deserialize_from_path<T: serde::de::DeserializeOwned>(
    path: impl AsRef<std::path::Path>,
) -> Result<T, anyhow::Error> {
    let content = async_read_encoding_independent(path.as_ref()).await?;
    let extension = path.as_ref().extension().and_then(|ext| ext.to_str());
    deserialize_serde_content(extension, &content)
}

pub(crate) fn abs_parent_path(path: &Path) -> Result<PathBuf, anyhow::Error> {
    Ok(std::path::absolute(
        path.parent()
            .map(|p| p.to_path_buf())
            .filter(|p| p != &PathBuf::from(""))
            .unwrap_or(PathBuf::from("./")),
    )?)
}
