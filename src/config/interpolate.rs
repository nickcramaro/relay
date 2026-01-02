use std::collections::HashMap;
use std::sync::LazyLock;

static ENV_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\$\{env:([^}]+)\}").unwrap()
});

/// Interpolate environment variables in a string.
/// Supports ${env:VAR_NAME} syntax.
pub fn interpolate_env(value: &str) -> String {
    let mut result = value.to_string();

    for cap in ENV_REGEX.captures_iter(value) {
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

    #[test]
    fn test_interpolate_env_map() {
        std::env::set_var("MAP_TEST_VAR", "map_value");
        std::env::set_var("MAP_TEST_VAR2", "another_value");

        let mut input = HashMap::new();
        input.insert("key1".to_string(), "${env:MAP_TEST_VAR}".to_string());
        input.insert("key2".to_string(), "prefix_${env:MAP_TEST_VAR2}_suffix".to_string());
        input.insert("key3".to_string(), "no_interpolation".to_string());
        input.insert("key4".to_string(), "${env:NONEXISTENT_MAP_VAR}".to_string());

        let result = interpolate_env_map(&input);

        assert_eq!(result.get("key1").unwrap(), "map_value");
        assert_eq!(result.get("key2").unwrap(), "prefix_another_value_suffix");
        assert_eq!(result.get("key3").unwrap(), "no_interpolation");
        assert_eq!(result.get("key4").unwrap(), "${env:NONEXISTENT_MAP_VAR}");

        std::env::remove_var("MAP_TEST_VAR");
        std::env::remove_var("MAP_TEST_VAR2");
    }
}
