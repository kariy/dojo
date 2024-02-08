#[cfg(feature = "blockifier")]
pub mod blockifier;
#[cfg(feature = "native")]
pub mod sir;

const LOG_TARGET: &str = "executor";

pub struct ExecutionConfig {
    execute: bool,
    validate: bool,
    fee_charge: bool,
    nonce_check: bool,
}
