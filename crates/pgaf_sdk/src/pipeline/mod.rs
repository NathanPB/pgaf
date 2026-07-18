mod deserializer;

use crate::context::{Context, ContextEvaluationError, ContextValue};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::fmt::Display;
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub enum PipelineStepTypeArgs {
    One(ContextValue),
    Array(Vec<ContextValue>),
    Map(HashMap<String, ContextValue>),
}

#[derive(thiserror::Error, Debug, PartialEq)]
#[error("Runtime error: {0}")]
pub struct PipelineStepTypeRuntimeError(pub String);

#[derive(thiserror::Error, Debug)]
pub enum PipelineStepTypeArgParseError {
    #[error("Failed to evaluate: {0}")]
    Evaluation(#[from] ContextEvaluationError),
    #[error("Failed to parse: {0}")]
    Custom(String),
}

impl serde::de::Error for PipelineStepTypeArgParseError {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

/// A pipeline step that transforms a lazy stream of [`Context`]s.
///
/// Implementations receive a stream of `(A, Context)` pairs where `A` is the
/// step's fully deserialized — and lazily evaluated — argument type. The step
/// controls output cardinality: it may map (1-to-1), filter (1-to-0), flat-map
/// (1-to-N), or perform side-effects before forwarding the context downstream.
///
/// Argument deserialization happens per item, so any [`ContextValue`] expressions
/// in the args (identifiers, templates, function calls) are resolved against the
/// specific [`Context`] flowing through at that moment.
pub trait PipelineStepType<A>: Send + Sync {
    fn invoke(stream: Box<dyn Iterator<Item = (A, Context)>>) -> Box<dyn Iterator<Item = Context>>;
}

pub struct PipelineStepTypeDriver<F: PipelineStepType<A>, A> {
    _marker_f: PhantomData<F>,
    _marker_a: PhantomData<A>,
}

impl<F: PipelineStepType<A> + 'static, A: DeserializeOwned + 'static> Default
    for PipelineStepTypeDriver<F, A>
{
    fn default() -> Self {
        Self {
            _marker_f: Default::default(),
            _marker_a: Default::default(),
        }
    }
}

impl<F: PipelineStepType<A> + 'static, A: DeserializeOwned + 'static> PipelineStepTypeDriver<F, A> {
    pub fn coerce_to_dynamic(self) -> Driver {
        Driver(invoker_impl::<F, A>)
    }
}

type InvokeFn = fn(
    Arc<PipelineStepTypeArgs>,
    Box<dyn Iterator<Item = Context>>,
) -> Box<dyn Iterator<Item = Context>>;

fn invoker_impl<F: PipelineStepType<A>, A: DeserializeOwned + 'static>(
    args: Arc<PipelineStepTypeArgs>,
    input: Box<dyn Iterator<Item = Context>>,
) -> Box<dyn Iterator<Item = Context>> {
    let deserialized = Box::new(input.filter_map(move |ctx| {
        let a = deserializer::deserialize_args::<A>(&args, &ctx)
            .inspect_err(|e| eprintln!("Pipeline step error at unit {}: {e}", ctx.unit.id))
            .ok()?;
        Some((a, ctx))
    }));
    F::invoke(deserialized)
}

#[derive(Clone, Debug)]
pub struct Driver(InvokeFn);

/// **WARNING:** [`PartialEq`] is implemented for [`Driver`] solely to fulfil a badly-placed `derive(PartialEq)`.
/// This comparison always results in `true`.
impl PartialEq for Driver {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Driver {
    /// Wraps `input` in a lazy adapter that deserializes args per item, then
    /// passes the typed stream to the step. Nothing is consumed until the
    /// returned iterator is driven.
    pub fn invoke(
        &self,
        args: Arc<PipelineStepTypeArgs>,
        input: Box<dyn Iterator<Item = Context>>,
    ) -> Box<dyn Iterator<Item = Context>> {
        self.0(args, input)
    }
}
