use serde_json::Value;
use tabled::{Table, builder::Builder};

/// Format output as a table using the tabled crate.
pub fn print_table(value: &Value) {
    match value {
        Value::Object(map) => {
            // Check if "result" key holds the primary data
            if let Some(result) = map.get("result") {
                print_result_table(result, map);
            } else {
                print_flat_object(value);
            }
        }
        Value::Array(arr) => {
            print_array_table(arr);
        }
        _ => {
            println!("{}", value);
        }
    }
}

fn print_result_table(result: &Value, envelope: &serde_json::Map<String, Value>) {
    // Print the result section
    if let Value::Object(res_map) = result {
        let mut builder = Builder::default();
        builder.push_record(["Field", "Value"]);
        for (key, val) in res_map {
            builder.push_record([key.as_str(), &format_value(val)]);
        }
        let table = Table::from(builder);
        println!("{}", table);
    } else {
        print_flat_object(&Value::Object(envelope.clone()));
    }

    // Print warnings if any
    if let Some(Value::Array(warnings)) = envelope.get("warnings") {
        if !warnings.is_empty() {
            println!("\nWarnings:");
            for w in warnings {
                if let Value::String(s) = w {
                    println!("  - {}", s);
                }
            }
        }
    }

    // Print methodology
    if let Some(Value::String(meth)) = envelope.get("methodology") {
        println!("\nMethodology: {}", meth);
    }
}

fn print_flat_object(value: &Value) {
    if let Value::Object(map) = value {
        let mut builder = Builder::default();
        builder.push_record(["Field", "Value"]);
        for (key, val) in map {
            builder.push_record([key.as_str(), &format_value(val)]);
        }
        let table = Table::from(builder);
        println!("{}", table);
    }
}

fn print_array_table(arr: &[Value]) {
    if arr.is_empty() {
        println!("(empty)");
        return;
    }

    // Collect all keys from first object for headers
    if let Some(Value::Object(first)) = arr.first() {
        let headers: Vec<String> = first.keys().cloned().collect();
        let mut builder = Builder::default();
        builder.push_record(&headers);

        for item in arr {
            if let Value::Object(map) = item {
                let row: Vec<String> = headers
                    .iter()
                    .map(|h| {
                        map.get(h.as_str())
                            .map(|v| format_value(v))
                            .unwrap_or_default()
                    })
                    .collect();
                builder.push_record(row);
            }
        }

        let table = Table::from(builder);
        println!("{}", table);
    } else {
        // Simple array of values
        for item in arr {
            println!("{}", format_value(item));
        }
    }
}

fn format_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(|v| format_value(v)).collect();
            items.join(", ")
        }
        Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}
