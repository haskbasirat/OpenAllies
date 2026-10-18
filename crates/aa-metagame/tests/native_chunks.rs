//! Replays research/phase8-metagame/native-chunks.json: the original
//! chunk-file container ("TGCK") writer (859690 / 859150 / 858E40 / 859330)
//! and reader (859380 / 859030 / 858C90) under Unicorn, against
//! `aa_metagame::chunk_file`. Written files must match byte for byte except
//! the directory flag bits 1..63, which the original leaves as uninitialised
//! stack memory (859090 / 8582B0); every native read, error message and
//! "not found" result must be reproduced.
use aa_metagame::chunk_file::{ChunkFileReader, ChunkFileWriter};
use aa_metagame::Error;
use serde_json::Value;

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../research/phase8-metagame/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

/// The written file with the directory flag bits 1..63 cleared.
fn mask_flags(bytes: &[u8]) -> Vec<u8> {
    let mut b = bytes.to_vec();
    let count = u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
    let start = b.len() - count * 0x60;
    for i in 0..count {
        let o = start + i * 0x60 + 0x50;
        let flags = u64::from_le_bytes(b[o..o + 8].try_into().unwrap()) & 1;
        b[o..o + 8].copy_from_slice(&flags.to_le_bytes());
    }
    b
}

/// Replays one native read script; returns the number of checked steps.
fn replay_reads(path: &str, bytes: &[u8], requests: &Value, native: &Value, what: &str) -> usize {
    let mut steps = 0;
    let opened = ChunkFileReader::open(path, bytes);
    match native["open"].as_str().unwrap() {
        "ok" => {}
        "throw" => {
            match opened {
                Err(Error::NativeData(m)) => assert_eq!(m, native["message"].as_str().unwrap(), "{what}"),
                other => panic!("{what}: expected {:?}, got {other:?}", native["message"]),
            }
            return 1;
        }
        "eof" => {
            assert!(matches!(opened, Err(Error::Unsupported(_))), "{what}: engine file end");
            return 1;
        }
        other => panic!("{what}: {other}"),
    }
    let mut r = opened.unwrap_or_else(|e| panic!("{what}: {e}"));
    for (req, got) in requests.as_array().unwrap().iter().zip(native["requests"].as_array().unwrap()) {
        let name = req[0].as_str().unwrap();
        let required = req[1].as_u64().unwrap() != 0;
        let res = r.open_chunk(name, required);
        if let Some(kind) = got.get("result") {
            assert_eq!(kind, "throw", "{what} {name}");
            match res {
                Err(Error::NativeData(m)) => assert_eq!(m, got["message"].as_str().unwrap(), "{what} {name}"),
                other => panic!("{what} {name}: {other:?}"),
            }
            steps += 1;
            continue;
        }
        let found = res.unwrap_or_else(|e| panic!("{what} {name}: {e}"));
        assert_eq!(u64::from(found), got["found"].as_u64().unwrap(), "{what} {name} found");
        for rd in got["reads"].as_array().unwrap() {
            let n = rd["size"].as_u64().unwrap() as usize;
            let res = r.read(n);
            match rd["result"].as_str().unwrap() {
                "ok" => assert_eq!(res.unwrap(), hex(rd["bytes"].as_str().unwrap()).as_slice(), "{what} {name} read"),
                "throw" => match res {
                    Err(Error::NativeData(m)) => assert_eq!(m, rd["message"].as_str().unwrap(), "{what} {name} read"),
                    other => panic!("{what} {name}: {other:?}"),
                },
                other => panic!("{what}: {other}"),
            }
            steps += 1;
        }
        steps += 1;
    }
    steps
}

#[test]
fn chunk_file_matches_native_tgck() {
    let fx: Value = serde_json::from_slice(&read("native-chunks.json")).unwrap();
    let mut steps = 0;
    for f in fx["files"].as_array().unwrap() {
        let what = format!("file {}", f["case"]);
        let path = f["path"].as_str().unwrap();
        let native = hex(f["bytes"].as_str().unwrap());
        // writer
        let mut w = ChunkFileWriter::create(path);
        for ch in f["chunks"].as_array().unwrap() {
            w.open_chunk(ch[0].as_str().unwrap(), false).unwrap();
            for piece in ch[1].as_array().unwrap() {
                w.write(&hex(piece.as_str().unwrap())).unwrap();
            }
        }
        let bytes = w.close().unwrap();
        assert!(bytes == mask_flags(&native), "{what}: writer bytes differ");
        assert_eq!(ChunkFileWriter::create(path).close().unwrap().len(), 12);
        // reader on the native bytes
        let r = ChunkFileReader::open(path, &native).unwrap();
        let dir = &f["directory"];
        assert_eq!(r.version, dir["version"].as_u64().unwrap() as u32);
        for (c, d) in r.chunks.iter().zip(dir["records"].as_array().unwrap()) {
            assert_eq!(c.c_name(), d["name"].as_str().unwrap());
            assert_eq!(c.offset, d["offset"].as_u64().unwrap());
            assert_eq!(c.size, d["size"].as_u64().unwrap());
            assert_eq!(c.flags, d["flags"].as_u64().unwrap(), "{what}: flags as read");
            assert!(!c.compressed());
        }
        steps += replay_reads(path, &native, &f["read_requests"], &f["read"], &what);
    }
    for c in fx["crafted"].as_array().unwrap() {
        let what = format!("crafted {}", c["case"]);
        let bytes = hex(c["bytes"].as_str().unwrap());
        steps += replay_reads(c["path"].as_str().unwrap(), &bytes, &c["read_requests"], &c["read"], &what);
    }
    for e in fx["write_errors"].as_array().unwrap() {
        let mut w = ChunkFileWriter::create("x.sav");
        match w.open_chunk("", false) {
            Err(Error::NativeData(m)) => assert_eq!(m, e["result"]["message"].as_str().unwrap()),
            other => panic!("{other:?}"),
        }
        steps += 1;
    }
    let mut w = ChunkFileWriter::create("x.sav");
    assert!(matches!(w.write(b"x"), Err(Error::NativeData(_))));
    //The additive compressed backend is qualified separately by native
    //inflate interoperability. All original uncompressed rows above remain
    //byte-identical to their original writer fixtures.
    w.open_chunk("METAGAME", true).unwrap();
    assert!(steps > 200, "{steps} steps");
}
