Adaptive Brightness
===================
This uses an external brightness sensor to implement adaptive brightness for desktop external monitors. It simply polls the current brightness every 5s, and adjusts the monitor brightness accordingly. This is intended to be run as a systemd (or similar) service, and relies on the to restart it in case of failures. (for example on resume from sleep, it will get a USB IO error and needs to reconnect to the sensor. Restarting the whole process is an easy way to do this) Stuff under `python/` is a prototype & test scripts. The actual project is under `rust/`.

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
