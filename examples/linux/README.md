# Linux examples

The Linux examples can communicate with either a BlueZ HCI device or a Unix
socket. The first positional argument selects the transport:

- No argument or a decimal number selects a BlueZ HCI device. The default is
  device `0` (`hci0`).
- Any other value is used as a Unix socket path.

```sh
cargo run --bin ble_scanner             # BlueZ hci0
cargo run --bin ble_scanner -- 1        # BlueZ hci1
cargo run --bin ble_scanner -- /tmp/hci.sock
```

## BlueZ HCI user channel

This example opens a "user channel" with the [Linux HCI socket interface](https://github.com/bluez/bluez/wiki/HCI), which assumes full control of the device.

To bind this channel, the device must first be down (e.g. `hciconfig hci0 down`) and the process must have the `CAP_NET_ADMIN` capability.

To run an example with `CAP_NET_ADMIN`, either just run as root or try an incantation like the following (requires privileges to launch but runs as a regular user):
```
systemd-run \
  --pty \
  --uid=$(id -u) \
  --gid=$(id -g) \
  --same-dir \
  --setenv RUST_LOG=info \
  --setenv PATH \
  --property "AmbientCapabilities=CAP_NET_ADMIN" \
  cargo run --bin ble_scanner
```
