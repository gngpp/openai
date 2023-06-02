pub mod bindings;
use crate::bindings::Add;

pub fn call_add_from_c(a: i32, b: i32) -> i32 {
    unsafe { Add(a, b) }
}