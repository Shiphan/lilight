# Lilight

A lightweight tool to control screen brightness on Linux.

## Roadmap

- [ ] v0.1
    - [x] change brightness using sysfs
    - [x] change brightness using dbus (<https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.login1.html>)
    - [x] reading iio sensor
    - [x] cli arguments
    - [x] config file
    - [x] ipc (talk to daemon)
    - [ ] a codebase I like
    - [ ] release
- [ ] v0.2
    - [ ] use illuminance reading to automatically adjust brightness
    - [ ] automatically reload config file
    - [ ] remember the brightness adjust from user and update sensor to brightness map function
