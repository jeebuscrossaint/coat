/// Windows-specific theming: accent color, dark/light mode, Windows Terminal.
/// Compiled only on Windows — Linux builds ignore this entire file.
use anyhow::{Context, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::config::CoatConfig;
use crate::detail;
use crate::scheme::Scheme;

/// Run one apply step behind a spinner that freezes into a ✓/✗ result line.
fn step<F: FnOnce() -> Result<()>>(label: &str, f: F) {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
    );
    pb.set_message(label.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    let result = f();
    pb.finish_and_clear();
    match result {
        Ok(_) => println!("{} {}", style("✓").green(), label),
        Err(e) => println!("{} {}  {}", style("✗").red(), label, style(e.to_string()).red()),
    }
}

// ── Accent palette ───────────────────────────────────────────────────────

/// Build the 8-slot AccentPalette (RGBA × 8 = 32 bytes) from three RGB colors.
/// Byte order: [R, G, B, alpha] where alpha is ignored by Windows.
///
/// Slot semantics (AveYo's research):
///   0 = Links in action center / apps          ← accent
///   1 = Taskbar icon underline                 ← accent
///   2 = Start button hover                     ← accent
///   3 = Settings icons and links               ← accent (the main one)
///   4 = Start menu bg / active taskbar button  ← bg (base01 for slight contrast)
///   5 = Taskbar front / start list folder bg   ← bg (base00)
///   6 = Taskbar background                     ← bg (base00)
///   7 = Unused                                 ← bg (base00)
fn build_accent_palette(
    accent: (u8, u8, u8), // base0D
    bg: (u8, u8, u8),     // base00
    bg1: (u8, u8, u8),    // base01
) -> [u8; 32] {
    let slots: [(u8, u8, u8); 8] = [
        accent,  // 0 accent — links / action center
        accent,  // 1 accent — taskbar icon underline
        accent,  // 2 accent — Start button hover
        accent,  // 3 accent — settings icons (the main accent slot)
        bg1,     // 4 base01 — active taskbar button / Start bg
        bg,      // 5 base00 — taskbar front
        bg,      // 6 base00 — taskbar background
        bg,      // 7 base00 — unused
    ];
    let mut out = [0u8; 32];
    for (i, (r, g, b)) in slots.iter().enumerate() {
        out[i * 4]     = *r;
        out[i * 4 + 1] = *g;
        out[i * 4 + 2] = *b;
        out[i * 4 + 3] = 0xFF;
    }
    out
}

// ── Registry helpers ───────────────────────────────────────────────────────

#[cfg(windows)]
fn set_accent_color(scheme: &Scheme) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let (r,   g,   b)   = Scheme::hex_to_rgb(&scheme.base0d); // accent
    let (r0,  g0,  b0)  = Scheme::hex_to_rgb(&scheme.base00); // background
    let (r1,  g1,  b1)  = Scheme::hex_to_rgb(&scheme.base01); // lighter bg

    // AccentColorMenu / AccentColor DWORDs are ABGR (0xAA_BB_GG_RR in memory).
    let abgr:          u32 = 0xFF000000 | ((b  as u32) << 16) | ((g  as u32) << 8) | (r  as u32);
    let abgr_bg:       u32 = 0xFF000000 | ((b0 as u32) << 16) | ((g0 as u32) << 8) | (r0 as u32);
    let abgr_inactive: u32 = 0xFF000000 | ((b1 as u32) << 16) | ((g1 as u32) << 8) | (r1 as u32);

    let palette = build_accent_palette((r, g, b), (r0, g0, b0), (r1, g1, b1));

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // Explorer accent
    let (acc, _) = hkcu.create_subkey(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Accent",
    )?;
    acc.set_value("AccentColorMenu", &abgr)?;
    acc.set_raw_value("AccentPalette", &winreg::RegValue {
        bytes: palette.to_vec(),
        vtype: winreg::enums::REG_BINARY,
    })?;
    acc.set_value("StartColorMenu", &abgr_bg)?; // UWP modal bg = scheme background

    // DWM — window borders
    let (dwm, _) = hkcu.create_subkey(r"SOFTWARE\Microsoft\Windows\DWM")?;
    dwm.set_value("AccentColor", &abgr)?;
    dwm.set_value("AccentColorInactive", &abgr_inactive)?;
    dwm.set_value("ColorPrevalence", &1u32)?; // show accent on title bars

    // Control Panel\Colors — legacy RGB "R G B" string values used by some UWP surfaces
    let (cp, _) = hkcu.create_subkey(r"Control Panel\Colors")?;
    cp.set_value("Hilight",          &format!("{} {} {}", r, g, b))?;
    cp.set_value("HotTrackingColor", &format!("{} {} {}", r, g, b))?;
    cp.set_value("MenuHilight",      &format!("{} {} {}", r, g, b))?;
    cp.set_value("ActiveBorder",     &format!("{} {} {}", r, g, b))?;

    detail!("  ✓ Windows accent color → #{:02X}{:02X}{:02X}", r, g, b);
    Ok(())
}

#[cfg(not(windows))]
fn set_accent_color(_scheme: &Scheme) -> Result<()> {
    Ok(())
}

#[cfg(windows)]
fn set_dark_mode(dark: bool) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize",
    )?;
    let light: u32 = if dark { 0 } else { 1 };
    key.set_value("AppsUseLightTheme", &light)?;
    key.set_value("SystemUsesLightTheme", &light)?;
    key.set_value("ColorPrevalence", &1u32)?;   // show accent on Start/taskbar/action center
    key.set_value("EnableTransparency", &1u32)?; // needed for active taskbar button highlight
    key.set_value("EnabledBlurBehind", &0u32)?;
    detail!(
        "  ✓ Windows mode → {}",
        if dark { "dark" } else { "light" }
    );
    Ok(())
}

#[cfg(not(windows))]
fn set_dark_mode(_dark: bool) -> Result<()> {
    Ok(())
}

// ── Win32 ──────────────────────────────────────────────────────────────────
//
// These four operations (find the taskbar window, post to it, broadcast a
// settings change, relaunch elevated) are called on every `coat set`. Doing
// them through `powershell -Command Add-Type ...` meant invoking the C#
// compiler two or three times per apply — several seconds of latency, and a
// hard failure under constrained language mode or when PowerShell is absent
// from PATH. They are plain P/Invoke, so we call them directly.
#[cfg(windows)]
mod win32 {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, HWND};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, PostMessageW, SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };

    /// NUL-terminated UTF-16, as every `*W` entry point expects.
    pub fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    /// `HWND_BROADCAST`. Spelled out rather than imported because its type
    /// changes shape between windows-sys releases.
    fn hwnd_broadcast() -> HWND {
        0xFFFF_usize as HWND
    }

    /// The taskbar window, or null when the shell isn't running.
    pub fn shell_tray() -> HWND {
        let class = wide("Shell_TrayWnd");
        unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) }
    }

    /// True while the shell is up. Replaces polling `tasklist` in a loop —
    /// the tray window is also a truer signal than the process, since an
    /// explorer.exe file-manager window can outlive the shell.
    pub fn shell_running() -> bool {
        !shell_tray().is_null()
    }

    /// Post the taskbar's undocumented "Exit Explorer" message (WM_USER+436),
    /// the same one Ctrl+Shift+right-click on the taskbar sends.
    pub fn post_exit_explorer() -> bool {
        let hwnd = shell_tray();
        if hwnd.is_null() {
            return false;
        }
        unsafe { PostMessageW(hwnd, 0x5B4, 0, 0) != 0 }
    }

    /// Tell every top-level window the immersive colour set changed.
    pub fn broadcast_immersive_color_set() {
        let param = wide("ImmersiveColorSet");
        let mut result: usize = 0;
        unsafe {
            SendMessageTimeoutW(
                hwnd_broadcast(),
                WM_SETTINGCHANGE,
                0,
                param.as_ptr() as isize,
                SMTO_ABORTIFHUNG,
                5000,
                &mut result,
            );
        }
    }

    /// Relaunch `exe` with the `runas` verb (UAC prompt), wait for it, and
    /// return its exit code. `None` if the user dismissed the prompt or the
    /// process could not be started.
    pub fn run_elevated_and_wait(exe: &str, params: &str) -> Option<u32> {
        use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
        use windows_sys::Win32::UI::Shell::{
            ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

        let verb = wide("runas");
        let file = wide(exe);
        let args = wide(params);

        unsafe {
            let mut sei: SHELLEXECUTEINFOW = std::mem::zeroed();
            sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            sei.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
            sei.lpVerb = verb.as_ptr();
            sei.lpFile = file.as_ptr();
            sei.lpParameters = args.as_ptr();
            sei.nShow = SW_HIDE;

            if ShellExecuteExW(&mut sei) == 0 || sei.hProcess.is_null() {
                return None; // user clicked No, or the shell refused to launch it
            }

            // 60s is far longer than a dozen registry writes need; it only
            // matters if the child wedges.
            WaitForSingleObject(sei.hProcess, 60_000);
            let mut code: u32 = 1;
            let got = GetExitCodeProcess(sei.hProcess, &mut code);
            CloseHandle(sei.hProcess as HANDLE);
            if got == 0 {
                None
            } else {
                Some(code)
            }
        }
    }

    /// Is this process running with an elevated token?
    pub fn is_elevated() -> bool {
        use windows_sys::Win32::Security::{
            GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
        };
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

        unsafe {
            let mut token: HANDLE = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return false;
            }
            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut returned: u32 = 0;
            let ok = GetTokenInformation(
                token,
                TokenElevation,
                &mut elevation as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut returned,
            );
            CloseHandle(token);
            ok != 0 && elevation.TokenIsElevated != 0
        }
    }
}

/// Restart explorer.exe so the taskbar re-reads AccentPalette from the registry
/// (broadcasting WM_SETTINGCHANGE alone no longer refreshes the taskbar tint on
/// Windows 10 22H2+/11, so a shell restart is still required).
///
/// The desktop/taskbar disappears for ~1 second while it restarts.
///
/// We exit the shell *gracefully* — the same way the taskbar's hidden
/// "Exit Explorer" command (Ctrl+Shift+right-click) does — by posting message
/// 0x5B4 (WM_USER+436) to the `Shell_TrayWnd` window. Unlike `taskkill /f`, this
/// lets Explorer save shell state and tear down its hosted Start/Search windows
/// cleanly, so they re-attach on relaunch. Force-killing explorer orphans those
/// hosts (StartMenuExperienceHost.exe / SearchHost.exe), which is what left the
/// taskbar search box unresponsive after a theme change. Force kill is kept only
/// as a fallback, with a host refresh limited to that path.
fn restart_explorer() {
    #[cfg(windows)]
    {
        use std::process::Command;

        // 1. Graceful "Exit Explorer": PostMessage(Shell_TrayWnd, 0x5B4).
        let posted = win32::post_exit_explorer();

        // 2. Wait (up to ~5s) for the shell to actually exit on its own.
        let mut exited = false;
        if posted {
            for _ in 0..25 {
                std::thread::sleep(Duration::from_millis(200));
                if !win32::shell_running() {
                    exited = true;
                    break;
                }
            }
        }

        // 3. Fallback: if the graceful exit didn't take, force it. This path can
        //    orphan the Start/Search hosts, so refresh them afterwards.
        let forced = !exited;
        if forced {
            let _ = Command::new("taskkill").args(["/f", "/im", "explorer.exe"]).output();
            std::thread::sleep(Duration::from_millis(800));
        }

        // 4. Relaunch the shell (a clean "Exit Explorer" does not auto-restart it).
        let _ = Command::new("explorer.exe").spawn();

        // 5. Only after a forced kill: clear the Start/Search hosts so they
        //    re-attach to the fresh shell (they respawn on next use). Covers
        //    Win11 (SearchHost.exe) and Win10 (SearchUI.exe).
        if forced {
            std::thread::sleep(Duration::from_millis(600));
            let kill = |image: &str| {
                let _ = Command::new("taskkill").args(["/f", "/im", image]).output();
            };
            kill("StartMenuExperienceHost.exe");
            kill("SearchHost.exe");
            kill("SearchUI.exe");
        }
    }
}

/// Broadcast WM_SETTINGCHANGE so Explorer/taskbar refresh live.
fn broadcast_settings_change() {
    #[cfg(windows)]
    {
        win32::broadcast_immersive_color_set();
    }
}

// ── Windows Terminal ───────────────────────────────────────────────────────

fn windows_terminal_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Try LOCALAPPDATA for Store installs
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let local = PathBuf::from(local);

        // Stable
        let stable = local.join(r"Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json");
        if stable.exists() {
            paths.push(stable);
        }
        // Preview
        let preview = local.join(r"Packages\Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe\LocalState\settings.json");
        if preview.exists() {
            paths.push(preview);
        }
        // Portable / unpackaged
        let portable = local.join(r"Microsoft\Windows Terminal\settings.json");
        if portable.exists() {
            paths.push(portable);
        }
    }

    paths
}

fn build_wt_scheme(scheme: &Scheme) -> serde_json::Value {
    let h = |c: &str| format!("#{}", c);
    serde_json::json!({
        "name": "coat",
        "background":     h(&scheme.base00),
        "foreground":     h(&scheme.base05),
        "cursorColor":    h(&scheme.base05),
        "selectionBackground": h(&scheme.base02),
        "black":          h(&scheme.base00),
        "brightBlack":    h(&scheme.base03),
        "red":            h(&scheme.base08),
        "brightRed":      h(&scheme.base08),
        "green":          h(&scheme.base0b),
        "brightGreen":    h(&scheme.base0b),
        "yellow":         h(&scheme.base0a),
        "brightYellow":   h(&scheme.base0a),
        "blue":           h(&scheme.base0d),
        "brightBlue":     h(&scheme.base0d),
        "purple":         h(&scheme.base0e),
        "brightPurple":   h(&scheme.base0e),
        "cyan":           h(&scheme.base0c),
        "brightCyan":     h(&scheme.base0c),
        "white":          h(&scheme.base05),
        "brightWhite":    h(&scheme.base07),
    })
}

/// Point `profiles.defaults` at the coat scheme and carry over the font and
/// opacity from coat.yaml, so every profile that hasn't overridden them picks
/// the theme up without the user hand-editing settings.json.
///
/// Only the keys coat owns are touched — anything else already in `defaults`
/// (and any per-profile override in `profiles.list`) is left alone.
fn apply_wt_defaults(
    settings: &mut serde_json::Map<String, JsonValue>,
    config: &CoatConfig,
) -> Result<()> {
    // The modern schema is {"profiles": {"defaults": {...}, "list": [...]}}.
    // Very old settings.json files have "profiles" as a bare array, which has
    // nowhere to put defaults — leave those alone rather than restructure them.
    let profiles = settings
        .entry("profiles")
        .or_insert_with(|| JsonValue::Object(serde_json::Map::new()));
    if profiles.is_array() {
        detail!("    (legacy 'profiles' array — set \"colorScheme\": \"coat\" by hand)");
        return Ok(());
    }
    let profiles = profiles
        .as_object_mut()
        .context("settings.json 'profiles' is neither an object nor an array")?;

    let defaults = profiles
        .entry("defaults")
        .or_insert_with(|| JsonValue::Object(serde_json::Map::new()))
        .as_object_mut()
        .context("settings.json 'profiles.defaults' is not an object")?;

    defaults.insert("colorScheme".into(), JsonValue::String("coat".into()));

    // Font and opacity are read as raw Options rather than through the
    // `config.*()` accessors: those substitute coat's own defaults for absent
    // keys, and stamping those into settings.json would silently override
    // whatever the user had set in Windows Terminal itself. A key coat.yaml
    // doesn't mention is a key coat doesn't write. Explicitly setting the
    // default value still writes it, so changes revert cleanly.

    // Written under the nested "font" object; the flat fontFace/fontSize keys
    // are deprecated.
    let face = Some(config.font_monospace()).filter(|f| !f.is_empty());
    let size = config.font.sizes.as_ref().and_then(|s| s.terminal);
    if face.is_some() || size.is_some() {
        let font = defaults
            .entry("font")
            .or_insert_with(|| JsonValue::Object(serde_json::Map::new()))
            .as_object_mut()
            .context("settings.json 'profiles.defaults.font' is not an object")?;
        if let Some(face) = face {
            font.insert("face".into(), JsonValue::String(face.to_string()));
        }
        if let Some(size) = size {
            font.insert("size".into(), JsonValue::Number(size.into()));
        }
    }

    // coat.yaml stores opacity as 0.0–1.0, Windows Terminal wants 0–100.
    // `useAcrylic: false` gives plain transparency, matching what the Linux
    // terminal module does with the same setting (foot doesn't blur).
    if let Some(opacity) = config.opacity.terminal {
        let pct = (opacity.clamp(0.0, 1.0) * 100.0).round() as u64;
        defaults.insert("opacity".into(), JsonValue::Number(pct.into()));
        defaults.insert("useAcrylic".into(), JsonValue::Bool(false));
    }

    Ok(())
}

fn apply_windows_terminal_to(
    path: &std::path::Path,
    scheme: &Scheme,
    config: &CoatConfig,
) -> Result<()> {
    // JSONC-tolerant read; errors out instead of clobbering an unparseable file.
    let mut settings: serde_json::Map<String, JsonValue> =
        crate::modules::read_json_settings(path)?;

    // Ensure "schemes" array exists
    let schemes_arr = settings
        .entry("schemes")
        .or_insert_with(|| JsonValue::Array(vec![]))
        .as_array_mut()
        .context("settings.json 'schemes' is not an array")?;

    // Replace existing "coat" entry or append
    let new_scheme = build_wt_scheme(scheme);
    let existing = schemes_arr.iter().position(|s| {
        s.get("name").and_then(|n| n.as_str()) == Some("coat")
    });
    match existing {
        Some(i) => schemes_arr[i] = new_scheme,
        None    => schemes_arr.push(new_scheme),
    }

    // Select it, and match the window chrome to the scheme's polarity.
    apply_wt_defaults(&mut settings, config)?;
    settings.insert(
        "theme".into(),
        JsonValue::String(if scheme.is_dark() { "dark" } else { "light" }.into()),
    );

    let out = serde_json::to_string_pretty(&settings)
        .context("Failed to serialize Windows Terminal settings")?;
    fs::write(path, out)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    detail!("  ✓ {}", path.display());
    Ok(())
}

// ── Discord on Windows ────────────────────────────────────────────────────

fn discord_theme_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        let base = PathBuf::from(appdata);
        // Vencord standalone
        let vencord = base.join(r"Vencord\themes");
        if vencord.parent().map(|p| p.is_dir()).unwrap_or(false) {
            paths.push(vencord);
        }
        // BetterDiscord
        let bd = base.join(r"BetterDiscord\themes");
        if bd.parent().map(|p| p.is_dir()).unwrap_or(false) {
            paths.push(bd);
        }
        // Vesktop (Windows build)
        let vesktop = base.join(r"vesktop\themes");
        if vesktop.parent().map(|p| p.is_dir()).unwrap_or(false) {
            paths.push(vesktop);
        }
    }
    paths
}

pub fn apply_discord(scheme: &Scheme, config: &CoatConfig) -> Result<()> {
    let paths = discord_theme_paths();
    if paths.is_empty() {
        detail!("  (no Discord mod found — skipping)");
        detail!("  Supported: Vencord, BetterDiscord, Vesktop");
        return Ok(());
    }
    for dir in &paths {
        fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create {}", dir.display()))?;
        let dest = dir.join("coat.theme.css");
        crate::modules::apply_vesktop_shared(scheme, config, &dest)?;
    }
    detail!("  Enable the 'coat' theme in your Discord mod's theme settings.");
    Ok(())
}

// ── VSCode on Windows ──────────────────────────────────────────────────────

fn vscode_settings_path_windows() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    let path = PathBuf::from(appdata).join(r"Code\User\settings.json");
    Some(path)
}

// ── Public entry points ────────────────────────────────────────────────────

pub fn apply_accent(scheme: &Scheme) -> Result<()> {
    set_accent_color(scheme)?;
    broadcast_settings_change();
    Ok(())
}

pub fn apply_mode(scheme: &Scheme) -> Result<()> {
    set_dark_mode(scheme.is_dark())?;
    broadcast_settings_change();
    Ok(())
}

pub fn apply_terminal(scheme: &Scheme, config: &CoatConfig) -> Result<()> {
    let paths = windows_terminal_paths();
    if paths.is_empty() {
        detail!("  (Windows Terminal not found — skipping)");
        return Ok(());
    }
    for path in &paths {
        apply_windows_terminal_to(path, scheme, config)?;
    }
    Ok(())
}

/// `font.monospace` from coat.yaml, or `None` when it's unset — the shared
/// The VSCode writer leaves the editor's own font alone in that case.
fn configured_font(config: &CoatConfig) -> Option<&str> {
    Some(config.font_monospace()).filter(|f| !f.is_empty())
}

pub fn apply_vscode(scheme: &Scheme, config: &CoatConfig) -> Result<()> {
    // Re-use the same JSON building logic from modules.rs but on the Windows path
    let Some(path) = vscode_settings_path_windows() else {
        detail!("  (VSCode not found at %APPDATA%\\Code — skipping)");
        return Ok(());
    };
    crate::modules::apply_vscode_shared(scheme, &path, configured_font(config))
}

/// Write the registry keys that require admin: the logon-screen accent
/// (HKU\.DEFAULT) and the DWM effect mode (HKLM). Fails with the first
/// access-denied error rather than reporting partial success.
#[cfg(windows)]
fn write_elevated_keys(accent: &str, bg: &str, bg1: &str, dark: bool) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let (r,  g,  b)  = Scheme::hex_to_rgb(accent);
    let (r0, g0, b0) = Scheme::hex_to_rgb(bg);
    let (r1, g1, b1) = Scheme::hex_to_rgb(bg1);
    let abgr:          u32 = 0xFF000000 | ((b  as u32) << 16) | ((g  as u32) << 8) | (r  as u32);
    let abgr_bg:       u32 = 0xFF000000 | ((b0 as u32) << 16) | ((g0 as u32) << 8) | (r0 as u32);
    let abgr_inactive: u32 = 0xFF000000 | ((b1 as u32) << 16) | ((g1 as u32) << 8) | (r1 as u32);
    let palette = build_accent_palette((r, g, b), (r0, g0, b0), (r1, g1, b1));
    let light: u32 = if dark { 0 } else { 1 };

    // Mirror HKCU\...\Explorer\Accent onto HKU\.DEFAULT for the logon screen
    let hku = RegKey::predef(HKEY_USERS);
    let (acc, _) = hku
        .create_subkey(r".DEFAULT\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Accent")
        .context("HKU\\.DEFAULT accent key")?;
    acc.set_value("AccentColorMenu", &abgr)?;
    acc.set_raw_value("AccentPalette", &winreg::RegValue {
        bytes: palette.to_vec(),
        vtype: winreg::enums::REG_BINARY,
    })?;
    acc.set_value("StartColorMenu", &abgr_bg)?;

    let (dwm, _) = hku
        .create_subkey(r".DEFAULT\SOFTWARE\Microsoft\Windows\DWM")
        .context("HKU\\.DEFAULT DWM key")?;
    dwm.set_value("AccentColor", &abgr)?;
    dwm.set_value("AccentColorInactive", &abgr_inactive)?;
    dwm.set_value("ColorPrevalence", &1u32)?;

    let (pers, _) = hku
        .create_subkey(r".DEFAULT\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .context("HKU\\.DEFAULT Personalize key")?;
    pers.set_value("AppsUseLightTheme", &light)?;
    pers.set_value("SystemUsesLightTheme", &light)?;
    pers.set_value("ColorPrevalence", &1u32)?;
    pers.set_value("EnableTransparency", &1u32)?;

    // HKLM\...\Dwm: ForceEffectMode=1 keeps taskbar dark while transparency is on
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let (key, _) = hklm
        .create_subkey(r"SOFTWARE\Microsoft\Windows\Dwm")
        .context("HKLM DWM key")?;
    key.set_value("ForceEffectMode", &1u32)?;

    Ok(())
}

/// Hidden `coat __winelevate <base0D> <base00> <base01> <dark|light>` entry
/// point. This is what the UAC-elevated child process runs: only the registry
/// writes above, with the colours passed on the command line so the child
/// never has to re-read coat.yaml or the scheme library (and so can't pick a
/// different scheme than the parent did).
#[cfg(windows)]
pub fn cmd_elevated_keys(args: &[String]) -> Result<()> {
    if args.len() < 4 {
        anyhow::bail!("__winelevate expects <accent> <bg> <bg1> <dark|light>");
    }
    write_elevated_keys(&args[0], &args[1], &args[2], args[3] == "dark")
}

/// Apply the admin-only keys, relaunching under UAC when `elevate` is set.
///
/// Without `--elevate` we still attempt the writes — they succeed outright if
/// coat was launched from an already-elevated shell — and only then report
/// that the flag exists.
#[cfg(windows)]
fn apply_elevated(scheme: &Scheme, dark: bool, elevate: bool) -> Result<()> {
    let write = || write_elevated_keys(&scheme.base0d, &scheme.base00, &scheme.base01, dark);

    if win32::is_elevated() {
        return write();
    }
    if !elevate {
        return write().context("needs admin — re-run with --elevate");
    }

    let exe = std::env::current_exe()
        .context("Cannot locate the coat executable to relaunch")?;
    let params = format!(
        "__winelevate {} {} {} {}",
        scheme.base0d,
        scheme.base00,
        scheme.base01,
        if dark { "dark" } else { "light" },
    );

    match win32::run_elevated_and_wait(&exe.to_string_lossy(), &params) {
        Some(0) => {
            detail!("  ✓ Logon screen + HKLM keys (elevated)");
            Ok(())
        }
        Some(code) => anyhow::bail!("elevated helper exited with code {}", code),
        None => anyhow::bail!("elevation declined or failed"),
    }
}

#[cfg(not(windows))]
fn apply_elevated(_scheme: &Scheme, _dark: bool, _elevate: bool) -> Result<()> {
    Ok(())
}

/// Apply all Windows platform defaults for a given scheme.
///
/// `elevate` triggers a UAC prompt for the logon-screen and HKLM keys; without
/// it that one step is reported as skipped and everything else still applies.
pub fn apply_all(scheme: &Scheme, config: &CoatConfig, elevate: bool) -> Result<()> {
    println!("Applying Windows theme: {}\n", scheme.name);

    crate::modules::set_quiet(true);

    step("Accent color", || apply_accent(scheme));
    step("Dark/light mode", || apply_mode(scheme));
    step("Logon/HKLM keys", || apply_elevated(scheme, scheme.is_dark(), elevate));
    step("Windows Terminal", || apply_terminal(scheme, config));
    step("VSCode", || apply_vscode(scheme, config));
    step("Firefox", || {
        let tera = crate::modules::make_tera()?;
        crate::modules::apply_module("firefox", scheme, config, &tera)
    });
    step("Discord (Vencord/BetterDiscord)", || apply_discord(scheme, config));
    step("Explorer restart", || {
        restart_explorer();
        Ok(())
    });

    crate::modules::set_quiet(false);

    println!();
    println!("{}", style("Done!").green().bold());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(yaml: &str) -> CoatConfig {
        serde_yaml::from_str(yaml).expect("test config should parse")
    }

    fn merge(settings_json: &str, yaml: &str) -> serde_json::Map<String, JsonValue> {
        let mut settings: serde_json::Map<String, JsonValue> =
            serde_json::from_str(settings_json).expect("test settings should parse");
        apply_wt_defaults(&mut settings, &config(yaml)).expect("merge should succeed");
        settings
    }

    const FULL: &str = "
scheme: gruvbox
font:
  monospace: Iosevka
  sizes:
    terminal: 14
opacity:
  terminal: 0.85
";

    #[test]
    fn creates_defaults_in_empty_settings() {
        let out = merge("{}", FULL);
        let defaults = &out["profiles"]["defaults"];
        assert_eq!(defaults["colorScheme"], "coat");
        assert_eq!(defaults["font"]["face"], "Iosevka");
        assert_eq!(defaults["font"]["size"], 14);
        // coat.yaml stores 0.0–1.0; Windows Terminal wants 0–100.
        assert_eq!(defaults["opacity"], 85);
        assert_eq!(defaults["useAcrylic"], false);
    }

    #[test]
    fn preserves_unrelated_keys() {
        let out = merge(
            r#"{
                "profiles": {
                    "defaults": { "historySize": 9001, "font": { "weight": "bold" } },
                    "list": [ { "name": "PowerShell", "colorScheme": "Campbell" } ]
                }
            }"#,
            FULL,
        );
        let defaults = &out["profiles"]["defaults"];
        // Keys coat doesn't own survive, at both levels of nesting.
        assert_eq!(defaults["historySize"], 9001);
        assert_eq!(defaults["font"]["weight"], "bold");
        assert_eq!(defaults["font"]["face"], "Iosevka");
        // Per-profile overrides in the list are never rewritten.
        assert_eq!(out["profiles"]["list"][0]["colorScheme"], "Campbell");
    }

    #[test]
    fn legacy_profiles_array_is_left_alone() {
        let out = merge(r#"{"profiles": [ { "name": "cmd" } ]}"#, FULL);
        // No defaults grafted onto a shape that has nowhere to put them.
        assert!(out["profiles"].is_array());
        assert_eq!(out["profiles"][0]["name"], "cmd");
    }

    #[test]
    fn explicit_full_opacity_is_still_written() {
        // So that going 0.85 → 1.0 in coat.yaml actually resets the terminal
        // rather than leaving it stuck translucent.
        let out = merge("{}", "scheme: gruvbox\nopacity:\n  terminal: 1.0\n");
        assert_eq!(out["profiles"]["defaults"]["opacity"], 100);
    }

    #[test]
    fn keys_absent_from_coat_yaml_are_not_written() {
        // A bare coat.yaml should select the scheme and touch nothing else —
        // no stamping coat's own font/opacity defaults over the user's
        // Windows Terminal settings.
        let out = merge(
            r#"{"profiles": {"defaults": {"font": {"face": "Cascadia Code"}}}}"#,
            "scheme: gruvbox",
        );
        let defaults = &out["profiles"]["defaults"];
        assert_eq!(defaults["colorScheme"], "coat");
        assert_eq!(defaults["font"]["face"], "Cascadia Code");
        assert!(defaults["font"].get("size").is_none());
        assert!(defaults.get("opacity").is_none());
        assert!(defaults.get("useAcrylic").is_none());
    }
}
