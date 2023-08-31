//! Handles *wiki-link* requirements.
//!
//! [req:wiki.link]

use std::path::Path;

use crate::req::ReqId;

use super::Wiki;

impl Wiki {
    /// Sets the given URL prefix as a required prefix for *wiki-links*.
    pub fn set_url_prefix(&mut self, url_prefix: String) {
        self.wiki_url_prefix = Some(url_prefix);
    }

    /// Returns `true` if the given link is a valid *wiki-link* for the given requirement ID.
    /// Otherwise, returns `false` to indicate that the given link is invalid.
    ///
    /// **This function may return the following `Error`:**
    ///
    /// - [`WikiLinkError::MissingUrlPrefix`]
    /// - [`WikiLinkError::ReqNotInWiki`]
    /// - [`WikiLinkError::UnsupportedWiki`]
    ///
    /// [req:wiki.link.check]
    pub fn is_valid_link(&self, req_id: &ReqId, link: &str) -> Result<bool, WikiLinkError> {
        let url_prefix = self
            .wiki_url_prefix
            .as_deref()
            .ok_or(WikiLinkError::MissingUrlPrefix)?;
        let req = self
            .req(req_id)
            .ok_or(WikiLinkError::ReqNotInWiki(req_id.clone()))?;

        let filepath = &req.filepath;
        let heading = format!("{}: {}", req_id, &req.head.title);

        match self.kind {
            super::WikiKind::GitHub => {
                Ok(is_valid_github_link(url_prefix, filepath, &heading, link))
            }
            _ => Err(WikiLinkError::UnsupportedWiki),
        }
    }

    /// This function tries to create a *wiki-link* for the given requirement.
    ///
    /// **This function may return the following `Error`:**
    ///
    /// - [`WikiLinkError::MissingUrlPrefix`]
    /// - [`WikiLinkError::ReqNotInWiki`]
    /// - [`WikiLinkError::UnsupportedWiki`]
    ///
    /// [req:wiki.link.update]
    pub fn wiki_link(&self, req_id: &ReqId) -> Result<String, WikiLinkError> {
        let url_prefix = self
            .wiki_url_prefix
            .as_deref()
            .ok_or(WikiLinkError::MissingUrlPrefix)?;
        let req = self
            .req(req_id)
            .ok_or(WikiLinkError::ReqNotInWiki(req_id.clone()))?;

        let filepath = &req.filepath;
        let heading = format!("{}: {}", req_id, &req.head.title);

        match self.kind {
            super::WikiKind::GitHub => Ok(github_wiki_link(url_prefix, filepath, &heading)),
            _ => Err(WikiLinkError::UnsupportedWiki),
        }
    }
}

fn is_valid_github_link(url_prefix: &str, filepath: &Path, heading: &str, link: &str) -> bool {
    link == github_wiki_link(url_prefix, filepath, heading)
}

fn github_wiki_link(url_prefix: &str, filepath: &Path, heading: &str) -> String {
    let filename = filepath
        .file_stem()
        .expect("Requirement had invalid file associated with it in the wiki")
        .to_string_lossy()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join("-");

    let converted_heading = heading
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join("-")
        .to_lowercase()
        // Note: Should handle most cases. Full list of Unicode punctuations would be too big to check against.
        .split([
            '.', '!', '?', ';', ':', ',', '&', '$', '%', '#', '*', '`', '~', '"', '\'', '/', '^',
            '(', ')', '{', '}', '[', ']',
        ])
        .collect::<Vec<&str>>()
        .join("");

    if url_prefix.ends_with('/') {
        format!("{url_prefix}{filename}#{converted_heading}")
    } else {
        format!("{url_prefix}/{filename}#{converted_heading}")
    }
}

/// Errors that may occur while handling *wiki-links*.
#[derive(Debug, thiserror::Error, logid::ErrLogId)]
pub enum WikiLinkError {
    #[error("The '--wiki-url-prefix' option was not set.")]
    MissingUrlPrefix,
    #[error("The requirement ID '{}' was not found in the wiki.", .0)]
    ReqNotInWiki(ReqId),
    #[error("Only GitHub wiki is supported at the moment.")]
    UnsupportedWiki,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_github_link_simple_title() {
        let url_prefix = "https://github.com/mhatzl/mantra/wiki/";
        let filepath =
            std::path::PathBuf::from(r".\\mantra-wiki\5-Requirements\5-REQ-req_id\5-REQ-req_id.md");
        let heading = "req_id: Requirement ID";

        let wiki_link = github_wiki_link(url_prefix, &filepath, heading);

        assert_eq!(
            wiki_link, "https://github.com/mhatzl/mantra/wiki/5-REQ-req_id#req_id-requirement-id",
            "Wiki-link with simple title was not correctly created."
        );
    }

    #[test]
    fn check_github_link_simple_title() {
        let url_prefix = "https://github.com/mhatzl/mantra/wiki/";
        let filepath =
            std::path::PathBuf::from(r".\\mantra-wiki\5-Requirements\5-REQ-req_id\5-REQ-req_id.md");
        let heading = "req_id: Requirement ID";

        assert!(
            is_valid_github_link(
                url_prefix,
                &filepath,
                heading,
                "https://github.com/mhatzl/mantra/wiki/5-REQ-req_id#req_id-requirement-id"
            ),
            "Wiki-link with simple title not detected as correct."
        );
    }

    #[test]
    fn create_github_link_for_sub_requirement() {
        let url_prefix = "https://github.com/mhatzl/mantra/wiki/";
        let filepath = std::path::PathBuf::from(
            r".\mantra-wiki\5-Requirements\5-REQ-req_id\5-REQ-req_id.sub_req_id.md",
        );
        let heading = "req_id.sub_req_id: Sub-requirements for high-level requirements";

        let wiki_link = github_wiki_link(url_prefix, &filepath, heading);

        assert_eq!(
            wiki_link, "https://github.com/mhatzl/mantra/wiki/5-REQ-req_id.sub_req_id#req_idsub_req_id-sub-requirements-for-high-level-requirements",
            "Wiki-link for sub-requirement was not correctly created."
        );
    }
}
