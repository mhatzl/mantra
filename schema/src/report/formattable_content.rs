use serde::ser::SerializeStruct;

use crate::media_type::MediaType;

#[derive(Debug, Clone, PartialEq, Eq, Hash, schemars::JsonSchema)]
pub struct FormattableContent {
    pub content: String,
    pub media_type: Option<MediaType>,
}

impl FormattableContent {
    pub fn new(content: String, media_type: Option<MediaType>) -> Self {
        Self {
            content,
            media_type,
        }
    }
}

impl serde::Serialize for FormattableContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Some(media_type) = &self.media_type {
            let mut obj = serializer.serialize_struct("FormattableContent", 2)?;

            obj.serialize_field("content", &self.content)?;
            obj.serialize_field("media_type", media_type)?;

            obj.end()
        } else {
            serializer.serialize_str(&self.content)
        }
    }
}

impl<'de> serde::Deserialize<'de> for FormattableContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        fn deserialize_str_or_obj<'de, D>(deserializer: D) -> Result<FormattableContent, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct StrOrObj;

            impl<'de> serde::de::Visitor<'de> for StrOrObj {
                type Value = FormattableContent;

                fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    formatter.write_str("string or a map with keys `content` and `media_type`")
                }

                fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(FormattableContent {
                        content: value.to_owned(),
                        media_type: None,
                    })
                }

                fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::MapAccess<'de>,
                {
                    #[derive(serde::Deserialize)]
                    #[serde(field_identifier, rename_all = "snake_case")]
                    enum Field {
                        Content,
                        MediaType,
                    }

                    let mut content = None;
                    let mut media_type = None;
                    while let Some(field) = map.next_key()? {
                        match field {
                            Field::Content => {
                                if content.is_some() {
                                    return Err(serde::de::Error::duplicate_field("content"));
                                }
                                content = Some(map.next_value()?);
                            }
                            Field::MediaType => {
                                if media_type.is_some() {
                                    return Err(serde::de::Error::duplicate_field("media_type"));
                                }
                                media_type = Some(map.next_value()?);
                            }
                        }
                    }
                    let content =
                        content.ok_or_else(|| serde::de::Error::missing_field("content"))?;
                    let media_type = media_type.unwrap_or_default();
                    Ok(FormattableContent {
                        content,
                        media_type,
                    })
                }
            }

            deserializer.deserialize_any(StrOrObj)
        }

        deserialize_str_or_obj(deserializer)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use mime_guess::Mime;

    use crate::{media_type::MediaType, report::formattable_content::FormattableContent};

    #[test]
    fn de_content_str() {
        let fc: FormattableContent =
            serde_json::from_str(r#""Plain content; No media type""#).unwrap();

        assert_eq!(
            fc.content, "Plain content; No media type",
            "Content taken from string"
        );
        assert!(
            fc.media_type.is_none(),
            "Media type ignored when deserializing from string"
        );
    }

    #[test]
    fn de_content_obj() {
        let fc: FormattableContent = serde_json::from_value(serde_json::json!({
            "content": "Content with media type",
            "media_type": "text/plain"
        }))
        .unwrap();

        assert_eq!(
            fc.content, "Content with media type",
            "Content taken from 'content' field"
        );
        assert_eq!(
            fc.media_type,
            Some(MediaType::from_str("text/plain").unwrap()),
            "Media type taken from 'media_type' field"
        );
    }

    #[test]
    fn de_content_obj_list() {
        let fc: FormattableContent = serde_json::from_value(serde_json::json!({
            "content": "Content with media type",
            "media_type": ["text/plain", "text/css", "text/xml"]
        }))
        .unwrap();

        assert_eq!(
            fc.content, "Content with media type",
            "Content taken from 'content' field"
        );
        assert_eq!(
            fc.media_type,
            Some(MediaType::new(vec![
                Mime::from_str("text/plain").unwrap(),
                Mime::from_str("text/css").unwrap(),
                Mime::from_str("text/xml").unwrap()
            ])),
            "Media type taken from 'media_type' field"
        );
    }

    #[test]
    fn de_content_obj_unknown_field() {
        let fc: Result<FormattableContent, serde_json::Error> =
            serde_json::from_value(serde_json::json!({
                "content": "Some content",
                "media_type": "text/plain",
                "other-field": "bad"
            }));

        assert!(fc.is_err());
        assert_eq!(
            fc.unwrap_err().to_string(),
            "unknown field `other-field`, expected `content` or `media_type`"
        );
    }

    #[test]
    fn de_content_obj_no_media_type() {
        let fc: FormattableContent = serde_json::from_value(serde_json::json!({
            "content": "Some content"
        }))
        .unwrap();

        assert_eq!(
            fc.content, "Some content",
            "Content taken from 'content' field"
        );
        assert!(fc.media_type.is_none(), "Media type is optional");
    }
}
