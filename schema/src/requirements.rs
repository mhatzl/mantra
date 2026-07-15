use crate::{IdentError, Origin, Properties, product::ProductId};

/// Defines the schema to exchange requirements related information.
/// [req("exchange.requirements.schema")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct RequirementSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    /// Optional product ID to specify the product the requirements are part of.
    /// If this field is not set, the ID of the product whose configuration included this schema is used.
    pub product_id: Option<ProductId>,
    /// List of requirements.
    pub requirements: Vec<Requirement>,
    /// Optional properties related to all requirements in this entry.
    ///
    /// **Note:** If a requirement sets a property key directly,
    /// the value set at the requirement will be taken.
    pub properties: Option<Properties>,
    /// Optional base origin of the requirements in this entry.
    /// e.g. specific branch or commit from a git repository
    pub origin: Option<Origin>,
}

/// Type for a requirement ID.
/// [req("req.id")]
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
pub struct ReqId(String);

impl ReqId {
    pub fn new(id: String) -> Result<Self, IdentError> {
        Ok(Self(id))
    }
}

impl std::ops::Deref for ReqId {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::str::FromStr for ReqId {
    type Err = IdentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ReqId::new(s.to_owned())
    }
}

impl TryFrom<String> for ReqId {
    type Error = IdentError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ReqId::new(value)
    }
}

impl std::fmt::Display for ReqId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// This struct defines the information *mantra* stores about a requirement.
/// [req("req")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Requirement {
    /// ID of the requirement.
    /// [req("req.id")]
    pub id: ReqId,
    /// Optional list of parent requirement IDs.
    /// [req("req.hierarchy.mult_parents")]
    pub parents: Option<Vec<RequirementPk>>,
    /// Title of the requirement.
    /// [req("req.title")]
    pub title: String,
    /// Optional description of the requirement.
    /// [req("req.description")]
    pub description: Option<String>,
    /// Origin where the requirement is defined at.
    /// [req("req.origin")]
    pub origin: Origin,
    /// true: Marks the requirement to require manual verification.
    ///
    /// **Note:** All potential children of such a requirement are also marked
    /// to require manual verification.
    /// [req("req.manual")]
    #[serde(default)]
    pub manual_verification: bool,
    /// true: Marks the requirement to be deprecated.
    ///
    /// **Note:** All potential children of such a requirement are also marked as deprecated.
    /// [req("req.deprecated")]
    #[serde(default)]
    pub deprecated: bool,
    /// true: Instructs mantra to exclude the requirement for the product it is mapped to.
    ///
    /// **Note:** All potential children of such a requirement will also be excluded.
    /// [req("req.exclude")]
    #[serde(default)]
    pub exclude: bool,
    /// true: Instructs mantra to treat the requirement for the product as optional.
    ///
    /// **Note:** All potential children of such a requirement are also marked as optional.
    /// [req("req.optional")]
    #[serde(default)]
    pub optional: bool,
    /// Optional list of requirements that this requirement replaces.
    /// Replacing a requirement is only possible inside the same product and marks replaced requirements as *deprecated*.
    pub replaces: Option<Vec<ReqId>>,
    /// List of custom properties of a requirement.
    /// [req("req.properties")]
    pub properties: Option<Properties>,
}

impl Requirement {
    pub fn new_minimal(id: ReqId, title: String, origin: Origin) -> Self {
        Self {
            id,
            parents: None,
            title,
            description: None,
            origin,
            manual_verification: false,
            deprecated: false,
            exclude: false,
            optional: false,
            replaces: None,
            properties: None,
        }
    }
}

/// This struct defines the primary key to identify requirements for a product.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RequirementPk {
    /// ID of the parent requirement.
    /// [req("req.id")]
    pub id: ReqId,
    /// ID of the product the parent requirement is defined in.
    /// If `None`, the parent is assumed to be defined in the same product as the child requirement.
    pub product_id: Option<ProductId>,
}

impl<'de> serde::Deserialize<'de> for RequirementPk {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        fn deserialize_str_or_obj<'de, D>(deserializer: D) -> Result<RequirementPk, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct StrOrObj;

            impl<'de> serde::de::Visitor<'de> for StrOrObj {
                type Value = RequirementPk;

                fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    formatter.write_str("string or a map with keys `id` and `product_id`")
                }

                fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(RequirementPk {
                        id: ReqId::new(value.to_owned())
                            .map_err(|err| serde::de::Error::custom(err.to_string()))?,
                        product_id: None,
                    })
                }

                fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::MapAccess<'de>,
                {
                    #[derive(serde::Deserialize)]
                    #[serde(field_identifier, rename_all = "snake_case")]
                    enum Field {
                        Id,
                        ProductId,
                    }

                    let mut req_id = None;
                    let mut product_id = None;
                    while let Some(field) = map.next_key()? {
                        match field {
                            Field::Id => {
                                if req_id.is_some() {
                                    return Err(serde::de::Error::duplicate_field("id"));
                                }
                                req_id = Some(map.next_value()?);
                            }
                            Field::ProductId => {
                                if product_id.is_some() {
                                    return Err(serde::de::Error::duplicate_field("product_id"));
                                }
                                product_id = Some(map.next_value()?);
                            }
                        }
                    }
                    let req_id = req_id.ok_or_else(|| serde::de::Error::missing_field("id"))?;
                    let product_id = product_id.unwrap_or_default();
                    Ok(RequirementPk {
                        id: req_id,
                        product_id,
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
    use std::ops::Deref;

    use crate::requirements::RequirementPk;

    #[test]
    fn de_pk_str() {
        let pk: RequirementPk = serde_json::from_str(r#""req-id""#).unwrap();

        assert_eq!(pk.id.deref(), "req-id", "Requirement ID taken from string");
        assert!(
            pk.product_id.is_none(),
            "Product ID ignored when deserializing from string"
        );
    }

    #[test]
    fn de_pk_obj() {
        let pk: RequirementPk = serde_json::from_value(serde_json::json!({
            "id": "req-id",
            "product_id": "product-id"
        }))
        .unwrap();

        assert_eq!(
            pk.id.deref(),
            "req-id",
            "Requirement ID taken from 'id' field"
        );
        assert_eq!(
            pk.product_id.as_deref(),
            Some(&"product-id".to_string()),
            "Product ID taken from 'product-id' field"
        );
    }

    #[test]
    fn de_pk_obj_unknown_field() {
        let pk: Result<RequirementPk, serde_json::Error> =
            serde_json::from_value(serde_json::json!({
                "id": "req-id",
                "product_id": "product-id",
                "other-field": "bad"
            }));

        assert!(pk.is_err());
        assert_eq!(
            pk.unwrap_err().to_string(),
            "unknown field `other-field`, expected `id` or `product_id`"
        );
    }

    #[test]
    fn de_pk_obj_no_product_id() {
        let pk: RequirementPk = serde_json::from_value(serde_json::json!({
            "id": "req-id"
        }))
        .unwrap();

        assert_eq!(
            pk.id.deref(),
            "req-id",
            "Requirement ID taken from 'id' field"
        );
        assert!(pk.product_id.is_none(), "Product ID is optional");
    }
}
