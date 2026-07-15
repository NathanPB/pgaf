use super::{PublicIdentifier, PublicIdentifierSeed, Registry, Resource};
use serde::Deserializer;
use serde::de::DeserializeSeed;

pub struct DeserializedResource<T: Resource> {
    pub resource: T,
    pub id: PublicIdentifier,
}

/// Used to deserialize a [`Resource`] from a string.
/// This deserializer delegates the deserialization of registry identifiers to
///  the [`PublicIdentifierSeed`] deserializer.
#[derive(Clone)]
pub struct ResourceSeed<'a, T: Resource> {
    pub registry: &'a Registry<T>,
    pub id_seed: PublicIdentifierSeed,
}

impl<'de, T: Resource + 'de> DeserializeSeed<'de> for ResourceSeed<'de, T> {
    type Value = DeserializedResource<T>;

    /// Deserializes a [`Resource`] from a string.
    /// The string is expected to be a valid [`super::PublicIdentifier`] under the given [`Registry`].
    ///
    /// The deserialized value is a **clone** of the [`Resource`] under the given [`Registry`].
    ///
    /// # Errors
    /// This function fails if the string is not a valid [`PublicIdentifier`] or if the [`Resource`] is not registered under the given [`Registry`].
    ///
    /// # TODO
    /// Maybe it's a good idea to make the Registry store ``Rc<T>`` (or ``Arc<T>``) instead of ``T``, so we avoid cloning the resource.
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let id = self.id_seed.deserialize(deserializer)?;
        let res = self.registry.get(&id).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "Resource under the ID {} is not registered under the given registry.",
                id
            ))
        })?;

        Ok(DeserializedResource {
            id,
            resource: res.clone(),
        })
    }
}
