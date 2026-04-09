/// Windows-specific theming: accent color, dark/light mode, Windows Terminal.
/// Compiled only on Windows — Linux builds ignore this entire file.
use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::PathBuf;

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
/// Byte order per the Windows registry: [R, G, B, alpha] where alpha is ignored.
/// Slot semantics (from AveYo's research):
///   0 = Links in action center / apps
///   1 = Taskbar icon underline
///   2 = Start button hover
///   3 = Settings icons and links  ← the accent color itself
///   4 = Start menu bg / active taskbar button (when transparency off)
///   5 = Taskbar front / start list folder bg
///   6 = Taskbar bg (when transparency on)
///   7 = Unused
fn build_accent_palette(r: u8, g: u8, b: u8) -> [u8; 32] {
    let shades: [(u8, u8, u8); 8] = [
        shade(r, g, b, true,  0.60),  // 0 lightest
        shade(r, g, b, true,  0.45),  // 1
        shade(r, g, b, true,  0.30),  // 2
        (r, g, b),                     // 3 — the accent itself
        shade(r, g, b, false, 0.20),  // 4
        shade(r, g, b, false, 0.40),  // 5
        shade(r, g, b, false, 0.55),  // 6
        shade(r, g, b, false, 0.70),  // 7 darkest
    ];
    let mut out = [0u8; 32];
    for (i, (sr, sg, sb)) in shades.iter().enumerate() {
        // [R, G, B, alpha] — alpha byte is ignored by Windows
        out[i * 4]     = *sr;
        out[i * 4 + 1] = *sg;
        out[i * 4 + 2] = *sb;
        out[i * 4 + 3] = 0xFF;
    }
    out
}

// ── Registry helpers ───────────────────────────────────────────────────────

#[cfg(windows)]
fn set_accent_color(scheme: &Scheme) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let (r, g, b)    = Scheme::hex_to_rgb(&scheme.base0d);
    let (r1, g1, b1) = Scheme::hex_to_rgb(&scheme.base01); // for inactive/start

    // AccentColorMenu / AccentColor DWORDs are ABGR (0xAA_BB_GG_RR in memory).
    let abgr:          u32 = 0xFF000000 | ((b  as u32) << 16) | ((g  as u32) << 8) | (r  as u32);
    let abgr_inactive: u32 = 0xFF000000 | ((b1 as u32) << 16) | ((g1 as u32) << 8) | (r1 as u32);

    let palette = build_accent_palette(r, g, b);

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
    acc.set_value("StartColorMenu", &abgr_inactive)?; // UWP modal bg

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
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let mut settings: serde_json::Map<String, JsonValue> =
        serde_json::from_str(&content).unwrap_or_default();

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

/// Try to write registry keys that require admin (HKLM + HKU\.DEFAULT).
/// Silently skips on access-denied; prints a hint if any key fails.
#[cfg(windows)]
fn try_set_elevated(scheme: &Scheme, dark: bool) {
    use winreg::enums::*;
    use winreg::RegKey;

    let (r, g, b)    = Scheme::hex_to_rgb(&scheme.base0d);
    let (r1, g1, b1) = Scheme::hex_to_rgb(&scheme.base01);
    let abgr:          u32 = 0xFF000000 | ((b  as u32) << 16) | ((g  as u32) << 8) | (r  as u32);
    let abgr_inactive: u32 = 0xFF000000 | ((b1 as u32) << 16) | ((g1 as u32) << 8) | (r1 as u32);
    let palette = build_accent_palette(r, g, b);
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
        acc.set_value("StartColorMenu", &abgr_inactive)?;
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
pub fn apply_all(scheme: &Scheme) -> Result<()> {
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

    println!("✓ Done! Some changes (accent color) may require signing out and back in.");
    Ok(())
}
