use std::path::PathBuf;

const DOMAINS_CONFIG_PATH: &str = ".reasons/domains.toml";

#[derive(Debug, serde::Deserialize)]
struct DomainsConfig {
    default: Option<String>,
    #[serde(default)]
    domain: Vec<DomainEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct DomainEntry {
    name: String,
    path: String,
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(rest)
    } else {
        PathBuf::from(path)
    }
}

pub fn domains_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(DOMAINS_CONFIG_PATH)
}

pub fn default_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("reasons.db")
}

pub fn load_domains() -> (Vec<(String, PathBuf)>, String) {
    let config_path = domains_config_path();
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(config) = toml::from_str::<DomainsConfig>(&content) {
            if !config.domain.is_empty() {
                let domains: Vec<(String, PathBuf)> = config.domain.iter()
                    .map(|d| (d.name.clone(), expand_tilde(&d.path)))
                    .collect();
                let default = config.default
                    .unwrap_or_else(|| domains[0].0.clone());
                return (domains, default);
            }
        }
    }
    let db_path = default_db_path();
    (vec![("default".to_string(), db_path)], "default".to_string())
}

pub fn ensure_default_config() {
    let config_path = domains_config_path();
    if config_path.exists() {
        return;
    }
    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let default_content = format!(
        r#"# Reasons domains configuration
# Each [[domain]] entry registers a reasons database.
# The "default" key sets which domain is used when no domain is specified.

default = "default"

[[domain]]
name = "default"
path = "{}"
"#,
        default_db_path().display()
    );
    let _ = std::fs::write(&config_path, default_content);
}
