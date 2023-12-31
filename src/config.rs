use std::collections::HashMap;
use std::fs;
use registorder_map::RegistOrderMap;
use serde::{Deserialize, Serialize};

#[cfg(debug_assertions)]
const CONFIG_FILE: &str = "./etc/ezcron.toml";
#[cfg(not(debug_assertions))]
const CONFIG_FILE: &str = "/etc/ezcron/ezcron.toml";


#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigEzCron {
    pub log_dir: String,
    pub pid_dir: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigOption {
    #[serde(default)]
    pub reports: Vec<String>,
    #[serde(default)]
    pub notifies: Vec<String>,
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: RegistOrderMap<String, String>,
}

impl ConfigOption {
    pub fn new() -> Self {
        Self {
            reports: Vec::<String>::new(),
            notifies: Vec::<String>::new(),
            cwd: None,
            env: RegistOrderMap::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub ezcron: ConfigEzCron,
    pub option: Option<ConfigOption>,
    #[serde(default)]
    pub options: HashMap<String, ConfigOption>,
}

pub fn load(conf: Option<String>) -> Result<Config, Box<dyn std::error::Error>> {
    let conf = match conf {
        Some(conf) => conf,
        None => CONFIG_FILE.to_string(),
    };

    let toml_str = fs::read_to_string(conf)?;
    let config: Config = toml::from_str(&toml_str)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use crate::config;

    struct TestConfigFile {
        path: Box<PathBuf>,
    }

    impl TestConfigFile {
        fn new(path: &str, config_str: &str) -> Self {
            let path = Path::new(path);
            let mut fs = File::create(path).unwrap();
            fs.write(config_str.as_bytes()).unwrap();
            Self {
                path: Box::new(path.to_path_buf()),
            }
        }
    }

    impl Drop for TestConfigFile {
        fn drop(&mut self) {
            if self.path.is_file() {
                std::fs::remove_file(self.path.as_path()).unwrap();
            }
        }
    }

    #[test]
    fn test_config_basic() {
        const CONFIG_FILE: &str = "test_config_basic.toml";
        let _test_confg_file = TestConfigFile::new(CONFIG_FILE, r#"[ezcron]
log_dir="var/log/ezcron"
pid_dir="run/ezcron"
"#);
        let config = config::load(Some(CONFIG_FILE.to_string())).unwrap();
        assert_eq!(config.ezcron.log_dir, "var/log/ezcron".to_string());
        assert_eq!(config.ezcron.pid_dir, "run/ezcron".to_string());
        assert_eq!(config.option.is_none(), true);
    }

    #[test]
    fn test_config_option() {
        const CONFIG_FILE: &str = "test_config_option.toml";
        let _test_confg_file = TestConfigFile::new(CONFIG_FILE, r#"[ezcron]
log_dir="var/log/ezcron"
pid_dir="run/ezcron"
[option]
reports=["report.sh"]
notifies=["notify.sh"]
cwd="/path/to"
[option.env]
TEST1="VALUE1"
TEST2="VALUE2"
"#);
        let config = config::load(Some(CONFIG_FILE.to_string())).unwrap();
        assert_eq!(config.ezcron.log_dir, "var/log/ezcron".to_string());
        assert_eq!(config.ezcron.pid_dir, "run/ezcron".to_string());
        assert_eq!(config.option.is_some(), true);
        let option = config.option.unwrap();
        assert_eq!(option.reports, vec!["report.sh"]);
        assert_eq!(option.notifies, vec!["notify.sh"]);
        assert_eq!(option.cwd, Some("/path/to".to_string()));
        assert_eq!(option.env.get(&"TEST1".to_string()), Some("VALUE1".to_string()).as_ref());
        assert_eq!(option.env.get(&"TEST2".to_string()), Some("VALUE2".to_string()).as_ref());
    }

    #[test]
    fn test_config_options() {
        const CONFIG_FILE: &str = "test_config_options.toml";
        let _test_confg_file = TestConfigFile::new(CONFIG_FILE, r#"[ezcron]
log_dir="var/log/ezcron"
pid_dir="run/ezcron"
[options.key1]
reports=["report.sh"]
notifies=["notify.sh"]
cwd="/path/to"
[options.key1.env]
TEST1="VALUE1"
TEST2="VALUE2"
"#);
        let config = config::load(Some(CONFIG_FILE.to_string())).unwrap();
        assert_eq!(config.ezcron.log_dir, "var/log/ezcron".to_string());
        assert_eq!(config.ezcron.pid_dir, "run/ezcron".to_string());
        assert_eq!(config.option.is_none(), true);
        assert_eq!(config.options.get("key1").is_some(), true);
        let option = config.options.get("key1").unwrap();
        assert_eq!(option.reports, vec!["report.sh"]);
        assert_eq!(option.notifies, vec!["notify.sh"]);
        assert_eq!(option.cwd, Some("/path/to".to_string()));
        assert_eq!(option.env.get(&"TEST1".to_string()), Some("VALUE1".to_string()).as_ref());
        assert_eq!(option.env.get(&"TEST2".to_string()), Some("VALUE2".to_string()).as_ref());
    }
}