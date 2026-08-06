use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub(crate) struct Config {
    #[serde(default)]
    pub(crate) waka: WakaConfig,
}

#[derive(Debug, Deserialize)]
#[derive(Default)]
pub(crate) struct WakaConfig {
    #[serde(default)]
    pub assume_yes: bool,
}


const CONFIG_PATHS: &[&str] = &[
    "/etc/waka/waka.conf",
    "/usr/etc/waka/waka.conf",
];

impl Config {
    pub fn load() -> Result<Self, anyhow::Error> {
        let user_paths = [
            std::env::var("XDG_CONFIG_HOME").map(|p| format!("{p}/waka/waka.conf")).ok(),
            std::env::var("HOME").map(|p| format!("{p}/.config/waka/waka.conf")).ok(),
            std::env::var("HOME").map(|p| format!("{p}/.waka.conf")).ok(),
        ];

        for opt in user_paths.iter().flatten().cloned().chain(CONFIG_PATHS.iter().map(|s| s.to_string())) {
            let path = std::path::Path::new(&opt);
            match std::fs::read_to_string(path) {
                Ok(contents) => {
                    match toml::from_str::<Config>(&contents) {
                        Ok(config) => return Ok(config),
                        Err(e) => {
                            eprintln!("  {} failed to parse config '{}': {} — skipping", crate::display::style::yellow("warn"), path.display(), e);
                            continue;
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => {
                    eprintln!("  {} failed to read config '{}': {} — skipping", crate::display::style::yellow("warn"), path.display(), e);
                    continue;
                }
            }
        }
        Ok(Config::default())
    }
}
