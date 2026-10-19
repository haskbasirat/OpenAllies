//!VActor+144/+148 Track array built by VActor Init 451F90 (45207B..452103,
//!original and v14 bytes equal):
//!
//!* +148 = 45E700 count (stored BEFORE the allocation);
//!* 6F7810(count*0x74+4), leading dword = count, then
//!  eh_vector_constructor_iterator(arr+4, 0x74, count, 42D7B0, 4B2830)
//!  (4B2830 is an empty RET destructor);
//!* +144 = arr+4, or NULL when 6F7810 returned NULL;
//!* for i < +148 (re-read each pass): 42D830(this = +144 + i*0x74,
//!  owner = this VActor, info = 45E710(i) = ActorData+25C + i*0x24).
//!
//!42D7B0 zeroes +00, +04, byte +08, +0C..+30 and byte +34, copies the B43CF8
//!process vector to +54 and again to +60 (reading B43CF8/CFC/D00 twice), then
//!+6C = +38 = -1.0f and +70 = 0. 42D830 copies the 0x24-byte TrackInfo and
//!halves its size (9FBDB8 = 0.5f). Bytes +09..+0B, +35..+37 and +3C..+53 are
//!never written by either and have no Rust value.
//!Proof: research/development10/view-actor-tracks/native-actor-tracks.json.
//!The native 32-bit allocation size (count*0x74+4) is not reproduced: the
//!array is a growable Rust allocation (64-bit, no address-space ceiling).
use aa_runtime::data_pre_request::actor_view_source::track_info::TrackInfo;
use anyhow::{Context, Result, bail, ensure};

pub(crate) const TRACK_SIZE: usize = 0x74;
///The 42D7B0 read order of the B43CF8 process vector.
const VECTOR: [u32; 3] = [0xb43cf8, 0xb43cfc, 0xb43d00];
const MINUS_ONE: u32 = 0xbf80_0000;

///The current process globals 42D7B0 reads.
pub(crate) trait Globals {
    fn global_word(&mut self, address: u32) -> Result<u32>;
}

///The retained process seed owner, bound for one 451F90 iterator call; each
///word read revalidates the catalog it was captured for.
pub(crate) struct SeedGlobals<'b, 'a, 'assets>(pub(crate) &'b mut aa_runtime::view_grid_startup::Bound<'a, 'assets>);
impl Globals for SeedGlobals<'_, '_, '_> {
    fn global_word(&mut self, address: u32) -> Result<u32> {
        aa_runtime::view_grid_state::Globals::word(self.0, address).map_err(anyhow::Error::msg)
    }
}

///Track+00: 42D7B0 stores NULL; 42D830 stores the VActor that owns this
///array (451F90 passes its own `this`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Owner {
    Null,
    ContainingVActor,
}

///One 0x74-byte Track; the texture type is the +10 resource pointer.
#[derive(Clone, Debug)]
pub(crate) struct Track<T> {
    owner: Owner,
    track_type: u32,
    mirrored: u8,
    placement_pcg: u32,
    texture: Option<T>,
    offset: [u32; 2],
    word_1c: u32,
    size: [u32; 2],
    half_size: [u32; 2],
    fade_sec: u32,
    active: u8,
    word_38: u32,
    point_54: [u32; 3],
    point_60: [u32; 3],
    word_6c: u32,
    word_70: u32,
}
impl<T: Clone> Track<T> {
    ///42D7B0 (this in eax). Its only reads are the six B43CF8 words.
    pub(crate) fn construct(g: &mut impl Globals) -> Result<Self> {
        let mut point_54 = [0; 3];
        for (i, address) in VECTOR.into_iter().enumerate() {
            point_54[i] = g.global_word(address)?;
        }
        let mut point_60 = [0; 3];
        for (i, address) in VECTOR.into_iter().enumerate() {
            point_60[i] = g.global_word(address)?;
        }
        Ok(Self {
            owner: Owner::Null,
            track_type: 0,
            mirrored: 0,
            placement_pcg: 0,
            texture: None,
            offset: [0; 2],
            word_1c: 0,
            size: [0; 2],
            half_size: [0; 2],
            fade_sec: 0,
            active: 0,
            word_38: MINUS_ONE,
            point_54,
            point_60,
            word_6c: MINUS_ONE,
            word_70: 0,
        })
    }
    ///42D830(this, owner, info): plain copies, +1C = 0, the two halves and
    ///byte +34 = 1. +00..+30 come from the ActorData+25C entry verbatim.
    pub(crate) fn initialize(&mut self, info: &TrackInfo<T>) {
        self.owner = Owner::ContainingVActor;
        self.track_type = info.track_type as u32;
        self.mirrored = info.mirrored;
        self.placement_pcg = info.placement_pcg_bits;
        self.texture = info.texture.clone();
        self.size = info.size_bits;
        self.fade_sec = info.fade_sec_bits;
        self.offset = info.offset_bits;
        self.word_1c = 0;
        self.half_size = [half_9fbdb8(self.size[0]), half_9fbdb8(self.size[1])];
        self.active = 1;
    }
}
impl<T> Track<T> {
    pub(crate) fn owner(&self) -> Owner {
        self.owner
    }
    pub(crate) fn texture(&self) -> Option<&T> {
        self.texture.as_ref()
    }
    ///A dword of the 0x74 bytes. Pointer slots +00/+10 are typed above;
    ///None is a dword holding bytes native code never writes.
    pub(crate) fn word(&self, offset: usize) -> Result<Option<u32>> {
        ensure!(offset < TRACK_SIZE && offset % 4 == 0, "Track dword outside 0x74 bytes");
        Ok(match offset {
            0x00 | 0x10 => bail!("Track+{offset:02X} is a pointer slot, not a scalar word"),
            0x04 => Some(self.track_type),
            0x08 | 0x34 => None,
            0x0c => Some(self.placement_pcg),
            0x14 => Some(self.offset[0]),
            0x18 => Some(self.offset[1]),
            0x1c => Some(self.word_1c),
            0x20 => Some(self.size[0]),
            0x24 => Some(self.size[1]),
            0x28 => Some(self.half_size[0]),
            0x2c => Some(self.half_size[1]),
            0x30 => Some(self.fade_sec),
            0x38 => Some(self.word_38),
            0x3c..=0x50 => None,
            0x54..=0x5c => Some(self.point_54[(offset - 0x54) / 4]),
            0x60..=0x68 => Some(self.point_60[(offset - 0x60) / 4]),
            0x6c => Some(self.word_6c),
            _ => Some(self.word_70),
        })
    }
    pub(crate) fn byte_08(&self) -> u8 {
        self.mirrored
    }
    pub(crate) fn byte_34(&self) -> u8 {
        self.active
    }
}

///42D876..42D88D: fld dword, fmul dword 9FBDB8 (0.5f), fstp dword. Scaling
///by a power of two is exact in every x87 precision; only the m32 store
///rounds (nearest-even, binary32 subnormal range). fld quiets a signalling
///NaN; a quiet NaN passes unchanged.
pub(crate) fn half_9fbdb8(bits: u32) -> u32 {
    let value = f32::from_bits(bits);
    if value.is_nan() {
        return bits | 0x0040_0000;
    }
    ((f64::from(value) * 0.5) as f32).to_bits()
}

///The constructed allocation before +144 publishes it.
#[derive(Debug)]
pub(crate) struct Array<T> {
    tracks: Vec<Track<T>>,
    ///arr-4: the count the eh vector iterator was given.
    header: u32,
}
///A constructor that did not complete keeps its allocation and prefix.
pub(crate) struct Pending<T> {
    pub(crate) array: Array<T>,
    pub(crate) error: anyhow::Error,
}
impl<T: Clone> Array<T> {
    ///6F7810 then the 42D7B0 iterator. Rust allocation failure is an error:
    ///native would publish NULL and 42D830 would then write through it.
    pub(crate) fn construct(count: i32, g: &mut impl Globals) -> std::result::Result<Self, Pending<T>> {
        let mut array = Self { tracks: Vec::new(), header: count as u32 };
        let n = match usize::try_from(count) {
            Ok(n) if n > 0 => n,
            _ => return Err(Pending { array, error: anyhow::anyhow!("451F90 Track count must be positive") }),
        };
        if let Err(e) = array.tracks.try_reserve_exact(n) {
            let error = anyhow::Error::from(e).context("6F7810 Track allocation failed; native 42D830 would write through NULL");
            return Err(Pending { array, error });
        }
        for _ in 0..n {
            match Track::construct(g) {
                Ok(track) => array.tracks.push(track),
                Err(error) => return Err(Pending { array, error }),
            }
        }
        Ok(array)
    }
    ///42D830 on element `index` of the published array.
    pub(crate) fn initialize(&mut self, index: usize, info: &TrackInfo<T>) -> Result<()> {
        self.tracks.get_mut(index).context("42D830 Track index outside the actual +148 allocation")?.initialize(info);
        Ok(())
    }
}
impl<T> Array<T> {
    pub(crate) fn tracks(&self) -> &[Track<T>] {
        &self.tracks
    }
    pub(crate) fn header(&self) -> u32 {
        self.header
    }
}

#[cfg(test)]
#[path = "view_actor_tracks_tests.rs"]
mod tests;
