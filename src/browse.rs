use anyhow::{bail, Result};
use console::{Key, Term};

use crate::scheme::{color_preview_string, header_string, load_all_schemes, Scheme};

/// Case-insensitive substring match across a scheme's name, author, and slug.
fn matches_query(scheme: &Scheme, query_lower: &str) -> bool {
    scheme.name.to_lowercase().contains(query_lower)
        || scheme.author.to_lowercase().contains(query_lower)
        || scheme.slug.to_lowercase().contains(query_lower)
}

/// Interactively scroll through the scheme library one preview at a time.
/// Returns `Some(scheme)` when the user presses Enter to choose one, or `None`
/// if they quit. `variant_filter` optionally restricts to "dark" or "light".
pub fn browse(variant_filter: Option<&str>) -> Result<Option<Scheme>> {
    let mut schemes = load_all_schemes()?;
    if let Some(vf) = variant_filter {
        schemes.retain(|s| s.variant.to_lowercase().contains(vf));
    }
    schemes.sort_by(|a, b| a.name.cmp(&b.name));
    if schemes.is_empty() {
        bail!("No schemes found matching criteria.");
    }

    let term = Term::stdout();
    if !term.is_term() {
        bail!("`coat browse` needs an interactive terminal — try `coat list` instead.");
    }

    // Indices into `schemes` that satisfy the live filter query.
    let filter = |query: &str| -> Vec<usize> {
        if query.is_empty() {
            return (0..schemes.len()).collect();
        }
        let ql = query.to_lowercase();
        schemes
            .iter()
            .enumerate()
            .filter(|(_, s)| matches_query(s, &ql))
            .map(|(i, _)| i)
            .collect()
    };

    let mut query = String::new();
    let mut matches = filter(&query);
    let mut sel: usize = 0;
    let mut filtering = false;

    term.hide_cursor().ok();
    // Run the loop inside a closure so we always restore the cursor afterward.
    let outcome = (|| -> Result<Option<Scheme>> {
        loop {
            if matches.is_empty() {
                sel = 0;
            } else if sel >= matches.len() {
                sel = matches.len() - 1;
            }

            let mut frame = String::new();
            let badge = variant_filter
                .map(|v| format!("  [{}]", v))
                .unwrap_or_default();
            if matches.is_empty() {
                frame.push_str(&format!(
                    "\x1b[1mcoat browse\x1b[0m  0 matches{}\n\n",
                    badge
                ));
                frame.push_str(&format!("No schemes match \"{}\".\n", query));
            } else {
                frame.push_str(&format!(
                    "\x1b[1mcoat browse\x1b[0m  {}/{}{}\n\n",
                    sel + 1,
                    matches.len(),
                    badge
                ));
                let scheme = &schemes[matches[sel]];
                frame.push_str(&header_string(scheme));
                frame.push_str(&color_preview_string(scheme));
            }

            if filtering {
                frame.push_str(&format!(
                    "\n\x1b[1mFilter:\x1b[0m {}\u{2588}   \x1b[2m(Enter accept · Esc clear)\x1b[0m\n",
                    query
                ));
            } else {
                frame.push_str(
                    "\n\x1b[2m↑/↓ or j/k scroll · / filter · Enter apply · q quit\x1b[0m\n",
                );
                if !query.is_empty() {
                    frame.push_str(&format!("\x1b[2mfilter: \"{}\"\x1b[0m\n", query));
                }
            }

            term.move_cursor_to(0, 0)?;
            term.clear_to_end_of_screen()?;
            print!("{}", frame);
            term.flush()?;

            let key = term.read_key()?;

            if filtering {
                match key {
                    Key::Char(c) => {
                        query.push(c);
                        matches = filter(&query);
                        sel = 0;
                    }
                    Key::Backspace => {
                        query.pop();
                        matches = filter(&query);
                        sel = 0;
                    }
                    Key::Enter => filtering = false,
                    Key::Escape => {
                        filtering = false;
                        query.clear();
                        matches = filter(&query);
                        sel = 0;
                    }
                    _ => {}
                }
                continue;
            }

            match key {
                Key::ArrowDown | Key::Char('j') => {
                    if !matches.is_empty() {
                        sel = (sel + 1) % matches.len();
                    }
                }
                Key::ArrowUp | Key::Char('k') => {
                    if !matches.is_empty() {
                        sel = (sel + matches.len() - 1) % matches.len();
                    }
                }
                Key::Char('/') => filtering = true,
                Key::Enter => {
                    if !matches.is_empty() {
                        return Ok(Some(schemes[matches[sel]].clone()));
                    }
                }
                Key::Char('q') | Key::Escape => return Ok(None),
                _ => {}
            }
        }
    })();

    term.show_cursor().ok();
    term.clear_screen().ok();
    outcome
}
