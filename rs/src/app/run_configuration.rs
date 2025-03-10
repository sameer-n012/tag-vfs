use clap::{Arg, ArgAction, Command};
use std::collections::HashMap;
use std::env::Args;
use std::fmt;
use std::fs;

use crate::exceptions::config_parse_error::ConfigParseError;

#[derive(Clone)]
pub struct RunConfiguration {
    config_map: HashMap<String, String>,
    session_id: u64,
    args: Vec<String>,
}

// static project information
const GITHUB_WEB_PATH: &'static str = "https://github.com/sameer-n012/tag-vfs";
const PROJECT_WEB_PATH: &'static str = "";
const APP_NAME: &'static str = "file-vault";
const APP_NAME_PRETTY: &'static str = "File Vault";

// static file locations in JAR
const DEFAULT_CONFIG_FILE_PATH: &'static str = "/.conf.json";
const USER_CONFIG_FILE_PATH: &'static str = "/user.conf.json";

// static final locations not in JAR
const APP_DATA_DIR: &'static str = "filevault";
const CACHE_DIR: &'static str = "tmp_";
const APP_ARCHIVE_FILE: &'static str = "archive.dat";

impl RunConfiguration {
    pub fn new(args: Args) -> Self {
        let mut config = RunConfiguration {
            config_map: HashMap::new(),
            session_id: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            args: Vec::from_iter(args),
        };
        config.setup_config();
        config
    }

    pub fn get_app_name(&self) -> String {
        return self
            .config_map
            .get("appName")
            .unwrap_or(&APP_NAME.to_string())
            .to_string();
    }

    pub fn get_app_name_pretty(&self) -> String {
        return self
            .config_map
            .get("appNamePretty")
            .unwrap_or(&APP_NAME_PRETTY.to_string())
            .to_string();
    }

    pub fn get_session_id(&self) -> u64 {
        return self.session_id;
    }

    fn setup_config(&mut self) {
        self.config_map
            .insert("appName".to_string(), APP_NAME.to_string());
        self.config_map
            .insert("appNamePretty".to_string(), APP_NAME_PRETTY.to_string());
        self.config_map.insert(
            "runPath".to_string(),
            std::env::current_dir()
                .unwrap_or_default()
                .display()
                .to_string(),
        );
    }

    pub fn update_config(&mut self, key: String, value: String, persist: bool) -> String {
        if !RunConfiguration::check_config_constraints(&key) {
            print!("{}", ConfigParseError::new("Invalid config value"));
        }

        let old = self.config_map.insert(key, value).unwrap_or("".to_string());
        if persist {
            // TODO write to file
        }
        return old;
    }

    pub fn parse_command_line_args(&mut self) -> Result<(), String> {
        let matches = Command::new("tvfs")
            .version("1.0")
            .about("A tag-based file system")
            .arg(
                Arg::new("gui")
                    .short('g')
                    .long("gui")
                    .help("uses the GUI")
                    .required(false)
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("help")
                    .short('h')
                    .long("help")
                    .help("prints the usage text")
                    .required(false)
                    .action(ArgAction::Help),
            )
            .try_get_matches_from(self.args.clone())
            .unwrap_or_default();

        if matches.contains_id("gui") {
            self.config_map
                .insert("gui".to_string(), "true".to_string());
        }

        if matches.contains_id("help") {
            std::process::exit(0);
        }

        Ok(())
    }

    pub fn parse_default_config_file(&mut self) -> Result<(), String> {
        let config_data = fs::read_to_string("resources/.conf.json").map_err(|e| e.to_string())?;
        let config_json: serde_json::Value =
            serde_json::from_str(&config_data).map_err(|e| e.to_string())?;
        if let serde_json::Value::Object(map) = config_json {
            for (key, value) in map {
                self.config_map.insert(key, value.to_string());
            }
        }
        return Ok(());
    }

    pub fn parse_user_config_file(&mut self) -> Result<(), String> {
        let config_data =
            fs::read_to_string("resources/user.conf.json").map_err(|e| e.to_string())?;
        let config_json: serde_json::Value =
            serde_json::from_str(&config_data).map_err(|e| e.to_string())?;
        if let serde_json::Value::Object(map) = config_json {
            for (key, value) in map {
                if !RunConfiguration::check_config_constraints(&key) {
                    continue;
                }

                if !self.config_map.contains_key(&key) {
                    continue;
                }

                self.config_map.insert(key, value.to_string());
            }
        }

        return Ok(());
    }

    pub fn get_app_home_path_absolute(&self) -> String {
        format!("{}/filevault", dirs::home_dir().unwrap().display())
    }

    pub fn get_cache_path_absolute(&self) -> String {
        format!(
            "{}/tmp_{}",
            self.get_app_home_path_absolute(),
            self.session_id
        )
    }

    pub fn get_archive_path_absolute(&self) -> String {
        format!("{}/archive.dat", self.get_app_home_path_absolute())
    }

    pub fn contains(&self, key: &str) -> bool {
        return self.config_map.contains_key(key);
    }

    pub fn reload_config(&mut self) {
        self.parse_default_config_file();
        self.parse_user_config_file();
        self.parse_command_line_args();
    }

    pub fn reset_config(&mut self) {
        self.setup_config();
    }

    pub fn get_config_int(&self, key: &str) -> i32 {
        return self
            .config_map
            .get(key)
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or_default();
    }

    pub fn get_config_long(&self, key: &str) -> i64 {
        return self
            .config_map
            .get(key)
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or_default();
    }

    pub fn get_config_double(&self, key: &str) -> f64 {
        return self
            .config_map
            .get(key)
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or_default();
    }

    pub fn get_config_bool(&self, key: &str) -> bool {
        return self
            .config_map
            .get(key)
            .and_then(|value| value.parse::<bool>().ok())
            .unwrap_or_default();
    }

    pub fn get_config_char(&self, key: &str) -> String {
        return self
            .config_map
            .get(key)
            .map(|value| value.chars().next().unwrap_or('\0').to_string())
            .unwrap_or_else(|| '\0'.to_string());
    }

    pub fn get_config_string(&self, key: &str) -> String {
        self.config_map
            .get(key)
            .unwrap_or(&"".to_string())
            .to_string()
    }

    // Run configuration constraints
    const INT_FIELDS: [&'static str; 3] = ["fontSizeLG", "fontSizeMD", "fontSizeSM"];
    const POS_INT_FIELDS: [&'static str; 3] = ["fontSizeLG", "fontSizeMD", "fontSizeSM"];
    const DOUBLE_FIELDS: [&'static str; 0] = [];
    const STRING_FIELDS: [&'static str; 1] = ["cliPrefix"];
    const CHAR_FIELDS: [&'static str; 0] = [];
    const BOOL_FIELDS: [&'static str; 2] = ["gui", "darkMode"];

    fn check_config_constraints(key: &str) -> bool {
        if RunConfiguration::INT_FIELDS.contains(&key) {
            // try to parse as int, if not return falsereturn key
            if !key.parse::<i32>().is_ok() {
                return false;
            }
        } else if RunConfiguration::POS_INT_FIELDS.contains(&key) {
            if !key.parse::<i32>().is_ok_and(|x| x > 0) {
                return false;
            }
        } else if RunConfiguration::DOUBLE_FIELDS.contains(&key) {
            if !key.parse::<f64>().is_ok() {
                return false;
            }
        } else if RunConfiguration::STRING_FIELDS.contains(&key) {
            // Any string is valid
        } else if RunConfiguration::CHAR_FIELDS.contains(&key) {
            if key.chars().count() != 1 {
                return false;
            }
        } else if RunConfiguration::BOOL_FIELDS.contains(&key) {
            if !key.parse::<bool>().is_ok() {
                return false;
            }
        }
        true
    }
}

impl fmt::Display for RunConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO pretty print
        write!(f, "Run Configuration")
    }
}
