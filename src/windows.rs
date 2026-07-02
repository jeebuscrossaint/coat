/// Windows-specific theming: accent color, dark/light mode, Windows Terminal.
/// Compiled only on Windows — Linux builds ignore this entire file.
use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::PathBuf;

use crate::config::CoatConfig;
use crate::scheme::Scheme;

// ── Palette shading ────────────────────────────────────────────────────────

/// Blend `color` toward white (positive t) or black (negative t), t in [0,1].
fn shade(r: u8, g: u8, b: u8, toward_white: bool, t: f32) -> (u8, u8, u8) {
    let blend = |c: u8, target: u8| -> u8 {
        (c as f32 + (target as f32 - c as f32) * t).round() as u8
    };
    let target = if toward_white { 255 } else { 0 };
    (blend(r, target), blend(g, target), blend(b, target))
}

/// Build the 8-shade AccentPalette (RGBA × 8 = 32 bytes) from a base color.
/// Build the 8-slot AccentPalette.
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
    ar: u8, ag: u8, ab: u8,   // accent color  (base0D)
    br: u8, bg: u8, bb: u8,   // background    (base00)
    b1r: u8, b1g: u8, b1b: u8, // lighter bg   (base01)
) -> [u8; 32] {
    let slots: [(u8, u8, u8); 8] = [
        (ar,  ag,  ab),   // 0 accent — links / action center
        (ar,  ag,  ab),   // 1 accent — taskbar icon underline
        (ar,  ag,  ab),   // 2 accent — Start button hover
        (ar,  ag,  ab),   // 3 accent — settings icons (the main accent slot)
        (b1r, b1g, b1b),  // 4 base01 — active taskbar button / Start bg
        (br,  bg,  bb),   // 5 base00 — taskbar front
        (br,  bg,  bb),   // 6 base00 — taskbar background
        (br,  bg,  bb),   // 7 base00 — unused
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

    let palette = build_accent_palette(r, g, b, r0, g0, b0, r1, g1, b1);

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

    println!("  ✓ Windows accent color → #{:02X}{:02X}{:02X}", r, g, b);
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
    println!(
        "  ✓ Windows mode → {}",
        if dark { "dark" } else { "light" }
    );
    Ok(())
}

#[cfg(not(windows))]
fn set_dark_mode(_dark: bool) -> Result<()> {
    Ok(())
}

/// Is at least one explorer.exe process currently running?
#[cfg(windows)]
fn explorer_running() -> bool {
    std::process::Command::new("tasklist")
        .args(["/fi", "IMAGENAME eq explorer.exe", "/nh"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .to_lowercase()
                .contains("explorer.exe")
        })
        .unwrap_or(false)
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
        use std::time::Duration;

        print!("  Restarting Explorer... ");

        // 1. Graceful "Exit Explorer": PostMessage(Shell_TrayWnd, 0x5B4).
        //    Done via a PowerShell P/Invoke shim (no extra crate dependency).
        let ps = r#"$s='[DllImport("user32.dll",SetLastError=true,CharSet=CharSet.Auto)] public static extern IntPtr FindWindow(string c,string w); [DllImport("user32.dll")] public static extern int PostMessage(IntPtr h,uint m,IntPtr wp,IntPtr lp);'; $t=Add-Type -MemberDefinition $s -Name Shell -Namespace CoatWin -PassThru; $h=$t::FindWindow('Shell_TrayWnd',$null); if($h -ne [IntPtr]::Zero){[void]$t::PostMessage($h,0x5B4,[IntPtr]::Zero,[IntPtr]::Zero)}"#;
        let posted = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        // 2. Wait (up to ~5s) for the shell to actually exit on its own.
        let mut exited = false;
        if posted {
            for _ in 0..25 {
                std::thread::sleep(Duration::from_millis(200));
                if !explorer_running() {
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

        println!("✓");
    }
}

/// Broadcast WM_SETTINGCHANGE so Explorer/taskbar refresh live.
fn broadcast_settings_change() {
    #[cfg(windows)]
    {
        // PowerShell one-liner to SendMessageTimeout(HWND_BROADCAST, WM_SETTINGCHANGE, ...)
        let ps = concat!(
            r#"$t=Add-Type -PassThru -TypeDefinition '"#,
            r#"using System;using System.Runtime.InteropServices;"#,
            r#"public class U{"#,
            r#"[DllImport(\"user32.dll\")]"#,
            r#"public static extern IntPtr SendMessageTimeout(IntPtr h,uint m,IntPtr w,string l,uint f,uint t,out IntPtr r);"#,
            r#"}'"#,
            r#";$r=[IntPtr]::Zero;"#,
            r#"[U]::SendMessageTimeout([IntPtr]0xFFFF,0x1A,[IntPtr]0,'ImmersiveColorSet',2,5000,[ref]$r)"#,
        );
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .output();
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

fn apply_windows_terminal_to(path: &std::path::Path, scheme: &Scheme) -> Result<()> {
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

    let out = serde_json::to_string_pretty(&settings)
        .context("Failed to serialize Windows Terminal settings")?;
    fs::write(path, out)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    println!("  ✓ {}", path.display());
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
        println!("  (no Discord mod found — skipping)");
        println!("  Supported: Vencord, BetterDiscord, Vesktop");
        return Ok(());
    }
    for dir in &paths {
        fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create {}", dir.display()))?;
        let dest = dir.join("coat.theme.css");
        crate::modules::apply_vesktop_shared(scheme, config, &dest)?;
    }
    println!("  Enable the 'coat' theme in your Discord mod's theme settings.");
    Ok(())
}

// ── VSCode on Windows ──────────────────────────────────────────────────────

fn vscode_settings_path_windows() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    let path = PathBuf::from(appdata).join(r"Code\User\settings.json");
    Some(path)
}

// ── Zed on Windows ─────────────────────────────────────────────────────────

/// Zed stores its config under %APPDATA%\Zed on Windows.
fn zed_paths_windows() -> Option<(PathBuf, PathBuf)> {
    let appdata = std::env::var("APPDATA").ok()?;
    let base = PathBuf::from(appdata).join("Zed");
    let settings = base.join("settings.json");
    let themes = base.join("themes");
    Some((settings, themes))
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

pub fn apply_terminal(scheme: &Scheme) -> Result<()> {
    let paths = windows_terminal_paths();
    if paths.is_empty() {
        println!("  (Windows Terminal not found — skipping)");
        return Ok(());
    }
    for path in &paths {
        apply_windows_terminal_to(path, scheme)?;
    }
    println!("    Add \"colorScheme\": \"coat\" to your Windows Terminal profile.");
    Ok(())
}

pub fn apply_vscode(scheme: &Scheme) -> Result<()> {
    // Re-use the same JSON building logic from modules.rs but on the Windows path
    let Some(path) = vscode_settings_path_windows() else {
        println!("  (VSCode not found at %APPDATA%\\Code — skipping)");
        return Ok(());
    };
    // Delegate to the shared VSCode apply function in modules.rs (no font on Windows)
    crate::modules::apply_vscode_shared(scheme, &path, None)
}

pub fn apply_zed(scheme: &Scheme) -> Result<()> {
    let Some((settings, themes)) = zed_paths_windows() else {
        println!("  (Zed not found at %APPDATA%\\Zed — skipping)");
        return Ok(());
    };
    // Only apply if Zed's config dir already exists — avoids creating it blind.
    if !settings.parent().map(|p| p.is_dir()).unwrap_or(false) {
        println!("  (Zed not found at %APPDATA%\\Zed — skipping)");
        return Ok(());
    }
    // Delegate to the shared Zed apply function in modules.rs (no font on Windows)
    crate::modules::apply_zed_shared(scheme, &settings, &themes, None)
}

/// Try to write registry keys that require admin (HKLM + HKU\.DEFAULT).
/// Silently skips on access-denied; prints a hint if any key fails.
#[cfg(windows)]
fn try_set_elevated(scheme: &Scheme, dark: bool) {
    use winreg::enums::*;
    use winreg::RegKey;

    let (r,  g,  b)  = Scheme::hex_to_rgb(&scheme.base0d);
    let (r0, g0, b0) = Scheme::hex_to_rgb(&scheme.base00);
    let (r1, g1, b1) = Scheme::hex_to_rgb(&scheme.base01);
    let abgr:          u32 = 0xFF000000 | ((b  as u32) << 16) | ((g  as u32) << 8) | (r  as u32);
    let abgr_bg:       u32 = 0xFF000000 | ((b0 as u32) << 16) | ((g0 as u32) << 8) | (r0 as u32);
    let abgr_inactive: u32 = 0xFF000000 | ((b1 as u32) << 16) | ((g1 as u32) << 8) | (r1 as u32);
    let palette = build_accent_palette(r, g, b, r0, g0, b0, r1, g1, b1);
    let light: u32 = if dark { 0 } else { 1 };
    let mut any_failed = false;

    // Mirror HKCU\...\Explorer\Accent onto HKU\.DEFAULT for the logon screen
    let hku_accent = || -> std::io::Result<()> {
        let hku = RegKey::predef(HKEY_USERS);
        let (acc, _) = hku.create_subkey(
            r".DEFAULT\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Accent",
        )?;
        acc.set_value("AccentColorMenu", &abgr)?;
        acc.set_raw_value("AccentPalette", &winreg::RegValue {
            bytes: palette.to_vec(),
            vtype: winreg::enums::REG_BINARY,
        })?;
        acc.set_value("StartColorMenu", &abgr_bg)?;
        let (dwm, _) = hku.create_subkey(r".DEFAULT\SOFTWARE\Microsoft\Windows\DWM")?;
        dwm.set_value("AccentColor", &abgr)?;
        dwm.set_value("AccentColorInactive", &abgr_inactive)?;
        dwm.set_value("ColorPrevalence", &1u32)?;
        let (pers, _) = hku.create_subkey(
            r".DEFAULT\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize",
        )?;
        pers.set_value("AppsUseLightTheme", &light)?;
        pers.set_value("SystemUsesLightTheme", &light)?;
        pers.set_value("ColorPrevalence", &1u32)?;
        pers.set_value("EnableTransparency", &1u32)?;
        Ok(())
    };
    if let Err(_) = hku_accent() { any_failed = true; }

    // HKLM\...\Dwm: ForceEffectMode=1 keeps taskbar dark while transparency is on
    let hklm_dwm = || -> std::io::Result<()> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let (key, _) = hklm.create_subkey(r"SOFTWARE\Microsoft\Windows\Dwm")?;
        key.set_value("ForceEffectMode", &1u32)?;
        Ok(())
    };
    if let Err(_) = hklm_dwm() { any_failed = true; }

    if any_failed {
        println!("  (some logon/HKLM keys skipped — re-run as administrator for full effect)");
    } else {
        println!("  ✓ Logon screen + HKLM keys");
    }
}

#[cfg(not(windows))]
fn try_set_elevated(_scheme: &Scheme, _dark: bool) {}

/// Apply all Windows platform defaults for a given scheme.
pub fn apply_all(scheme: &Scheme, config: &CoatConfig) -> Result<()> {
    println!("Applying Windows theme: {}\n", scheme.name);

    println!("Applying accent color...");
    apply_accent(scheme)?;
    println!();

    println!("Applying dark/light mode...");
    apply_mode(scheme)?;
    println!();

    println!("Applying logon/HKLM keys (best-effort, needs admin)...");
    try_set_elevated(scheme, scheme.is_dark());
    println!();

    println!("Applying Windows Terminal color scheme...");
    apply_terminal(scheme)?;
    println!();

    println!("Applying VSCode colors...");
    apply_vscode(scheme)?;
    println!();

    println!("Applying Zed colors...");
    apply_zed(scheme)?;
    println!();

    println!("Applying Discord theme (Vencord/BetterDiscord)...");
    apply_discord(scheme, config)?;
    println!();

    println!("Restarting Explorer (taskbar will flicker briefly)...");
    restart_explorer();
    println!();

    println!("✓ Done!");
    Ok(())
}
