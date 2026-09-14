use std::{
    path::{Path, PathBuf}, sync::{LazyLock, mpsc::channel},
};

use serde::{Deserialize, Serialize};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static XDG_RUNTIME_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(x) => x.into(),
        None => [
            Path::new("/run/user"),
            nix::unistd::getuid().to_string().as_ref(),
        ]
        .into_iter()
        .collect(), // This is not specified in XDG Base Directory
    }
});

pub static DEFAULT_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    XDG_RUNTIME_DIR.join("lilight/daemon.sock")
});

// TODO: use async
// TODO: a client that is not closing the socket can block daemon
// TODO: different device can change brightness at the same time
pub fn daemon() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::filter::Targets::new()
                .with_default(tracing::Level::WARN)
                .with_target(
                    env!("CARGO_CRATE_NAME"),
                    if cfg!(debug_assertions) {
                        tracing::Level::INFO
                    } else {
                        tracing::Level::WARN
                    },
                ),
        )
        .init();

    if let Some(parent) = DEFAULT_PATH.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::remove_file(DEFAULT_PATH.as_path());
    let listener = std::os::unix::net::UnixListener::bind(DEFAULT_PATH.as_path()).unwrap();

    let (tx, rx) = channel();

    std::thread::spawn(move || {
        for request in rx.iter() {
            match request {
                Request::Set { subsystem, name, value, transition } => {
                    crate::set_brightness(&subsystem, &name, value, transition);
                }
            }
        }
    });

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => match serde_json::from_reader::<_, Request>(&mut stream) {
                Ok(request) => {
                    tx.send(request).unwrap();
                }
                Err(e) => {
                    tracing::warn!(error = %e)
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "connection failed");
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum Request {
    Set {
        subsystem: String, name: String, value: crate::cli::Value, transition: Option<crate::Transition>,
    },
}
