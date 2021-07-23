# studiox-switcher

A input switcher and silence detector for JACK.

* Can switch between 3 stereo inputs
* A fallback channel is activated if the active input is below a volume threshold for some amount of seconds
* Channels may be switched via OSC and HTTP (TODO)

## Roadmap

- [x] Core DSP engine via Faust for switching and silence detection
- [x] JACK client with autoconnect
- [x] OSC interface
- [ ] HTTP interface
- [ ] Web frontend
- [ ] Authentication 

## Installation

```
git clone https://github.com/Frando/studiox-switcher.git
cd studiox-switcher
cargo build --release
sudo cp rust_target/release/studiox-switcher /usr/local/bin
```

## Configuration & usage

`studiox-switcher` may be called with a `-c path/to/config.toml` option. In the config file, the labels and JACK connection target ports for inputs and outputs can be specified. See [`studiox-switcher.toml`](studiox-switcher.toml) for an example config file.

### Usage via OSC

`studiox-switcher` listens for OSC messages on port 7000. The only supported path is `/switcher` with two integers, the first being the input number (1-3) and the second either 0 or 1 to enable or disable the channel.

If more than one channel is enabled, the "higher" channel wins. If none is enabled, the fallback channel is on. If an enabled channel goes silent, the fallback channel kicks in.

Example: Enable channel 1
```
oscsend localhost 7000 /switcher ii 1 1
```
