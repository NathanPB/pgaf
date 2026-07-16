use crate::data::GeoDeg;
use std::any::Any;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

/// Constructs a new [`DomainGenerator`] of type [`G`] from the config [`C`].
#[allow(type_alias_bounds)] // I prefer to keep the constraint here for when this makes its way into stable Rust.
type DomainGeneratorFactory<G: DomainGenerator, C> = Arc<dyn Fn(C) -> Result<G, Box<dyn Error>>>;

/// Deserializes a config of type [`C`] from a [`serde_json::Value`].
type DomainConfigDeserializer<C> =
    Arc<dyn Fn(serde_json::Value) -> Result<C, serde_json::error::Error>>;

/// Allows for streaming [`ExecutionUnit`]s from an undetermined source.
/// The order of the [`ExecutionUnit`]s is not guaranteed, as different file formats may index their data differently, and pre-sorting is not possible.
pub trait DomainGenerator: Iterator<Item = ExecutionUnit> {}
impl<T: Iterator<Item = ExecutionUnit>> DomainGenerator for T {}

pub struct DomainGeneratorDriver<G: DomainGenerator, C> {
    pub create: DomainGeneratorFactory<G, C>,
    pub config_deserializer: DomainConfigDeserializer<C>,
}

impl<G: DomainGenerator, C> Clone for DomainGeneratorDriver<G, C> {
    fn clone(&self) -> Self {
        DomainGeneratorDriver {
            create: self.create.clone(),
            config_deserializer: self.config_deserializer.clone(),
        }
    }
}

impl<G: DomainGenerator, C> DomainGeneratorDriver<G, C> {
    pub fn coerce_to_dynamic(self) -> DomainGeneratorDriver<Box<dyn DomainGenerator>, Box<dyn Any>>
    where
        G: DomainGenerator + 'static,
        C: Any + 'static,
    {
        DomainGeneratorDriver {
            create: Arc::new(move |c: Box<dyn Any>| {
                let config = c
                    .downcast::<C>()
                    .map_err(|_| Box::<dyn Error>::from("Failed to downcast config"))?;
                let concrete_generator = (self.create)(*config)?;
                Ok(Box::new(concrete_generator) as Box<dyn DomainGenerator>)
            }),
            config_deserializer: Arc::new(move |v| {
                let concrete_config = (self.config_deserializer)(v)?;
                Ok(Box::new(concrete_config) as Box<dyn Any>)
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnitId {
    Int(i64),
    BiggusIntus(u64),
    Float(f64),
    Text(Arc<str>),
}

impl fmt::Display for UnitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnitId::Int(v) => write!(f, "{v}"),
            UnitId::BiggusIntus(v) => write!(f, "{v}"),
            UnitId::Float(v) => write!(f, "{v}"),
            UnitId::Text(v) => f.write_str(v),
        }
    }
}

impl From<i8> for UnitId {
    fn from(v: i8) -> Self {
        UnitId::Int(v as i64)
    }
}

impl From<i16> for UnitId {
    fn from(v: i16) -> Self {
        UnitId::Int(v as i64)
    }
}

impl From<i32> for UnitId {
    fn from(v: i32) -> Self {
        UnitId::Int(v as i64)
    }
}

impl From<i64> for UnitId {
    fn from(v: i64) -> Self {
        UnitId::Int(v)
    }
}

impl From<u8> for UnitId {
    fn from(v: u8) -> Self {
        UnitId::Int(v as i64)
    }
}

impl From<u16> for UnitId {
    fn from(v: u16) -> Self {
        UnitId::Int(v as i64)
    }
}

impl From<u32> for UnitId {
    fn from(v: u32) -> Self {
        UnitId::Int(v as i64)
    }
}

impl From<u64> for UnitId {
    fn from(v: u64) -> Self {
        UnitId::BiggusIntus(v)
    }
}

impl From<f32> for UnitId {
    fn from(v: f32) -> Self {
        UnitId::Float(v as f64)
    }
}

impl From<f64> for UnitId {
    fn from(v: f64) -> Self {
        UnitId::Float(v)
    }
}

impl From<String> for UnitId {
    fn from(v: String) -> Self {
        UnitId::Text(Arc::from(v))
    }
}

impl From<&str> for UnitId {
    fn from(v: &str) -> Self {
        UnitId::Text(Arc::from(v))
    }
}

impl From<Arc<str>> for UnitId {
    fn from(v: Arc<str>) -> Self {
        UnitId::Text(v)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionUnit {
    pub id: UnitId,
    pub lon: GeoDeg,
    pub lat: GeoDeg,
}
