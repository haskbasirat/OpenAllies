# Integration and remaining compatibility work

The reusable entry points are `send`, `receive`, `send_file`, and
`receive_file_new`. The first pair accepts any blocking `Read` / `Write` stream;
the second opens a caller-selected source read-only or creates a caller-selected
new destination. `Receipt` contains the transferred `u64` byte count and SHA-256.

The loopback example connects both endpoints locally, streams the file, checks
both receipts, and prints the received byte count and digest:

```powershell
cargo run --manifest-path rust-rebuild/Cargo.toml -p aa-transfer --example loopback_transfer -- "C:/caller/chosen/source.rsg" "C:/caller/chosen/new-copy.rsg"
```

Choose existing parent directories and a destination that does not exist. The
example binds only `127.0.0.1` on an OS-selected ephemeral port. It does not expose
a configurable listening address, connect to another computer, or join a game.
It leaves a new partial destination on receive failure, as the library contract
requires. No source or existing destination is overwritten. No sleep, transfer
rate cap, payload-size cap, or payload-sized allocation is added by the example.

For a future multiplayer application, an authenticated session should authorize
the artifact and select the destination locally. After verification succeeds,
the application can validate the artifact format and publish it according to its
own policy. On failure, keep it inaccessible to the game and arrange explicit
cleanup or retry. Set cancellation and socket deadlines at that application
boundary; do not infer success on the remote endpoint merely from `send` returning.

This component transfers opaque bytes. An original `.RSG` can be copied exactly,
but this does **not** make the new Rust engine able to deserialize or run it.
Compatibility still requires recovered save/object schemas, resource identities,
simulation state, deterministic command/tick handling, and game-level validation.
The original game's discovery, login, lobby, peer negotiation, packet layout,
save-transfer protocol, and synchronization are not implemented here. This new
framing is incompatible with the original native game's sockets. There is no
claim of a working lobby, cross-build multiplayer, resume, or Internet service.

The source review and eleven integration tests cover the library's byte-stream,
file safety, and loopback behavior. They are foundation evidence, not a complete
game or network service test.
