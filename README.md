# studiox-switcher

A input switcher and silence detector for JACK.

* Can switch between 3 stereo inputs
* A fallback channel is activated if the active input is below a volume threshold for some amount of seconds

## Installation

```
git clone https://github.com/Frando/studiox-switcher.git
cd studiox-switcher
cargo build --release
sudo cp rust_target/release/studiox-switcher /usr/local/bin
```

