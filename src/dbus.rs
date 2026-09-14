// <https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.login1.html>
#[cfg(feature = "dbus")]
#[zbus::proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto" // TODO: auto vs self???
)]
pub trait Logind {
    fn set_brightness(&self, subsystem: &str, name: &str, brightness: u32) -> zbus::Result<()>;
}
