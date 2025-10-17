pub mod hyperdrive;

pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
pub fn version() -> &'static str {
    PKG_VERSION
}
