//! Native Ni property constructors used by 8C3ED0 and Camera7016A0.
//!
//! References share the same Rust allocation; no property values are cloned.
//! Native vtable/refcount/pointer words are represented by typed ownership,
//! never fabricated ABI addresses. Unknown padding remains unreadable.
use std::sync::{Arc, Mutex};

type Result<T> = std::result::Result<T, String>;
const VTABLES: [u32; 12] = [
    0xa4d92c, 0xa84670, 0xa846ac, 0xab91e0, 0xab9db0, 0xa846e8, 0xa84724, 0xa84634, 0xab72a4,
    0xa44efc, 0x9fdc0c, 0xa84760,
];
const SIZES: [usize; 12] = [
    0x24, 0x24, 0x34, 0x64, 0x24, 0x24, 0x24, 0x40, 0x48, 0x2c, 0x24, 0x28,
];

///The actual virtual+34 classification. Names are deliberately offset neutral
///where RTTI has not been independently pinned; later SDK enums are not used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kind(u8);
impl Kind {
    pub fn from_slot(slot: usize) -> Result<Self> {
        if slot < VTABLES.len() {
            Ok(Self(slot as u8))
        } else {
            Err("native Ni property slot outside 12 proven classes".into())
        }
    }
    pub fn slot(self) -> usize {
        usize::from(self.0)
    }
    pub fn native_vtable_identity(self) -> u32 {
        VTABLES[self.slot()]
    }
}

#[derive(Clone, Copy)]
struct Word {
    bits: u32,
    known: u32,
}
struct Property {
    kind: Kind,
    words: Vec<Word>,
    ///86CCC0 constructs seven actual null texture-map slots; metadata capacity
    ///is not retained as an artificial allocation ceiling.
    texture_slots: Option<Vec<Option<()>>>,
}
impl Property {
    fn put(&mut self, offset: u32, bits: u32) {
        self.words[offset as usize / 4] = Word {
            bits,
            known: u32::MAX,
        };
    }
    fn byte(&self, offset: u32) -> Result<u8> {
        if self.kind.slot() == 8 && (0x34..0x38).contains(&offset) {
            let n = self
                .texture_slots
                .as_ref()
                .ok_or("Ni texture slot array is unbound")?
                .len() as u32;
            return Ok((n >> (8 * (offset - 0x34))) as u8);
        }
        let w = self
            .words
            .get(offset as usize / 4)
            .ok_or("Ni property byte outside class")?;
        let shift = 8 * (offset & 3);
        if w.known & (0xff << shift) != 0xff << shift {
            return Err("Ni property byte has no reached writer".into());
        }
        Ok((w.bits >> shift) as u8)
    }
    fn put_byte(&mut self, offset: u32, bits: u8) {
        let w = &mut self.words[offset as usize / 4];
        let shift = 8 * (offset & 3);
        w.bits = (w.bits & !(0xff << shift)) | (u32::from(bits) << shift);
        w.known |= 0xff << shift;
    }
    fn short(&self, offset: u32) -> Result<u16> {
        Ok(u16::from(self.byte(offset)?) | (u16::from(self.byte(offset + 1)?) << 8))
    }
    fn put_short(&mut self, offset: u32, bits: u16) {
        self.put_byte(offset, bits as u8);
        self.put_byte(offset + 1, (bits >> 8) as u8);
    }
    fn word(&self, offset: u32) -> Result<u32> {
        if offset == 0x34 && self.kind.slot() == 8 {
            return Ok(self
                .texture_slots
                .as_ref()
                .ok_or("Ni texture slot array is unbound")?
                .len() as u32);
        }
        let w = self
            .words
            .get(offset as usize / 4)
            .filter(|w| offset % 4 == 0 && w.known == u32::MAX)
            .ok_or_else(|| {
                format!("native Ni property+{offset:X} has no reached numeric writer")
            })?;
        Ok(w.bits)
    }
}

///Cloning this reference retains the SAME allocation and current fields.
///There is no Clone implementation for the property itself.
#[derive(Clone)]
pub struct Reference(Arc<Mutex<Property>>);
impl Reference {
    pub fn kind(&self) -> Result<Kind> {
        Ok(self
            .0
            .lock()
            .map_err(|_| "Ni property owner is poisoned")?
            .kind)
    }
    pub fn word(&self, offset: u32) -> Result<u32> {
        self.0
            .lock()
            .map_err(|_| "Ni property owner is poisoned")?
            .word(offset)
    }
    pub fn byte(&self, offset: u32) -> Result<u8> {
        self.0
            .lock()
            .map_err(|_| "Ni property owner is poisoned")?
            .byte(offset)
    }
    pub fn short(&self, offset: u32) -> Result<u16> {
        self.0
            .lock()
            .map_err(|_| "Ni property owner is poisoned")?
            .short(offset)
    }
    pub fn same_allocation(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
pub struct Failure {
    pub property: Reference,
    pub error: String,
}
impl std::fmt::Debug for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativePropertyFailure")
            .field("error", &self.error)
            .finish()
    }
}
pub trait Globals {
    fn word(&mut self, address: u32) -> Result<u32>;
}

///The one process/application material identity allocator. Original image
///DWORD B323C0 is 1;87D280 reads it and advances it with native u32 wrapping.
///The shared default set and later camera must retain this SAME factory.
pub struct Factory {
    material_id_b323c0: u32,
}
impl Factory {
    pub fn from_native_process_image() -> Self {
        Self {
            material_id_b323c0: 1,
        }
    }
    pub fn material_next_id(&self) -> u32 {
        self.material_id_b323c0
    }
    pub fn construct(
        &mut self,
        kind: Kind,
        g: &mut impl Globals,
    ) -> std::result::Result<Reference, Failure> {
        let mut p = Property {
            kind,
            words: vec![Word { bits: 0, known: 0 }; SIZES[kind.slot()] / 4],
            texture_slots: None,
        };
        //86A680/86B240 base fields. Type/refcount are typed Rust ownership.
        for offset in [8, 0x10, 0xc, 0x14, 0x1c, 0x18] {
            p.put(offset, 0);
        }
        let result = (|| -> Result<()> {
            match kind.slot() {
                0 => {
                    p.put_short(0x20, 0xec);
                    p.put_byte(0x22, 0);
                }
                1 | 6 | 10 => p.put_short(0x20, 0),
                2 => {
                    p.put(0x28, 0);
                    p.put(0x2c, 0);
                    p.put(0x30, 0);
                    p.put(0x24, 0x3f800000);
                    p.put_short(0x20, 0);
                    for (offset, address) in [(0x28, 0xb43e68), (0x2c, 0xb43e6c), (0x30, 0xb43e70)]
                    {
                        p.put(offset, g.word(address)?);
                    }
                }
                3 => {
                    for offset in (0x24..=0x50).step_by(4) {
                        p.put(offset, 0)
                    }
                    for offset in [0x2c, 0x28, 0x24, 0x38, 0x34, 0x30] {
                        p.put(offset, 0x3f000000)
                    }
                    for offset in [0x44, 0x40, 0x3c, 0x50, 0x4c, 0x48] {
                        p.put(offset, 0)
                    }
                    p.put(0x54, 0x40800000);
                    p.put(0x58, 0x3f800000);
                    p.put(0x20, self.material_id_b323c0);
                    self.material_id_b323c0 = self.material_id_b323c0.wrapping_add(1);
                    p.put(0x60, 0);
                    p.put(0x5c, 1);
                }
                4 => p.put(0x20, 0),
                5 => p.put_short(0x20, 1),
                7 => {
                    p.put_byte(0x20, 0);
                    for (offset, value) in [
                        (0x24, 4),
                        (0x28, 0),
                        (0x2c, u32::MAX),
                        (0x30, 0),
                        (0x34, 0),
                        (0x38, 3),
                        (0x3c, 0),
                    ] {
                        p.put(offset, value)
                    }
                }
                8 => {
                    //86CCC0's seven882070(index,NULL) stores. Pointers/vtables
                    //remain typed; initial capacity7 is not a runtime ceiling.
                    p.texture_slots = Some(vec![None; 7]);
                    p.put(0x38, 0);
                    p.put(0x3c, 1);
                    p.put(0x40, 0);
                    p.put_byte(0x24, 0);
                    p.put(0x44, 0);
                    p.put(0x20, 2);
                }
                9 => {
                    p.put(0x24, 0);
                    p.put(0x28, 1);
                }
                11 => {
                    p.put_short(0x20, 3);
                    p.put(0x24, 3);
                }
                _ => unreachable!("closed native property Kind"),
            }
            Ok(())
        })();
        let reference = Reference(Arc::new(Mutex::new(p)));
        match result {
            Ok(()) => Ok(reference),
            Err(error) => Err(Failure {
                property: reference,
                error,
            }),
        }
    }
}

pub trait CameraInputs {
    fn stencil_shadow_color_c0(&mut self) -> Result<[u32; 3]>;
    fn renderer_word_24c(&mut self) -> Result<u32>;
}
///7016A0's ordered per-class changes, before installation in the quad set.
///The reached source/renderer read may fail after earlier property stores;
///the reference keeps those stores in its original allocation on failure.
pub fn apply_camera_7016a0(p: &Reference, input: &mut impl CameraInputs) -> Result<()> {
    let mut p = p.0.lock().map_err(|_| "Ni property owner is poisoned")?;
    match p.kind.slot() {
        0 => {
            let byte = p.byte(0x20)?;
            p.put_byte(0x20, byte | 1);
            let saved = p.short(0x20)?;
            p.put_short(0x20, (saved & 0xffe3) | 2);
            p.put_short(0x20, (saved & 0xfe43) | 0x42);
            let byte = p.byte(0x21)?;
            p.put_byte(0x21, byte | 2);
            let byte = p.byte(0x21)?;
            p.put_byte(0x21, byte & 0xe3);
        }
        1 | 2 | 5 | 6 | 10 => {
            let v = p.byte(0x20)?;
            p.put_byte(0x20, v & 0xfe);
        }
        3 => {
            let color = input.stencil_shadow_color_c0()?;
            p.put(0x48, color[0]);
            p.put(0x4c, color[1]);
            p.put(0x50, color[2]);
            let old = p.word(0x5c)?;
            p.put(0x5c, old.wrapping_add(1));
        }
        4 => {}
        7 => {
            p.put_byte(0x20, 1);
            p.put(0x24, 5);
            p.put(0x28, input.renderer_word_24c()?);
            for (offset, value) in [(0x38, 0), (0x30, 0), (0x34, 0), (0x3c, 3)] {
                p.put(offset, value)
            }
        }
        8 => p.put(0x20, 2),
        9 => {
            p.put(0x24, 0);
            p.put(0x28, 0);
        }
        11 => {
            let v = p.byte(0x20)?;
            p.put_byte(0x20, v & 0xfc);
        }
        _ => unreachable!("closed native property Kind"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Source {
        words: [u32; 3],
        next: usize,
        cut: Option<usize>,
    }
    impl Globals for Source {
        fn word(&mut self, address: u32) -> Result<u32> {
            assert_eq!(address, [0xb43e68, 0xb43e6c, 0xb43e70][self.next]);
            let current = self.next;
            self.next += 1;
            if self.cut == Some(current) {
                return Err("required current Fog source".into());
            }
            Ok(self.words[current])
        }
    }
    fn compare(p: &Reference, row: &serde_json::Value) {
        let slot = row["slot"].as_u64().unwrap() as usize;
        assert_eq!(p.kind().unwrap().slot(), slot);
        let words = row["words"].as_array().unwrap();
        let known = row["known"].as_array().unwrap();
        assert_eq!(
            words[0].as_u64().unwrap(),
            u64::from(p.kind().unwrap().native_vtable_identity())
        );
        for offset in 8..SIZES[slot] {
            //Actual native allocations/vtables/capacity are represented by the
            //same typed texture owner and growable storage, not raw addresses.
            if slot == 8 && (0x28..0x34).contains(&offset) {
                continue;
            }
            let shift = 8 * (offset & 3);
            let mask = known[offset / 4].as_u64().unwrap() as u32;
            if mask & (0xff << shift) != 0xff << shift {
                continue;
            }
            let expected = (words[offset / 4].as_u64().unwrap() >> (shift)) as u8;
            assert_eq!(
                p.byte(offset as u32).unwrap(),
                expected,
                "slot{slot}/byte{offset:X}"
            );
        }
    }
    #[test]
    fn native_fresh_factories_counter_and_fog_error_prefixes() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-native-camera/native-render-properties.json"
        ))
        .unwrap();
        for row in fixture["rows"].as_array().unwrap() {
            let seed = row["seed"].as_u64().unwrap() as u32;
            let cut = row["cut"].as_u64().map(|v| v as usize);
            //Explicit oracle input, never a public live counter constructor.
            let mut factory = Factory {
                material_id_b323c0: seed,
            };
            for property in row["output"]["properties"].as_array().unwrap() {
                let kind = Kind::from_slot(property["slot"].as_u64().unwrap() as usize).unwrap();
                let mut source = Source {
                    words: [0x3f800000, 0x3f000000, 0xc0000000],
                    next: 0,
                    cut,
                };
                let result = factory.construct(kind, &mut source);
                let p = if cut.is_some() {
                    result.err().unwrap().property
                } else {
                    result.unwrap()
                };
                compare(&p, property);
            }
            assert_eq!(
                u64::from(factory.material_next_id()),
                row["output"]["counter"].as_u64().unwrap()
            );
        }
    }
    struct CameraSource;
    impl CameraInputs for CameraSource {
        fn stencil_shadow_color_c0(&mut self) -> Result<[u32; 3]> {
            Ok([0x3e4ccccd, 0x3ecccccd, 0x3f19999a])
        }
        fn renderer_word_24c(&mut self) -> Result<u32> {
            Ok(2)
        }
    }
    #[test]
    fn native_camera_mutations_keep_the_same_property_allocation() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-native-camera/native-camera-properties.json"
        ))
        .unwrap();
        for row in fixture["rows"].as_array().unwrap() {
            let mut factory = Factory::from_native_process_image();
            for slot in [4, 9, 3, 8, 5, 2, 10, 11, 7, 6, 1, 0] {
                let kind = Kind::from_slot(slot).unwrap();
                let mut source = Source {
                    words: [0; 3],
                    next: 0,
                    cut: None,
                };
                let p = factory.construct(kind, &mut source).unwrap();
                let retained = p.clone();
                apply_camera_7016a0(&p, &mut CameraSource).unwrap();
                assert!(p.same_allocation(&retained));
                compare(&retained, &row["properties"][slot]);
            }
            assert_eq!(
                u64::from(factory.material_next_id()),
                row["counter"].as_u64().unwrap()
            );
        }
    }
    struct RequiredCameraSource;
    impl CameraInputs for RequiredCameraSource {
        fn stencil_shadow_color_c0(&mut self) -> Result<[u32; 3]> {
            Err("required actual C0 owner".into())
        }
        fn renderer_word_24c(&mut self) -> Result<u32> {
            Err("required actual renderer owner".into())
        }
    }
    #[test]
    fn actual_camera_source_refusals_keep_prior_property_stores() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-native-camera/native-camera-property-prefix.json"
        ))
        .unwrap();
        for row in fixture["rows"].as_array().unwrap() {
            let expected = &row["output"]["property"];
            let slot = expected["slot"].as_u64().unwrap() as usize;
            let mut factory = Factory::from_native_process_image();
            let mut globals = Source {
                words: [0; 3],
                next: 0,
                cut: None,
            };
            let p = factory
                .construct(Kind::from_slot(slot).unwrap(), &mut globals)
                .unwrap();
            assert!(apply_camera_7016a0(&p, &mut RequiredCameraSource).is_err());
            compare(&p, expected);
        }
    }
}
