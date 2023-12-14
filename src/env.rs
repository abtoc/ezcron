use regex::Regex;

pub fn set_var(name: &str, value: &str)  {
    // 値に環境変数があれば変更する
    let re = Regex::new(r"(\$[_a-zA-Z][_0-9a-zA-Z]*|\$\{[_a-zA-Z][_0-9a-zA-Z]*\})").unwrap();
    let mut value = value.to_string();
    while let Some(caps) = re.captures(&value) {
        let Some(env_name) = caps.get(1) else { break; };
        let env_name = env_name.as_str();
        let mut env_key = &env_name[1..];
        if &env_key[0..1] == "{" {
            env_key = &env_key[1..env_key.len()-1];
        }
        let env_val = std::env::var(env_key).unwrap_or("".to_string());
        value = value.replace(env_name, &env_val);
    }

    // 環境変数にセット
    std::env::set_var(name, value);
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_env_base() {
        crate::env::set_var("AAA", "BBB");
        assert_eq!(std::env::var("AAA").unwrap(), "BBB");
        crate::env::set_var("AAA", "");
        assert_eq!(std::env::var("AAA").unwrap(), "");
        crate::env::set_var("a1", "value");
        assert_eq!(std::env::var("a1").unwrap(), "value");
        crate::env::set_var("1a", "value");
        assert_eq!(std::env::var("1a").unwrap(), "value");
    }

    #[test]
    fn test_parse_env_value() {
        std::env::set_var("TEST_A", "aaaa");
        std::env::set_var("TEST_B", "bbbb");
        crate::env::set_var("TEST_C", "${TEST_A}:$TEST_B");
        assert_eq!(std::env::var("TEST_C").unwrap(), "aaaa:bbbb");
    }
}