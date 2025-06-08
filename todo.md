

- [x] Figure out C/rust connection to the sensor
- [ ] connect to monitor controls: is there a ddc library, or call ddcutil manually?
  - [ ] libddcutil: https://deepwiki.com/rockowitz/ddcutil (& corresponding rust crate)
- [ ] smarter monitor detection
- [ ] make into a daemon + systemd service
  - [x] need udev rules to unload the vcp driver when attaching the thingy?
- [ ] interface to plasma libs for software brighness on second monitor?


# Rust version
- [ ] DDC detection/control
  - [ ] update existing ddcutil wrapper to use the newer versions of the APIs of the base library?
      See https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html for how to override the thing when building...
  - [ ] for now, maybe use the CLI...
- [ ] put brightness curve in a config file?



AFTER plugging in the thing, need to unload kernel VCP (virtual COM port) driver to talk to it.
```sh
  sudo rmmod ftdi_iso
  sudo rmmod usbserial  # actually this doesn't seem to work, not sure if it's needed
```
