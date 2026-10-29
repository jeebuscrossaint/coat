//! Where things live.
//!
//! On Linux that is $HOME and the XDG base-directory variables, which is all
//! the `dirs` crate read there anyway -- three crates to look up five
//! environment variables. Windows asks the system for its known folders, so it
//! keeps `dirs`.

#[cfg(not(windows))]
mod imp {
    use std::path::PathBuf;

    /// A set, absolute path from `var`. The XDG spec says to ignore a relative
    /// one, and an empty one means unset.
    fn env(var: &str) -> Option<PathBuf> {
        std::env::var_os(var)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
    }

    fn xdg(var: &str, under_home: &str) -> Option<PathBuf> {
        env(var).or_else(|| home_dir().map(|h| h.join(under_home)))
    }

    /// $HOME, or the account database when it is unset -- `env -i`, some
    /// services and launchers start coat with no environment at all.
    pub fn home_dir() -> Option<PathBuf> {
        env("HOME").or_else(passwd_home)
    }

    fn passwd_home() -> Option<PathBuf> {
        use std::ffi::CStr;
        use std::os::unix::ffi::OsStrExt;

        let mut buf = vec![0u8; 4096];
        let mut pw: libc::passwd = unsafe { std::mem::zeroed() };
        let mut result: *mut libc::passwd = std::ptr::null_mut();
        // SAFETY: every pointer is to a live local; getpwuid_r writes only
        // into `pw` and `buf`, and `result` is checked before `pw` is read.
        let rc = unsafe {
            libc::getpwuid_r(libc::getuid(), &mut pw, buf.as_mut_ptr().cast(), buf.len(), &mut result)
        };
        if rc != 0 || result.is_null() || pw.pw_dir.is_null() {
            return None;
        }
        // SAFETY: pw_dir points into `buf`, NUL-terminated, still alive.
        let dir = unsafe { CStr::from_ptr(pw.pw_dir) };
        Some(PathBuf::from(std::ffi::OsStr::from_bytes(dir.to_bytes()))).filter(|p| p.is_absolute())
    }
    pub fn config_dir() -> Option<PathBuf> {
        xdg("XDG_CONFIG_HOME", ".config")
    }
    pub fn cache_dir() -> Option<PathBuf> {
        xdg("XDG_CACHE_HOME", ".cache")
    }
    pub fn state_dir() -> Option<PathBuf> {
        xdg("XDG_STATE_HOME", ".local/state")
    }
    pub fn data_local_dir() -> Option<PathBuf> {
        xdg("XDG_DATA_HOME", ".local/share")
    }
}

#[cfg(windows)]
mod imp {
    pub use dirs::{cache_dir, config_dir, data_local_dir, home_dir, state_dir};
}

pub use imp::*;
