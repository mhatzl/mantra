use std::borrow::Cow;

use sha2::Digest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TargetEncoding {
    Os,
    Url,
}

pub(crate) fn encode(content: &str, target: TargetEncoding) -> String {
    let encoded = urlencoding::encode(content);

    // Allow characters in OS version for better readability
    let encoded = encoded.replace("%20", " ");
    let encoded = encoded.replace("%40", "@");

    if target == TargetEncoding::Url {
        // double encoding re-encode @ and space
        // and to get encoding of percent-encoded chars that are part of the OS path
        urlencoding::encode(&encoded).to_string()
    } else {
        encoded
    }
}

pub(crate) fn limit_str_len<'a>(s: &'a str) -> Cow<'a, str> {
    // Many modern operating systems still limit file/folder names to ~250 characters.
    // We are a bit more restrictive at 200 code points, because that is already quite long.
    const MAX_PATH_PART_LEN: usize = 200;

    if s.len() > MAX_PATH_PART_LEN {
        let mut hash = sha2::Sha256::new();
        hash.update(s.as_bytes());
        Cow::Owned(base16ct::lower::encode_string(&hash.finalize()))
    } else {
        Cow::Borrowed(s)
    }
}
