

- [x] Figure out C/rust connection to the sensor
- [x] control monitors calling ddcutil
  - [ ] libddcutil: https://deepwiki.com/rockowitz/ddcutil (there is a rust crate, but it's out of date and doesn't work)
- [ ] smarter monitor detection
- [ ] make into a nix pkg + daemon + systemd service
  - [x] need udev rules to unload the vcp driver when attaching the thingy?
- [ ] interface to plasma libs for software brighness on second monitor?
- [ ] put brightness curve in a config file?


- "error=Io(Custom { kind: Other, error: "libusb error code -1" })" error = restartable