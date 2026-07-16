use pgaf_sdk::config::RunConfig;
use pgaf_sdk::context::Context;
use pgaf_sdk::domain::{DomainGenerator, ExecutionUnit};

/// Generates a sequence of [`Context`]s to be processed.
///
/// The order of the generated [`Context`]s is determined by a permutation over the runs and the domain
/// generator, prioritizing outputting all the runs before moving to the next [`ExecutionUnit`].
///
/// TODO: decouple from config. Maybe create a registry for [`DomainGenerator`] (abstract factory?) and couple it with config instead. Will allow for plugin extensibility later.
pub struct ContextGenerator {
    domain_generator: Box<dyn DomainGenerator>,
    curr_unit: Option<ExecutionUnit>,
    sample_size: Option<usize>,
    current_count: usize,
    runs: Vec<RunConfig>,
    current_run: usize,
}

impl ContextGenerator {
    pub fn new(
        domain_generator: Box<dyn DomainGenerator>,
        runs: Vec<RunConfig>,
        sample_size: Option<usize>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(ContextGenerator {
            domain_generator,
            curr_unit: None,
            sample_size,
            current_count: 0,
            runs,
            current_run: 0,
        })
    }
}

impl Iterator for ContextGenerator {
    type Item = Context;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample_size) = self.sample_size
            && self.current_count >= sample_size
        {
            return None;
        }

        if self.current_run >= self.runs.len() {
            self.current_run = 0;
            self.curr_unit = None;
        }

        if self.curr_unit.is_none() {
            self.curr_unit = self.domain_generator.next();
            self.curr_unit.as_ref()?;
        }

        let run = self.runs[self.current_run].clone();
        self.current_run += 1;
        self.current_count += 1;
        Some(Context {
            unit: self.curr_unit.clone()?,
            run,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pgaf_sdk::data::GeoDeg;
    use pgaf_sdk::domain::UnitId;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn context_gen() {
        let domain_gen: Box<dyn DomainGenerator> =
            Box::new((0..200).map(|id: i64| ExecutionUnit {
                id: id.into(),
                lon: GeoDeg::from(0.0),
                lat: GeoDeg::from(0.0),
            }));

        let runs = vec![
            RunConfig {
                name: String::from("r1"),
                extra: HashMap::new(),
                template: PathBuf::from("dummy"),
            },
            RunConfig {
                name: String::from("r2"),
                extra: HashMap::new(),
                template: PathBuf::from("dummy"),
            },
        ];

        let generator = ContextGenerator::new(domain_gen, runs, None).unwrap();
        let mut max = i64::MIN;

        for (i, ctx) in generator.enumerate() {
            assert_eq!(UnitId::Int((i / 2) as i64), ctx.unit.id);

            if i % 2 == 0 {
                assert_eq!(ctx.run.name, "r1");
            } else {
                assert_eq!(ctx.run.name, "r2");
            }

            if let UnitId::Int(v) = ctx.unit.id {
                max = max.max(v);
            }
        }

        assert_eq!(max, 199);
    }

    #[test]
    fn test_sample_size() {
        let domain_src: Box<dyn DomainGenerator> =
            Box::new((0..200).map(|id: i64| ExecutionUnit {
                id: id.into(),
                lon: GeoDeg::from(0.0),
                lat: GeoDeg::from(0.0),
            }));

        let runs = vec![RunConfig {
            name: String::from("r1"),
            extra: HashMap::new(),
            template: PathBuf::from("dummy"),
        }];

        let generator = ContextGenerator::new(domain_src, runs, Some(50)).unwrap();
        assert_eq!(generator.count(), 50);
    }
}
