

- [x] Figure out C/rust connection to the sensor
- [x] control monitors calling ddcutil
  - [x] libddcutil: https://deepwiki.com/rockowitz/ddcutil (there is a rust crate, but it's out of date and doesn't work)
  - [x] new wrapper for libddcutil
- [x] smarter monitor detection
- [x] configuration file
- [x] make into a nix pkg + daemon + systemd service
- [ ] interface to plasma libs for software brightness on second monitor?
- [ ] error handling:
  - (in-process vs letting systemd restart)
  - [ ] Detecting monitors = fatal
  - [ ] reading config = fatal? warning + use default?
  - [ ] errors reading sensor = retryable in general, too many = fatal?
  - [ ] error setting brightness = retryable in general, too many = fatal
- [ ] commands:
  - [x] parsing config file
  - [x] detecting monitors
  - [ ] directly setting brightness
- [ ] Service lifecycle
  - [ ] retrying errors
  - [ ] fatal vs retryable errors
  - [ ] config file changes?
  - [ ] notify on error?
- [ ] proper logging library?
- [ ] home-manager module?
- [ ] configure sensor details: specify serial number of the ftdi device?


- "error=Io(Custom { kind: Other, error: "libusb error code -1" })" error = restartable


libddcutil:
 - see `ldconfig -p` to find dynamic library path, then `nm -D` to find the symbols in the library
 - https://github.com/arcnmx/ddcutil-rs/issues/2