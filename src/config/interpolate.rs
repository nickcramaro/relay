use std::collections::HashMap;

/// Interpolate environment variables in a string.
/// Supports ${env:VAR_NAME} syntax.
pub fn interpolate_env(value: &str) -> String {
    let mut result = value.to_string();
    let re = regex::Regex::new(r"\$\{env:([^}]+)\}").unwrap();

    for cap in re.captures_iter(value) {
        let full_match = cap.get(0).unwrap().as_str();
        let var_name = cap.get(1).unwrap().as_str();
        if let Ok(var_value) = std::env::var(var_name) {
            result = result.replace(full_match, &var_value);
        }
    }

    result
}

/// Interpolate all env values in a HashMap
pub fn interpolate_env_map(env: &HashMap<String, String>) -> HashMap<String, String> {
    env.iter()
        .map(|(k, v)| (k.clone(), interpolate_env(v)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_env() {
        std::env::set_var("TEST_VAR", "test_value");

        assert_eq!(interpolate_env("${env:TEST_VAR}"), "test_value");
        assert_eq!(interpolate_env("prefix_${env:TEST_VAR}_suffix"), "prefix_test_value_suffix");
        assert_eq!(interpolate_env("no_interpolation"), "no_interpolation");
        assert_eq!(interpolate_env("${env:NONEXISTENT}"), "${env:NONEXISTENT}");

        std::env::remove_var("TEST_VAR");
    }
}
