//! `cargo run --example tls --features=blocking,chrome`

#![deny(warnings)]

use reqwest_impersonate::browser::ChromeVersion;

// This is using the `tokio` runtime. You'll need the following dependency:
//
// `tokio = { version = "1", features = ["full"] }`
#[cfg(not(target_arch = "wasm32"))]

fn main() -> Result<(), reqwest_impersonate::Error> {
    // Build a client to mimic Chrome 104
    let client = reqwest_impersonate::blocking::Client::builder()
        .chrome_builder(ChromeVersion::V108)
        .build()
        .unwrap();

    // Use the API you're already familiar with
    match client.get("https://tls.peet.ws/api/all").send() {
        Ok(res) => {
            println!("{}", res.text().unwrap());
        }
        Err(err) => {
            dbg!(err);
        }
    };

    Ok(())
}

// The [cfg(not(target_arch = "wasm32"))] above prevent building the tokio::main function
// for wasm32 target, because tokio isn't compatible with wasm32.
// If you aren't building for wasm32, you don't need that line.
// The two lines below avoid the "'main' function not found" error when building for wasm32 target.
#[cfg(target_arch = "wasm32")]
fn main() {}
