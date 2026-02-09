pub mod csv_out;
pub mod json;
pub mod minimal;
pub mod table;

use crate::OutputFormat;
use serde_json::Value;

/// Dispatch output to the appropriate formatter.
pub fn format_output(format: &OutputFormat, value: &Value) {
    match format {
        OutputFormat::Json => json::print_json(value),
        OutputFormat::Table => table::print_table(value),
        OutputFormat::Csv => csv_out::print_csv(value),
        OutputFormat::Minimal => minimal::print_minimal(value),
    }
}
