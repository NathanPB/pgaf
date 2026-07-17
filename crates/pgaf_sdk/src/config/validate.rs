use std::{borrow::Cow, collections::HashSet, path::Path, sync::LazyLock};

use validator::ValidationError;

use crate::config::PipelineStep;

use super::RunConfig;

pub static ERRCODE_RUN_NAME_DUPE: &str = "ERRCODE_RUN_NAME_DUPE";
pub static ERRCODE_PIPELINE_STEP_NAME_DUPE: &str = "ERRCODE_PIPELINE_STEP_NAME_DUPE";
pub static ERRCODE_TEMPLATE_FILE_NOT_FOUND: &str = "ERRCODE_TEMPLATE_FILE_NOT_FOUND";

pub static RE_VALID_RUN_NAME: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap());

pub fn validate_unique_run_names(runs: &Vec<RunConfig>) -> Result<(), ValidationError> {
    let mut run_names = HashSet::new();

    for run in runs {
        if run_names.contains(&run.name) {
            let msg = format!("Run name {} is not unique", run.name);
            return Err(ValidationError::new(ERRCODE_RUN_NAME_DUPE).with_message(Cow::from(msg)));
        }
        run_names.insert(run.name.clone());
    }

    Ok(())
}

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

pub fn validate_template_file_exists(path: &Path) -> Result<(), ValidationError> {
    if !path.exists() || path.is_dir() {
        let msg = format!(
            "Template file {} does not exist or is not a file",
            path.display()
        );

        return Err(
            ValidationError::new(ERRCODE_TEMPLATE_FILE_NOT_FOUND).with_message(Cow::from(msg))
        );
    }
    Ok(())
}
