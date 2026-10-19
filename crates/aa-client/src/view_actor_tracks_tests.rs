use super::*;
use crate::live_scene::view_actor_init::{Services, initialize};
use aa_numeric::Precision;
use serde_json::{Value, json};

struct Seeds<'a>(&'a Value);
impl Globals for Seeds<'_> {
    fn global_word(&mut self, address: u32) -> Result<u32> {
        let i = VECTOR.iter().position(|a| *a == address).context("42D7B0 read another global")?;
        Ok(self.0[i].as_u64().unwrap() as u32)
    }
}
fn info(words: &Value) -> TrackInfo<u32> {
    let w = |i: usize| words[i].as_u64().unwrap() as u32;
    TrackInfo {
        track_type: w(0) as i32,
        mirrored: w(1) as u8,
        placement_pcg_bits: w(2),
        texture: (w(3) != 0).then_some(w(3)),
        offset_bits: [w(4), w(5)],
        size_bits: [w(6), w(7)],
        fade_sec_bits: w(8),
    }
}
///Only the 451F90 suffix with the base Init and a NULL Thing; the Track
///children are the real ported code.
struct Replay<'a> {
    c: &'a Value,
    words: std::collections::BTreeMap<usize, u32>,
    rng: aa_simulation::NativeRng,
    events: Vec<Value>,
    array: Option<Array<u32>>,
}
impl Services for Replay<'_> {
    type TrackArray = Array<u32>;
    fn base_init(&mut self) -> Result<bool> {
        self.events.push(json!(["457A40", true]));
        Ok(true)
    }
    fn actor_present(&mut self) -> Result<bool> {
        Ok(false)
    }
    fn set_current_owner_color(&mut self) -> Result<()> {
        unreachable!()
    }
    fn position(&self) -> Result<[u32; 3]> {
        let p = &self.c["pose"];
        Ok([0, 1, 2].map(|i| p[i].as_u64().unwrap() as u32))
    }
    fn facing(&self) -> Result<[u32; 3]> {
        let p = &self.c["pose"];
        Ok([3, 4, 5].map(|i| p[i].as_u64().unwrap() as u32))
    }
    fn store_tail(&mut self, o: usize, v: u32) -> Result<()> {
        self.words.insert(o, v);
        Ok(())
    }
    fn source_actor_38(&mut self) -> Result<bool> {
        Ok(true)
    }
    fn track_count_260(&mut self) -> Result<i32> {
        Ok(self.c["count"].as_i64().unwrap() as i32)
    }
    fn construct_track_array(&mut self, count: i32) -> Result<Array<u32>> {
        self.events.push(json!(["6F7810", count as usize * TRACK_SIZE + 4]));
        Array::construct(count, &mut Seeds(&self.c["globals"])).map_err(|p| p.error)
    }
    fn store_track_array(&mut self, array: Array<u32>) -> Result<()> {
        self.array = Some(array);
        Ok(())
    }
    fn current_track_count_148(&self) -> Result<i32> {
        Ok(self.words[&0x148] as i32)
    }
    fn initialize_track(&mut self, index: usize) -> Result<()> {
        let info = info(&self.c["infos"][index]);
        self.array.as_mut().unwrap().initialize(index, &info)
    }
    fn ui_random_word_d4(&mut self) -> Result<u32> {
        Ok(self.rng.next_u32())
    }
}
///The 0x74-byte image; None is a byte native code never writes.
fn image(t: &Track<u32>, view: u32) -> Vec<Option<u8>> {
    let mut out = vec![None; TRACK_SIZE];
    let mut put = |offset: usize, word: u32| {
        for (i, b) in word.to_le_bytes().into_iter().enumerate() {
            out[offset + i] = Some(b);
        }
    };
    put(0, if t.owner() == Owner::ContainingVActor { view } else { 0 });
    put(0x10, t.texture().copied().unwrap_or(0));
    for offset in (4..TRACK_SIZE).step_by(4) {
        if offset == 0x10 {
            continue;
        }
        if let Some(word) = t.word(offset).unwrap() {
            put(offset, word);
        }
    }
    out[8] = Some(t.byte_08());
    out[0x34] = Some(t.byte_34());
    out
}

#[test]
fn actual_451f90_track_constructor_init_and_unwritten_bytes_all_pc() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/development10/view-actor-tracks/native-actor-tracks.json"
    ))
    .unwrap();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 18);
    let mut replayed = 0;
    for row in rows {
        let c = &row["input"];
        let out = &row["output"];
        if c["alloc_null"] == true {
            //Native stores NULL at +144 and 42D830 writes through it; the
            //Rust allocation failure is an explicit error instead.
            assert_eq!(out["fault"], json!(["0x42d834", "0x0", "0x4"]));
            assert_eq!(out["fields"]["0x144"], 0);
            continue;
        }
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let mut s = Replay {
            c,
            words: Default::default(),
            rng: aa_simulation::NativeRng::new(c["seed"].as_u64().unwrap() as u32),
            events: vec![],
            array: None,
        };
        assert!(initialize(&mut s, pc).unwrap());
        assert_eq!(json!(s.events), out["events"], "{}", c["name"]);
        assert_eq!(s.words[&0x148] as u64, out["fields"]["0x148"].as_u64().unwrap());
        assert_eq!(s.words[&0x14c] as u64, out["fields"]["0x14c"].as_u64().unwrap());
        assert_eq!(s.rng.state as u64, out["seed"].as_u64().unwrap());
        let array = s.array.as_ref().unwrap();
        assert_eq!(array.header() as u64, out["header"].as_u64().unwrap());
        let view = out["view"].as_u64().unwrap() as u32;
        let images: Vec<Vec<Option<u8>>> = array.tracks().iter().map(|t| image(t, view)).collect();
        assert_eq!(json!(images), out["tracks"], "{} {pc:?}", c["name"]);
        replayed += 1;
    }
    assert_eq!(replayed, 15);
}

#[test]
fn half_size_rounds_once_at_the_binary32_store() {
    for (input, half) in [(0x3eb33333, 0x3e333333), (0x00000003, 0x00000002), (0x00000001, 0x00000000),
        (0x80000003, 0x80000002), (0x00800000, 0x00400000), (0x7f800000, 0x7f800000), (0x7f800001, 0x7fc00001),
        (0x7fc00001, 0x7fc00001), (0xff800123, 0xffc00123)] {
        assert_eq!(half_9fbdb8(input), half, "{input:08x}");
    }
}

#[test]
fn failed_constructor_keeps_its_prefix_and_never_publishes() {
    struct Once(usize);
    impl Globals for Once {
        fn global_word(&mut self, _: u32) -> Result<u32> {
            self.0 += 1;
            ensure!(self.0 <= 9, "B43CF8 owner withdrawn");
            Ok(7)
        }
    }
    let pending = Array::<u32>::construct(3, &mut Once(0)).unwrap_err();
    assert_eq!(pending.array.tracks().len(), 1);
    assert_eq!(pending.array.header(), 3);
    assert!(Array::<u32>::construct(0, &mut Once(0)).is_err());
    let track = &pending.array.tracks()[0];
    for offset in (0x3c..0x54).step_by(4) {
        assert_eq!(track.word(offset).unwrap(), None);
    }
    assert!(track.word(0).is_err() && track.word(0x10).is_err());
    assert_eq!(track.owner(), Owner::Null);
}
