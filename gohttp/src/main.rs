pub mod bindings;
use crate::bindings::Add;

fn call_add_from_c(a: i32, b: i32) -> i32 {
    unsafe { Add(a, b) }
}

fn main() {
    println!("Hello from Rust.");
    println!("{}", call_add_from_c(1, 2))
}
