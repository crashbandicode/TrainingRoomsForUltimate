// tests/lock_order.rs fixture: a plugin thread's lock uses.
use hos_lock::Mutex;

pub static A: Mutex<u32> = Mutex::new(0);

fn close_like() {
    let mut g = A.lock();
    *g += 1;
    publish::publish_like();
}

fn dropped_first() {
    let g = A.lock();
    drop(g);
    publish::publish_like();
}

fn statement_only() {
    let v = *A.lock();
    publish::publish_like();
}

fn relock() {
    let _g = A.lock();
    helper_takes_a();
}

fn helper_takes_a() {
    *A.lock() += 1;
}

fn closure_only() {
    let x = Some(1).filter(|_| *A.lock() == 1);
    publish::publish_like();
}

fn condition_only() {
    if *A.lock() == 1 {
        publish::publish_like();
    }
}

fn if_let_holds() {
    if let Some(v) = A.lock().checked_add(1) {
        publish::publish_like();
    }
}

fn block_statement_ends() {
    if let Some(v) = A.lock().checked_add(1) {
        v;
    }
    publish::publish_like();
}

fn sleeps_holding() {
    let _g = A.lock();
    std::thread::sleep(D);
}
