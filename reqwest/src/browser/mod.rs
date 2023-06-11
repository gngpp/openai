//! Holds structs and information to aid in impersonating a set of browsers

use std::sync::Arc;

use boring::ssl::SslConnectorBuilder;
use http::HeaderMap;

#[cfg(feature = "__chrome")]
pub use chrome::ChromeVersion;

#[cfg(feature = "__chrome")]
mod chrome;

#[cfg(feature = "__chrome")]
pub(crate) use chrome::configure_chrome;

struct BrowserSettings {
    pub tls_builder_func: Arc<dyn Fn() -> SslConnectorBuilder + Send + Sync>,
    pub http2: Http2Data,
    pub headers: HeaderMap,
    pub gzip: bool,
    pub brotli: bool,
}

struct Http2Data {
    pub initial_stream_window_size: u32,
    pub initial_connection_window_size: u32,
    pub max_concurrent_streams: u32,
    pub max_header_list_size: u32,
    pub header_table_size: u32,
    pub enable_push: Option<bool>,
}
