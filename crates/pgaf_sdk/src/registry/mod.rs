//! Module _registry_ is the scaffolding for extensibility of the engine.
//! It provides registry stores for resources, as well as namespaces that owns the resources and identifiers that identify them.
//!
//! The [`Registry`] stores many different [`Resource`]s, identified by [`Identifier`] (that are scoped by a [`Namespace`]).
//! Finally, the [`Registries`] stores [`Registry`] instances for different kinds of [`Resource`]s, and provides a way to claim a [`Namespace`].
//!
//! At the moment, only one namespace is claimed and is used to register all the resources that are part of the application's standard library.
//! Please note, however, that the amount of resources is **zero** at the moment, as this module is work in progress.

mod identifier;
mod namespace;
mod resources;
mod serialize;

use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

pub use identifier::{PublicIdentifier, PublicIdentifierError, PublicIdentifierSeed};
pub use namespace::Namespace;
pub use resources::{FunctionDriverResource, SiteGeneratorDriverResource};
pub use serialize::{DeserializedResource, ResourceSeed};

/// Validates if the given string is a valid name/id for a [`Namespace`] or [`super::PublicIdentifier`].
pub static RE_VALID_NAMESPACE_OR_ID: LazyLock<regex::Regex> =
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

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum RegistryError {
    #[error("Identifier '{0}' is already registered.")]
    AlreadyRegistered(PublicIdentifier),
    #[error("Namespace '{0}' is already claimed.")]
    NamespaceAlreadyClaimed(String),
    #[error(transparent)]
    PublicIdentifierError(#[from] PublicIdentifierError),
}

/// Used to define valid resources that can be registered on the [`Registry`].
/// Resources must be safe-[`Clone`]able.
pub trait Resource: Sized + Clone {}

/// Stores [`Resource`]s, identified by [`Identifier`], and provides basic operations on them.
pub struct Registry<T: Resource> {
    map: HashMap<(String, String), T>,
}

impl<T: Resource> Registry<T> {
    /// Creates a new blank [`Registry`].
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Registers a [`Resource`] `resource` under the given [`Identifier`] `id`.
    /// Will throw:
    /// - [`IllegalNameError`] if `id` is not a valid name (see [`RE_VALID_NAMESPACE_OR_ID`]).
    /// - [`AlreadyRegisteredError`] if `id` is already registered.
    ///
    /// Returns itself on success, for convenience.
    pub fn register(
        &mut self,
        namespace: &Namespace,
        id: &str,
        resource: T,
    ) -> Result<&mut Self, RegistryError> {
        let identifier = namespace.id(id)?;
        if self.is_registered(&identifier) {
            return Err(RegistryError::AlreadyRegistered(identifier));
        }

        self.map
            .insert((namespace.to_string(), id.to_string()), resource);
        Ok(self)
    }

    /// Checks if there is something registered under the given namespace and id.
    pub fn is_registered(&self, identifier: &PublicIdentifier) -> bool {
        self.map
            .contains_key(&(identifier.namespace.to_string(), identifier.id.to_string()))
    }

    /// Returns the [`Resource`] registered under the given namespace and id, if any.
    pub fn get(&self, identifier: &PublicIdentifier) -> Option<&T> {
        self.map
            .get(&(identifier.namespace.to_string(), identifier.id.to_string()))
    }

    /// Returns the [`Identifier`] of all registered [`Resource`]s.
    pub fn ids(&self) -> Vec<PublicIdentifier> {
        self.map
            .keys()
            .map(|(k1, k2)| {
                // TODO: This can probably be fixed if we make the inner HashMap store
                // PublicIdentifier, there's no reason to store this tuple at all.
                PublicIdentifier::build(k1, k2).expect("Failed to construct PublicIdentifier: Invalid name alredy registered on registry.")
            })
            .collect()
    }

    /// Returns all registered [`Resource`]s.
    pub fn resources(&self) -> Vec<&T> {
        self.map.values().collect()
    }

    /// Returns all registered [`Resource`]s and their [`Identifier`]s.
    pub fn entries(&self) -> Vec<(PublicIdentifier, &T)> {
        self.map
            .iter()
            .map(|((k1, k2), v)| (PublicIdentifier::build(k1, k2).expect("Failed to construct PublicIdentifier: Invalid name alredy registered on registry."), v))
            .collect()
    }

    /// Returns the number of registered [`Resource`]s.
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
        let namespace = Namespace::build(namespace)?;
        if self.namespaces.contains(&namespace) {
            return Err(RegistryError::NamespaceAlreadyClaimed(
                namespace.to_string(),
            ));
        }

        //TODO: I think Registries::Namespaces should be HashSet<String> after all.
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
    fn claim_namespace() {
        assert_eq!(&*Registries::new().claim_namespace("foo").unwrap(), "foo");
    }

    #[test]
    fn dupe_namespace() {
        let mut registries = Registries::new();
        let namespace = registries.claim_namespace("foo").unwrap();
        assert_eq!(&*namespace, "foo");

        assert!(
            registries.claim_namespace("foo").is_err(),
            "Expected to disallow claiming duplicate namespace"
        );
    }

    #[test]
    fn identifier() {
        let mut registries = Registries::new();
        let namespace = registries.claim_namespace("foo").unwrap();
        assert_eq!(namespace.id("bar").unwrap().to_string(), "foo:bar");
    }

    #[derive(Debug, PartialEq, Clone)]
    struct DummyResource;
    impl Resource for DummyResource {}

    #[test]
    fn registry_invalid_id() {
        let namespace = Namespace::build("foo").unwrap();
        let mut reg: Registry<DummyResource> = Registry::new();
        assert!(
            reg.register(&namespace, "inv@lid", DummyResource).is_err(),
            "Expected to disallow invalid id"
        );
    }

    #[test]
    fn register() {
        let namespace = Namespace::build("foo").unwrap();
        let mut reg: Registry<DummyResource> = Registry::new();
        let identifier = namespace.id("bar").unwrap();
        reg.register(&namespace, &identifier.id, DummyResource)
            .unwrap();

        assert_eq!(reg.get(&identifier), Some(&DummyResource));

        assert_eq!(
            reg.get(&identifier),
            reg.get(&PublicIdentifier::build("foo", "bar").unwrap())
        );
        assert_eq!(reg.ids(), vec![namespace.id("bar").unwrap()]);
        assert_eq!(reg.resources(), vec![&DummyResource]);
        assert_eq!(reg.entries(), vec![(identifier, &DummyResource)]);
        assert_eq!(reg.len(), 1);
    }
}
