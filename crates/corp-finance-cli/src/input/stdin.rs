use serde_json::Value;
use std::io::{self, Read};

/// Attempt to read JSON from stdin if data is being piped.
/// Returns None if stdin is a TTY (interactive).
pub fn read_stdin() -> Result<Option<Value>, Box<dyn std::error::Error>> {
    if atty::is(atty::Stream::Stdin) {
        return Ok(None);
    }

    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    let trimmed = buffer.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let value: Value = serde_json::from_str(trimmed)?;
    Ok(Some(value))
}
