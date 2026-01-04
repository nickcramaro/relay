use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;

/// Convert camelCase to kebab-case (e.g., "libraryName" -> "library-name")
fn camel_to_kebab(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert kebab-case to camelCase (e.g., "library-name" -> "libraryName")
fn kebab_to_camel(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for c in s.chars() {
        if c == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Represents a CLI flag derived from a JSON Schema property
#[derive(Debug, Clone)]
pub struct SchemaFlag {
    pub name: String,
    #[allow(dead_code)]
    pub description: String,
    pub required: bool,
    pub flag_type: FlagType,
    pub default: Option<Value>,
}

/// The type of a flag, derived from JSON Schema types
#[derive(Debug, Clone, PartialEq)]
pub enum FlagType {
    String,
    Integer,
    Number,
    Boolean,
    Array,
    Object,
    Enum(Vec<String>),
}

/// Parse a JSON Schema into a list of CLI flags
pub fn parse_schema(schema: &Value) -> Result<Vec<SchemaFlag>> {
    let properties = schema
        .get("properties")
        .and_then(|p| p.as_object())
        .ok_or_else(|| anyhow!("Schema must have properties object"))?;

    let required_fields: Vec<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let mut flags: Vec<SchemaFlag> = properties
        .iter()
        .map(|(name, prop)| {
            let description = prop
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();

            let required = required_fields.contains(&name.as_str());
            let flag_type = parse_type(prop).unwrap_or(FlagType::String);
            let default = prop.get("default").cloned();

            SchemaFlag {
                name: name.clone(),
                description,
                required,
                flag_type,
                default,
            }
        })
        .collect();

    // Sort: required first, then alphabetically
    flags.sort_by(|a, b| match (a.required, b.required) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(flags)
}

/// Parse the type from a JSON Schema property
pub fn parse_type(prop: &Value) -> Result<FlagType> {
    // Check for enum first
    if let Some(enum_values) = prop.get("enum").and_then(|e| e.as_array()) {
        let values: Vec<String> = enum_values
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        return Ok(FlagType::Enum(values));
    }

    let type_str = prop
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("string");

    match type_str {
        "string" => Ok(FlagType::String),
        "integer" => Ok(FlagType::Integer),
        "number" => Ok(FlagType::Number),
        "boolean" => Ok(FlagType::Boolean),
        "array" => Ok(FlagType::Array),
        "object" => Ok(FlagType::Object),
        _ => Ok(FlagType::String),
    }
}

/// Parse command line arguments according to the schema flags
pub fn parse_args(args: &[String], flags: &[SchemaFlag]) -> Result<HashMap<String, Value>> {
    let mut result: HashMap<String, Value> = HashMap::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if !arg.starts_with("--") {
            i += 1;
            continue;
        }

        let flag_name = arg.trim_start_matches("--");

        // Find matching flag (support camelCase, underscore, and hyphen variations)
        let flag = flags.iter().find(|f| {
            f.name == flag_name
                || f.name.replace('_', "-") == flag_name
                || f.name == flag_name.replace('-', "_")
                || camel_to_kebab(&f.name) == flag_name
                || f.name == kebab_to_camel(flag_name)
        });

        if let Some(flag) = flag {
            let value = match &flag.flag_type {
                FlagType::Boolean => {
                    // Boolean flags don't require a value
                    if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                        let next = &args[i + 1];
                        if next == "true" || next == "false" {
                            i += 1;
                            Value::Bool(next == "true")
                        } else {
                            Value::Bool(true)
                        }
                    } else {
                        Value::Bool(true)
                    }
                }
                _ => {
                    if i + 1 >= args.len() {
                        return Err(anyhow!("Flag --{} requires a value", flag_name));
                    }
                    i += 1;
                    parse_value(&args[i], &flag.flag_type)?
                }
            };

            result.insert(flag.name.clone(), value);
        } else {
            return Err(anyhow!("Unknown flag: --{}", flag_name));
        }

        i += 1;
    }

    // Apply defaults for missing optional flags
    for flag in flags {
        if !result.contains_key(&flag.name) {
            if let Some(default) = &flag.default {
                result.insert(flag.name.clone(), default.clone());
            }
        }
    }

    // Validate required flags are present
    for flag in flags {
        if flag.required && !result.contains_key(&flag.name) {
            return Err(anyhow!("Required flag --{} is missing", flag.name));
        }
    }

    Ok(result)
}

/// Parse a string value into a typed JSON Value
pub fn parse_value(s: &str, flag_type: &FlagType) -> Result<Value> {
    match flag_type {
        FlagType::String => Ok(Value::String(s.to_string())),
        FlagType::Integer => {
            let n: i64 = s.parse().map_err(|_| anyhow!("Invalid integer: {}", s))?;
            Ok(Value::Number(n.into()))
        }
        FlagType::Number => {
            let n: f64 = s.parse().map_err(|_| anyhow!("Invalid number: {}", s))?;
            Ok(serde_json::Number::from_f64(n)
                .map(Value::Number)
                .unwrap_or(Value::Null))
        }
        FlagType::Boolean => {
            let b = match s.to_lowercase().as_str() {
                "true" | "1" | "yes" => true,
                "false" | "0" | "no" => false,
                _ => return Err(anyhow!("Invalid boolean: {}", s)),
            };
            Ok(Value::Bool(b))
        }
        FlagType::Array => {
            // Try to parse as JSON array, otherwise split by comma
            if let Ok(arr) = serde_json::from_str::<Value>(s) {
                if arr.is_array() {
                    return Ok(arr);
                }
            }
            let items: Vec<Value> = s
                .split(',')
                .map(|item| Value::String(item.trim().to_string()))
                .collect();
            Ok(Value::Array(items))
        }
        FlagType::Object => {
            serde_json::from_str(s).map_err(|e| anyhow!("Invalid JSON object: {}", e))
        }
        FlagType::Enum(values) => {
            if values.contains(&s.to_string()) {
                Ok(Value::String(s.to_string()))
            } else {
                Err(anyhow!(
                    "Invalid enum value '{}'. Must be one of: {}",
                    s,
                    values.join(", ")
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_simple_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The name"
                },
                "count": {
                    "type": "integer",
                    "description": "The count"
                },
                "enabled": {
                    "type": "boolean",
                    "default": false
                }
            },
            "required": ["name"]
        });

        let flags = parse_schema(&schema).unwrap();
        assert_eq!(flags.len(), 3);

        // Required flags come first
        assert_eq!(flags[0].name, "name");
        assert!(flags[0].required);

        // Then alphabetically
        assert_eq!(flags[1].name, "count");
        assert!(!flags[1].required);

        assert_eq!(flags[2].name, "enabled");
        assert!(!flags[2].required);
        assert_eq!(flags[2].default, Some(Value::Bool(false)));
    }

    #[test]
    fn test_parse_args() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string"
                },
                "count": {
                    "type": "integer"
                },
                "verbose": {
                    "type": "boolean"
                }
            },
            "required": ["name"]
        });

        let flags = parse_schema(&schema).unwrap();
        let args: Vec<String> = vec![
            "--name".to_string(),
            "test".to_string(),
            "--count".to_string(),
            "42".to_string(),
            "--verbose".to_string(),
        ];

        let result = parse_args(&args, &flags).unwrap();
        assert_eq!(result.get("name"), Some(&Value::String("test".to_string())));
        assert_eq!(result.get("count"), Some(&json!(42)));
        assert_eq!(result.get("verbose"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_parse_enum() {
        let schema = json!({
            "type": "object",
            "properties": {
                "level": {
                    "enum": ["low", "medium", "high"],
                    "description": "Priority level"
                }
            },
            "required": ["level"]
        });

        let flags = parse_schema(&schema).unwrap();
        assert_eq!(
            flags[0].flag_type,
            FlagType::Enum(vec![
                "low".to_string(),
                "medium".to_string(),
                "high".to_string(),
            ])
        );

        // Valid enum value
        let args = vec!["--level".to_string(), "high".to_string()];
        let result = parse_args(&args, &flags).unwrap();
        assert_eq!(
            result.get("level"),
            Some(&Value::String("high".to_string()))
        );

        // Invalid enum value
        let args = vec!["--level".to_string(), "invalid".to_string()];
        let result = parse_args(&args, &flags);
        assert!(result.is_err());
    }
}
