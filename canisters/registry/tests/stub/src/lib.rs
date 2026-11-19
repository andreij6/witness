use serde_bytes::ByteBuf;
use std::cell::{Cell, RefCell};

thread_local! {
    static CALLS: RefCell<Vec<(String, ByteBuf)>> = const { RefCell::new(Vec::new()) };
    static DOWN: Cell<bool> = const { Cell::new(false) };
}

fn record(method: &str) {
    if DOWN.get() {
        ic_cdk::trap("stub down");
    }
    let arg = ByteBuf::from(ic_cdk::api::msg_arg_data());
    CALLS.with_borrow_mut(|c| c.push((method.to_owned(), arg)));
}

#[ic_cdk::init]
fn init() {}

#[ic_cdk::update]
fn put() {
    record("put");
}

#[ic_cdk::update]
fn put_shell() {
    record("put_shell");
}

#[ic_cdk::update]
fn tombstone() {
    record("tombstone");
}

#[ic_cdk::update]
fn sync() {
    record("sync");
}

#[ic_cdk::update]
fn erase() {
    record("erase");
}

#[ic_cdk::update]
fn set_down(down: bool) {
    DOWN.set(down);
}

#[ic_cdk::query]
fn calls() -> Vec<(String, ByteBuf)> {
    CALLS.with_borrow(Clone::clone)
}
