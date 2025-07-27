use std::{path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Set {
        #[arg(allow_hyphen_values = true)]
        value: Value,

        #[arg(short, long)]
        device: Option<PathBuf>,
        #[arg(short, long)]
        transition_time: Option<u64>,
        #[arg(short = 's', long)]
        transition_step: Option<u64>,
    },
    Get {
        #[arg(short, long)]
        max: bool,
        #[arg(short, long, group = "devices")]
        device: Option<PathBuf>,
        #[arg(short, long, group = "devices")]
        all: bool,
    },
    List,
    Daemon {
        #[arg(short, long)]
        device: Option<PathBuf>,
        #[arg(short, long)]
        transition_time: Option<u64>,
        #[arg(short = 's', long)]
        transition_step: Option<u64>,
        #[arg(short, long)]
        iio: Option<PathBuf>,
    },
}

#[derive(Clone, Debug)]
pub struct Value {
    pub prefix: Prefix,
    pub r#type: Type,
    pub num: i32,
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
        let (r#type, s) = if s.ends_with("%") {
            (Type::Percentage, &s[..s.len() - 1])
        } else {
            (Type::Number, s)
        };
        let num = s.parse().map_err(|e| format!("parsing error: {e}"))?;
        Ok(Self {
            prefix,
            r#type,
            num,
        })
    }
}

#[derive(Clone, Debug)]
pub enum Prefix {
    None,
    Plus,
    Minus,
}

#[derive(Clone, Debug)]
pub enum Type {
    Number,
    Percentage,
}
