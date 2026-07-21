use std::{borrow::Cow, collections::HashSet, sync::LazyLock};

use validator::ValidationError;

use crate::config::{PipelineStep, Sink};

pub static ERRCODE_PIPELINE_STEP_NAME_DUPE: &str = "ERRCODE_PIPELINE_STEP_NAME_DUPE";
pub static ERRCODE_SINK_NAME_DUPE: &str = "ERRCODE_SINK_NAME_DUPE";

pub static RE_GENERAL_NAME: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap());

pub fn validate_unique_pipeline_names(steps: &[PipelineStep]) -> Result<(), ValidationError> {
    if let Some(dupe) = find_first_dupe(steps.iter().map(|it| &it.name)) {
        let msg = format!("The pipeline step named '{dupe}' is not unique.");
        return Err(
            ValidationError::new(ERRCODE_PIPELINE_STEP_NAME_DUPE).with_message(Cow::from(msg))
        );
    };

    Ok(())
}

pub fn validate_unique_sink_names(sinks: &[Sink]) -> Result<(), ValidationError> {
    if let Some(dupe) = find_first_dupe(sinks.iter().map(|it| &it.name)) {
        let msg = format!("The sink named '{dupe}' is not unique.");
        return Err(ValidationError::new(ERRCODE_SINK_NAME_DUPE).with_message(Cow::from(msg)));
    };

    Ok(())
}

fn find_first_dupe<'a>(names: impl Iterator<Item = &'a String>) -> Option<&'a String> {
    let mut seen = HashSet::new();
    for name in names {
        if seen.contains(name) {
            return Some(name);
        }
        seen.insert(name.clone());
    }

    None
}
