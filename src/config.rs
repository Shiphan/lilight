use std::path::PathBuf;

use serde::Deserialize;

use crate::curve::{Curve, CurvePoint};

#[derive(Deserialize)]
#[serde(default)]
pub struct Config {
    pub default_device: Option<PathBuf>,
    pub transition: Transition,
    pub iio: Iio,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_device: None,
            transition: Transition::default(),
            iio: Iio::default(),
        }
    }
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
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
