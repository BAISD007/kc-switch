pub mod discovery;
pub mod error;
pub mod http;
pub mod fs_ops;
pub mod mcp;
pub mod model_discovery;
pub mod paths;
pub mod plugins;
pub mod providers;
pub mod skills;
pub mod write_guard;

#[cfg(test)]
pub mod test_support;

pub use error::Result;
