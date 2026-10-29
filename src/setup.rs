//! One-time steps a module needs root for.
//!
//! An apply runs as you and never touches a file outside your home. A few
//! things can only be done from outside it, and only once -- the console
//! palette at boot is the case that started this. Those are a module's `Setup`:
//!
//! - the first apply of the module from a terminal offers it, asking for sudo
//! - a "no" is remembered, so an apply never asks twice
//! - `coat setup <module>` does it on request, whatever was answered before
//! - `coat remove <module>` undoes it
//!
//! Nothing here runs from a script, a theme picker or `coat watch`: no terminal,
//! no prompt.

use anyhow::{bail, Context, Result};
use console::style;
use std::fs;
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Setup {
    /// What it does, one sentence, for the prompt and `coat setup`.
    pub what: &'static str,
    /// Why it cannot be done on this machine, when it cannot.
    pub unavailable: fn() -> Option<&'static str>,
    pub installed: fn(&Path) -> bool,
    pub install: fn(&Path) -> Result<()>,
    pub uninstall: fn(&Path) -> Result<()>,
}

// ── Remembered answers ─────────────────────────────────────────────────────

fn declined_path() -> Result<PathBuf> {
    Ok(crate::config::state_path()?.with_file_name("declined"))
}

fn declined_list() -> Vec<String> {
    declined_path()
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .map(|s| s.lines().map(str::to_string).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default()
}

fn set_declined(module: &str, declined: bool) {
    let mut list = declined_list();
    list.retain(|m| m != module);
    if declined {
        list.push(module.to_string());
    }
    let Ok(path) = declined_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = fs::write(path, list.iter().map(|m| format!("{}\n", m)).collect::<String>());
}

// ── Offering and running ───────────────────────────────────────────────────

/// After an apply: offer the setup of each module in `applied` that has one,
/// is not in place, can be done here, and was not turned down before.
pub fn offer(applied: &[String]) {
    if !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
        return;
    }
    let Some(home) = crate::paths::home_dir() else { return };
    let declined = declined_list();
    for name in applied {
        let Some(module) = crate::modules::find(name) else { continue };
        let Some(setup) = module.setup else { continue };
        if (setup.installed)(&home)
            || (setup.unavailable)().is_some()
            || declined.iter().any(|d| d == module.name)
        {
            continue;
        }

        println!();
        println!("{} {}", style(format!("{}:", module.name)).bold(), setup.what);
        print!("Set it up now? It runs sudo once. [Y/n] ");
        let _ = std::io::stdout().flush();
        let mut answer = String::new();
        if std::io::stdin().lock().read_line(&mut answer).is_err() {
            return;
        }
        if matches!(answer.trim().to_lowercase().as_str(), "n" | "no") {
            set_declined(module.name, true);
            println!("{}", style(format!("Skipped. `coat setup {}` does it later.", module.name)).dim());
            continue;
        }
        match run(module.name, setup, &home) {
            Ok(()) => println!("{} {} set up.", style("✓").green(), module.name),
            Err(e) => println!(
                "{} {}  {}\n  `coat setup {}` tries again.",
                style("✗").red(),
                module.name,
                style(format!("{:#}", e)).red(),
                module.name
            ),
        }
    }
}

/// Do `setup` for `module` now. Clears a remembered "no".
pub fn run(module: &str, setup: &Setup, home: &Path) -> Result<()> {
    if let Some(why) = (setup.unavailable)() {
        bail!("{}", why);
    }
    (setup.install)(home)?;
    set_declined(module, false);
    Ok(())
}

/// Run `script` as root through sudo, on this terminal, so the password
/// prompt reaches the user. One sudo for the whole script: one prompt.
fn sudo_sh(script: &str) -> Result<()> {
    let status = Command::new("sudo")
        .args(["sh", "-c", script])
        .status()
        .context("cannot run sudo")?;
    if !status.success() {
        bail!("sudo exited with {}", status);
    }
    Ok(())
}

/// `s` as one single-quoted shell word.
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

fn which(bin: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .and_then(|p| std::env::split_paths(&p).map(|d| d.join(bin)).find(|p| p.is_file()))
}

// ── tty: the console palette at boot, and in step with every apply ────────
//
// `setvtrgb` sets the palette of every virtual console -- the login prompts,
// the VTs nobody is logged in on -- and the default new ones start with. It
// needs root. Two units do it from coat's tty.vtrgb:
//
// - coat-vtrgb.service runs it once at boot, before the gettys start
// - coat-vtrgb.path runs that service again whenever the file changes, which
//   is every apply that changes the scheme
//
// So sudo is needed once, to install them. After that every `coat set`
// recolours every console, as root, with no password. The file they read is in
// your home and yours to write; the worst it can do through setvtrgb is set
// sixteen colours.

pub static TTY: Setup = Setup {
    what: "theme the login prompt and every console from boot, and keep them \
           in step with each scheme change (installs two systemd units that run \
           setvtrgb).",
    unavailable: tty_unavailable,
    installed: tty_installed,
    install: tty_install,
    uninstall: tty_uninstall,
};

const UNIT_DIR: &str = "/etc/systemd/system";

fn tty_unavailable() -> Option<&'static str> {
    if !Path::new("/run/systemd/system").is_dir() {
        return Some("this system does not run systemd");
    }
    if which("setvtrgb").is_none() {
        return Some("setvtrgb is not installed (it is in the kbd package)");
    }
    None
}

/// (unit name, contents) for this user's tty.vtrgb.
fn tty_units(home: &Path) -> Option<[(&'static str, String); 2]> {
    let setvtrgb = which("setvtrgb")?;
    let vtrgb = home.join(".config/coat/tty.vtrgb");
    let header = "# Written by coat's tty module. `coat remove tty` removes it.\n";
    Some([
        (
            "coat-vtrgb.service",
            format!(
                "{header}\
                 [Unit]\n\
                 Description=Console palette from coat\n\
                 After=systemd-vconsole-setup.service\n\
                 Before=getty.target\n\
                 ConditionPathExists={vtrgb}\n\
                 \n\
                 [Service]\n\
                 Type=oneshot\n\
                 ExecStart={setvtrgb} {vtrgb}\n\
                 \n\
                 [Install]\n\
                 WantedBy=getty.target\n",
                vtrgb = vtrgb.display(),
                setvtrgb = setvtrgb.display(),
            ),
        ),
        (
            "coat-vtrgb.path",
            format!(
                "{header}\
                 [Unit]\n\
                 Description=Re-apply coat's console palette when it changes\n\
                 \n\
                 [Path]\n\
                 PathChanged={vtrgb}\n\
                 Unit=coat-vtrgb.service\n\
                 \n\
                 [Install]\n\
                 WantedBy=multi-user.target\n",
                vtrgb = vtrgb.display(),
            ),
        ),
    ])
}

/// In place means both units hold exactly what this coat would write, so a
/// moved home or a changed unit is offered again.
fn tty_installed(home: &Path) -> bool {
    tty_units(home).is_some_and(|units| {
        units.iter().all(|(name, content)| {
            fs::read_to_string(Path::new(UNIT_DIR).join(name)).ok().as_deref() == Some(content.as_str())
        })
    })
}

fn tty_install(home: &Path) -> Result<()> {
    let units = tty_units(home).context("setvtrgb is not installed")?;
    // Staged in coat's own state directory, then copied into place as root:
    // the unit text never passes through a shell.
    let stage = crate::config::state_path()?.with_file_name("units");
    fs::create_dir_all(&stage).with_context(|| format!("cannot create {}", stage.display()))?;
    let mut script = String::from("set -e\n");
    for (name, content) in &units {
        let src = stage.join(name);
        fs::write(&src, content).with_context(|| format!("cannot write {}", src.display()))?;
        script.push_str(&format!(
            "install -m 0644 -o root -g root {} {}\n",
            quote(&src.display().to_string()),
            quote(&format!("{}/{}", UNIT_DIR, name))
        ));
    }
    script.push_str(
        "systemctl daemon-reload\n\
         systemctl enable coat-vtrgb.service\n\
         systemctl enable --now coat-vtrgb.path\n\
         systemctl start coat-vtrgb.service\n",
    );
    sudo_sh(&script)
}

fn tty_uninstall(_home: &Path) -> Result<()> {
    // `setvtrgb vga` puts the stock palette back now, rather than at the next
    // boot.
    sudo_sh(&format!(
        "systemctl disable --now coat-vtrgb.path coat-vtrgb.service 2>/dev/null\n\
         rm -f {dir}/coat-vtrgb.service {dir}/coat-vtrgb.path\n\
         systemctl daemon-reload\n\
         setvtrgb vga 2>/dev/null\n\
         true",
        dir = UNIT_DIR
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_survives_a_single_quote() {
        assert_eq!(quote("/home/o'neil/x"), r"'/home/o'\''neil/x'");
    }

    /// The units coat would install, checked by systemd itself where it can be:
    /// `systemd-analyze verify` parses them and installs nothing.
    #[test]
    fn tty_units_are_valid_systemd_units() {
        let home = Path::new("/home/someone");
        let Some(units) = tty_units(home) else {
            return; // no setvtrgb on this machine: nothing to generate
        };
        let service = &units[0].1;
        assert!(service.contains("ExecStart=") && service.contains("/home/someone/.config/coat/tty.vtrgb"));
        assert!(units[1].1.contains("PathChanged=/home/someone/.config/coat/tty.vtrgb"));
        assert!(units[1].1.contains("Unit=coat-vtrgb.service"));

        let Some(analyze) = which("systemd-analyze") else { return };
        let dir = std::env::temp_dir().join(format!("coat-units-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let paths: Vec<PathBuf> = units
            .iter()
            .map(|(name, content)| {
                let p = dir.join(name);
                fs::write(&p, content).unwrap();
                p
            })
            .collect();
        let out = Command::new(analyze).arg("verify").args(&paths).output().unwrap();
        let _ = fs::remove_dir_all(&dir);
        let stderr = String::from_utf8_lossy(&out.stderr);
        // Lines about other units on this machine are not ours to judge.
        let ours: Vec<&str> = stderr.lines().filter(|l| l.contains("coat-vtrgb")).collect();
        assert!(out.status.success() && ours.is_empty(), "systemd-analyze: {}", stderr);
    }
}
