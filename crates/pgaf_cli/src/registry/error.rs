use super::*;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum RegistryError {
    #[error("Identifier {0} is already registered.")]
    AlreadyRegistered(PublicIdentifier),
    #[error("Namespace {0} is already claimed.")]
    NamespaceAlreadyClaimed(Namespace),
    #[error("The provided name is empty or contains illegal characters. Only lowercase alphanumeric and dash characters are allowed.")]
    IllegalName(String),
}
