use super::{FunctionArgParseError, FunctionArgs};
use crate::context::{Context, ContextValue, PrimitiveContextValue};
use serde::de::value::StringDeserializer;
use serde::de::{DeserializeSeed, MapAccess, Visitor};
use serde::{Deserializer, forward_to_deserialize_any};

pub fn deserialize_args<A: serde::de::DeserializeOwned>(
    map: FunctionArgs,
    ctx: &Context,
) -> Result<A, FunctionArgParseError> {
    A::deserialize(ContextMapDeserializer { map, ctx })
}

struct ContextMapDeserializer<'ctx> {
    map: FunctionArgs,
    ctx: &'ctx Context,
}

impl<'de, 'ctx> Deserializer<'de> for ContextMapDeserializer<'ctx> {
    type Error = FunctionArgParseError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(ContextMapAccess {
            iter: self.map.into_iter(),
            ctx: self.ctx,
            current_value: None,
        })
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct enum identifier ignored_any
    }
}

struct ContextMapAccess<'ctx> {
    iter: std::collections::hash_map::IntoIter<String, ContextValue>,
    ctx: &'ctx Context,
    current_value: Option<ContextValue>,
}

impl<'de, 'ctx> MapAccess<'de> for ContextMapAccess<'ctx> {
    type Error = FunctionArgParseError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.current_value = Some(value);
                seed.deserialize(StringDeserializer::<FunctionArgParseError>::new(key))
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let cv = self
            .current_value
            .take()
            .expect("next_value_seed called before next_key_seed");
        let prim = cv.to_prim(self.ctx)?;
        seed.deserialize(PrimDeserializer(prim))
    }
}

struct PrimDeserializer(PrimitiveContextValue);

impl<'de> Deserializer<'de> for PrimDeserializer {
    type Error = FunctionArgParseError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.0 {
            PrimitiveContextValue::Bool(b) => visitor.visit_bool(b),
            PrimitiveContextValue::Int(i) => visitor.visit_i64(i),
            PrimitiveContextValue::Float(f) => visitor.visit_f64(f),
            PrimitiveContextValue::String(s) => visitor.visit_string(s),
            PrimitiveContextValue::Null => visitor.visit_unit(),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}
