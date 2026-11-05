# aa-eos

Online play through Epic Online Services: the game's Internet button. It fills the
screen GameSpy filled in the original. Built only with the client's `eos` feature:

    cd crates/aa-bevy
    cargo build --release --features eos

Without the feature, LAN and a direct address work and the client holds nothing of
Epic's. This crate has its own workspace, so the main workspace never resolves it.

## What is needed at run time, and is never in this repository

- The SDK's library beside the program: `EOSSDK-Win64-Shipping.dll` from Epic's SDK,
  which every developer downloads from Epic under Epic's agreement. It is found at
  run time, so a missing library is a message box and not a failed start. A developer
  may set `EOS_SDK_DIR` to the SDK folder instead (its `Bin` is used).
- `openallies-ids.txt`: `EOS_PRODUCT_ID`, `EOS_SANDBOX_ID`, `EOS_DEPLOYMENT_ID` and
  `EOS_CLIENT_ID`, one `KEY=value` per line.
- The client secret: `client-secret.txt` beside that file, or `AA_EOS_CLIENT_SECRET`.

The folder for the two files is `AA_EOS_CONFIG_DIR`, else `eos/` beside the program,
else `target/eos/` of any folder above the program or the current directory.

A release that carries the DLL must carry `END-USER-NOTICE.txt` too.

## How it is built

- `api.rs`: the SDK's functions found at run time. Each is checked at compile time
  against the signature `eos-sys` declares. Only the bindings' types are used, so
  nothing links against the library and no SDK is needed to build.
- `worker.rs`: one thread owns the SDK, makes every call and ticks the platform.
- `service.rs`: what the game holds. Commands in, events and datagrams out. Nothing
  blocks and nothing touches the SDK.
- `frame.rs`: datagrams up to 65,535 bytes over P2P packets of 1,170.
- `addr.rs`: a player's IPv4 address on this path, computed from the ProductUserId, so
  every machine computes the same one (the session sends these to its peers).
- `config.rs`: the IDs and the secret. Nothing prints the secret.

The client's `net/eos.rs` is the front end: the Internet panel of `LobbyMenu`, the
list, and `aa_netcode::udp::Transport` over the link.

## Trying it without the game

    cargo run --release --example pair -- library
    cargo run --release --example pair -- host Alice
    AA_EOS_NEW_DEVICE_ID=1 cargo run --release --example pair -- join Bob

`host` makes a lobby and answers every datagram; `join` finds it, joins it and sends
datagrams of many sizes, up to the largest.

Epic keeps the device ID per product and Windows user, in the Windows Credential
Manager (`eos-sdk:<product>:1`). Separate `APPDATA` folders do not separate players.
Two players on one machine therefore need `AA_EOS_NEW_DEVICE_ID=1` on the second:
`EOS_Connect_DeleteDeviceId` replaces that product's anonymous player for this Windows
user, and the first keeps playing with the sign-in it has.
