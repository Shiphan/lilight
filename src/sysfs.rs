use std::{borrow::Cow, path::Path};

pub struct Device {
    path: Cow<'static, Path>,
}

impl Device {
    pub fn new(subsystem: &Path, name: &Path) -> Self {
        Self::new_with_path(Cow::Owned(
            [Path::new("/sys/class"), subsystem, name]
                .into_iter()
                .collect(),
        ))
    }
    pub fn new_with_path(path: impl Into<Cow<'static, Path>>) -> Self {
        Self { path: path.into() }
    }
    pub fn get_brightness(&self) -> Result<u32, Error> {
        Ok(std::fs::read_to_string(self.path.join("brightness"))?
            .trim()
            .parse()?)
    }
    pub fn set_brightness(&self, brightness: u32) -> Result<(), std::io::Error> {
        std::fs::write(self.path.join("brightness"), brightness.to_string())
    }
    pub fn get_actual_brightness(&self) -> Result<u32, Error> {
        Ok(
            std::fs::read_to_string(self.path.join("actual_brightness"))?
                .trim()
                .parse()?,
        )
    }
    pub fn get_max_brightness(&self) -> Result<u32, Error> {
        Ok(std::fs::read_to_string(self.path.join("max_brightness"))?
            .trim()
            .parse()?)
    }
}

pub struct Iio {
    path: Cow<'static, Path>,
    input_available: bool,
}

impl Iio {
    pub fn new(device: &Path) -> Self {
        Self::new_with_path(Path::new("/sys/bus/iio/devices").join(device))
    }
    pub fn new_with_path(path: impl Into<Cow<'static, Path>>) -> Self {
        let path = path.into();
        let input_available = std::fs::File::open(&path.join("in_illuminance_input")).is_ok();
        Self {
            path,
            input_available,
        }
    }
    pub fn refresh(&mut self) {
        self.input_available = std::fs::File::open(&self.path.join("in_illuminance_input")).is_ok();
    }
    pub fn get_illuminance(&self) -> Result<u32, Error> {
        if self.input_available {
            Ok(
                std::fs::read_to_string(self.path.join("in_illuminance_input"))?
                    .trim()
                    .parse()?,
            )
        } else {
            // TODO: check type
            let read_optional = |path| match std::fs::read_to_string(path) {
                Ok(s) => Ok(Some(s)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e),
            };
            let offset = read_optional(self.path.join("in_illuminance_offset"))?
                .map(|x| x.trim().parse())
                .transpose()?
                .unwrap_or(0);
            let scale = read_optional(self.path.join("in_illuminance_scale"))?
                .map(|x| x.trim().parse())
                .transpose()?
                .unwrap_or(1);
            let raw =
                std::fs::read_to_string(self.path.join("in_illuminance_raw"))?.parse::<u32>()?;
            Ok((raw + offset) * scale)
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    PraseInt(std::num::ParseIntError),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::PraseInt(value)
    }
}
