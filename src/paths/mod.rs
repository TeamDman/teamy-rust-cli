mod app_home;
mod cache;

pub use app_home::*;
pub use cache::*;

pub const APP_HOME_ENV_VAR: &str = "APP_HOME_DIR"; // TODO: REPLACE ME
pub const APP_HOME_DIR_NAME: &str = "teamy-rust-cli"; // TODO: REPLACE ME

pub const APP_CACHE_ENV_VAR: &str = "APP_CACHE_DIR"; // TODO: REPLACE ME
pub const APP_CACHE_DIR_NAME: &str = "teamy-rust-cli"; // TODO: REPLACE ME
