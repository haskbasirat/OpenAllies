use aa_transfer::{
    COPY_BUFFER_BYTES, DIGEST_BYTES, HEADER_BYTES, Receipt, Stage, TransferError, receive,
    receive_file_new, send, send_file,
};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::fs;
use std::io::{self, Cursor, Read, Write};
use std::net::{Ipv4Addr, Shutdown, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn encode(bytes: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();
    send(&mut Cursor::new(bytes), &mut frame, bytes.len() as u64).unwrap();
    frame
}

fn is_io(error: TransferError, expected_stage: Stage, expected_kind: io::ErrorKind) {
    match error {
        TransferError::Io { stage, source } => {
            assert_eq!(stage, expected_stage);
            assert_eq!(source.kind(), expected_kind);
        }
        other => panic!("expected I/O error, got {other:?}"),
    }
}

#[test]
fn wire_format_and_known_sha256_are_exact() {
    let frame = encode(b"abc");
    assert_eq!(&frame[..8], b"AAXFER01");
    assert_eq!(&frame[8..16], &3u64.to_le_bytes());
    assert_eq!(&frame[16..19], b"abc");
    let expected = [
        0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22,
        0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00,
        0x15, 0xad,
    ];
    assert_eq!(&frame[19..], &expected);
    let mut output = Vec::new();
    assert_eq!(
        receive(&mut Cursor::new(frame), &mut output).unwrap(),
        Receipt {
            length: 3,
            sha256: expected
        }
    );
    assert_eq!(output, b"abc");
}

#[test]
fn empty_and_buffer_boundary_lengths_round_trip() {
    for length in [
        0,
        1,
        COPY_BUFFER_BYTES - 1,
        COPY_BUFFER_BYTES,
        COPY_BUFFER_BYTES + 1,
    ] {
        let bytes: Vec<_> = (0..length)
            .map(|i| (i.wrapping_mul(17) & 255) as u8)
            .collect();
        let mut output = Vec::new();
        let receipt = receive(&mut Cursor::new(encode(&bytes)), &mut output).unwrap();
        assert_eq!(receipt.length, length as u64);
        assert_eq!(output, bytes);
        assert_eq!(receipt.sha256.as_slice(), Sha256::digest(&bytes).as_slice());
    }
}

#[test]
fn one_frame_consumes_only_its_declared_bytes() {
    let mut source = Cursor::new(b"first-second");
    let mut frames = Vec::new();
    send(&mut source, &mut frames, 5).unwrap();
    assert_eq!(source.position(), 5);
    frames.extend_from_slice(&encode(b"next"));
    frames.extend_from_slice(b"trailing");
    let mut transport = Cursor::new(frames);
    let mut first = Vec::new();
    let mut second = Vec::new();
    receive(&mut transport, &mut first).unwrap();
    receive(&mut transport, &mut second).unwrap();
    let mut trailing = String::new();
    transport.read_to_string(&mut trailing).unwrap();
    assert_eq!(first, b"first");
    assert_eq!(second, b"next");
    assert_eq!(trailing, "trailing");
}

#[test]
fn corruption_and_all_truncation_offsets_fail() {
    let valid = encode(b"a deterministic corruption fixture");
    for length in 0..valid.len() {
        let error = receive(&mut Cursor::new(&valid[..length]), &mut io::sink()).unwrap_err();
        match error {
            TransferError::Io { source, .. } => {
                assert_eq!(source.kind(), io::ErrorKind::UnexpectedEof)
            }
            other => panic!("unexpected truncated error at {length}: {other:?}"),
        }
    }
    for offset in [HEADER_BYTES, valid.len() - 1] {
        let mut corrupted = valid.clone();
        corrupted[offset] ^= 0x80;
        assert!(matches!(
            receive(&mut Cursor::new(corrupted), &mut io::sink()),
            Err(TransferError::HashMismatch { .. })
        ));
    }
    let mut bad_magic = valid.clone();
    bad_magic[7] = b'2';
    assert!(matches!(
        receive(&mut Cursor::new(bad_magic), &mut io::sink()),
        Err(TransferError::InvalidMagic { .. })
    ));
    for declared in [1u64, 100, (1u64 << 32) + 17, u64::MAX] {
        let mut bad_length = valid.clone();
        bad_length[8..16].copy_from_slice(&declared.to_le_bytes());
        assert!(receive(&mut Cursor::new(bad_length), &mut io::sink()).is_err());
    }
}

struct Fragmented<T> {
    inner: T,
    calls: usize,
}

impl<T: Read> Read for Fragmented<T> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.calls += 1;
        if self.calls.is_multiple_of(4) {
            return Err(io::ErrorKind::Interrupted.into());
        }
        let length = buffer.len().min(3);
        self.inner.read(&mut buffer[..length])
    }
}

impl<T: Write> Write for Fragmented<T> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        if self.calls.is_multiple_of(4) {
            return Err(io::ErrorKind::Interrupted.into());
        }
        self.inner.write(&buffer[..buffer.len().min(5)])
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[test]
fn short_and_interrupted_reads_and_writes_are_handled() {
    let source = b"fragmented transports are reliable ordered byte streams";
    let mut input = Fragmented {
        inner: Cursor::new(source),
        calls: 0,
    };
    let mut wire = Fragmented {
        inner: Vec::new(),
        calls: 0,
    };
    let sent = send(&mut input, &mut wire, source.len() as u64).unwrap();
    let mut reader = Fragmented {
        inner: Cursor::new(wire.inner),
        calls: 0,
    };
    let mut sink = Fragmented {
        inner: Vec::new(),
        calls: 0,
    };
    let received = receive(&mut reader, &mut sink).unwrap();
    assert_eq!(sent, received);
    assert_eq!(sink.inner, source);
}

struct FailingWriter {
    remaining: usize,
    fail_flush: bool,
}
impl Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let count = bytes.len().min(self.remaining);
        self.remaining -= count;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.fail_flush {
            Err(io::ErrorKind::Other.into())
        } else {
            Ok(())
        }
    }
}

#[test]
fn failed_source_sink_and_flush_are_never_success() {
    is_io(
        send(&mut Cursor::new(b"short"), &mut Vec::new(), 6).unwrap_err(),
        Stage::PayloadRead,
        io::ErrorKind::UnexpectedEof,
    );
    for (remaining, stage) in [
        (0, Stage::HeaderWrite),
        (HEADER_BYTES, Stage::PayloadWrite),
        (HEADER_BYTES + 3, Stage::DigestWrite),
    ] {
        let mut sink = FailingWriter {
            remaining,
            fail_flush: false,
        };
        is_io(
            send(&mut Cursor::new(b"abc"), &mut sink, 3).unwrap_err(),
            stage,
            io::ErrorKind::BrokenPipe,
        );
    }
    let mut sink = FailingWriter {
        remaining: usize::MAX,
        fail_flush: true,
    };
    is_io(
        send(&mut Cursor::new(b"abc"), &mut sink, 3).unwrap_err(),
        Stage::Flush,
        io::ErrorKind::Other,
    );
    let mut sink = FailingWriter {
        remaining: 1,
        fail_flush: false,
    };
    is_io(
        receive(&mut Cursor::new(encode(b"abc")), &mut sink).unwrap_err(),
        Stage::PayloadWrite,
        io::ErrorKind::BrokenPipe,
    );
    let mut sink = FailingWriter {
        remaining: usize::MAX,
        fail_flush: true,
    };
    is_io(
        receive(&mut Cursor::new(encode(b"abc")), &mut sink).unwrap_err(),
        Stage::Flush,
        io::ErrorKind::Other,
    );
}

struct AssertBoundedReader {
    requested: usize,
}
impl Read for AssertBoundedReader {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.requested = bytes.len();
        assert!(bytes.len() <= COPY_BUFFER_BYTES);
        Ok(0)
    }
}

#[test]
fn u64_length_does_not_allocate_payload_or_narrow_to_u32() {
    for length in [(1u64 << 32) + 17, u64::MAX] {
        let mut source = AssertBoundedReader { requested: 0 };
        let mut frame = Vec::new();
        is_io(
            send(&mut source, &mut frame, length).unwrap_err(),
            Stage::PayloadRead,
            io::ErrorKind::UnexpectedEof,
        );
        assert_eq!(frame.len(), HEADER_BYTES);
        assert_eq!(u64::from_le_bytes(frame[8..16].try_into().unwrap()), length);
        assert_eq!(source.requested, COPY_BUFFER_BYTES);
        let mut body = AssertBoundedReader { requested: 0 };
        let mut reader = Cursor::new(frame).chain(&mut body);
        is_io(
            receive(&mut reader, &mut io::sink()).unwrap_err(),
            Stage::PayloadRead,
            io::ErrorKind::UnexpectedEof,
        );
        assert_eq!(body.requested, COPY_BUFFER_BYTES);
    }
}

struct TemporaryDirectory {
    root: PathBuf,
    files: RefCell<Vec<PathBuf>>,
}
impl TemporaryDirectory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "aa-transfer-tests-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self {
            root,
            files: RefCell::new(Vec::new()),
        }
    }
    fn file(&self, name: &str) -> PathBuf {
        assert!(!name.contains(['/', '\\']));
        let path = self.root.join(name);
        self.files.borrow_mut().push(path.clone());
        path
    }
}
impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        for path in self.files.get_mut().iter() {
            let _ = fs::remove_file(path);
        }
        let _ = fs::remove_dir(&self.root);
    }
}

#[test]
fn file_receive_never_overwrites_and_does_not_trust_peer_paths() {
    let dir = TemporaryDirectory::new();
    let original = dir.file("original.rsg");
    fs::write(&original, b"caller original").unwrap();
    let mut transport = Cursor::new(encode(b"../not-a-wire-path"));
    is_io(
        receive_file_new(&mut transport, &original).unwrap_err(),
        Stage::DestinationCreate,
        io::ErrorKind::AlreadyExists,
    );
    assert_eq!(fs::read(&original).unwrap(), b"caller original");
    // Only the header was consumed before the create-new refusal.
    assert_eq!(transport.position(), HEADER_BYTES as u64);
    let output = dir.file("authorized-output.rsg");
    let receipt =
        receive_file_new(&mut Cursor::new(encode(b"../not-a-wire-path")), &output).unwrap();
    assert_eq!(receipt.length, 18);
    assert_eq!(fs::read(&output).unwrap(), b"../not-a-wire-path");
    let missing = dir.file("invalid-header.rsg");
    assert!(receive_file_new(&mut Cursor::new([0u8; HEADER_BYTES]), &missing).is_err());
    assert!(!missing.exists());
    let mut wire = Vec::new();
    send_file(&original, &mut wire).unwrap();
    assert_eq!(wire, encode(b"caller original"));
    assert_eq!(fs::read(&original).unwrap(), b"caller original");
}

#[test]
fn failed_file_receive_retains_only_its_new_unverified_partial_file() {
    let dir = TemporaryDirectory::new();
    let output = dir.file("partial.rsg");
    let mut frame = encode(b"payload");
    *frame.last_mut().unwrap() ^= 1;
    assert!(matches!(
        receive_file_new(&mut Cursor::new(frame), &output),
        Err(TransferError::HashMismatch { .. })
    ));
    assert_eq!(fs::read(&output).unwrap(), b"payload");
    assert!(receive_file_new(&mut Cursor::new(encode(b"new")), &output).is_err());
    assert_eq!(fs::read(&output).unwrap(), b"payload");
}

fn pattern(index: u64) -> u8 {
    ((index.wrapping_mul(17) ^ (index >> 8)) & 255) as u8
}
struct PatternReader {
    position: u64,
    length: u64,
}
impl Read for PatternReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        assert!(buffer.len() <= COPY_BUFFER_BYTES);
        let count = (self.length - self.position).min(buffer.len() as u64) as usize;
        for (i, byte) in buffer[..count].iter_mut().enumerate() {
            *byte = pattern(self.position + i as u64);
        }
        self.position += count as u64;
        Ok(count)
    }
}
struct CheckingWriter {
    position: u64,
}
impl Write for CheckingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        assert!(bytes.len() <= COPY_BUFFER_BYTES);
        for (i, byte) in bytes.iter().enumerate() {
            assert_eq!(*byte, pattern(self.position + i as u64));
        }
        self.position += bytes.len() as u64;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn configure(stream: &TcpStream) {
    // Test hang bounds only; the library sets no application speed/deadline limits.
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .unwrap();
}

#[test]
fn loopback_multi_mib_is_byte_exact_with_bounded_streaming() {
    let length = 4 * 1024 * 1024 + 123;
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut socket, peer) = listener.accept().unwrap();
        assert!(peer.ip().is_loopback());
        configure(&socket);
        let mut output = CheckingWriter { position: 0 };
        let receipt = receive(&mut socket, &mut output).unwrap();
        assert_eq!(output.position, length);
        receipt
    });
    let mut socket = TcpStream::connect(address).unwrap();
    configure(&socket);
    let sent = send(
        &mut PatternReader {
            position: 0,
            length,
        },
        &mut socket,
        length,
    )
    .unwrap();
    socket.shutdown(Shutdown::Write).unwrap();
    assert_eq!(server.join().unwrap(), sent);
    assert_eq!(sent.length, length);
}

#[test]
fn loopback_rejects_corruption_and_truncation() {
    for corrupt in [false, true] {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            configure(&socket);
            receive(&mut socket, &mut io::sink()).unwrap_err()
        });
        let mut socket = TcpStream::connect(address).unwrap();
        configure(&socket);
        let mut frame = encode(b"loopback corruption and truncation");
        if corrupt {
            frame[HEADER_BYTES + 2] ^= 1;
        } else {
            frame.truncate(frame.len() - DIGEST_BYTES - 1);
        }
        socket.write_all(&frame).unwrap();
        socket.shutdown(Shutdown::Write).unwrap();
        let error = server.join().unwrap();
        if corrupt {
            assert!(matches!(error, TransferError::HashMismatch { .. }));
        } else {
            is_io(error, Stage::PayloadRead, io::ErrorKind::UnexpectedEof);
        }
    }
}
