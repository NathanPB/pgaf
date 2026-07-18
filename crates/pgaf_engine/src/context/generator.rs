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
}

impl ContextGenerator {
    pub fn new(
        domain_generator: Box<dyn DomainGenerator>,
        sample_size: Option<usize>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(ContextGenerator {
            domain_generator,
            curr_unit: None,
            sample_size,
            current_count: 0,
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

        if self.curr_unit.is_none() {
            self.curr_unit = self.domain_generator.next();
            self.curr_unit.as_ref()?;
        }

        self.current_count += 1;
        Some(Context {
            unit: self.curr_unit.clone()?,
            data: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pgaf_sdk::data::GeoDeg;

    #[test]
    fn test_sample_size() {
        let domain_src: Box<dyn DomainGenerator> =
            Box::new((0..200).map(|id: i64| ExecutionUnit {
                id: id.into(),
                lon: GeoDeg::from(0.0),
                lat: GeoDeg::from(0.0),
            }));

        let generator = ContextGenerator::new(domain_src, Some(50)).unwrap();
        assert_eq!(generator.count(), 50);
    }
}
