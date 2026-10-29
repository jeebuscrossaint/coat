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

    pub fn home_dir() -> Option<PathBuf> {
        env("HOME")
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
