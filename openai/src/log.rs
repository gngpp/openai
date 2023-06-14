#[macro_export]
macro_rules! info {
    // info!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // info!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => (log::info!($($arg)+));

    // info!("a {} event", "log")
    ($($arg:tt)+) => (log::info!($($arg)+))
}

#[macro_export]
macro_rules! debug {
    // debug!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // debug!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => (log::debug!("[{}] {}",std::panic::Location::caller(), $($arg)+));

    // debug!("a {} event", "log")
    ($($arg:tt)+) => (log::debug!("[{}] {}", std::panic::Location::caller(),  format!($($arg)+)))
}

#[macro_export]
macro_rules! warn {
    // warn!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // warn!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => (log::warn!("[{}] {}",std::panic::Location::caller(), $($arg)+));

    // warn!("a {} event", "log")
    ($($arg:tt)+) => (log::warn!("[{}] {}", std::panic::Location::caller(),  format!($($arg)+)))
}

#[macro_export]
macro_rules! trace {
    // trace!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // trace!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => (log::trace!("[{}] {}",std::panic::Location::caller(), $($arg)+));

    // trace!("a {} event", "log")
    ($($arg:tt)+) => (log::trace!("[{}] {}", std::panic::Location::caller(),  format!($($arg)+)))
}

#[macro_export]
macro_rules! error {
    // error!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // error!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => (log::error!("[{}] {}",std::panic::Location::caller(), $($arg)+));

    // error!("a {} event", "log")
    ($($arg:tt)+) => (log::error!("[{}] {}", std::panic::Location::caller(),  format!($($arg)+)))
}
