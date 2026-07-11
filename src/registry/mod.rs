//! Module _registry_ is the scaffolding for extensibility of the engine.
//! It provides registry stores for resources, as well as namespaces that owns the resources and identifiers that identify them.
//!
//! The [`Registry`] stores many different [`Resource`]s, identified by [`Identifier`] (that are scoped by a [`Namespace`]).
//! Finally, the [`Registries`] stores [`Registry`] instances for different kinds of [`Resource`]s, and provides a way to claim a [`Namespace`].
//!
//! At the moment, only one namespace is claimed and is used to register all the resources that are part of the application's standard library.
//! Please note, however, that the amount of resources is **zero** at the moment, as this module is work in progress.

pub mod error;
mod identifier;
pub mod itself;
pub mod resources;
mod serialize;

use crate::utils::threehashmap::K2HashMap;
use error::*;
pub use identifier::{PublicIdentifier, PublicIdentifierSeed};
use resources::*;
pub use serialize::ResourceSeed;
use std::collections::HashSet;
use std::sync::LazyLock;

/// Validates if the given string is a valid name/id for a [`Namespace`] or [`Identifier`].
static RE_VALID_NAMESPACE_OR_ID: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^[a-z0-9-]+$").unwrap());

/// Validates if the given string represents a valid namespace and id in the format `namespace:id`.
/// The namespace can be omitted, in which case the default namespace is assumed.
/// E.g.
/// - `foo:bar`     -> Namespace=``foo``, Id=``bar``
/// - `bar`         -> Namespace=<default>, Id=``bar``
/// - `foo:bar:baz` -> Invalid
/// - `foo:`        -> Invalid
/// - `:bar`        -> Invalid
/// - `:`           -> Invalid
///   Any other permutation of Namespace or Id that doesn't match [`RE_VALID_NAMESPACE_OR_ID`] is invalid.
///
///   E.g. `FOO:b@r`  -> Invalid (uppercase or symbols are not allowed)
///
/// Namespace is captured in the group named `ns` and Id is captured in the group named `id`.
pub static RE_VALID_NAMESPACE_AND_ID: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^(?:(?<ns>[a-z0-9._-]+):)?(?<id>[a-z0-9._-]+)$").unwrap());

/// A namespace is a name that is used to group [`Identifier`]s. It effectively owns the resources that are registered on the [`Registry`].
/// Namespaces are supposed to be PRIVATE to the plugin/extension that owns them. They shouldn't ever be shared with other plugins/extensions.
/// Sharing them would allow other plugins/extensions to register resources impersonating the namespace of the plugin/extension that owns it.
/// A namespace is only instantiated through the [`Registries::claim_namespace`] method.
#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub struct Namespace {
    namespace: String,
}

impl Namespace {
    /// Creates a new [`Identifier`] under the current namespace with the given `id`.
    /// Due to ergonomics, this doesn't check the `id` formatting (see [`RE_VALID_NAMESPACE_OR_ID`]). Instead, the value is checked when written to the [`Registry`].
    pub fn id(&self, id: &str) -> PublicIdentifier {
        PublicIdentifier::new(self.namespace.clone(), id.to_string())
    }

    /// Gets the namespace string.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }
}

impl std::fmt::Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.namespace)
    }
}

/// Used to define valid resources that can be registered on the [`Registry`].
/// Resources must be safe-[`Clone`]able.
pub trait Resource: Sized + Clone {}

/// Stores [`Resource`]s, identified by [`Identifier`], and provides basic operations on them.
pub struct Registry<T: Resource> {
    map: K2HashMap<String, String, T>,
}

impl<T: Resource> Registry<T> {
    /// Creates a new blank [`Registry`].
    fn new() -> Self {
        Self {
            map: K2HashMap::new(),
        }
    }

    /// Registers a [`Resource`] `resource` under the given [`Identifier`] `id`.
    /// Will throw:
    /// - [`IllegalNameError`] if `id` is not a valid name (see [`RE_VALID_NAMESPACE_OR_ID`]).
    /// - [`AlreadyRegisteredError`] if `id` is already registered.
    ///
    /// Returns itself on success, for convenience.
    #[allow(dead_code)]
    pub fn register(
        &mut self,
        namespace: &Namespace,
        id: &str,
        resource: T,
    ) -> Result<&mut Self, RegistryError> {
        if !RE_VALID_NAMESPACE_OR_ID.is_match(id) {
            return Err(RegistryError::IllegalName(id.to_string()));
        }

        let identifier = PublicIdentifier::new(namespace.namespace().to_string(), id.to_string());
        if self.is_registered(&identifier) {
            return Err(RegistryError::AlreadyRegistered(identifier));
        }

        self.map.insert(
            namespace.namespace.clone(),
            id.to_string().clone(),
            resource,
        );
        Ok(self)
    }

    /// Checks if there is something registered under the given namespace and id.
    #[allow(dead_code)]
    pub fn is_registered(&self, identifier: &PublicIdentifier) -> bool {
        self.map.contains_key(&identifier.namespace, &identifier.id)
    }

    /// Returns the [`Resource`] registered under the given namespace and id, if any.
    #[allow(dead_code)]
    pub fn get(&self, identifier: &PublicIdentifier) -> Option<&T> {
        self.map.get(&identifier.namespace, &identifier.id)
    }

    /// Returns the [`Identifier`] of all registered [`Resource`]s.
    #[allow(dead_code)]
    pub fn ids(&self) -> Vec<PublicIdentifier> {
        self.map
            .keys()
            .map(|(k1, k2)| PublicIdentifier::new(k1.clone(), k2.clone()))
            .collect()
    }

    /// Returns all registered [`Resource`]s.
    #[allow(dead_code)]
    pub fn resources(&self) -> Vec<&T> {
        self.map.values().collect()
    }

    /// Returns all registered [`Resource`]s and their [`Identifier`]s.
    #[allow(dead_code)]
    pub fn entries(&self) -> Vec<(PublicIdentifier, &T)> {
        self.map
            .iter()
            .map(|(k1, k2, v)| (PublicIdentifier::new(k1.clone(), k2.clone()), v))
            .collect()
    }

    /// Returns the number of registered [`Resource`]s.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

/// Holds the Registries ([`Registry`]) for the existing [`Resource`] types.
/// It also manages claiming of [`Namespace`]s (see [`Registries::claim_namespace`]).
///
/// [`Registries`] must expose mutable and non-mutable access to the [`Registry`]s inside it via
///   functions like [`Registries::regmut_sitegen_drivers`] (``&mut``) and [`Registries::reg_sitegen_drivers`] (``&``).
pub struct Registries {
    namespaces: HashSet<Namespace>,
    reg_sitegen_drivers: Registry<SiteGeneratorDriverResource>,
    reg_function_drivers: Registry<FunctionDriverResource>,
}

impl Registries {
    /// Creates a new instance.
    pub fn new() -> Self {
        Self {
            namespaces: HashSet::new(),
            reg_sitegen_drivers: Registry::new(),
            reg_function_drivers: Registry::new(),
        }
    }

    /// Claims a [`Namespace`] for the given `namespace` string.
    ///
    /// Namespaces are supposed to be claimed only once per plugin/extension.
    /// For instance, the embedded module will claim the `std` namespace upon application startup.
    /// Plugins that wish to extend the functionality and register their own [`Resource`]s will be provided with a namespace for themselves
    /// and shall it to register all of their [`Resource`]s.
    pub fn claim_namespace(&mut self, namespace: &'static str) -> Result<Namespace, RegistryError> {
        if !RE_VALID_NAMESPACE_OR_ID.is_match(namespace) {
            return Err(RegistryError::IllegalName(namespace.to_string()));
        }

        let namespace = Namespace {
            namespace: namespace.to_string(),
        };
        if self.namespaces.contains(&namespace) {
            return Err(RegistryError::NamespaceAlreadyClaimed(namespace));
        }

        self.namespaces.insert(namespace.clone());
        Ok(namespace)
    }

    pub fn reg_sitegen_drivers(&self) -> &Registry<SiteGeneratorDriverResource> {
        &self.reg_sitegen_drivers
    }

    pub fn regmut_sitegen_drivers(&mut self) -> &mut Registry<SiteGeneratorDriverResource> {
        &mut self.reg_sitegen_drivers
    }

    pub fn reg_function_drivers(&self) -> &Registry<FunctionDriverResource> {
        &self.reg_function_drivers
    }

    pub fn regmut_function_drivers(&mut self) -> &mut Registry<FunctionDriverResource> {
        &mut self.reg_function_drivers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace() {
        match Registries::new().claim_namespace("foo") {
            Ok(ns) => assert_eq!(ns.namespace, "foo"),
            Err(_) => panic!("Expected to claim the namespace"),
        }
    }

    #[test]
    fn invalid_namespace() {
        match Registries::new().claim_namespace("inv@lid") {
            Ok(_) => panic!("Expected to disallow namespaces with invalid characters"),
            Err(_) => {}
        }
    }

    #[test]
    fn dupe_namespace() {
        let mut registries = Registries::new();
        let namespace = registries.claim_namespace("foo").unwrap();
        assert_eq!(namespace.namespace, "foo");

        match registries.claim_namespace("foo") {
            Ok(_) => panic!("Expected to disallow claiming duplicate namespace"),
            Err(_) => {}
        }
    }

    #[test]
    fn identifier() {
        let mut registries = Registries::new();
        let namespace = registries.claim_namespace("foo").unwrap();
        assert_eq!(namespace.id("bar").to_string(), "foo:bar");
    }

    #[derive(Debug, PartialEq, Clone)]
    struct DummyResource;
    impl Resource for DummyResource {}

    #[test]
    fn registry_invalid_id() {
        let namespace = Namespace {
            namespace: "foo".to_string(),
        };
        let mut reg: Registry<DummyResource> = Registry::new();
        match reg.register(&namespace, "inv@lid", DummyResource.into()) {
            Ok(_) => panic!("Expected to disallow invalid id"),
            Err(_) => {}
        }
    }

    #[test]
    fn register() {
        let namespace = Namespace {
            namespace: "foo".to_string(),
        };
        let mut reg: Registry<DummyResource> = Registry::new();
        let id = namespace.id("bar");
        reg.register(&namespace, id.id.as_str(), DummyResource.into())
            .unwrap();

        match reg.get(&id) {
            Some(res) => assert_eq!(
                res,
                &DummyResource.into(),
                "Registered and retrieved resources do not match"
            ),
            None => panic!("Expected to find resource"),
        }

        assert_eq!(
            reg.get(&id),
            reg.get(&PublicIdentifier::new("foo".to_string(), "bar".to_string()))
        );
        assert_eq!(reg.ids(), vec![namespace.id("bar")]);
        assert_eq!(reg.resources(), vec![&DummyResource.into()]);
        assert_eq!(reg.entries(), vec![(id, &DummyResource.into())]);
        assert_eq!(reg.len(), 1);
    }
}
