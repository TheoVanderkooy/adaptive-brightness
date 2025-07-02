Adaptive Brightness
===================
This uses an external brightness sensor to implement adaptive brightness for desktop external monitors. It simply polls the current brightness every 5s, and adjusts the monitor brightness accordingly. This is intended to be run as a systemd (or similar) service, and relies on the to restart it in case of failures. (for example on resume from sleep, it will get a USB IO error and needs to reconnect to the sensor. Restarting the whole process is an easy way to do this) Stuff under `python/` is a prototype & test scripts. The actual project is under `rust/`.

Configuration
-------------
Configuration goes in `~/.config/adaptive-brightness/config.ron`. (or more specifically, `adaptive-brightness/config.ron` under your [XDG](https://specifications.freedesktop.org/basedir-spec/latest/) config directory)

The configuration file is formatted as [rust object notation (RON)](https://docs.rs/ron/latest/ron/). This was chosen because it was convenient, but also it was hard to find anything else that could represent the lux -> brightness mapping of the brightness curve in a not-too-ugly way.

The general configuration structure is:
```
(
    monitors: [
        (
            identifier: <identifier>,
            curve: [
                (<lux_1>, <brightness_1>),
                (<lux_2>, <brightness_2>),
                ...
            ],
        ),
        ...
    ]
)
```
where there could be multiple monitors in the list, one or more (lux, brightness) pairs for each curve, and `<identifier>` is an enum representing how to identify the monitor(s) that should follow that particular curve. The allowed values are:
- `I2cBus(<busno>)`: the bus number of the corresponding `/dev/i2c-<busno>` device.
- `ModelSerial(<manufacturer>, <model>, <serial code>)`: the strings representing manufacturer, model, and serial code. The easiest way to find these strings is `adaptive-brightness check`, or `ddcutil detect`.
- `Model(<manufacturer>, <model>)`: the same as `ModelSerial`, but without the serial number. This can be useful to configure multiple of the same monitor with the same curve. `ModelSerial` will take precedence if multiple rules apply to the same display.
- `Serial(<serial code>)`: the same as `ModelSerial` but omitting the manufacturer & model.
- `Default`: will apply to any display that doesn't match a more specific rule. If there is no default, displays that don't match any rule will be ignored.

Hardware
--------
- Brightness sensor: TSL2591 breakout board from adafruit
- Intermediate board to translate USB <-> I2C: FT232H breakout board from adafruit


Software
--------
The following udev rules are needed to connect to the USB device. (only the product ID = 6014 is relevant for the FT232H, the rest are other similar chips)
```
SUBSYSTEM=="usb", ATTR{idVendor}=="0403", ATTR{idProduct}=="6001", GROUP="plugdev", MODE="0666"
SUBSYSTEM=="usb", ATTR{idVendor}=="0403", ATTR{idProduct}=="6011", GROUP="plugdev", MODE="0666"
SUBSYSTEM=="usb", ATTR{idVendor}=="0403", ATTR{idProduct}=="6010", GROUP="plugdev", MODE="0666"
SUBSYSTEM=="usb", ATTR{idVendor}=="0403", ATTR{idProduct}=="6014", GROUP="plugdev", MODE="0666"
SUBSYSTEM=="usb", ATTR{idVendor}=="0403", ATTR{idProduct}=="6015", GROUP="plugdev", MODE="0666"
```

If that isn't enough on its own, you may need to unload built-in ftdi drivers:
```sh
sudo rmmod ftdi_sio
```

Also, `ddcutil` needs to be installed, and needs to be callable by the user. On nixos, it is enough to install the `ddcutil` package and set `hardware.i2c.enable = true;`


Resources
---------
- [TSL2591 datsheet](https://cdn-shop.adafruit.com/datasheets/TSL25911_Datasheet_EN_v1.pdf)
- Adafruit TSL2591 board [datasheet](https://cdn-learn.adafruit.com/downloads/pdf/adafruit-tsl2591.pdf)
