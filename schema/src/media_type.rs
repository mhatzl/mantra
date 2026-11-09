use std::{path::Path, str::FromStr};

use mime_guess::Mime;
use serde::ser::SerializeSeq;

use crate::path::RelativePath;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, schemars::JsonSchema)]
#[schemars(try_from = "Option<Vec<String>>", into = "Option<Vec<String>>")]
pub struct MediaType(Option<Vec<Mime>>);

impl MediaType {
    pub fn new(mimes: Vec<Mime>) -> Self {
        Self(Some(mimes))
    }

    pub fn from_rel_path(filepath: &RelativePath) -> Self {
        Self::from_opt_ext(filepath.extension())
    }

    pub fn from_path(filepath: &Path) -> Self {
        Self::from_opt_ext(filepath.extension().and_then(|os_str| os_str.to_str()))
    }

    pub fn from_ext(extension: &str) -> Self {
        Self::from_opt_ext(Some(extension))
    }

    fn from_opt_ext(extension: Option<&str>) -> Self {
        Self(extension.and_then(|ext| {
            let m: Vec<_> = mime_guess::from_ext(ext).into_iter().collect();

            if m.is_empty() { None } else { Some(m) }
        }))
    }

    pub fn mimes(&self) -> Option<&[Mime]> {
        self.0.as_deref()
    }
}

impl From<&RelativePath> for MediaType {
    fn from(value: &RelativePath) -> Self {
        Self::from_rel_path(value)
    }
}

impl From<&Path> for MediaType {
    fn from(value: &Path) -> Self {
        Self::from_path(value)
    }
}

impl FromStr for MediaType {
    type Err = mime_guess::mime::FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mime = mime_guess::Mime::from_str(s)?;

        Ok(Self(Some(vec![mime])))
    }
}

impl serde::Serialize for MediaType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.0 {
            Some(mimes) => {
                let mut seq = serializer.serialize_seq(Some(mimes.len()))?;

                for mime in mimes {
                    let raw_mime = mime.to_string();
                    seq.serialize_element(&raw_mime)?;
                }

                seq.end()
            }
            None => serializer.serialize_none(),
        }
    }
}

impl<'de> serde::Deserialize<'de> for MediaType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        fn deserialize_opt_str_or_list<'de, D>(deserializer: D) -> Result<MediaType, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct OptStrOrVec;

            impl<'de> serde::de::Visitor<'de> for OptStrOrVec {
                type Value = MediaType;

                fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    formatter.write_str(
                        "optional string or list of strings that are valid MIME/Media Types",
                    )
                }

                fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    MediaType::from_str(value).map_err(|e| E::custom(e))
                }

                fn visit_unit<E>(self) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(MediaType(None))
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::SeqAccess<'de>,
                {
                    let mut ms = if let Some(size) = seq.size_hint() {
                        Vec::with_capacity(size)
                    } else {
                        Vec::new()
                    };

                    while let Some(element) = seq.next_element::<String>()? {
                        let mime = mime_guess::Mime::from_str(&element)
                            .map_err(serde::de::Error::custom)?;
                        ms.push(mime);
                    }

                    if ms.is_empty() {
                        Ok(MediaType(None))
                    } else {
                        Ok(MediaType(Some(ms)))
                    }
                }
            }

            deserializer.deserialize_any(OptStrOrVec)
        }

        deserialize_opt_str_or_list(deserializer)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use mime_guess::Mime;

    use crate::media_type::MediaType;

    #[test]
    fn mediatype_des_null_value() {
        let m: MediaType = serde_json::from_str(r#"null"#).unwrap();

        assert_eq!(m.0, None, "Null value should be 'None'");
    }

    #[test]
    fn mediatype_des_str_valid_value() {
        let m: MediaType = serde_json::from_str(r#""text/plain""#).unwrap();

        let exp_mime = Mime::from_str("text/plain").unwrap();
        assert_eq!(
            m.0,
            Some(vec![exp_mime]),
            "Deserialize valid string to Mime"
        );
    }

    #[test]
    fn mediatype_des_str_invalid_value() {
        let err: Result<MediaType, _> = serde_json::from_str(r#""bad MIME/Media Type""#);

        assert!(err.is_err(), "Deserialize invalid string must fail");
    }

    #[test]
    fn mediatype_des_list_valid_values() {
        let m: MediaType = serde_json::from_str(r#"["text/plain", "text/html"]"#).unwrap();

        let exp_mime_1 = Mime::from_str("text/plain").unwrap();
        let exp_mime_2 = Mime::from_str("text/html").unwrap();
        assert_eq!(
            m.0,
            Some(vec![exp_mime_1, exp_mime_2]),
            "Deserialize valid list of strings to Mime"
        );
    }
}
