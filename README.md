# Lilight

A lightweight tool to control screen brightness on Linux.

## Roadmap

-[ ] v0.1
    -[x] change brightness using sysfs
    -[x] reading iio sensor
    -[x] cli arguments
    -[x] config file
    -[x] change brightness based on iio sensor and mapping function from config file
    -[ ] release
-[ ] v0.2
    -[ ] good error messages
    -[ ] udev rule
    -[ ] other way to change brightness (e.g. dbus)
-[ ] v0.3
    -[ ] ipc (talk to daemon)
    -[ ] remember the brightness adjust from user and update sensor to brightness map function
    -[ ] handle too fast "set" event within the transition time
