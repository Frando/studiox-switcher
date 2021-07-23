use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    pub inputs: Vec<Port>,
    pub fallback_input: Port,
    pub output: Option<Port>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Port {
    pub label: Option<String>,
    pub ports: Option<[String; 2]>,
}

impl Config {
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        match std::fs::read(path.as_ref()) {
            Ok(buf) => Ok(toml::from_slice(&buf[..])?),
            Err(err) => Err(err.into()),
        }
    }

    pub fn load_from_default_dirs() -> anyhow::Result<Self> {
        let name = "studiox-switcher.toml";
        let paths = vec![std::env::current_dir()?];
        for path in paths {
            let path = path.join(name);
            if let Ok(res) = Self::load_from_path(path) {
                return Ok(res);
            }
            // eprintln!("path {:?}", path);
            // let res = std::fs::read(path);
            // if let Ok(buf) = res {
            //     let config: Config = toml::from_slice(&buf[..])?;
            //     return Ok(config);
            // }
        }
        Ok(Config::default())
    }
}
