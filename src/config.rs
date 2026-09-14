use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use serde::Deserialize;

// use crate::curve::{Curve, CurvePoint};

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub default: Device,
    pub transition: Transition,
    // pub iio: Iio,
}

static XDG_CONFIG_HOME: LazyLock<Cow<'static, Path>> = LazyLock::new(|| {
    if let Some(xdg_config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        Cow::Owned(PathBuf::from(xdg_config_home))
    } else if let Some(home) = std::env::home_dir() {
        Cow::Owned(home.join(".config"))
    } else {
        Cow::Borrowed(Path::new("~/.config"))
    }
});

pub static DEFAULT_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| XDG_CONFIG_HOME.join("lilight/lilight.toml"));

impl Config {
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read(path) {
            Ok(x) => match toml::from_slice(&x) {
                Ok(x) => Ok(x),
                Err(e) => Err(format!("Failed to parse config file at `{path:?}`: {e}")),
            },
            Err(e) => Err(format!("Failed to read config file at `{path:?}`: {e}")),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default: Device::default(),
            transition: Transition::default(),
            // iio: Iio::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Device {
    pub subsystem: String,
    pub name: String,
}

impl Default for Device {
    fn default() -> Self {
        Self {
            subsystem: "backlight".into(),
            name: "amdgpu_bl1".into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Transition {
    pub enable: bool,
    pub time: u64,
    pub step: u64,
}

impl Default for Transition {
    fn default() -> Self {
        Self {
            enable: true,
            time: 100,
            step: 17,
        }
    }
}

/*
#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Iio {
    pub default_sensor: Option<PathBuf>,
    // TODO: maybe support for different unit (e.g. value, percentage ...)
    pub curve: Curve,
}

impl Default for Iio {
    fn default() -> Self {
        Self {
            default_sensor: None,
            curve: Curve::new(vec![
                CurvePoint { x: 0, y: 0 },
                CurvePoint { x: 100, y: 100 },
            ]),
        }
    }
}
*/
