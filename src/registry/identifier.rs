use crate::registry::RE_VALID_NAMESPACE_AND_ID;
use serde::de::DeserializeSeed;
use serde::{Deserialize, Serialize};

/// Represents a combination of [`super::Namespace`] and [`super::Identifier`]. The different between this and [`super::Identifier`] is that
///  [`super::Identifier`] requires the namespace to be in the [`super::Namespace`] struct, that is supposed to be private and owned by the plugin/extension.
/// [`PublicIdentifier`], however, allows for any part of the program to encapsulate a namespace and ID.
#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub struct PublicIdentifier {
    pub namespace: String,
    pub id: String,
}

impl PublicIdentifier {
    /// Creates a new [`PublicIdentifier`] under the given `namespace` and `id`.
    pub fn new(namespace: String, id: String) -> Self {
        Self { namespace, id }
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
        let captures = RE_VALID_NAMESPACE_AND_ID
            .captures(&s)
            .ok_or(serde::de::Error::custom(format!( "Identifier must be in the format of `<namespace>:<id>`. Examples are `foo:bar` or `bar` (assumed to be in the default namespace `{}`).", self.default_namespace)))?;

        let namespace = captures
            .name("ns")
            .map(|m| m.as_str())
            .unwrap_or(self.default_namespace.as_str());

        let id = captures.name("id").map(|m| m.as_str()).unwrap(); // The regex ensures that "id" exists. If it doesn't, it's a good reason to panic.

        Ok(PublicIdentifier {
            namespace: namespace.to_string(),
            id: id.to_string(),
        })
    }
}
