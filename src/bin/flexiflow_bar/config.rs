use std::path::PathBuf;

use mlua::LuaSerdeExt as _;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Colors {
    pub fg: String,
    pub sep_fg: String,
    pub sep_bg: String,
    pub accents: Vec<(String, String)>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct BarConfig {
    pub interval_ms: u64,
    pub colors: Colors,
    pub segments: Vec<String>,
}

const DEFAULT_LUA: &str = include_str!("bar.lua");

fn from_lua_source(src: &str) -> Result<BarConfig, Box<dyn std::error::Error>> {
    let lua = mlua::Lua::new();
    let value: mlua::Value = lua.load(src).eval()?;
    Ok(lua.from_value(value)?)
}

fn config_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        Some(PathBuf::from(xdg).join("flexiflow"))
    } else if let Ok(home) = std::env::var("HOME") {
        Some(PathBuf::from(home).join(".config").join("flexiflow"))
    } else {
        None
    }
}

/// `~/.config/flexiflow/flexiflow_bar/`: where the updates-segment
/// caches its 15-minute-TTL `paru -Qu` count between ticks.
pub fn cache_dir() -> Option<PathBuf> {
    config_dir().map(|d| d.join("flexiflow_bar"))
}

/// Load the baked-in default config, then layer
/// `$XDG_CONFIG_HOME/flexiflow/bar.lua` over it if present - same
/// "quiet if simply absent, loud if present-but-broken" behavior as
/// the window manager's own `Config::load_home`.
pub fn load() -> BarConfig {
    let default =
        from_lua_source(DEFAULT_LUA).expect("embedded default bar.lua is valid");

    let Some(path) = config_dir().map(|d| d.join("bar.lua")) else {
        return default;
    };
    if !path.exists() {
        log::debug!("no user bar config at {path:?}, using defaults");
        return default;
    }

    match std::fs::read_to_string(&path)
        .map_err(Into::into)
        .and_then(|src| from_lua_source(&src))
    {
        Ok(cfg) => cfg,
        Err(e) => {
            log::error!("failed to read bar config file {path:?}: {e:?}");
            default
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_default_parses() {
        let cfg = from_lua_source(DEFAULT_LUA).unwrap();
        assert_eq!(cfg.interval_ms, 500);
        assert_eq!(cfg.colors.accents.len(), 7);
        assert_eq!(
            cfg.segments,
            vec![
                "updates", "kernel", "battery", "ram", "cpu", "keyboard",
                "network", "volume", "clock"
            ]
        );
    }

    #[test]
    fn rejects_malformed_lua() {
        assert!(from_lua_source("return { interval_ms = \"not a number\" }").is_err());
    }
}
