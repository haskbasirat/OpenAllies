//! Original Lair (slot 25: ctor 574FA0, Load 5751F0, writer 574E90) and City
//! (slot 29: ctor 564160, Load 564CE0, writer 5643F0) components. Oracle:
//! research/phase8-restoration/native-lair-city.json.
use crate::{ActorId, ground_states::ComponentHeader};
use aa_body::F32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LairState {
    pub header: ComponentHeader,
    /// +10/+14 word vector (saved order).
    pub words_10: Vec<u32>,
    pub word_1c: u32,
    /// +20, constructor -1.0.
    pub word_20: F32,
}
impl LairState {
    pub fn new(owner: ActorId) -> Self {
        Self {
            header: ComponentHeader::new(owner),
            words_10: Vec::new(),
            word_1c: 0,
            word_20: F32(0xbf80_0000),
        }
    }
    /// 5751F0 after the Updatable record and version 0.
    pub fn load(&mut self, words: &[u32], word_1c: u32, word_20: u32) {
        self.header.scheduled_head = None;
        self.words_10.extend_from_slice(words);
        self.word_1c = word_1c;
        self.word_20 = F32(word_20);
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CityState {
    pub header: ComponentHeader,
    /// +10/+14 4D0890 append list (head = first loaded).
    pub values: Vec<u32>,
    /// +18/+1C actual candidate list; never mirrors native pointers as words.
    pub candidates: crate::city_candidates::State,
    /// +20 low byte (the constructor writes only this byte).
    pub byte_20: u8,
}
#[derive(Debug, thiserror::Error)]
#[error("Map Load Canceled: City no longer has a city zone (564D58 -> 5A0910)")]
pub struct CityZoneCanceled;
impl CityState {
    pub fn new(owner: ActorId) -> Self {
        Self {
            header: ComponentHeader::new(owner),
            values: Vec::new(),
            candidates: crate::city_candidates::State::constructed(),
            byte_20: 0,
        }
    }
    /// 564CE0 after the Updatable record: version 1 appends the saved words
    /// (4D1AF0), then 45D6A0 requires ActorData+5B4 zone_radius > 0 or the
    /// whole map load is canceled. Version 0 reads and checks nothing.
    pub fn load(&mut self, words: Option<&[u32]>, zone_radius_bits: u32) -> Result<(), CityZoneCanceled> {
        self.header.scheduled_head = None;
        let Some(words) = words else {
            return Ok(());
        };
        self.values.extend_from_slice(words);
        let nan = zone_radius_bits & 0x7f80_0000 == 0x7f80_0000 && zone_radius_bits & 0x7f_ffff != 0;
        let positive = !nan && zone_radius_bits >> 31 == 0 && zone_radius_bits & 0x7fff_ffff != 0;
        if positive { Ok(()) } else { Err(CityZoneCanceled) }
    }
}
