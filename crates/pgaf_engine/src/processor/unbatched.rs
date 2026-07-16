use super::super::template::TemplateEngine;
use super::Processor;
use pgaf_sdk::context::Context;
use std::error::Error;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::sync::mpmc::{Receiver, Sender};

pub struct UnbatchedProcessor {
    pub workdir: PathBuf,
}

impl Processor for UnbatchedProcessor {
    type Output = Context;

    fn process(
        &self,
        tx: &Sender<Self::Output>,
        rx: &Receiver<Self::Output>,
        templates: &TemplateEngine,
    ) -> Result<(), Box<dyn Error + Send>> {
        // TODO: better error handling
        rx.iter()
            .inspect(|ctx| {
                let path = ctx.dir(&self.workdir);
                if let Err(err) = create_dir_all(&path) {
                    eprintln!("UnbatchedProcessor: Failed to create directory: {}", err);
                }

                let filename = match templates.file_name(ctx.run.name.as_str()) {
                    Some(filename) => filename,
                    None => {
                        panic!(
                            "Failed to render template for context ID {} ({}, {}): Template file name not registered",
                            ctx.unit.id, ctx.unit.lon, ctx.unit.lat
                        );
                    }
                };

                let rendered = templates.render(ctx).unwrap();
                let mut template_path = ctx.dir(&self.workdir);
                template_path.push(filename);

                if let Err(err) = std::fs::write(template_path, rendered) {
                    panic!(
                        "Failed to render template for context ID {} ({}, {}): {}",
                        ctx.unit.id, ctx.unit.lon, ctx.unit.lat, err
                    );
                }
            })
            .try_for_each(|ctx| tx.send(ctx))
            .map_err(|err| Box::new(err) as Box<dyn Error + Send>)
    }
}
