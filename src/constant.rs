pub const APP_NAME: &str = "cls";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MIN_WIDTH: u16 = 45;
pub const MIN_HEIGHT: u16 = 32;
pub const CONFIG_VERSION: u32 = 1;

#[cfg(target_endian = "big")]
pub const ENDIANESS: &str = "be";
#[cfg(target_endian = "little")]
pub const ENDIANESS: &str = "le";