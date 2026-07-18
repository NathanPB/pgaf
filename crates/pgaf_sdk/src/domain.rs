use crate::data::GeoDeg;
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

/// Allows for streaming [`ExecutionUnit`]s from an undetermined source.
/// The order of the [`ExecutionUnit`]s is not guaranteed, as different file formats may index their data differently, and pre-sorting is not possible.
pub trait DomainGenerator: Iterator<Item = ExecutionUnit> {}
impl<T: Iterator<Item = ExecutionUnit>> DomainGenerator for T {}

/// Trait for constructing a [`DomainGenerator`] from a deserialized config.
pub trait DomainGeneratorCreate<C>: Send + Sync {
    type Generator: DomainGenerator + 'static;
    fn create(config: C) -> Result<Self::Generator, Box<dyn Error>>;
}

pub struct DomainGeneratorDriverTyped<F: DomainGeneratorCreate<C>, C> {
    _marker_f: PhantomData<F>,
    _marker_c: PhantomData<C>,
}

impl<F: DomainGeneratorCreate<C> + 'static, C: DeserializeOwned + 'static> Default
    for DomainGeneratorDriverTyped<F, C>
{
    fn default() -> Self {
        Self {
            _marker_f: PhantomData,
            _marker_c: PhantomData,
        }
    }
}

impl<F: DomainGeneratorCreate<C> + 'static, C: DeserializeOwned + 'static>
    DomainGeneratorDriverTyped<F, C>
{
    pub fn coerce_to_dynamic(self) -> Driver {
        Driver(create_impl::<F, C>)
    }
}

type CreateFn = fn(serde_json::Value) -> Result<Box<dyn DomainGenerator>, Box<dyn Error>>;

fn create_impl<F: DomainGeneratorCreate<C>, C: DeserializeOwned>(
    config_json: serde_json::Value,
) -> Result<Box<dyn DomainGenerator>, Box<dyn Error>> {
    let config: C = serde_json::from_value(config_json)?;
    let generator = F::create(config)?;
    Ok(Box::new(generator))
}

#[derive(Clone)]
pub struct Driver(CreateFn);

impl Driver {
    pub fn create(
        &self,
        config_json: serde_json::Value,
    ) -> Result<Box<dyn DomainGenerator>, Box<dyn Error>> {
        self.0(config_json)
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
