

- [ ] interface to plasma libs for software brightness on second monitor?
- [ ] error handling:
  - (in-process vs letting systemd restart)
  - [ ] Detecting monitors = fatal
  - [ ] reading config = fatal? warning + use default?
  - [ ] errors reading sensor = retryable in general, too many = fatal?
  - [ ] error setting brightness = retryable in general, too many = fatal
  - [ ] "required" monitors?
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
- [x] separate brightness server:
  - [x] split out a binary just for reading & exposing brightness over a socket (to multiple consumers)
  - [x] move the main functionality to read from the socket
  - [x] separate CLI to read the current brightness?
- [ ] make _monitor brightness_ (not lux) observable.. abc daemon could publish that to a socket?
- [ ] UI!
  - [ ] make the daemon expose stuff over a socket, similar to brightness daemon
    - [ ] more than single-byte "commands" -- can still be single threaded if client sends partial command?
    - [ ] also needs args for where to expose its socket, as well as brightness input socket
  - [ ] expose lux, brightness, monitors, config, ...
  - [ ] allow restarting daemon and/or reloading the config/redetecting monitors
  - [ ] editing config from the UI
  - [x] export stats to some realtime DB (or just a file...) for analysis (prometheus ?)



libddcutil:
 - see `ldconfig -p` to find dynamic library path, then `nm -D` to find the symbols in the library
 - https://github.com/arcnmx/ddcutil-rs/issues/2
