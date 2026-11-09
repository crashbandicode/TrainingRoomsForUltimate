//! Fixture (never compiled): a typed static whose plugin-thread read takes a hook's lock.
static EDITED: Mutex<u8> = Mutex::new(0);
pub static MENU: Menu = Menu;

fn plugin_view() -> u8 {
    *EDITED.lock()
}

impl Readable for Menu {
    fn read_value(&self) -> u8 {
        plugin_view()
    }
}
