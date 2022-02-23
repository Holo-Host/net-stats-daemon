# net-stats-daemon

HPOS daemon for collecting statistics from holoport and PUSHing them to database via `match-service-api`.

Currently script is collecting following information:

```
  Holo Network  # can be one of devNet, alphaNet, flexNet...
  Channel       # nix-channel that HPOS is following
  Model         # HP or HP+
  SSH status    # is SSH enabled?
  ZT IP         # IP address on Zerotier network
  IP address    # IPv4 address on internet
  Holoport ID   # base36 encoded public key of the host
```

Once collected payload is signed with holoport's private key and sent to `match-service-api`.

### Prerequisites

Location of an `hpos-config.toml` file is hard-coded in `keypair.rs` module. Binary needs an env var `DEVICE_SEED_DEFAULT_PASSWORD` for unlocking this file.

### Debugging

`export RUST_LOG=DEBUG; cargo run` for debugging info.
