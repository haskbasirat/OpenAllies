//! Replays research/phase8-net/native-file-transfer.json: the original
//! TGC_FileSender / TGC_FileReceiver (0x7611B0 .. 0x761DB0), the AdminConn
//! generation check 0x74C8B0 and, in the channel scenarios, the original
//! BlockSender 0x760A80 / 0x7608E0 and BlockReceiver 0x760F20 / 0x75FEA0
//! carrying file blocks, executed under Unicorn on the original AA.exe and on
//! the accepted v14 AA.exe. Every step compares results, wire bytes, reader
//! positions, sender/receiver/channel state, TGC_File call sequences and log
//! lines; every scenario compares the files written on the receiving side.
use aa_netcode::channel::{BlockSender, BlockSource, BlockSink};
use aa_netcode::file::*;
use aa_orders::commands::WireWriter;
use aa_orders::{Cursor, Position};
use serde_json::{Value, json};
use std::sync::OnceLock;

fn fixture() -> &'static Value {
    static F: OnceLock<Value> = OnceLock::new();
    F.get_or_init(|| {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-file-transfer.json");
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    })
}
fn fnv(b: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for &x in b {
        h = (h ^ u64::from(x)).wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}
fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}
fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
fn units(v: &Value) -> Vec<u16> {
    v.as_array().unwrap().iter().map(|u| u.as_u64().unwrap() as u16).collect()
}
fn int(v: &Value) -> i32 {
    v.as_i64().unwrap() as i32
}
fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
fn config(v: &Value, tail: TerminalTail) -> FileTransferConfig {
    FileTransferConfig {
        blocks_per_frame: int(&v["blocks_per_frame"]),
        block_size: int(&v["block_size"]),
        max_pending_blocks: int(&v["max_pending_blocks"]),
        tail,
    }
}
fn position(v: &Value) -> Position {
    Position { byte: v["byte"].as_u64().unwrap() as usize, bit: v["bit"].as_u64().unwrap() as u8 }
}
fn written(w: &WireWriter) -> Vec<u8> {
    let bits = w.position().absolute().unwrap();
    w.bytes()[..bits.div_ceil(8)].to_vec()
}
fn queue_json(q: &BlockWindow) -> Value {
    let slots: Vec<Value> = q.slots().map(|(i, b)| json!([i, b.len(), fnv(b)])).collect();
    json!({"begin": q.begin(), "end": q.end(), "slots": slots})
}
fn call_json(role: &str, c: &FileCall) -> Value {
    match c {
        FileCall::Open { path, mode } => json!({"file": role, "op": "open", "path": path, "mode": mode}),
        FileCall::IsOpen { result } => json!({"file": role, "op": "is_open", "result": result}),
        FileCall::Read { len, returned } => json!({"file": role, "op": "read", "len": len, "returned": returned}),
        FileCall::Write { bytes } => json!({"file": role, "op": "write", "bytes": to_hex(bytes)}),
        FileCall::Close => json!({"file": role, "op": "close"}),
        FileCall::Size { result } => json!({"file": role, "op": "size", "result": result}),
    }
}
fn log_json(l: &FileLog) -> Value {
    match l {
        FileLog::DotDotRejected { name } => json!({"line": 0xbf, "name": name}),
        FileLog::StartingDownload { name, size } => {
            json!({"line": 0xac, "component": utf16(RECEIVER_NAME), "name": name, "size": size})
        }
        FileLog::OpenFailed { name } => json!({"line": 0xb9, "name": name}),
    }
}

struct Run {
    unit: bool,
    sender: FileSender<MemoryFile>,
    channel: BlockSender,
    receiver: FileReceiver<MemoryFile>,
    sender_generation: i32,
    receiver_generation: i32,
}

fn replay(image: &str, sc: &Value) -> usize {
    let name = sc["name"].as_str().unwrap();
    let tail = |img: &str| if img == "original" { TerminalTail::Original } else { TerminalTail::V14 };
    // Interop scenarios run one side's code from each image.
    let (stail, rtail) = match (sc["sender_image"].as_str(), sc["receiver_image"].as_str()) {
        (Some(a), Some(b)) => (tail(a), tail(b)),
        _ => (tail(image), tail(image)),
    };
    let scfg = config(&sc["sender_config"], stail);
    let rcfg = config(&sc["receiver_config"], rtail);
    let profile = units(&sc["profile_dir"]);
    let mut source = MemoryFile::with_files(sc["sources"].as_array().unwrap().iter().map(|s| {
        ([profile.clone(), units(&s[0])].concat(), hex(s[1].as_str().unwrap()))
    }));
    for r in sc["reported_sizes"].as_array().unwrap() {
        source.reported_size.insert([profile.clone(), units(&r[0])].concat(), int(&r[1]));
    }
    let mut run = Run {
        unit: sc["kind"] == "unit",
        sender: FileSender::new(scfg, profile.clone(), source),
        channel: FileSender::<MemoryFile>::new_channel(&scfg),
        receiver: FileReceiver::new(rcfg, profile.clone(), MemoryFile::default()).unwrap(),
        sender_generation: 0,
        receiver_generation: 0,
    };
    let steps = sc["steps"].as_array().unwrap();
    for (n, step) in steps.iter().enumerate() {
        let ctx = format!("{image}/{name} step {n} {}", step["op"]);
        let mut generation_log: Vec<Value> = Vec::new();
        match step["op"].as_str().unwrap() {
            "construct" => {
                // 0x7611B0 leaves +0x48 unset (0xCDCDCDCD sentinel survives).
                assert_eq!(int(&step["sender_size_after_ctor"]), 0xCDCDCDCDu32 as i32, "{ctx}");
                run.sender.reset(&mut run.channel);
                run.receiver.reset().unwrap();
                run.sender_generation = 0;
                run.receiver_generation = 0;
            }
            "start" => {
                start_file_transfer(&mut run.sender_generation, &mut run.channel, &mut run.sender, &units(&step["name"]))
                    .unwrap();
            }
            "sender_reset" => run.sender.reset(&mut run.channel),
            "receiver_reset" => run.receiver.reset().unwrap(),
            "available" => assert_eq!(run.sender.available().unwrap(), int(&step["result"]), "{ctx}"),
            "progress" => assert_eq!(run.sender.progress().unwrap(), int(&step["result"]), "{ctx}"),
            "estimate" => {
                assert_eq!(run.sender.estimate(int(&step["index"])).unwrap(), int(&step["result"]), "{ctx}")
            }
            "release" => run.sender.release(int(&step["index"])).unwrap(),
            "serialize" => {
                let mut w = WireWriter::new(Position::PACKET_START).unwrap();
                run.sender.serialize(&mut w, int(&step["index"])).unwrap();
                assert_eq!(to_hex(&written(&w)), step["bytes"].as_str().unwrap(), "{ctx}");
                assert_eq!(w.position(), position(&step["end"]), "{ctx}");
            }
            op @ ("receive" | "skip") => {
                let bytes = hex(step["bytes"].as_str().unwrap());
                let mut r = Cursor::with_bounds(&bytes, Position::PACKET_START, bytes.len() * 8).unwrap();
                let index = int(&step["index"]);
                if op == "receive" {
                    run.receiver.receive(&mut r, index).unwrap();
                } else {
                    run.receiver.skip(&mut r, index).unwrap();
                }
                assert_eq!(r.position(), position(&step["end"]), "{ctx}");
            }
            "section" => {
                let mut w = WireWriter::new(Position::PACKET_START).unwrap();
                w.write_uint(run.sender_generation as u32, 32).unwrap();
                run.channel
                    .serialize(&mut run.sender, &mut w, int(&step["sequence"]), int(&step["limit"]))
                    .unwrap();
                assert_eq!(to_hex(&written(&w)), step["bytes"].as_str().unwrap(), "{ctx}");
                assert_eq!(w.position(), position(&step["end"]), "{ctx}");
            }
            "deliver" => {
                let bytes = hex(step["bytes"].as_str().unwrap());
                let mut r = Cursor::with_bounds(&bytes, Position::PACKET_START, bytes.len() * 8).unwrap();
                let channel = run.receiver.channel;
                let outcome =
                    receive_file_section(&mut run.receiver_generation, &channel, &mut run.receiver, &mut r).unwrap();
                match outcome {
                    Generation::Old { packet, mine } => {
                        generation_log.push(json!({"line": 0x168c, "packet": packet, "mine": mine}))
                    }
                    Generation::Newer { packet, mine } => {
                        generation_log.push(json!({"line": 0x1694, "packet": packet, "mine": mine}))
                    }
                    Generation::Current => {}
                }
                assert_eq!(r.position(), position(&step["end"]), "{ctx}");
            }
            "packet_result" => run
                .channel
                .packet_result(&mut run.sender, int(&step["sequence"]), step["acked"].as_bool().unwrap())
                .unwrap(),
            other => panic!("unknown op {other}"),
        }
        if run.unit {
            run.receiver_generation = run.sender_generation;
        }
        // Sender, its BlockSender base and the AdminConn generation.
        let s = &step["sender"];
        let q = queue_json(&run.sender.queue);
        assert_eq!(q["begin"], s["queue"]["begin"], "{ctx} sender queue begin");
        assert_eq!(q["end"], s["queue"]["end"], "{ctx} sender queue end");
        assert_eq!(q["slots"], s["queue"]["slots"], "{ctx} sender queue slots");
        assert_eq!(run.sender.size, int(&s["size"]), "{ctx} sender size");
        assert_eq!(run.sender_generation, int(&s["generation"]), "{ctx} sender generation");
        assert_eq!(run.channel.span, int(&s["span"]), "{ctx}");
        assert_eq!(run.channel.copies, int(&s["copies"]), "{ctx}");
        let packets: Vec<Value> = run
            .channel
            .packets
            .items
            .iter()
            .map(|p| json!([p.state, p.first, p.end, p.mask]))
            .collect();
        assert_eq!(run.channel.packets.begin, int(&s["packets"]["begin"]), "{ctx} packets begin");
        assert_eq!(run.channel.packets.end(), int(&s["packets"]["end"]), "{ctx} packets end");
        assert_eq!(Value::from(packets), s["packets"]["records"], "{ctx} packet records");
        let blocks: Vec<Value> =
            run.channel.blocks.items.iter().map(|b| json!([b.inflight, b.estimate])).collect();
        assert_eq!(run.channel.blocks.begin, int(&s["blocks"]["begin"]), "{ctx} blocks begin");
        assert_eq!(run.channel.blocks.end(), int(&s["blocks"]["end"]), "{ctx} blocks end");
        assert_eq!(Value::from(blocks), s["blocks"]["records"], "{ctx} block records");
        // Receiver.
        let r = &step["receiver"];
        let q = queue_json(&run.receiver.queue);
        assert_eq!(q["begin"], r["queue"]["begin"], "{ctx} receiver queue begin");
        assert_eq!(q["end"], r["queue"]["end"], "{ctx} receiver queue end");
        assert_eq!(q["slots"], r["queue"]["slots"], "{ctx} receiver queue slots");
        assert_eq!(run.receiver.channel.span, int(&r["span"]), "{ctx}");
        assert_eq!(run.receiver.size, int(&r["size"]), "{ctx} receiver size");
        assert_eq!(run.receiver_generation, int(&r["generation"]), "{ctx} receiver generation");
        // TGC_File call sequences, per object.
        for (role, calls) in [("sender", &mut run.sender.file.calls), ("receiver", &mut run.receiver.file.calls)] {
            let got: Vec<Value> = calls.drain(..).map(|c| call_json(role, &c)).collect();
            let want: Vec<Value> =
                step["calls"].as_array().unwrap().iter().filter(|c| c["file"] == role).cloned().collect();
            assert_eq!(got, want, "{ctx} {role} file calls");
        }
        // Log lines in emission order.
        generation_log.extend(run.receiver.log.drain(..).map(|l| log_json(&l)));
        assert_eq!(Value::from(generation_log), step["log"], "{ctx} log");
    }
    // Files on the receiving machine.
    let disk: Vec<Value> = run
        .receiver
        .file
        .disk
        .iter()
        .map(|(path, bytes)| json!([path, to_hex(bytes), run.receiver.file.open_path() == Some(path.as_slice())]))
        .collect();
    assert_eq!(Value::from(disk), sc["disk"], "{image}/{name} receiving disk");
    steps.len()
}

fn replay_image(image: &str) -> (usize, usize) {
    let scenarios = fixture()["images"][image].as_array().unwrap();
    let steps = scenarios.iter().map(|sc| replay(image, sc)).sum();
    (scenarios.len(), steps)
}

#[test]
fn native_file_transfer_original_image() {
    let (scenarios, steps) = replay_image("original");
    assert!(scenarios >= 35 && steps > 1000, "{scenarios} scenarios, {steps} steps");
}

#[test]
fn native_file_transfer_v14_image() {
    let (scenarios, steps) = replay_image("v14");
    assert!(scenarios >= 34 && steps > 1000, "{scenarios} scenarios, {steps} steps");
}

/// Original host with a v14 client and the reverse: the exact-multiple repair
/// needs both ends.
#[test]
fn native_file_transfer_interop_images() {
    let (scenarios, steps) = replay_image("interop");
    assert!(scenarios == 4 && steps > 50, "{scenarios} scenarios, {steps} steps");
    assert_eq!(received("interop", "interop_original_sender_v14_receiver_size_128"), [(64, false)]);
    assert_eq!(received("interop", "interop_v14_sender_original_receiver_size_128"), [(64, false)]);
    assert_eq!(received("interop", "interop_v14_sender_original_receiver_size_197"), [(197, false)]);
}

fn scenario(image: &str, name: &str) -> &'static Value {
    fixture()["images"][image].as_array().unwrap().iter().find(|s| s["name"] == name).unwrap()
}
fn received(image: &str, name: &str) -> Vec<(usize, bool)> {
    scenario(image, name)["disk"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| (d[1].as_str().unwrap().len() / 2, d[2].as_bool().unwrap()))
        .collect()
}

/// The hazards the oracle demonstrates, stated explicitly.
#[test]
fn native_hazards_in_the_fixture() {
    let f = fixture();
    // The only executed bytes that differ between the images: the two
    // terminal-length sites and the v14 caves.
    for d in f["differing_windows"].as_array().unwrap() {
        let start = u32::from_str_radix(d[1].as_str().unwrap().trim_start_matches("0x"), 16).unwrap();
        assert!([0x761220, 0x761420, 0x9eff80, 0x9effaa].contains(&start), "{d}");
    }
    // Exact multiples lose their last block in the original, not in v14.
    assert_eq!(received("original", "size_64"), [(0, false)]);
    assert_eq!(received("original", "size_128"), [(64, false)]);
    assert_eq!(received("original", "size_192"), [(128, false)]);
    assert_eq!(received("original", "block_size_7_size_49_exact"), [(42, false)]);
    assert_eq!(received("original", "channel_exact_multiple_256"), [(192, false)]);
    assert_eq!(received("v14", "size_128"), [(128, false)]);
    assert_eq!(received("v14", "channel_exact_multiple_256"), [(256, false)]);
    // ... while the original still reports full progress.
    let steps = scenario("original", "channel_exact_multiple_256")["steps"].as_array().unwrap();
    assert_eq!(steps.last().unwrap()["result"], 256);
    // Size 0: header only, the received file is never closed (both images).
    assert_eq!(received("original", "size_0"), [(0, true)]);
    assert_eq!(received("v14", "size_0"), [(0, true)]);
    // ".." anywhere before the first NUL rejects; after an embedded NUL it is
    // accepted and the path keeps the NUL (CreateFileW would stop there).
    assert!(received("original", "name_dotdot_inside_name").is_empty());
    assert_eq!(received("original", "name_dotdot_after_embedded_nul"), [(100, false)]);
    // Names of 65536+ units are cut to (len & 0xFFFF) units on the wire.
    let wrap = scenario("original", "name_length_wraps_16_bits");
    let log = wrap["steps"].as_array().unwrap().iter().flat_map(|s| s["log"].as_array().unwrap()).next().unwrap();
    assert_eq!(log["name"].as_array().unwrap().len(), (5 + 65531 + 4) & 0xFFFF);
    // MaxPendingBlocks 0: only the header is ever offered.
    assert_eq!(received("original", "max_pending_0_size_200"), [(0, true)]);
}

fn memory_pair(cfg: FileTransferConfig, data: &[u8]) -> (FileSender<MemoryFile>, BlockSender, FileReceiver<MemoryFile>) {
    let profile = utf16("P\\");
    let source = MemoryFile::with_files([([profile.clone(), utf16("f.bin")].concat(), data.to_vec())]);
    let mut sender = FileSender::new(cfg, profile.clone(), source);
    let mut channel = FileSender::<MemoryFile>::new_channel(&cfg);
    sender.reset(&mut channel);
    let receiver = FileReceiver::new(cfg, profile, MemoryFile::default()).unwrap();
    (sender, channel, receiver)
}

/// Native crash / undefined-read paths are explicit errors (not oracle cases).
#[test]
fn explicit_errors_instead_of_native_faults() {
    // Block size 0: idiv fault.
    let cfg = FileTransferConfig { block_size: 0, ..FileTransferConfig::V14 };
    let (mut s, mut ch, _) = memory_pair(cfg, &[1, 2, 3]);
    s.start(&mut ch, &utf16("f.bin")).unwrap();
    assert!(s.available().is_err());
    assert!(s.progress().is_err());
    // Missing source: the TGC_FileException propagates out of start after the
    // queue was restarted at 0; size and header unchanged.
    let (mut s, mut ch, _) = memory_pair(FileTransferConfig::V14, &[1, 2, 3]);
    assert!(s.start(&mut ch, &utf16("missing.bin")).is_err());
    assert_eq!((s.queue.begin(), s.queue.end(), s.size), (0, 0, -1));
    // Serializing a slot without a buffer (native null deref).
    let (mut s, _, _) = memory_pair(FileTransferConfig::V14, &[1, 2, 3]);
    let mut w = WireWriter::new(Position::PACKET_START).unwrap();
    assert!(s.serialize(&mut w, 5).is_err());
    // Size of an unopened file (null impl).
    assert!(MemoryFile::default().size().is_err());
    // Receiver write longer than the received payload (heap over-read).
    let (mut s, mut ch, _) = memory_pair(FileTransferConfig::V14, &[7; 200]);
    s.start(&mut ch, &utf16("f.bin")).unwrap();
    let big = FileTransferConfig { block_size: 128, ..FileTransferConfig::V14 };
    let mut r = FileReceiver::new(big, utf16("Q\\"), MemoryFile::default()).unwrap();
    let mut header = WireWriter::new(Position::PACKET_START).unwrap();
    s.serialize(&mut header, 0).unwrap();
    let mut block = WireWriter::new(Position::PACKET_START).unwrap();
    s.available().unwrap();
    s.serialize(&mut block, 1).unwrap();
    for (i, w) in [(0, &header), (1, &block)] {
        let bytes = written(w);
        let mut c = Cursor::with_bounds(&bytes, Position::PACKET_START, bytes.len() * 8).unwrap();
        let res = r.receive(&mut c, i);
        assert_eq!(res.is_err(), i == 1);
    }
    // Index 2^31-1 would wrap the window end.
    let mut c = Cursor::with_bounds(&[0u8; 8], Position::PACKET_START, 64).unwrap();
    let (_, _, mut r) = memory_pair(FileTransferConfig::V14, &[]);
    assert!(r.receive(&mut c, i32::MAX).is_err());
    // Backward release.
    let (mut s, mut ch, _) = memory_pair(FileTransferConfig::V14, &[1; 100]);
    s.start(&mut ch, &utf16("f.bin")).unwrap();
    s.release(1).unwrap();
    assert!(s.release(0).is_err());
    // Hostile payload length: rejected before allocating.
    let mut w = WireWriter::new(Position::PACKET_START).unwrap();
    w.write_uint(0xFFFF_FFF0, 32).unwrap();
    let bytes = written(&w);
    let mut c = Cursor::with_bounds(&bytes, Position::PACKET_START, bytes.len() * 8).unwrap();
    assert!(decode_payload(&mut c).is_err());
    // Header block shorter than the pad field.
    assert!(decode_header(&[]).is_err());
}

/// Receiver open failure: Catch@0x761BCD logs and continues; data blocks are
/// skipped because the file is not open (pseudocode + machine code only).
#[test]
fn receiver_open_failure_is_logged_and_transfer_continues() {
    struct Refuse(MemoryFile);
    impl FileStore for Refuse {
        fn open(&mut self, path: &[u16], mode: u32) -> aa_netcode::Result<()> {
            self.0.calls.push(FileCall::Open { path: path.to_vec(), mode });
            Err(aa_netcode::Error::Unsupported("refused"))
        }
        fn is_open(&mut self) -> bool {
            self.0.is_open()
        }
        fn read(&mut self, buf: &mut [u8]) -> aa_netcode::Result<usize> {
            self.0.read(buf)
        }
        fn write(&mut self, data: &[u8]) -> aa_netcode::Result<()> {
            self.0.write(data)
        }
        fn size(&mut self) -> aa_netcode::Result<i32> {
            self.0.size()
        }
        fn close(&mut self) -> aa_netcode::Result<()> {
            self.0.close()
        }
    }
    let (mut s, mut ch, _) = memory_pair(FileTransferConfig::V14, &[9; 70]);
    s.start(&mut ch, &utf16("f.bin")).unwrap();
    let n = s.available().unwrap();
    let mut r = FileReceiver::new(FileTransferConfig::V14, utf16("Q\\"), Refuse(MemoryFile::default())).unwrap();
    for i in 0..n {
        let mut w = WireWriter::new(Position::PACKET_START).unwrap();
        s.serialize(&mut w, i).unwrap();
        let bytes = written(&w);
        let mut c = Cursor::with_bounds(&bytes, Position::PACKET_START, bytes.len() * 8).unwrap();
        r.receive(&mut c, i).unwrap();
    }
    assert_eq!(
        r.log,
        [
            FileLog::StartingDownload { name: utf16("f.bin"), size: 70 },
            FileLog::OpenFailed { name: utf16("f.bin") }
        ]
    );
    assert_eq!(r.queue.begin(), n);
    assert!(r.file.0.disk.is_empty());
}

/// StdFile adapter: one transfer between two temporary directories through
/// the channel layer (BlockSender section -> BlockReceiver), v14 rule.
#[test]
fn std_file_round_trip_through_channel_sections() {
    let base = std::env::temp_dir().join(format!("aa-netcode-file-{}", std::process::id()));
    let (src, dst) = (base.join("host"), base.join("client"));
    std::fs::create_dir_all(src.join("maps")).unwrap();
    let data: Vec<u8> = (0..1024u32).map(|i| (i * 7 + 3) as u8).collect();
    std::fs::write(src.join("maps").join("x.tgm"), &data).unwrap();
    let dir = |p: &std::path::Path| utf16(&format!("{}{}", p.display(), std::path::MAIN_SEPARATOR));
    let cfg = FileTransferConfig::V14;
    let mut sender = FileSender::new(cfg, dir(&src), StdFile::default());
    let mut channel = FileSender::<StdFile>::new_channel(&cfg);
    sender.reset(&mut channel);
    let mut receiver = FileReceiver::new(cfg, dir(&dst), StdFile::default()).unwrap();
    let (mut host_gen, mut client_gen) = (0, 0);
    let name = utf16(&format!("maps{}x.tgm", std::path::MAIN_SEPARATOR));
    start_file_transfer(&mut host_gen, &mut channel, &mut sender, &name).unwrap();
    for seq in 0..40 {
        let mut w = WireWriter::new(Position::PACKET_START).unwrap();
        w.write_uint(host_gen as u32, 32).unwrap();
        channel.serialize(&mut sender, &mut w, seq, 0x2403).unwrap();
        let bytes = written(&w);
        let mut c = Cursor::with_bounds(&bytes, Position::PACKET_START, bytes.len() * 8).unwrap();
        let ch = receiver.channel;
        receive_file_section(&mut client_gen, &ch, &mut receiver, &mut c).unwrap();
        channel.packet_result(&mut sender, seq, true).unwrap();
    }
    assert_eq!(sender.progress().unwrap(), 1024);
    assert!(!receiver.file.is_open());
    assert_eq!(std::fs::read(dst.join("maps").join("x.tgm")).unwrap(), data);
    std::fs::remove_dir_all(&base).unwrap();
}
