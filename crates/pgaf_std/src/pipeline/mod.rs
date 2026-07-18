pub mod cmd;
pub mod filter;
pub mod map;
pub mod template;
pub mod unset;
pub mod void;

#[cfg(test)]
fn make_ctx(id: i64) -> pgaf_sdk::context::Context {
    make_ctx_with_extras(id, std::default::Default::default())
}

#[cfg(test)]
fn make_ctx_with_extras(
    id: i64,
    data: std::collections::HashMap<String, pgaf_sdk::context::ContextValue>,
) -> pgaf_sdk::context::Context {
    use pgaf_sdk::{context::Context, data::GeoDeg, domain::ExecutionUnit};

    Context {
        unit: ExecutionUnit {
            id: id.into(),
            lon: GeoDeg::from(0.0),
            lat: GeoDeg::from(0.0),
        },
        data,
    }
}
