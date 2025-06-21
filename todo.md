

- [x] Figure out C/rust connection to the sensor
- [x] control monitors calling ddcutil
  - [x] libddcutil: https://deepwiki.com/rockowitz/ddcutil (there is a rust crate, but it's out of date and doesn't work)
  - [ ] new wrapper for libddcutil
- [ ] smarter monitor detection
- [ ] configuration file
- [x] make into a nix pkg + daemon + systemd service
- [ ] interface to plasma libs for software brightness on second monitor?
- [ ] error handling:
  - (in-process vs letting systemd restart)
  - [ ] Detecting monitors = fatal
  - [ ] reading config = fatal? warning + use default?
  - [ ] errors reading sensor = retryable in general, too many = fatal?
  - [ ] error setting brightness = retryable in general, too many = fatal
- [ ] commands:
  - [ ] parsing config file
  - [ ] detecting monitors
  - [ ] directly setting brightness
- [ ] Service lifecycle
  - [ ] retrying errors
  - [ ] fatal vs retryable errors
  - [ ] config file changes?
  - [ ] notify on error?


- "error=Io(Custom { kind: Other, error: "libusb error code -1" })" error = restartable
