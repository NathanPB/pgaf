use crate::shared::STD_NAMESPACE;

use pgaf_sdk::registry::{PublicIdentifier, PublicIdentifierSeed};
use serde::Deserializer;
use serde::de::DeserializeSeed;

pub fn deserialize_public_identifier<'de, D>(deserializer: D) -> Result<PublicIdentifier, D::Error>
where
    D: Deserializer<'de>,
{
    let seed = PublicIdentifierSeed {
        default_namespace: STD_NAMESPACE.to_string(),
    };

    seed.deserialize(deserializer)
}
