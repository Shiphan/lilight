use std::{path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,
    #[arg(long, global = true)]
    pub default_config: bool,
    #[cfg(feature = "daemon")]
    #[arg(long, global = true)]
    pub no_daemon: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Set {
        // TODO: explan the value format (-10%)
        #[arg(allow_hyphen_values = true)]
        value: Value,

        #[arg(short = 's', long)]
        subsystem: Option<String>,
        #[arg(short = 'n', long)]
        name: Option<String>,
        #[arg(short = 't', long)]
        transition: Option<bool>,
        #[arg(short = 'T', long)]
        transition_time: Option<u64>,
        #[arg(short = 'S', long)]
        transition_step: Option<u64>,
    },
    Get {
        #[arg(short = 's', long)]
        subsystem: Option<String>,
        #[arg(short = 'n', long)]
        name: Option<String>,
        #[arg(short = 'A', long, group = "value_type")]
        absolute: bool,
        #[arg(short = 'p', long, group = "value_type")]
        percentage: bool,
        #[arg(short = 'm', long, group = "value_type")]
        max: bool,
        #[arg(short = 'a', long)]
        all: bool,
    },
    #[cfg(feature = "daemon")]
    Daemon,
    /*
    #[cfg(feature = "daemon")]
    Daemon {
        #[arg(short = 's', long)]
        subsystem: Option<String>,
        #[arg(short = 'n', long)]
        name: Option<String>,
        #[arg(short = 't', long)]
        transition: Option<bool>,
        #[arg(short = 'T', long)]
        transition_time: Option<u64>,
        #[arg(short = 'S', long)]
        transition_step: Option<u64>,
        #[arg(short = 'i', long)]
        iio: Option<PathBuf>,
    },
    */
    // Curve,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Value {
    pub prefix: Prefix,
    pub kind: Kind,
    pub num: u32,
}

impl FromStr for Value {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (prefix, s) = match s.chars().next() {
            Some('+') => (Prefix::Plus, &s[1..]),
            Some('-') => (Prefix::Minus, &s[1..]),
            Some(_) => (Prefix::None, s),
            None => Err("the value is empty")?,
        };
        let (kind, s) = if s.ends_with("%") {
            (Kind::Percentage, &s[..s.len() - 1])
        } else {
            (Kind::Number, s)
        };
        let num = s.parse().map_err(|e| format!("parsing error: {e}"))?;
        Ok(Self { prefix, kind, num })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Prefix {
    None,
    Plus,
    Minus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Kind {
    Number,
    Percentage,
}
