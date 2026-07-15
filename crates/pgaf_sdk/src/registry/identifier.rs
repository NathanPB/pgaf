use super::{RE_VALID_NAMESPACE_AND_ID, RE_VALID_NAMESPACE_OR_ID, namespace::Namespace};
use serde::de::DeserializeSeed;
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum PublicIdentifierError {
    #[error("Illegal identifier name '{0}'")]
    IllegalName(String),
    #[error("Illegal identifier format '{0}'")]
    BadFormat(String),
}

/// Represents a combination of [`super::Namespace`] and [`super::PublicIdentifier`].
#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub struct PublicIdentifier {
    pub namespace: String,
    pub id: String,
}

impl PublicIdentifier {
    /// Creates a new [`PublicIdentifier`] under the given `namespace` and `id`.
    ///
    /// # Fail Cases:
    /// Fails with [`PublicIdentifierError::IllegalName`] if either the namespace or the id doesn't
    /// follow the naming rules.
    pub fn build(namespace: &str, id: &str) -> Result<Self, PublicIdentifierError> {
        if !RE_VALID_NAMESPACE_OR_ID.is_match(namespace) {
            return Err(PublicIdentifierError::IllegalName(namespace.to_string()));
        }

        if !RE_VALID_NAMESPACE_OR_ID.is_match(id) {
            return Err(PublicIdentifierError::IllegalName(id.to_string()));
        }

        Ok(Self {
            namespace: namespace.to_string(),
            id: id.to_string(),
        })
    }

    pub fn from_with_default_namespace(
        id: &str,
        default_namespace: &str,
    ) -> Result<PublicIdentifier, PublicIdentifierError> {
        let captures = RE_VALID_NAMESPACE_AND_ID
            .captures(id)
            .ok_or_else(|| PublicIdentifierError::BadFormat(id.to_string()))?;

        let namespace = captures
            .name("ns")
            .map(|m| m.as_str())
            .unwrap_or(default_namespace);

        let parsed_id = captures.name("id").unwrap().as_str();

        Ok(PublicIdentifier {
            namespace: namespace.to_string(),
            id: parsed_id.to_string(),
        })
    }
}

impl std::fmt::Display for PublicIdentifier {
    /// Formats the [`PublicIdentifier`] as `namespace:id`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.namespace, self.id)
    }
}

impl Serialize for PublicIdentifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Used to deserialize [`PublicIdentifier`] from a string. This is necessary to allow for the use of default namespaces case the namespace is not provided in the deserializable string.
#[derive(Clone)]
pub struct PublicIdentifierSeed {
    pub default_namespace: String,
}

impl<'de> DeserializeSeed<'de> for PublicIdentifierSeed {
    type Value = PublicIdentifier;

    /// Deserializes a string into a [`PublicIdentifier`]. The `self.default_namespace` value is used as the default namespace if the namespace is not provided in the string.
    ///
    /// E.g.
    /// - `foo:bar`     -> Namespace=``foo``, Id=``bar``
    /// - `bar`         -> Namespace=`self.default_namespace`, Id=``bar``
    ///
    /// For further details on formatting, check [`RE_VALID_NAMESPACE_OR_ID`].
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        match PublicIdentifier::from_with_default_namespace(&s, &self.default_namespace) {
            Ok(id) => Ok(id),
            Err(PublicIdentifierError::IllegalName(id)) => {
                let msg = format!(
                    "Identifier '{id}' is badly named: Only lowercase alphanumeric and dashes are allowed."
                );
                Err(serde::de::Error::custom(msg))
            }
            Err(PublicIdentifierError::BadFormat(id)) => {
                let msg = format!(
                    "Identifier '{id}' doesn't follow the format of `<namespace>:<id>`. Examples are `foo:bar` or `bar` (assumed to be in the default namespace `{}`).",
                    self.default_namespace
                );
                Err(serde::de::Error::custom(msg))
            }
        }
    }
}

impl Namespace {
    /// Creates a new [`Identifier`] under the current namespace with the given `id`.
    /// Due to ergonomics, this doesn't check the `id` formatting (see [`RE_VALID_NAMESPACE_OR_ID`]). Instead, the value is checked when written to the [`Registry`].
    pub fn id(&self, id: &str) -> Result<PublicIdentifier, PublicIdentifierError> {
        PublicIdentifier::build(self, id)
    }
}

#[cfg(test)]
mod tests {
    use crate::registry::{PublicIdentifier, identifier::PublicIdentifierError};

    #[test]
    fn build_valid_identifier() {
        assert_eq!(
            PublicIdentifier::build("foo", "bar").unwrap(),
            PublicIdentifier {
                namespace: "foo".to_string(),
                id: "bar".to_string()
            }
        )
    }

    #[test]
    fn build_invalid_namespace() {
        assert_eq!(
            PublicIdentifier::build("inv@lid", "bar").unwrap_err(),
            PublicIdentifierError::IllegalName("inv@lid".to_string())
        )
    }

    #[test]
    fn build_invalid_id() {
        assert_eq!(
            PublicIdentifier::build("foo", "inv@lid").unwrap_err(),
            PublicIdentifierError::IllegalName("inv@lid".to_string())
        )
    }

    #[test]
    fn display_identifier() {
        assert_eq!(
            PublicIdentifier::build("foo", "bar").unwrap().to_string(),
            "foo:bar".to_string()
        )
    }

    #[test]
    fn from_with_default_namespace_explicit() {
        assert_eq!(
            PublicIdentifier::from_with_default_namespace("foo:bar", "std").unwrap(),
            PublicIdentifier::build("foo", "bar").unwrap()
        )
    }

    #[test]
    fn from_with_default_namespace_implicit() {
        assert_eq!(
            PublicIdentifier::from_with_default_namespace("bar", "std").unwrap(),
            PublicIdentifier::build("std", "bar").unwrap()
        )
    }

    #[test]
    fn from_with_default_namespace_bad_format() {
        assert_eq!(
            PublicIdentifier::from_with_default_namespace("foo::bar", "std").unwrap_err(),
            PublicIdentifierError::BadFormat("foo::bar".to_string())
        )
    }
}
