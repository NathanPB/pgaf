use std::{borrow::Cow, collections::HashSet, sync::LazyLock};

use validator::ValidationError;

use crate::config::PipelineStep;

pub static ERRCODE_PIPELINE_STEP_NAME_DUPE: &str = "ERRCODE_PIPELINE_STEP_NAME_DUPE";

pub static RE_PIPELINE_STEP_NAME: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap());

pub fn validate_unique_pipeline_names(steps: &[PipelineStep]) -> Result<(), ValidationError> {
    let mut seen = HashSet::new();

    for step in steps {
        if seen.contains(&step.name) {
            let msg = format!("Pipeline step '{}' is not unique.", step.name);
            return Err(
                ValidationError::new(ERRCODE_PIPELINE_STEP_NAME_DUPE).with_message(Cow::from(msg))
            );
        }
        seen.insert(step.name.clone());
    }

    Ok(())
}
