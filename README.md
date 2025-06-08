TODO...

Adaptive Brightness
===================
This uses an external brightness sensor to implement adaptive brightness for desktop external monitors.


Hardware
--------
- Brightness sensor: TSL2591 breakout board from adafruit
- Intermediate board to translate USB <-> I2C: FT232H breakout board from adafruit




UDEV rules
----------
The following udev rules are needed to connect to the USB device. (actually onlt one of these applies, but I don't remember which one specifically)
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