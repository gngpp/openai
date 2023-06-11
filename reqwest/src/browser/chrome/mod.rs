//! Settings for impersonating the Chrome browser

use crate::ClientBuilder;

mod ver;

pub(crate) fn configure_chrome(ver: ChromeVersion, builder: ClientBuilder) -> ClientBuilder {
    let settings = ver::get_config_from_ver(ver);

    builder
        .use_boring_tls(settings.tls_builder_func)
        .http2_initial_stream_window_size(settings.http2.initial_stream_window_size)
        .http2_initial_connection_window_size(settings.http2.initial_connection_window_size)
        .http2_max_concurrent_streams(settings.http2.max_concurrent_streams)
        .http2_max_header_list_size(settings.http2.max_header_list_size)
        .http2_header_table_size(settings.http2.header_table_size)
        .http2_enable_push(settings.http2.enable_push)
        .replace_default_headers(settings.headers)
        .brotli(settings.brotli)
        .gzip(settings.gzip)
}

/// Defines the Chrome version to mimic when setting up a builder
#[derive(Debug)]
#[allow(missing_docs)]
pub enum ChromeVersion {
    V104,
    V105,
    V106,
    V108,
}
