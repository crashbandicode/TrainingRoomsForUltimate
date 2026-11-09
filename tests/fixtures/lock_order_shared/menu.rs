// tests/lock_order.rs fixture (never compiled): a typed static whose read differs by thread, and its store.
use hos_lock::Mutex;

static EDITED: Mutex<u32> = Mutex::new(0);
static PUBLISHING: Mutex<()> = Mutex::new(());
pub static MENU: Menu = Menu;

fn current_menu() -> u32 {
    *EDITED.lock()
}

fn plugin_view() -> u32 {
    let _turn = PUBLISHING.lock();
    0
}

fn store(value: u32) {
    *EDITED.lock() = value;
}

impl Readable for Menu {
    fn read_value(&self) -> u32 {
        if crate::runtime::threads::on_plugin_thread() {
            plugin_view()
        } else {
            current_menu()
        }
    }
}
