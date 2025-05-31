

- [ ] Figure out C/rust connection to the sensor
- [ ] connect to monitor controls: is there a ddc library, or call ddcutil manually?
- [ ] smarter monitor detection
- [ ] make into a daemon + systemd service
  - [ ] need udev rules to unload the vcp driver when attaching the thingy?
- [ ] interface to plasma libs for software brighness on second monitor?

AFTER plugging in the thing, need to unload kernel VCP (virtual COM port) driver to talk to it.
```sh
  sudo rmmod ftdi_iso
  sudo rmmod usbserial  # actually this doesn't seem to work, not sure if it's needed
  sudo EEPROM/read/read  # should show the adafruit board
```

