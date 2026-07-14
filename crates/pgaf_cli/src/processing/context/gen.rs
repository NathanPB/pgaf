use crate::config;
use crate::processing::context::Context;
use crate::sites::{Site, SiteGenerator};

/// Given a site source configuration, ContextGenerator will generate a sequence of Contexts to be processed.
///
/// The order of the generated Contexts is determined by a permutation over the runs and the sites iterator,
/// prioritizing outputting all the runs before moving to the next site.
///
/// TODO: decouple from config. Maybe create a registry for SiteGenerators (abstract factory?) and couple it with config instead. Will allow for plugin extensibility later.
pub struct ContextGenerator {
    site_generator: Box<dyn SiteGenerator>,
    curr_site: Option<Site>,
    site_sample_size: Option<usize>,
    current_site_count: usize,
    runs: Vec<config::runs::RunConfig>,
    current_run: usize,
}

impl ContextGenerator {
    /// Creates a new ContextGenerator from a SitesSource configuration and a vector of RunConfig.
    pub fn new(
        site_generator: Box<dyn SiteGenerator>,
        runs: Vec<config::runs::RunConfig>,
        site_sample_size: Option<usize>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(ContextGenerator {
            site_generator,
            curr_site: None,
            site_sample_size,
            current_site_count: 0,
            runs,
            current_run: 0,
        })
    }
}

impl Iterator for ContextGenerator {
    type Item = Context;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample_size) = self.site_sample_size {
            if self.current_site_count >= sample_size {
                return None;
            }
        }

        if self.current_run >= self.runs.len() {
            self.current_run = 0;
            self.curr_site = None;
        }

        if self.curr_site.is_none() {
            self.curr_site = self.site_generator.next();
            self.curr_site.as_ref()?;
        }

        let run = self.runs[self.current_run].clone();
        self.current_run += 1;
        self.current_site_count += 1;
        Some(Context {
            site: self.curr_site.clone()?,
            run,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::data::GeoDeg;
    use crate::sites::SiteGenerator;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn context_gen() {
        let site_src: Box<dyn SiteGenerator> = Box::new((0..200).map(|id| Site {
            id,
            lon: GeoDeg::from(0.0),
            lat: GeoDeg::from(0.0),
        }));

        let runs = vec![
            config::runs::RunConfig {
                name: String::from("r1"),
                extra: HashMap::new(),
                template: PathBuf::from("dummy"),
            },
            config::runs::RunConfig {
                name: String::from("r2"),
                extra: HashMap::new(),
                template: PathBuf::from("dummy"),
            },
        ];

        let generator = ContextGenerator::new(site_src, runs, None).unwrap();
        let mut max = i32::MIN;

        for (i, ctx) in generator.enumerate() {
            assert_eq!((i / 2) as i32, ctx.site.id);

            if i % 2 == 0 {
                assert_eq!(ctx.run.name, "r1");
            } else {
                assert_eq!(ctx.run.name, "r2");
            }

            max = max.max(ctx.site.id);
        }

        assert_eq!(max, 199);
    }

    #[test]
    fn test_sample_size() {
        let site_src: Box<dyn SiteGenerator> = Box::new((0..200).map(|id| Site {
            id,
            lon: GeoDeg::from(0.0),
            lat: GeoDeg::from(0.0),
        }));

        let runs = vec![config::runs::RunConfig {
            name: String::from("r1"),
            extra: HashMap::new(),
            template: PathBuf::from("dummy"),
        }];

        let generator = ContextGenerator::new(site_src, runs, Some(50)).unwrap();
        assert_eq!(generator.count(), 50);
    }
}
