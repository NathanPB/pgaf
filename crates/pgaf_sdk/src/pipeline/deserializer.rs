use super::{PipelineStepTypeArgParseError, PipelineStepTypeArgs};
use crate::context::{Context, ContextValue, PrimitiveContextValue};
use serde::de::value::StringDeserializer;
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserializer, forward_to_deserialize_any};

struct PrimDeserializer(PrimitiveContextValue);

impl<'de> Deserializer<'de> for PrimDeserializer {
    type Error = PipelineStepTypeArgParseError;

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

pub fn deserialize_args<A: serde::de::DeserializeOwned>(
    args: &PipelineStepTypeArgs,
    ctx: &Context,
) -> Result<A, PipelineStepTypeArgParseError> {
    A::deserialize(ArgsDeserializer { args, ctx })
}

struct ArgsDeserializer<'a, 'ctx> {
    args: &'a PipelineStepTypeArgs,
    ctx: &'ctx Context,
}

impl<'de, 'a, 'ctx> Deserializer<'de> for ArgsDeserializer<'a, 'ctx> {
    type Error = PipelineStepTypeArgParseError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.args {
            PipelineStepTypeArgs::One(value) => {
                let prim = value.to_prim(self.ctx)?;
                PrimDeserializer(prim).deserialize_any(visitor)
            }
            PipelineStepTypeArgs::Array(values) => visitor.visit_seq(ContextSeqAccess {
                iter: values.iter(),
                ctx: self.ctx,
            }),
            PipelineStepTypeArgs::Map(map) => visitor.visit_map(ContextMapAccess {
                iter: map.iter(),
                ctx: self.ctx,
                current_value: None,
            }),
        }
    }

    fn deserialize_struct<V>(
        self,
        _n: &'static str,
        _f: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct tuple
        tuple_struct map enum identifier ignored_any
    }
}

struct ContextMapAccess<'a, 'ctx> {
    iter: std::collections::hash_map::Iter<'a, String, ContextValue>,
    ctx: &'ctx Context,
    current_value: Option<&'a ContextValue>,
}

impl<'de, 'a, 'ctx> MapAccess<'de> for ContextMapAccess<'a, 'ctx> {
    type Error = PipelineStepTypeArgParseError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.current_value = Some(value);
                seed.deserialize(StringDeserializer::<PipelineStepTypeArgParseError>::new(
                    key.clone(),
                ))
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

struct ContextSeqAccess<'a, 'ctx> {
    iter: std::slice::Iter<'a, ContextValue>,
    ctx: &'ctx Context,
}

impl<'de, 'a, 'ctx> SeqAccess<'de> for ContextSeqAccess<'a, 'ctx> {
    type Error = PipelineStepTypeArgParseError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let Some(value) = self.iter.next() else {
            return Ok(None);
        };

        let prim = value.to_prim(self.ctx)?;
        seed.deserialize(PrimDeserializer(prim)).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{PipelineStepType, PipelineStepTypeArgs, PipelineStepTypeDriver};
    use crate::context::{Context, ContextValue, PrimitiveContextValue};
    use crate::data::GeoDeg;
    use crate::domain::{ExecutionUnit, UnitId};
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn make_ctx() -> Context {
        make_ctx_with_id(1)
    }

    fn make_ctx_with_id(id: i64) -> Context {
        Context {
            unit: ExecutionUnit {
                id: id.into(),
                lon: GeoDeg::from(0.0),
                lat: GeoDeg::from(0.0),
            },
            data: HashMap::default(),
        }
    }

    fn prim(v: PrimitiveContextValue) -> ContextValue {
        ContextValue::Prim(v)
    }

    fn run(
        driver: super::super::Driver,
        args: PipelineStepTypeArgs,
        ctxs: Vec<Context>,
    ) -> Vec<Context> {
        driver.invoke(Arc::new(args), Box::new(ctxs.into_iter())).collect()
    }

    // --- One ---

    struct StoreScalar;
    impl PipelineStepType<String> for StoreScalar {
        fn invoke(
            stream: Box<dyn Iterator<Item = (String, Context)>>,
        ) -> Box<dyn Iterator<Item = Context>> {
            Box::new(stream.map(|(s, mut ctx)| {
                ctx.data.insert(
                    "result".into(),
                    ContextValue::Prim(PrimitiveContextValue::String(s)),
                );
                ctx
            }))
        }
    }

    #[test]
    fn one_string() {
        let output = run(
            PipelineStepTypeDriver::<StoreScalar, String>::default().coerce_to_dynamic(),
            PipelineStepTypeArgs::One(prim(PrimitiveContextValue::String("hello".into()))),
            vec![make_ctx()],
        );
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0].data.get("result"),
            Some(&prim(PrimitiveContextValue::String("hello".into())))
        );
    }

    #[test]
    fn one_resolves_ident() {
        let mut ctx = make_ctx();
        ctx.data.insert(
            "greeting".into(),
            prim(PrimitiveContextValue::String("world".into())),
        );
        let output = run(
            PipelineStepTypeDriver::<StoreScalar, String>::default().coerce_to_dynamic(),
            PipelineStepTypeArgs::One(ContextValue::Ident("greeting".into())),
            vec![ctx],
        );
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0].data.get("result"),
            Some(&prim(PrimitiveContextValue::String("world".into())))
        );
    }

    // --- Array ---

    struct StoreSum;
    impl PipelineStepType<Vec<i64>> for StoreSum {
        fn invoke(
            stream: Box<dyn Iterator<Item = (Vec<i64>, Context)>>,
        ) -> Box<dyn Iterator<Item = Context>> {
            Box::new(stream.map(|(v, mut ctx)| {
                ctx.data.insert(
                    "result".into(),
                    ContextValue::Prim(PrimitiveContextValue::Int(v.iter().sum())),
                );
                ctx
            }))
        }
    }

    #[test]
    fn array_ints() {
        let output = run(
            PipelineStepTypeDriver::<StoreSum, Vec<i64>>::default().coerce_to_dynamic(),
            PipelineStepTypeArgs::Array(vec![
                prim(PrimitiveContextValue::Int(1)),
                prim(PrimitiveContextValue::Int(2)),
                prim(PrimitiveContextValue::Int(3)),
            ]),
            vec![make_ctx()],
        );
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0].data.get("result"),
            Some(&prim(PrimitiveContextValue::Int(6)))
        );
    }

    #[test]
    fn array_empty() {
        let output = run(
            PipelineStepTypeDriver::<StoreSum, Vec<i64>>::default().coerce_to_dynamic(),
            PipelineStepTypeArgs::Array(vec![]),
            vec![make_ctx()],
        );
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0].data.get("result"),
            Some(&prim(PrimitiveContextValue::Int(0)))
        );
    }

    // --- Map ---

    #[derive(Deserialize)]
    struct Point {
        x: f64,
        y: f64,
    }

    struct StorePoint;
    impl PipelineStepType<Point> for StorePoint {
        fn invoke(
            stream: Box<dyn Iterator<Item = (Point, Context)>>,
        ) -> Box<dyn Iterator<Item = Context>> {
            Box::new(stream.map(|(p, mut ctx)| {
                ctx.data.insert(
                    "result".into(),
                    ContextValue::Prim(PrimitiveContextValue::String(format!("{},{}", p.x, p.y))),
                );
                ctx
            }))
        }
    }

    #[test]
    fn map_struct() {
        let output = run(
            PipelineStepTypeDriver::<StorePoint, Point>::default().coerce_to_dynamic(),
            PipelineStepTypeArgs::Map(
                [
                    ("x".into(), prim(PrimitiveContextValue::Float(1.5))),
                    ("y".into(), prim(PrimitiveContextValue::Float(2.5))),
                ]
                .into(),
            ),
            vec![make_ctx()],
        );
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0].data.get("result"),
            Some(&prim(PrimitiveContextValue::String("1.5,2.5".into())))
        );
    }

    #[test]
    fn map_missing_field_drops_item() {
        let output = run(
            PipelineStepTypeDriver::<StorePoint, Point>::default().coerce_to_dynamic(),
            PipelineStepTypeArgs::Map(
                [
                    ("x".into(), prim(PrimitiveContextValue::Float(1.5))),
                    // "y" is missing — deserialization fails, item is silently dropped
                ]
                .into(),
            ),
            vec![make_ctx()],
        );
        assert!(
            output.is_empty(),
            "item with bad args should be dropped from the stream"
        );
    }

    #[test]
    fn map_resolves_ident() {
        let mut ctx = make_ctx();
        ctx.data
            .insert("my_x".into(), prim(PrimitiveContextValue::Float(9.0)));
        let output = run(
            PipelineStepTypeDriver::<StorePoint, Point>::default().coerce_to_dynamic(),
            PipelineStepTypeArgs::Map(
                [
                    ("x".into(), ContextValue::Ident("my_x".into())),
                    ("y".into(), prim(PrimitiveContextValue::Float(0.0))),
                ]
                .into(),
            ),
            vec![ctx],
        );
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0].data.get("result"),
            Some(&prim(PrimitiveContextValue::String("9,0".into())))
        );
    }

    // --- Stream behaviours ---

    struct FilterEvenIds;
    impl PipelineStepType<()> for FilterEvenIds {
        fn invoke(
            stream: Box<dyn Iterator<Item = ((), Context)>>,
        ) -> Box<dyn Iterator<Item = Context>> {
            Box::new(stream.filter_map(|(_, ctx)| match ctx.unit.id {
                UnitId::Int(id) if id % 2 == 0 => Some(ctx),
                _ => None,
            }))
        }
    }

    #[test]
    fn filter_step() {
        let ctxs: Vec<Context> = (0_i64..6).map(make_ctx_with_id).collect();
        let output = run(
            PipelineStepTypeDriver::<FilterEvenIds, ()>::default().coerce_to_dynamic(),
            PipelineStepTypeArgs::One(prim(PrimitiveContextValue::Null)),
            ctxs,
        );
        assert_eq!(output.len(), 3);
        assert!(
            output
                .iter()
                .all(|ctx| matches!(ctx.unit.id, UnitId::Int(id) if id % 2 == 0))
        );
    }

    struct Duplicate;
    impl PipelineStepType<i64> for Duplicate {
        fn invoke(
            stream: Box<dyn Iterator<Item = (i64, Context)>>,
        ) -> Box<dyn Iterator<Item = Context>> {
            Box::new(stream.flat_map(|(n, ctx)| (0_i64..n).map(move |_| ctx.clone())))
        }
    }

    #[test]
    fn flatmap_step() {
        let output = run(
            PipelineStepTypeDriver::<Duplicate, i64>::default().coerce_to_dynamic(),
            PipelineStepTypeArgs::One(prim(PrimitiveContextValue::Int(3))),
            vec![make_ctx()],
        );
        assert_eq!(output.len(), 3);
    }

    #[test]
    fn flatmap_chained_with_filter() {
        // Expand each context 3 times, then keep only even IDs.
        // Input: ids [1, 2] → after flatmap: 6 contexts → after filter: 3 (the copies of id=2)
        let ctxs: Vec<Context> = [1_i64, 2].iter().copied().map(make_ctx_with_id).collect();
        let driver_dup = PipelineStepTypeDriver::<Duplicate, i64>::default().coerce_to_dynamic();
        let driver_filt =
            PipelineStepTypeDriver::<FilterEvenIds, ()>::default().coerce_to_dynamic();

        let stream = driver_dup.invoke(
            Arc::new(PipelineStepTypeArgs::One(prim(PrimitiveContextValue::Int(3)))),
            Box::new(ctxs.into_iter()),
        );
        let output: Vec<Context> = driver_filt
            .invoke(
                Arc::new(PipelineStepTypeArgs::One(prim(PrimitiveContextValue::Null))),
                stream,
            )
            .collect();

        assert_eq!(output.len(), 3);
        assert!(output.iter().all(|ctx| ctx.unit.id == UnitId::Int(2)));
    }
}
