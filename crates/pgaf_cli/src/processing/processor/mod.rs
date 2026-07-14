pub mod unbatched;

use super::context::Context;
use super::template::TemplateEngine;
use super::PipelineData;
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
