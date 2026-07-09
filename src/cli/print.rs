use crate::core::presentation::{EnvItem, HistoryItem};
use std::io::Write;

// ─── Color Basics ────────────────────────────────────────────────────
fn use_color() -> bool {
    std::env::var("NO_COLOR").is_err() && std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true)
}

pub fn green(s: &str) -> String {
    if use_color() {
        format!("\x1b[32m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}
pub fn red(s: &str) -> String {
    if use_color() {
        format!("\x1b[31m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}
pub fn yellow(s: &str) -> String {
    if use_color() {
        format!("\x1b[33m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}
pub fn cyan(s: &str) -> String {
    if use_color() {
        format!("\x1b[36m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}
pub fn bold(s: &str) -> String {
    if use_color() {
        format!("\x1b[1m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}
pub fn dim(s: &str) -> String {
    if use_color() {
        format!("\x1b[2m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

// ─── Main Output Functions (stdout) ──────────────────────────────────────
/// ✓ Success
pub fn success(msg: &str) {
    println!("{} {}", green("✓"), msg);
}

/// ✗ Failure
pub fn failure(title: &str, reason: Option<&str>) {
    eprintln!("{} {}", red("✗"), bold(title));
    if let Some(r) = reason {
        eprintln!("  {}", dim(r));
    }
}

/// → Action start
pub fn action(msg: &str) {
    println!("{} {}", cyan("→"), bold(msg));
}

/// · Step detail
pub fn step(key: &str, val: &str) {
    println!("  {} {:<10} {}", dim("·"), dim(key), val);
}

/// Key-Value detail
pub fn detail(key: &str, val: &str) {
    println!("  {:<8} {}", dim(key), val);
}

/// ⚠ Warning (stderr)
pub fn warn(msg: &str) {
    let _ = writeln!(std::io::stderr(), "{} {}", yellow("⚠"), dim(msg));
}

// ─── List Formatting ────────────────────────────────────────────────────
pub fn format_envs(items: &[EnvItem]) -> String {
    let mut out = String::new();
    out.push_str(&format!("\n{} {}\n", dim("╭─"), bold("Environments")));
    if items.is_empty() {
        out.push_str(&format!(
            "{} {}\n",
            dim("│ "),
            dim("  (no environments found)")
        ));
    } else {
        let max_name = items.iter().map(|i| i.name.len()).max().unwrap_or(0);
        for item in items {
            let name_col = format!("{:<width$}", item.name, width = max_name + 2);
            let name_str = if item.is_current {
                bold(&name_col)
            } else {
                name_col.clone()
            };

            let mut tags = Vec::new();
            if item.is_current {
                tags.push(cyan("◉ current"));
            }
            if item.is_default {
                tags.push(yellow("★ default"));
            }
            if item.missing_key {
                tags.push(yellow("⚠ no key"));
            }
            let tag_str = if tags.is_empty() {
                String::new()
            } else {
                format!("  {}", tags.join("  "))
            };

            let extra_str = item.extra.as_deref().unwrap_or("");
            let extra_display = if extra_str.is_empty() {
                String::new()
            } else {
                format!("  {}", dim(extra_str))
            };

            out.push_str(&format!(
                "{}   {}{}{}{}\n",
                dim("│"),
                name_str,
                dim(&item.description),
                extra_display,
                tag_str
            ));
        }
    }
    out.push_str(&format!(
        "{}\n",
        dim("╰────────────────────────────────────────")
    ));
    out
}

// ─── History Formatting ────────────────────────────────────────────────────
pub fn format_history(items: &[HistoryItem]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "\n{} {}\n",
        dim("╭─"),
        bold("Recent switch history")
    ));
    if items.is_empty() {
        out.push_str(&format!("{} {}\n", dim("│ "), dim("  (no history found)")));
    } else {
        for item in items {
            let from_str = item.from.as_deref().unwrap_or("None");
            out.push_str(&format!(
                "{}  {}  {:<8}  {} {} {}\n",
                dim("│"),
                dim(&item.timestamp),
                cyan(&item.env_type),
                from_str,
                dim("→"),
                bold(&item.to)
            ));
        }
    }
    out.push_str(&format!(
        "{}\n",
        dim("╰────────────────────────────────────────")
    ));
    out
}
