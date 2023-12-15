#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod export_globals;
pub mod gas_metering;
pub mod stack_limiter;
pub mod utils;

pub use export_globals::export_mutable_globals;
pub use parity_wasm;
pub use stack_limiter::{
	inject as inject_stack_limiter, inject_with_config as inject_stack_limiter_with_config,
	InjectionConfig,
};
