# aa-transfer

> Phase 8 note: the ORIGINAL game's network protocol (datagram header,
> sequencing, admin channels, file transfer blocks) is documented in
> research/phase8-net/README.txt and ported in `aa_orders::net`, because it
> needs the native bit packing. This crate's `AAXFER01` framing remains a
> separate, non-native transport and is not wire compatible with AA.exe.

A working transport foundation for the independent Rust rebuild. It is **not**
compatible with the original game protocol and does not implement multiplayer
gameplay, authentication, encryption, resume, discovery, or acknowledgements.

`send` and `receive` accept generic `Read` / `Write` streams, including TCP. A
frame is eight bytes `AAXFER01`, a little-endian `u64` payload length, the exact
payload bytes, and a 32-byte SHA-256 trailer. The receiver consumes one frame,
so framed transfers can be sequential on the same reliable stream after success.
After an error, discard the connection unless your application has a separate
resynchronization protocol. The sender cannot infer remote verification from its
own successful return.

Payloads stream once through a 64 KiB buffer: no payload-sized allocations,
application bandwidth caps, block sleeps, worker pool limits, or disk size caps.
Blocking I/O provides backpressure. Caller-selected socket deadlines/cancellation
and authorization/resource policy belong at the application boundary. SHA-256
detects corruption, not a malicious sender replacing both data and digest.

`send_file` opens its source read-only. `receive_file_new` uses only the local
caller-selected output path; filenames never travel over the wire. It refuses an
existing destination and does not create directories. On success it verifies the
digest and syncs the file once. On failure after creation, it leaves the new partial
file and returns an error: callers must not use/publish it, and own cleanup. This
avoids deleting a path that another actor may have replaced. Use a trusted parent
directory; no atomic-publish, hostile ancestor-swap protection, or stable snapshot
of concurrently edited source files is claimed.

```rust,no_run
use aa_transfer::{send_file, receive_file_new};
use std::net::TcpStream;
# fn example(mut connection: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
let sent = send_file("caller-selected-save.rsg", &mut connection)?;
// On the receiving peer, with a caller-authorized destination:
let received = receive_file_new(&mut connection, "new-save.rsg")?;
assert_eq!(received.length, sent.length);
# Ok(()) }
```

Tests cover byte-accurate multi-MiB TCP transfer on an ephemeral loopback listener,
known hashes, short/interrupted I/O, corruption, truncated framing, large length
headers without large allocations, and create-new/no-overwrite file behavior.
There are no public-network tests or game-process calls.
