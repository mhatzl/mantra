use anyhow::bail;
use mantra_schema::{
    Properties,
    product::{Product, ProductId},
};

use crate::cmd::collect::cfg::{
    CollectAnnotationsConfig, CollectLsifConfig, CollectRequirementsConfig, CollectReviewsConfig,
    CollectTestRunsConfig,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MantraConfigFile {
    #[serde(alias = "product")]
    pub products: Vec<ProductConfig>,
    #[serde(flatten)]
    pub inheritable_product_cfg: InheritableProductConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InheritableProductConfig {
    /// The name inheritable for all products.
    ///
    /// TODO: map to requirement
    pub name: Option<String>,
    /// Optional baseline inheritable for all products.
    ///
    /// TODO: map to requirement
    pub base: Option<String>,
    /// Optional version inheritable for all products.
    ///
    /// TODO: map to requirement
    pub version: Option<String>,
    /// Optional link to the homepage inheritable for all products.
    ///
    /// TODO: map to requirement
    pub homepage: Option<String>,
    /// Optional link to the repository inheritable for all products.
    ///
    /// TODO: map to requirement
    pub repository: Option<String>,
    /// Optional license inheritable for all products.
    ///
    /// TODO: map to requirement
    pub license: Option<String>,
    /// Optional description inheritable for all products.
    ///
    /// TODO: map to requirement
    pub description: Option<String>,
    /// Optional properties inheritable for all products.
    ///
    /// TODO: map to requirement
    pub properties: Option<Properties>,
}

impl InheritableProductConfig {
    pub fn check_validity(&self) -> Result<(), anyhow::Error> {
        if let Some(name) = &self.name
            && !valid_non_inheritable(name)
        {
            bail!("The general name cannot inherit its value");
        } else if let Some(base) = &self.base
            && !valid_non_inheritable(base)
        {
            bail!("The general baseline cannot inherit its value");
        } else if let Some(version) = &self.version
            && !valid_non_inheritable(version)
        {
            bail!("The general version cannot inherit its value");
        } else if let Some(homepage) = &self.homepage
            && !valid_non_inheritable(homepage)
        {
            bail!("The general homepage cannot inherit its value");
        } else if let Some(repository) = &self.repository
            && !valid_non_inheritable(repository)
        {
            bail!("The general repository cannot inherit its value");
        } else if let Some(license) = &self.license
            && !valid_non_inheritable(license)
        {
            bail!("The general license cannot inherit its value");
        } else if let Some(description) = &self.description
            && !valid_non_inheritable(description)
        {
            bail!("The general description cannot inherit its value");
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProductConfig {
    #[serde(flatten)]
    pub product: ProductDataConfig,
    #[serde(default, alias = "requirement")]
    pub requirements: Vec<CollectRequirementsConfig>,
    #[serde(default, alias = "annotation")]
    pub annotations: Vec<CollectAnnotationsConfig>,
    #[serde(default, alias = "language_server_index_format")]
    pub lsif: Vec<CollectLsifConfig>,
    #[serde(default, alias = "test-run")]
    pub test_runs: Vec<CollectTestRunsConfig>,
    #[serde(default, alias = "review")]
    pub reviews: Vec<CollectReviewsConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProductDataConfig {
    /// The product ID.
    ///
    /// TODO: map to requirement
    pub id: Option<ProductId>,
    /// The name of the product.
    /// May be inherited by setting "$inherit".
    ///
    /// TODO: map to requirement
    pub name: Inheritable<String>,
    /// Optional baseline of the product.
    /// e.g. git branch or commit hash
    ///
    /// TODO: map to requirement
    pub base: InheritableOption<String>,
    /// Optional version of the product.
    /// May be inherited by setting "$inherit".
    ///
    /// TODO: map to requirement
    pub version: InheritableOption<String>,
    /// Optional link to the homepage of the product.
    /// May be inherited by setting "$inherit".
    ///
    /// TODO: map to requirement
    pub homepage: InheritableOption<String>,
    /// Optional link to the repository of the product.
    /// May be inherited by setting "$inherit".
    ///
    /// TODO: map to requirement
    pub repository: InheritableOption<String>,
    /// Optional license of the product.
    /// May be inherited by setting "$inherit".
    ///
    /// TODO: map to requirement
    pub license: InheritableOption<String>,
    /// Optional description of the product.
    /// May be inherited by setting "$inherit".
    ///
    /// TODO: map to requirement
    pub description: InheritableOption<String>,
    /// Optional properties of the product.
    /// May be inherited by setting "$inherit".
    ///
    /// TODO: map to requirement
    pub properties: InheritableOption<Properties>,
}

impl ProductDataConfig {
    pub fn to_product(
        self,
        inheritable_cfg: &InheritableProductConfig,
    ) -> Result<Product, anyhow::Error> {
        let name = resolve_product_name(self.name, &inheritable_cfg.name)?;
        let base = resolve_optional_inheritable(self.base, &inheritable_cfg.base)?;
        if let Some(base) = &base {
            valid_product_base(base)?;
        }

        let id = resolve_product_id(self.id, &name, base.as_deref())?;

        Ok(Product {
            id,
            name,
            base,
            version: resolve_optional_inheritable(self.version, &inheritable_cfg.version)?,
            homepage: resolve_optional_inheritable(self.homepage, &inheritable_cfg.homepage)?,
            repository: resolve_optional_inheritable(self.repository, &inheritable_cfg.repository)?,
            license: resolve_optional_inheritable(self.license, &inheritable_cfg.license)?,
            description: resolve_optional_inheritable(
                self.description,
                &inheritable_cfg.description,
            )?,
            properties: resolve_optional_inheritable(self.properties, &inheritable_cfg.properties)?,
        })
    }
}

const NAME_BASE_DIVIDER: &str = "@";

/// The product ID will be `<product name>@<product base>`
/// in case it is not explicitly given.
/// Therefore, neither product name nor base must contain `@`.
fn resolve_product_id(
    id: Option<ProductId>,
    name: &str,
    base: Option<&str>,
) -> Result<ProductId, anyhow::Error> {
    if let Some(id) = id {
        if valid_non_inheritable(&id) {
            Ok(id)
        } else {
            bail!("The product ID cannot be inherited!");
        }
    } else if let Some(base) = base {
        Ok(ProductId::new(format!("{name}@{}", base))?)
    } else {
        bail!("Project baseline must be set if no ID is given.")
    }
}

fn resolve_product_name(
    name: Inheritable<String>,
    inheritable_value: &Option<String>,
) -> Result<String, anyhow::Error> {
    let resolved_name = match name {
        Inheritable::Inherited => {
            if let Some(value) = inheritable_value {
                value.to_string()
            } else {
                bail!("Missing inheritable product name");
            }
        }
        Inheritable::Value(value) => value,
    };

    if resolved_name.contains(NAME_BASE_DIVIDER) {
        bail!("The product name must not contain '{}'", NAME_BASE_DIVIDER);
    }

    Ok(resolved_name)
}

fn resolve_optional_inheritable<T: Clone>(
    value: InheritableOption<T>,
    inheritable: &Option<T>,
) -> Result<Option<T>, anyhow::Error> {
    match value {
        Some(value) => match value {
            Inheritable::Inherited => {
                if let Some(inherited_value) = inheritable {
                    Ok(Some(inherited_value.clone()))
                } else {
                    bail!("Missing inheritable value!");
                }
            }
            Inheritable::Value(explicit_value) => Ok(Some(explicit_value)),
        },
        None => Ok(None),
    }
}

fn valid_non_inheritable(value: &str) -> bool {
    value != INHERIT_MARKER
}

fn valid_product_base(base: &str) -> Result<(), anyhow::Error> {
    if !valid_non_inheritable(base) {
        bail!("Product base cannot be inherited!");
    } else if base.contains(NAME_BASE_DIVIDER) {
        bail!("Product base must not contain '{}'", NAME_BASE_DIVIDER);
    }

    Ok(())
}

pub type InheritableOption<T> = Option<Inheritable<T>>;
const INHERIT_MARKER: &str = "$inherit";

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Inheritable<T> {
    #[serde(rename = "$inherit")]
    Inherited,
    #[serde(untagged)]
    Value(T),
}

#[derive(clap::Parser)]
pub struct CliConfig {
    #[command(flatten)]
    pub db: crate::db::Config,
    #[arg(long, default_value = "mantra.json5", alias = "config")]
    pub config_filepath: std::path::PathBuf,
    #[command(subcommand)]
    pub cmd: crate::cmd::Cmd,
}

#[cfg(test)]
mod tests {
    use mantra_schema::Properties;

    use crate::cfg::{INHERIT_MARKER, Inheritable, ProductDataConfig};

    #[test]
    fn inheritable_cfg() {
        let content = serde_json::json!(["some-value", INHERIT_MARKER, "other-value"]);

        let inheritable_strings: Vec<Inheritable<String>> =
            serde_json::from_value(content).unwrap();

        assert_eq!(inheritable_strings.len(), 3);
        assert_eq!(
            inheritable_strings[0],
            Inheritable::Value("some-value".to_string())
        );
        assert_eq!(inheritable_strings[1], Inheritable::Inherited);
        assert_eq!(
            inheritable_strings[2],
            Inheritable::Value("other-value".to_string())
        );
    }

    #[test]
    fn inheritable_explicit_props_cfg() {
        let explicit_props = serde_json::json!({
            "name": INHERIT_MARKER,
            "base": "<explicit-base>",
            "version": INHERIT_MARKER,
            "license": "<explicit licenese>",
            "properties": {
                "key": "value"
            }
        });

        let product_cfg: ProductDataConfig = serde_json::from_value(explicit_props).unwrap();

        assert_eq!(product_cfg.id, None);
        assert_eq!(product_cfg.name, Inheritable::Inherited);
        assert_eq!(
            product_cfg.base,
            Some(Inheritable::Value("<explicit-base>".to_string()))
        );
        assert_eq!(product_cfg.version, Some(Inheritable::Inherited));
        assert_eq!(
            product_cfg.license,
            Some(Inheritable::Value("<explicit licenese>".to_string()))
        );
        assert_eq!(product_cfg.description, None);

        let mut expected_props = Properties::new();
        expected_props.insert(
            "key".to_string(),
            serde_json::Value::String("value".to_string()),
        );
        assert_eq!(
            product_cfg.properties,
            Some(Inheritable::Value(expected_props))
        );
    }

    #[test]
    fn inherited_props_cfg() {
        let inherited_props = serde_json::json!({
            "name": INHERIT_MARKER,
            "base": "<explicit-base>",
            "version": "<explicit version>",
            "license": INHERIT_MARKER,
            "properties": INHERIT_MARKER
        });

        let product_cfg: ProductDataConfig = serde_json::from_value(inherited_props).unwrap();

        assert_eq!(product_cfg.id, None);
        assert_eq!(product_cfg.name, Inheritable::Inherited);
        assert_eq!(
            product_cfg.base,
            Some(Inheritable::Value("<explicit-base>".to_string()))
        );
        assert_eq!(
            product_cfg.version,
            Some(Inheritable::Value("<explicit version>".to_string()))
        );
        assert_eq!(product_cfg.license, Some(Inheritable::Inherited));
        assert_eq!(product_cfg.description, None);
        assert_eq!(product_cfg.properties, Some(Inheritable::Inherited));
    }

    #[test]
    fn id_uninheritable() {
        let inherited_props = serde_json::json!({
            "id": INHERIT_MARKER,
            "name": "some-product",
            "base": INHERIT_MARKER,
        });

        let product_cfg: ProductDataConfig = serde_json::from_value(inherited_props).unwrap();

        // Note: Conversion `to_product` must then fail if keyword `$inherit` is used
        // on non-inheritable fields.
        assert_eq!(product_cfg.id.as_deref(), Some(&INHERIT_MARKER.to_string()));
        assert_eq!(
            product_cfg.name,
            Inheritable::Value("some-product".to_string())
        );
        assert_eq!(product_cfg.base, Some(Inheritable::Inherited));
        assert_eq!(product_cfg.description, None);
        assert_eq!(product_cfg.properties, None);
    }
}
