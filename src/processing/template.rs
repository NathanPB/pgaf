use super::context::{Context, ContextEvaluationError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Default)]
pub struct TemplateEngine {
    tera: tera::Tera,
    filenames: HashMap<String, String>,
}

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Tera error: {0}")]
    TeraError(#[from] tera::Error),
    #[error("Template path {0} is not a file")]
    TemplateNotAFile(PathBuf),
    #[error("Context evaluation error: {0}")]
    ContextEvaluation(#[from] ContextEvaluationError),
}

impl TemplateEngine {
    pub fn register(&mut self, run_name: &str, file: &Path) -> Result<(), TemplateError> {
        let full_path = file.canonicalize()?;
        let contents = std::fs::read_to_string(full_path)?;

        self.tera.add_raw_template(run_name, contents.as_str())?;
        self.filenames.insert(
            run_name.to_string(),
            file.file_name()
                .ok_or(TemplateError::TemplateNotAFile(file.to_path_buf()))?
                .to_string_lossy()
                .to_string(),
        );
        Ok(())
    }

    pub fn file_name(&self, run_name: &str) -> Option<&String> {
        self.filenames.get(run_name)
    }

    /// TODO: error handling
    pub fn render(&self, ctx: &Context) -> Result<String, TemplateError> {
        let str = self.tera.render(ctx.run.name.as_str(), &ctx.tera()?)?;
        Ok(str)
    }
}
