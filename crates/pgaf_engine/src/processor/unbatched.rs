use super::Processor;
use crate::context::value::ContextValueDeserializeSeed;
use pgaf_sdk::context::Context;
use pgaf_sdk::pipeline::PipelineStepTypeArgs;
use pgaf_sdk::{config, pipeline};
use serde::de::value::{
    BoolDeserializer, F64Deserializer, I64Deserializer, StrDeserializer, StringDeserializer,
    UnitDeserializer,
};
use serde::de::{DeserializeSeed, Visitor};
use std::collections::HashMap;
use std::error::Error;
use std::sync::mpmc::{Receiver, Sender};

// TODO: I sould move these deserializers out of here

struct PipelineStepTypeArgsWrapper(PipelineStepTypeArgs);
struct PipelineStepTypeArgsDeserializer<'de>(ContextValueDeserializeSeed<'de>);
struct PipelineStepTypeArgsDeserializerVisitor<'de>(ContextValueDeserializeSeed<'de>);

impl<'de> DeserializeSeed<'de> for PipelineStepTypeArgsDeserializer<'de> {
    type Value = PipelineStepTypeArgsWrapper;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(PipelineStepTypeArgsDeserializerVisitor(self.0))
    }
}

impl<'de> Visitor<'de> for PipelineStepTypeArgsDeserializerVisitor<'de> {
    type Value = PipelineStepTypeArgsWrapper;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("to be defined")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut hashmap = HashMap::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(key) = map.next_key::<String>()? {
            let value = map.next_value_seed(self.0.clone())?;
            hashmap.insert(key, value);
        }

        Ok(PipelineStepTypeArgsWrapper(PipelineStepTypeArgs::Map(
            hashmap,
        )))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut list = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(value) = seq.next_element_seed(self.0.clone())? {
            list.push(value);
        }

        Ok(PipelineStepTypeArgsWrapper(PipelineStepTypeArgs::Array(
            list,
        )))
    }

    fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
        let cv = self.0.deserialize(BoolDeserializer::<E>::new(v))?;
        Ok(PipelineStepTypeArgsWrapper(PipelineStepTypeArgs::One(cv)))
    }

    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
        let cv = self.0.deserialize(I64Deserializer::<E>::new(v))?;
        Ok(PipelineStepTypeArgsWrapper(PipelineStepTypeArgs::One(cv)))
    }

    fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
        let cv = self.0.deserialize(F64Deserializer::<E>::new(v))?;
        Ok(PipelineStepTypeArgsWrapper(PipelineStepTypeArgs::One(cv)))
    }

    fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
        let cv = self.0.deserialize(StringDeserializer::<E>::new(v))?;
        Ok(PipelineStepTypeArgsWrapper(PipelineStepTypeArgs::One(cv)))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let cv = self.0.deserialize(StrDeserializer::<E>::new(v))?;
        Ok(PipelineStepTypeArgsWrapper(PipelineStepTypeArgs::One(cv)))
    }

    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        let cv = self.0.deserialize(UnitDeserializer::<E>::new())?;
        Ok(PipelineStepTypeArgsWrapper(PipelineStepTypeArgs::One(cv)))
    }
}

pub struct UnbatchedProcessor {
    pipeline: Vec<(pipeline::Driver, pipeline::PipelineStepTypeArgs)>,
}

impl UnbatchedProcessor {
    pub fn new(config: &config::Config, deserializer: &ContextValueDeserializeSeed) -> Self {
        let pipeline: Vec<_> = config
            .pipeline
            .iter()
            .map(|it| {
                (
                    it.driver.clone(),
                    PipelineStepTypeArgsDeserializer(deserializer.clone())
                        .deserialize(it.args.clone())
                        .expect("Failed to deserialize function arguments.")
                        .0,
                )
            })
            .collect();

        Self { pipeline }
    }
}

impl Processor for UnbatchedProcessor {
    type Output = Context;

    fn process(
        &self,
        tx: &Sender<Self::Output>,
        rx: &Receiver<Self::Output>,
    ) -> Result<(), Box<dyn Error + Send>> {
        rx.iter()
            .flat_map(|ctx| {
                let stream: Box<dyn Iterator<Item = Context>> = Box::new(std::iter::once(ctx));
                self.pipeline
                    .iter()
                    .fold(stream, |s, (driver, args)| driver.invoke(args.clone(), s))
            })
            .try_for_each(|ctx| tx.send(ctx))
            .map_err(|err| Box::new(err) as Box<dyn Error + Send>)
    }
}
