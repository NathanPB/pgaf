pub mod unbatched;

use super::template::TemplateEngine;
use super::PipelineData;
use pgaf_sdk::context::Context;
use std::error::Error;
use std::sync::mpmc::{Receiver, Sender};

pub trait Processor: Send + Sync {
    type Output: PipelineData;

    fn process(
        &self,
        tx: &Sender<Self::Output>,
        rx: &Receiver<Context>,
        templates: &TemplateEngine,
    ) -> Result<(), Box<dyn Error + Send>>;
}
