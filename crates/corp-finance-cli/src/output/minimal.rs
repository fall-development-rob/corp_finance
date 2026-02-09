use serde_json::Value;

/// Print just the key answer value from the output.
///
/// Heuristic: look for well-known result fields in order of priority,
/// then fall back to the first field in the result object.
pub fn print_minimal(value: &Value) {
    // Try to extract the "result" envelope
    let result_obj = value
        .as_object()
        .and_then(|m| m.get("result"))
        .unwrap_or(value);

    // Priority list of key output fields
    let priority_keys = [
        "wacc",
        "implied_rating",
        "irr",
        "moic",
        "sharpe_ratio",
        "var_pct",
        "full_kelly",
        "net_debt_to_ebitda",
        "enterprise_value",
        "equity_value",
    ];

    if let Value::Object(map) = result_obj {
        // Try priority keys first (skip null values)
        for key in &priority_keys {
            if let Some(val) = map.get(*key) {
                if !val.is_null() {
                    println!("{}", format_minimal(val));
                    return;
                }
            }
        }

        // Fall back to first field
        if let Some((key, val)) = map.iter().next() {
            println!("{}: {}", key, format_minimal(val));
            return;
        }
    }

    // Not an object, just print directly
    println!("{}", format_minimal(result_obj));
}

fn format_minimal(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}
