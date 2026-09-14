use std::{
    ffi::OsStr,
    ops::{Add, Not, Sub},
    path::Path,
    thread,
    time::Duration,
};

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::{
    cli::{Cli, Command, Kind, Prefix, Value},
    config::Config,
};

mod cli;
mod config;
// mod curve;
#[cfg(feature = "daemon")]
mod daemon;

fn main() {
    let cli = Cli::parse();

    let config = if cli.default_config {
        Config::default()
    } else {
        Config::load(cli.config.as_ref().unwrap_or_else(|| &config::DEFAULT_PATH)).unwrap_or_else(
            |e| {
                eprintln!("Failed to load config, using the default config ({e})");
                Config::default()
            },
        )
    };

    match cli.command {
        Command::Set {
            value,
            subsystem,
            name,
            transition,
            transition_time,
            transition_step,
        } => {
            let subsystem = subsystem.unwrap_or(config.default.subsystem);
            let name = name.unwrap_or(config.default.name);
            let transition = transition
                .unwrap_or(config.transition.enable)
                .then_some(Transition {
                    time: transition_time.unwrap_or(config.transition.time),
                    step: transition_step.unwrap_or(config.transition.step),
                });

            #[cfg(feature = "daemon")]
            if !cli.no_daemon {
                send_request_to_daemon(&daemon::Request::Set { subsystem: subsystem.clone(), name: name.clone(), value: value.clone(), transition });
                return;
            }
            set_brightness(&subsystem, &name, value, transition);
        }
        Command::Get {
            subsystem,
            name,
            absolute,
            percentage,
            max,
            all,
        } => {
            let name = name.unwrap_or(config.default.name);
            let name = name.as_ref();
            match (all, subsystem) {
                (true, Some(subsystem)) => {
                    print_brightness(subsystem.as_ref(), name, true, absolute, percentage, max);
                }
                (true, None) => {
                    for subsystem in ["backlight", "leds"].map(Path::new) {
                        print_brightness(subsystem, name, true, absolute, percentage, max);
                    }
                }
                (false, subsystem) => {
                    let subsystem = subsystem.unwrap_or(config.default.subsystem);
                    print_brightness(subsystem.as_ref(), name, false, absolute, percentage, max);
                }
            }
        }
        #[cfg(feature = "daemon")]
        Command::Daemon => {
            if cli.no_daemon {
                println!("What do you want?")
            } else {
                daemon::daemon();
            }
        }
    }
}

#[cfg(feature = "daemon")]
fn send_request_to_daemon(
    request: &daemon::Request,
) {
    let stream = std::os::unix::net::UnixStream::connect(daemon::DEFAULT_PATH.as_path()).unwrap();
    serde_json::to_writer(stream, request).unwrap();
}

fn print_brightness(
    subsystem: &Path,
    name: &Path,
    all: bool,
    absolute: bool,
    percentage: bool,
    max: bool,
) {
    let print_single_device_info = |device: lilight::sysfs::Device,
                                    name_and_subsystem: Option<(&OsStr, &OsStr)>|
     -> Result<(), lilight::sysfs::Error> {
        if let Some((name, subsystem)) = name_and_subsystem {
            println!("{} ({}):", name.display(), subsystem.display());
        }
        // TODO: use brightness or actual_brightness, that is the question
        if absolute {
            let actual_brightness = device
                .get_brightness()
                .or_else(|_| device.get_actual_brightness())?;
            println!("{actual_brightness}");
        } else if percentage {
            let actual_brightness = device
                .get_brightness()
                .or_else(|_| device.get_actual_brightness())?;
            let max_brightness = device.get_max_brightness()?;
            println!("{}", to_percentage(max_brightness, actual_brightness));
        } else if max {
            let max_brightness = device.get_max_brightness()?;
            println!("{max_brightness}");
        } else {
            let actual_brightness = device
                .get_brightness()
                .or_else(|_| device.get_actual_brightness())?;
            let max_brightness = device.get_max_brightness()?;
            println!("\tbrightness:     {actual_brightness}");
            println!("\tmax brightness: {max_brightness}");
            println!(
                "\tpercentage:     {}%",
                to_percentage(max_brightness, actual_brightness)
            );
        }
        Ok(())
    };

    if all {
        for entry in std::fs::read_dir(Path::new("/sys/class").join(&subsystem)).unwrap() {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    eprintln!("{e}");
                    continue;
                }
            };
            let device = lilight::sysfs::Device::new_with_path(entry.path());
            if let Err(e) =
                print_single_device_info(device, Some((&entry.file_name(), subsystem.as_os_str())))
            {
                eprintln!("{e:?}");
            }
        }
    } else {
        let device = lilight::sysfs::Device::new(&subsystem, &name);
        print_single_device_info(
            device,
            (absolute || percentage || max)
                .not()
                .then_some((name.as_os_str(), subsystem.as_os_str())),
        )
        .unwrap()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Transition {
    time: u64,
    step: u64,
}

fn to_percentage(max: u32, value: u32) -> u32 {
    (value * 100 + max / 2) / max
}

fn to_value(max: u32, percentage: u32) -> u32 {
    (percentage * max + 100 / 2) / 100
}

fn set_brightness(subsystem: &str, name: &str, value: Value, transition: Option<Transition>) {
    let device = lilight::sysfs::Device::new(subsystem.as_ref(), name.as_ref());

    let max_brightness = device.get_max_brightness().unwrap();

    let (current_brightness, new_brightness) = match value.prefix {
        Prefix::None => (
            None,
            match value.kind {
                Kind::Number => value.num,
                Kind::Percentage => to_value(max_brightness, value.num),
            },
        ),
        Prefix::Plus | Prefix::Minus => {
            let op = match value.prefix {
                Prefix::Plus => u32::saturating_add,
                Prefix::Minus => u32::saturating_sub,
                Prefix::None => unreachable!(),
            };
            let current_brightness = device.get_brightness().unwrap();
            (
                Some(current_brightness),
                match value.kind {
                    Kind::Number => op(current_brightness, value.num).max(max_brightness),
                    Kind::Percentage => to_value(
                        max_brightness,
                        op(to_percentage(max_brightness, current_brightness), value.num),
                    ),
                },
            )
        }
    };
    let new_brightness = new_brightness.min(max_brightness);

    let transition = match (current_brightness, transition) {
        (Some(x), Some(y)) => Some((x, y)),
        (None, Some(x)) => Some((device.get_brightness().unwrap(), x)),
        (_, None) => None,
    };

    #[cfg(feature = "dbus")]
    if try_set_brightness_with_dbus(subsystem, name, new_brightness, transition) {
        return;
    }

    set_brightness_with_sysfs(&device, new_brightness, transition).unwrap();
}

fn set_brightness_with_sysfs(
    device: &lilight::sysfs::Device,
    value: u32,
    transition: Option<(u32, Transition)>,
) -> std::io::Result<()> {
    match transition {
        None
        | Some((_, Transition { time: 0, step: _ }))
        | Some((_, Transition { time: _, step: 0 })) => device.set_brightness(value),
        Some((current_brightness, transition)) => {
            let steps = transition.time.div_ceil(transition.step);
            let (op, brightness_delta): (fn(u32, u32) -> u32, _) = if value > current_brightness {
                (Add::add, value - current_brightness)
            } else {
                (Sub::sub, current_brightness - value)
            };
            for i in 1..steps {
                let new_brightness = op(
                    current_brightness,
                    brightness_delta * i as u32 / steps as u32,
                );
                device.set_brightness(new_brightness)?;
                thread::sleep(Duration::from_millis(
                    transition.step.min(transition.time - transition.step * i),
                ));
            }
            Ok(())
        }
    }
}

#[cfg(feature = "dbus")]
fn try_set_brightness_with_dbus(
    subsystem: &str,
    name: &str,
    value: u32,
    transition: Option<(u32, Transition)>,
) -> bool {
    use zbus::blocking::Connection;

    let connection = match Connection::system() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Failed to connect to system bus: {e}");
            return false;
        }
    };
    let proxy = match lilight::dbus::LogindProxyBlocking::new(&connection) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Failed to create dbus proxy: {e}");
            return false;
        }
    };

    match transition {
        None
        | Some((_, Transition { time: 0, step: _ }))
        | Some((_, Transition { time: _, step: 0 })) => {
            match proxy.set_brightness(subsystem, name, value) {
                Ok(()) => true,
                Err(e) => {
                    eprintln!("Failed to set brightness: {e}");
                    false
                }
            }
        }
        Some((current_brightness, transition)) => {
            let steps = transition.time.div_ceil(transition.step);
            let (op, brightness_delta): (fn(u32, u32) -> u32, _) = if value > current_brightness {
                (Add::add, value - current_brightness)
            } else {
                (Sub::sub, current_brightness - value)
            };
            for i in 1..steps {
                let new_brightness = op(
                    current_brightness,
                    brightness_delta * i as u32 / steps as u32,
                );
                match proxy.set_brightness(subsystem, name, new_brightness) {
                    Ok(()) => (),
                    Err(e) => {
                        eprintln!("Failed to set brightness: {e}");
                        return false;
                    }
                }
                thread::sleep(Duration::from_millis(
                    transition.step.min(transition.time - transition.step * i),
                ));
            }
            true
        }
    }
}
