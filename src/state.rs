pub static mut RUNNING: bool = false;
pub static mut ERROR: String = String::new();

pub fn set_running(b: bool) {
	unsafe { RUNNING = b };
}