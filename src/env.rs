use crate::args::ServeArgs;

#[cfg(target_family = "unix")]
pub(crate) const PID_PATH: &str = "/var/run/opengpt.pid";
#[cfg(target_family = "unix")]
pub(crate) const DEFAULT_STDOUT_PATH: &str = "/var/run/opengpt.out";
#[cfg(target_family = "unix")]
pub(crate) const DEFAULT_STDERR_PATH: &str = "/var/run/opengpt.err";
#[cfg(target_family = "unix")]
pub(crate) const DEFAULT_WORK_DIR: &str = "/";

#[cfg(target_family = "unix")]
pub(crate) fn check_root() {
    use nix::unistd::Uid;

    if !Uid::effective().is_root() {
        println!("You must run this executable with root permissions");
        std::process::exit(-1)
    }
}

#[cfg(target_family = "unix")]
pub(crate) fn get_pid() -> Option<String> {
    if let Ok(data) = std::fs::read(PID_PATH) {
        let binding = String::from_utf8(data).expect("pid file is not utf8");
        return Some(binding.trim().to_string());
    }
    None
}

pub(crate) fn fix_relative_path(args: &mut ServeArgs) {
    if let Some(c) = args.config.as_mut() {
        // fix relative path
        if c.is_relative() {
            args.config = Some(
                std::env::current_dir()
                    .expect("cannot get current exe")
                    .join(c),
            )
        }
    }
}
