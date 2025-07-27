use std::{
    borrow::Cow,
    env,
    fs::{self, File},
    io::{self, Read, Seek, Write},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use clap::Parser;

use crate::{
    cli::{Cli, Command, Prefix, Type, Value},
    config::Config,
    curve::Curve,
};

mod cli;
mod config;
mod curve;

#[derive(Debug)]
struct Device {
    name: PathBuf,
    brightness: File,
    max_brightness: File, // TODO: maybe just store the value?
}

impl Device {
    fn new<P>(dir_path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            name: dir_path
                .as_ref()
                .strip_prefix("/sys/class/")
                .unwrap_or(dir_path.as_ref())
                .to_path_buf(),
            brightness: File::options()
                .read(true)
                .write(true)
                .open(dir_path.as_ref().join("brightness"))?,
            max_brightness: File::open(dir_path.as_ref().join("max_brightness"))?,
        })
    }
    fn all() -> (Vec<Self>, Vec<io::Error>) {
        [
            fs::read_dir("/sys/class/backlight"),
            fs::read_dir("/sys/class/leds"),
        ]
        .into_iter()
        .fold((vec![], vec![]), |(mut devices, mut errors), dir| {
            match dir {
                Ok(dir) => {
                    for device in dir {
                        match device.and_then(|x| Self::new(x.path())) {
                            Ok(x) => devices.push(x),
                            Err(e) => errors.push(e),
                        }
                    }
                }
                Err(e) => errors.push(e),
            };
            (devices, errors)
        })
    }
    fn first() -> (Option<Self>, Vec<io::Error>) {
        let mut errors = vec![];
        for dir in [
            fs::read_dir("/sys/class/backlight"),
            fs::read_dir("/sys/class/leds"),
        ] {
            match dir {
                Ok(dir) => {
                    for device in dir {
                        match device.and_then(|x| Self::new(x.path())) {
                            Ok(x) => return (Some(x), errors),
                            Err(e) => errors.push(e),
                        }
                    }
                }
                Err(e) => errors.push(e),
            }
        }
        (None, errors)
    }
    fn get_brightness(&mut self) -> io::Result<i32> {
        self.brightness.rewind()?;
        let mut buf = String::new();
        let _ = self.brightness.read_to_string(&mut buf)?;
        buf.trim().parse().map_err(|e| io::Error::other(e))
    }
    fn set_brightness(&mut self, new_brightness: i32) -> io::Result<()> {
        self.brightness
            .write_all(new_brightness.max(0).to_string().as_bytes())
    }
    fn get_max_brightness(&mut self) -> io::Result<i32> {
        self.max_brightness.rewind()?;
        let mut buf = String::new();
        let _ = self.max_brightness.read_to_string(&mut buf)?;
        buf.trim().parse().map_err(|e| io::Error::other(e))
    }
}

#[derive(Debug)]
struct Iio {
    name: PathBuf,
    in_illuminance_raw: File,
    in_illuminance_scale: f32,  // TODO: check type
    in_illuminance_offset: i32, // TODO: check type
}

impl Iio {
    fn new<P>(dir_path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            name: dir_path
                .as_ref()
                .strip_prefix("/sys/bus/iio/devices/")
                .unwrap_or(dir_path.as_ref())
                .to_path_buf(),
            in_illuminance_raw: File::open(dir_path.as_ref().join("in_illuminance_raw"))?,
            in_illuminance_scale: fs::read_to_string(
                dir_path.as_ref().join("in_illuminance_scale"),
            )?
            .trim()
            .parse()
            .map_err(|e| io::Error::other(e))?,
            in_illuminance_offset: fs::read_to_string(
                dir_path.as_ref().join("in_illuminance_offset"),
            )?
            .trim()
            .parse()
            .map_err(|e| io::Error::other(e))?,
        })
    }
    fn all() -> (Vec<Self>, Vec<io::Error>) {
        match fs::read_dir("/sys/bus/iio/devices/") {
            Ok(dir) => {
                let mut iios = vec![];
                let mut errors = vec![];
                for iio in dir {
                    match iio.and_then(|x| Self::new(x.path())) {
                        Ok(x) => iios.push(x),
                        Err(e) => errors.push(e),
                    }
                }
                (iios, errors)
            }
            Err(e) => (vec![], vec![e]),
        }
    }
    fn first() -> (Option<Self>, Vec<io::Error>) {
        match fs::read_dir("/sys/bus/iio/devices/") {
            Ok(dir) => {
                let mut errors = vec![];
                for iio in dir {
                    match iio.and_then(|x| Self::new(x.path())) {
                        Ok(x) => return (Some(x), errors),
                        Err(e) => errors.push(e),
                    }
                }
                (None, errors)
            }
            Err(e) => (None, vec![e]),
        }
    }
    // TODO: check type
    fn get_illuminance(&mut self) -> io::Result<i32> {
        self.in_illuminance_raw.rewind()?;
        let mut buf = String::new();
        let _ = self.in_illuminance_raw.read_to_string(&mut buf)?;
        match buf.trim().parse::<i32>() {
            Ok(raw) => Ok(if self.in_illuminance_scale.fract() == 0.0 {
                (raw + self.in_illuminance_offset) * self.in_illuminance_scale as i32
            } else {
                ((raw + self.in_illuminance_offset) as f32 * self.in_illuminance_scale) as i32
            }),
            Err(e) => Err(io::Error::other(e)),
        }
    }
}

#[derive(Debug)]
enum Setting {
    Set {
        value: Value,
        device: Option<PathBuf>,
        transition_enable: bool,
        transition_time: u64,
        transition_step: u64,
    },
    Get {
        max: bool,
        device: Option<PathBuf>,
        all: bool,
    },
    List,
    Daemon {
        device: Option<PathBuf>,
        transition_enable: bool,
        transition_time: u64,
        transition_step: u64,
        iio_sensor: Option<PathBuf>,
        curve: Curve,
    },
}

impl Setting {
    fn new(cli: Cli, config: Config) -> Self {
        match cli.command {
            Command::Set {
                value,
                device,
                transition_time,
                transition_step,
            } => Self::Set {
                value,
                device: device.or(config.default_device),
                transition_enable: config.transition.enable,
                transition_time: transition_time.unwrap_or(config.transition.time),
                transition_step: transition_step.unwrap_or(config.transition.step),
            },
            Command::Get { max, device, all } => Self::Get {
                max,
                device: device.or(config.default_device),
                all,
            },
            Command::List => Self::List,
            Command::Daemon {
                device,
                transition_time,
                transition_step,
                iio,
            } => Self::Daemon {
                device: device.or(config.default_device),
                transition_enable: config.transition.enable,
                transition_time: transition_time.unwrap_or(config.transition.time),
                transition_step: transition_step.unwrap_or(config.transition.step),
                iio_sensor: iio.or(config.iio.default_sensor),
                curve: config.iio.curve,
            },
        }
    }
}

fn to_percentage(max: i32, value: i32) -> i32 {
    (value * 100 + max / 2) / max
}

fn to_value(max: i32, percentage: i32) -> i32 {
    (percentage * max + 100 / 2) / 100
}

fn set_brightness(
    device: &mut Device,
    Value {
        prefix,
        r#type,
        num,
    }: Value,
    transition_enable: bool,
    transition_time: u64,
    transition_step: u64,
) -> io::Result<()> {
    let max_brightness = device.get_max_brightness()?;
    let current_brightness = device.get_brightness()?;
    let new_brightness = match prefix {
        Prefix::None => match r#type {
            Type::Number => num,
            Type::Percentage => to_value(max_brightness, num),
        },
        Prefix::Plus | Prefix::Minus => {
            let sign = match prefix {
                Prefix::Plus => 1,
                Prefix::Minus => -1,
                Prefix::None => unreachable!(),
            };
            match r#type {
                Type::Number => current_brightness + sign * num,
                Type::Percentage => to_value(
                    max_brightness,
                    to_percentage(max_brightness, current_brightness) + sign * num,
                ),
            }
        }
    }
    .min(max_brightness);

    if transition_enable && transition_time != 0 && transition_step != 0 {
        let steps = transition_time.div_ceil(transition_step);
        let brightness_delta = new_brightness - current_brightness;
        for i in 1..steps {
            let new_brightness = current_brightness + brightness_delta * i as i32 / steps as i32;
            device.set_brightness(new_brightness)?;
            thread::sleep(Duration::from_millis(
                transition_step.min(transition_time - transition_step * i),
            ));
        }
    }

    device.set_brightness(new_brightness)?;
    Ok(())
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    dbg!(&cli);

    let config_path = cli
        .config
        .as_ref()
        .map(|x| Cow::from(x.as_path()))
        .or_else(|| {
            env::var_os("XDG_CONFIG_HOME")
                .map(|x| Cow::from(PathBuf::from(x).join("lilight/lilight.toml")))
        })
        .or_else(|| {
            env::var_os("HOME")
                .map(|x| Cow::from(PathBuf::from(x).join(".config/lilight/lilight.toml")))
        })
        .unwrap_or(Cow::from(Path::new("~/.config/lilight/lilight.toml")));
    let config = match fs::read(&config_path) {
        Ok(x) => match toml::from_slice(&x) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("parser error while reading config file `{config_path:?}`: {e}");
                Config::default()
            }
        },
        Err(e) => {
            eprintln!("connot read the config file at `{config_path:?}`: {e}");
            Config::default()
        }
    };

    let setting = Setting::new(cli, config);

    dbg!(&setting);

    match setting {
        Setting::Set {
            value,
            device,
            transition_enable,
            transition_time,
            transition_step,
        } => {
            let mut device = match device {
                Some(name) => Device::new(Path::new("/sys/class").join(name))?,
                None => {
                    let (device, errors) = Device::first();
                    if !errors.is_empty() {
                        eprintln!("error while getting first device: {errors:#?}");
                    }
                    device.ok_or(io::Error::from(io::ErrorKind::NotFound))?
                }
            };
            set_brightness(
                &mut device,
                value,
                transition_enable,
                transition_time,
                transition_step,
            )
        }
        Setting::Get { max, device, all } => {
            if all {
                let (devices, errors) = Device::all();
                if !errors.is_empty() {
                    eprintln!("error while getting all devices: {errors:#?}");
                }
                for mut device in devices {
                    if max {
                        println!(
                            "device: {device:#?}, max-brightness: {}",
                            device.get_max_brightness()?
                        );
                    } else {
                        println!(
                            "device: {device:#?}, brightness: {}",
                            device.get_brightness()?
                        );
                    }
                }
                Ok(())
            } else {
                let mut device = match device {
                    Some(name) => Device::new(Path::new("/sys/class").join(name))?,
                    None => {
                        let (devices, errors) = Device::all();
                        if !errors.is_empty() {
                            eprintln!("error while getting all devices: {errors:#?}");
                        }
                        devices
                            .into_iter()
                            .next()
                            .ok_or(io::Error::from(io::ErrorKind::NotFound))?
                    }
                };
                if max {
                    println!(
                        "device: {device:#?}, max-brightness: {}",
                        device.get_max_brightness()?
                    );
                } else {
                    println!(
                        "device: {device:#?}, brightness: {}",
                        device.get_brightness()?
                    );
                }
                Ok(())
            }
        }
        Setting::List => {
            let (devices, errors) = Device::all();
            if !errors.is_empty() {
                eprintln!("error while getting all devices: {errors:#?}");
            }
            for device in devices {
                println!("{:?}", device.name);
            }
            Ok(())
        }
        Setting::Daemon {
            device,
            transition_enable,
            transition_time,
            transition_step,
            iio_sensor,
            curve,
        } => {
            let mut device = match device {
                Some(name) => Device::new(Path::new("/sys/class").join(name))?,
                None => {
                    let (devices, errors) = Device::all();
                    if !errors.is_empty() {
                        eprintln!("error while getting all devices: {errors:#?}");
                    }
                    devices
                        .into_iter()
                        .next()
                        .ok_or(io::Error::from(io::ErrorKind::NotFound))?
                }
            };
            let mut iio = match iio_sensor {
                Some(x) => Iio::new(Path::new("/sys/bus/iio/devices/").join(x))?,
                None => {
                    let (iio, errors) = Iio::first();
                    if !errors.is_empty() {
                        eprintln!("error while getting first iio: {errors:#?}");
                    }
                    iio.ok_or(io::Error::from(io::ErrorKind::NotFound))?
                }
            };
            loop {
                match iio.get_illuminance() {
                    Ok(illuminance) => {
                        let new_brightness = curve.apply(illuminance);
                        println!("illuminance: {illuminance}, y: {new_brightness}",);
                        if let Err(e) = set_brightness(
                            &mut device,
                            Value {
                                prefix: Prefix::None,
                                r#type: Type::Percentage,
                                num: new_brightness,
                            },
                            transition_enable,
                            transition_time,
                            transition_step,
                        ) {
                            eprintln!("error while setting brightness: {e}");
                        }
                    }
                    Err(e) => eprintln!("error: {e}"),
                }
                thread::sleep(Duration::from_millis(1000)); // TODO: let user control this
            }
        }
    }
}
