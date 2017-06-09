extern crate parity_wasm;
extern crate env_logger;
#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;

mod optimizer;
mod gas;
mod symbols;
mod logger;
mod ext;
mod pack;

pub use optimizer::{optimize, Error as OptimizerError};
pub use gas::inject_gas_counter;
pub use logger::init_log;
pub use ext::externalize;
pub use pack::pack_instance;