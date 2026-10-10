pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MIN_WIDTH: u16 = 45;
pub const MIN_HEIGHT: u16 = 32;
pub const NO_RENDER_WIDTH: u16 = 21;
pub const NO_RENDER_HEIGHT: u16 = 4;
pub const CONFIG_VERSION: u32 = 2;

#[cfg(target_endian = "big")]
pub const ENDIANESS: &str = "be";
#[cfg(target_endian = "little")]
pub const ENDIANESS: &str = "le";

pub const ADDRESS_COMMS: &str = "127.0.0.1:5630";
pub const ADDRESS_EVENT: &str = "127.0.0.1:5631";