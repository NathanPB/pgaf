use std::ops::Deref;

use super::RE_VALID_NAMESPACE_OR_ID;
use super::identifier::PublicIdentifierError;

/// A namespace is a name that is used to group [`super::PublicIdentifier`]s. It effectively owns the resources that are registered on the [`Registry`].
/// Namespaces are supposed to be PRIVATE to the plugin/extension that owns them. They shouldn't ever be shared with other plugins/extensions.
/// Sharing them would allow other plugins/extensions to register resources impersonating the namespace of the plugin/extension that owns it.
/// A namespace is only instantiated through the [`Registries::claim_namespace`] method.
///
/// TODO:
/// - Probably Namespace being "Clone" is a bad thing since it's supposed to be proof-of-ownership.
#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub struct Namespace {
    namespace: String,
}

impl Namespace {
    pub(super) fn build(id: &str) -> Result<Namespace, PublicIdentifierError> {
        if !RE_VALID_NAMESPACE_OR_ID.is_match(id) {
            return Err(PublicIdentifierError::IllegalName(id.to_string()));
        }

        Ok(Namespace {
            namespace: id.to_string(),
        })
    }
}

impl Deref for Namespace {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.namespace.as_str()
    }
}

impl std::fmt::Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.namespace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_namespace() {
        assert_eq!(&*Namespace::build("valid-123").unwrap(), "valid-123")
    }

    #[test]
    fn invalid_namespace() {
        assert_eq!(
            Namespace::build("inv@lid").unwrap_err(),
            PublicIdentifierError::IllegalName("inv@lid".to_string()),
            "Expected to disallow namespaces with invalid characters"
        )
    }
}
