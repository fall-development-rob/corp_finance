use serde_json::Value;
use std::io;

/// Write output as CSV to stdout.
pub fn print_csv(value: &Value) {
    let stdout = io::stdout();
    let mut wtr = csv::Writer::from_writer(stdout.lock());

    match value {
        Value::Object(map) => {
            if let Some(Value::Object(result)) = map.get("result") {
                // Two-column CSV: field, value
                let _ = wtr.write_record(["field", "value"]);
                for (key, val) in result {
                    let _ = wtr.write_record([key.as_str(), &format_csv_value(val)]);
                }
            } else if let Some(Value::Array(results)) = map.get("results") {
                // Sensitivity output or array of results
                write_array_csv(&mut wtr, results);
            } else {
                let _ = wtr.write_record(["field", "value"]);
                for (key, val) in map {
                    let _ = wtr.write_record([key.as_str(), &format_csv_value(val)]);
                }
            }
        }
        Value::Array(arr) => {
            write_array_csv(&mut wtr, arr);
        }
        _ => {
            let _ = wtr.write_record([&format_csv_value(value)]);
        }
    }

    let _ = wtr.flush();
}

fn write_array_csv(wtr: &mut csv::Writer<io::StdoutLock<'_>>, arr: &[Value]) {
    if arr.is_empty() {
        return;
    }

    // Extract headers from first object
    if let Some(Value::Object(first)) = arr.first() {
        let headers: Vec<&str> = first.keys().map(|k| k.as_str()).collect();
        let _ = wtr.write_record(&headers);

        for item in arr {
            if let Value::Object(map) = item {
                let row: Vec<String> = headers
                    .iter()
                    .map(|h| {
                        map.get(*h)
                            .map(|v| format_csv_value(v))
                            .unwrap_or_default()
                    })
                    .collect();
                let _ = wtr.write_record(&row);
            }
        }
    } else {
        for item in arr {
            let _ = wtr.write_record([&format_csv_value(item)]);
        }
    }
}

fn format_csv_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}
