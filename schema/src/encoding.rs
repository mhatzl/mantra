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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TargetEncoding {
    Os,
    Url,
}
