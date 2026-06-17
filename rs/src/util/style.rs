use std::io::IsTerminal;
use std::sync::OnceLock;

static COLOR_ENABLED: OnceLock<bool> = OnceLock::new();

/**
 * Initializes the color-enabled flag based on whether stdout is a TTY.
 * Call once at startup; subsequent calls are no-ops.
 */
pub fn init() {
    COLOR_ENABLED.get_or_init(|| std::io::stdout().is_terminal());
}

fn color_enabled() -> bool {
    *COLOR_ENABLED.get_or_init(|| std::io::stdout().is_terminal())
}

pub fn bold(s: &str) -> String {
    if color_enabled() { format!("\x1b[1m{}\x1b[0m", s) } else { s.to_string() }
}

pub fn dim(s: &str) -> String {
    if color_enabled() { format!("\x1b[2m{}\x1b[0m", s) } else { s.to_string() }
}

pub fn bold_red(s: &str) -> String {
    if color_enabled() { format!("\x1b[1;31m{}\x1b[0m", s) } else { s.to_string() }
}

pub fn green(s: &str) -> String {
    if color_enabled() { format!("\x1b[32m{}\x1b[0m", s) } else { s.to_string() }
}

pub fn bold_cyan(s: &str) -> String {
    if color_enabled() { format!("\x1b[1;36m{}\x1b[0m", s) } else { s.to_string() }
}

pub fn yellow(s: &str) -> String {
    if color_enabled() { format!("\x1b[33m{}\x1b[0m", s) } else { s.to_string() }
}

/**
 * Returns the REPL prompt symbol, colored when stdout is a TTY.
 */
pub fn prompt() -> String {
    if color_enabled() { "\x1b[1;36m›\x1b[0m".to_string() } else { ">".to_string() }
}
