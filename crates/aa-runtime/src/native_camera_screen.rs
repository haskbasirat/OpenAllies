//! The drag-box screen rectangle the KKC_UI box state attaches to the current
//! camera, and the camera's NiScreenPolygon array (Camera+168), from AA.exe.
//!
//! - 5CD850 (GameInterface+E0 box state, entered from the click state's
//!   5CDBE0): 5E0780 push, 599870 press copy to +8 (both the state machine's),
//!   `[B40048]+214 = 1`, four vertices = B43CF8/B43CFC/B43D00, four colors
//!   (0, 0, 0.5, 0.3) (5CD903..5CD927), `new(2C)` 887F40(4, vertices, NULL,
//!   colors) into +4, 5CCA80, then three properties installed by their
//!   vt+34 class into the polygon's 895550 property state (+C, slots +8):
//!   NiVertexColorProperty A44EFC (+24 = 1, +28 = 0), NiAlphaProperty A4D92C
//!   (+20 = 0xEC | 1, +22 = 0) and NiMaterialProperty 87D280 (+58 = 0.5,
//!   +5C incremented), and finally 5CD7E0 on `*B40048`.
//! - 5CCA80 (also the box vt+10 5CCC10): the vertex X/Y from the UI+58
//!   tracker (+4/+8 cursor, +C/+10 left press) and B396D0/B396D4, then
//!   polygon+20 |= 1. Z words are not written.
//! - 5CD7E0: 5CD730 (first NULL slot of the +168 NiTArray, else append at
//!   +C) on Camera+168, then polygon+20 |= 1.
//! - 5CCA30 (box vt+4): `[B40048]+214 = 0`, 874350 (first equal slot set
//!   NULL through 882300; the size is kept), release of +4, then the tail
//!   VWorld+13C vt+44 (the selection manager's, not here).
//! - Camera draw 6FF63E..6FF672: every non-NULL slot below +174 in index
//!   order -> 887E60 (renderer+10 = polygon+C, +14 = 0) -> renderer vt+D8
//!   (NiDX9Renderer). That renderer path is NOT ported here.
//!
//! NiCamera's constructor (874B7D..874BC9) gives +168 capacity 5 and grow-by
//! 5. Rust storage grows without that allocation step; no cap is imposed.
use crate::native_render_properties::{Factory, Globals, Kind, Reference};
use aa_numeric::{Binary, Precision};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
type Result<T> = std::result::Result<T, String>;

///5CD91B..5CD927: r, g, b, a of every box vertex.
pub const BOX_COLOR: [u32; 4] = [0, 0, 0x3f00_0000, 0x3e99_999a];
const ONE: u32 = 0x3f80_0000; //9F32E0

///A property allocation constructed by the installing code, with the stores
///that code made to it afterwards. The shared property owner exposes reads
///only, so the later stores are retained beside the SAME allocation.
pub struct Installed {
    allocation: Reference,
    stores: BTreeMap<u32, u8>,
}
impl Installed {
    fn new(allocation: Reference) -> Self {
        Self { allocation, stores: BTreeMap::new() }
    }
    pub fn allocation(&self) -> &Reference {
        &self.allocation
    }
    pub fn kind(&self) -> Result<Kind> {
        self.allocation.kind()
    }
    pub fn byte(&self, offset: u32) -> Result<u8> {
        match self.stores.get(&offset) {
            Some(v) => Ok(*v),
            None => self.allocation.byte(offset),
        }
    }
    pub fn short(&self, offset: u32) -> Result<u16> {
        Ok(u16::from_le_bytes([self.byte(offset)?, self.byte(offset + 1)?]))
    }
    pub fn word(&self, offset: u32) -> Result<u32> {
        if (0..4).all(|i| !self.stores.contains_key(&(offset + i))) {
            return self.allocation.word(offset);
        }
        let mut b = [0; 4];
        for (i, v) in b.iter_mut().enumerate() {
            *v = self.byte(offset + i as u32)?;
        }
        Ok(u32::from_le_bytes(b))
    }
    fn store_byte(&mut self, offset: u32, value: u8) {
        self.stores.insert(offset, value);
    }
    fn store_word(&mut self, offset: u32, value: u32) {
        for (i, v) in value.to_le_bytes().into_iter().enumerate() {
            self.stores.insert(offset + i as u32, v);
        }
    }
}

///One slot of the polygon's property state (+C, slot words from +8).
pub enum Slot {
    ///The 895550 state's reference to the shared default (or NULL).
    Shared(Option<Reference>),
    Installed(Installed),
}

///NiScreenPolygon (vtable AB990C) as 887F40 constructs it.
pub struct ScreenPolygon {
    vertices_14: Vec<[u32; 3]>,
    texture_coordinates_18: Option<Vec<[u32; 2]>>,
    colors_1c: Option<Vec<[u32; 4]>>,
    properties_0c: Vec<Slot>,
    flags_20: u16,
    word_24: u32,
    word_28: u32,
}
pub type Polygon = Arc<Mutex<ScreenPolygon>>;
impl ScreenPolygon {
    ///887F40(count, vertices, texture coordinates, colors): +10 the u16
    ///count, copies of the given arrays (NULL stays NULL), a new 895550
    ///property state in +C, +20 = 0 and +24 = +28 = 0.
    pub fn from_887f40(
        vertices: &[[u32; 3]],
        texture_coordinates: Option<&[[u32; 2]]>,
        colors: Option<&[[u32; 4]]>,
        state_895550: Vec<Option<Reference>>,
    ) -> Result<Self> {
        let count = u16::try_from(vertices.len()).map_err(|_| "887F40 vertex count is a native u16")?;
        let n = usize::from(count);
        if texture_coordinates.is_some_and(|t| t.len() < n) || colors.is_some_and(|c| c.len() < n) {
            return Err("887F40 copies count entries from every non-NULL array".into());
        }
        Ok(Self {
            vertices_14: vertices.to_vec(),
            texture_coordinates_18: texture_coordinates.map(|t| t[..n].to_vec()),
            colors_1c: colors.map(|c| c[..n].to_vec()),
            properties_0c: state_895550.into_iter().map(Slot::Shared).collect(),
            flags_20: 0,
            word_24: 0,
            word_28: 0,
        })
    }
    pub fn vertices(&self) -> &[[u32; 3]] {
        &self.vertices_14
    }
    pub fn texture_coordinates(&self) -> Option<&[[u32; 2]]> {
        self.texture_coordinates_18.as_deref()
    }
    pub fn colors(&self) -> Option<&[[u32; 4]]> {
        self.colors_1c.as_deref()
    }
    pub fn flags_20(&self) -> u16 {
        self.flags_20
    }
    pub fn word_24(&self) -> u32 {
        self.word_24
    }
    pub fn word_28(&self) -> u32 {
        self.word_28
    }
    pub fn property(&self, slot: usize) -> Result<&Slot> {
        self.properties_0c.get(slot).ok_or_else(|| "screen polygon property slot outside its state".into())
    }
    ///`[state + 8 + 4 * vt+34()] = property` (5CDA3C, 5CDABA, 5CDB36).
    fn install(&mut self, property: Installed) -> Result<()> {
        let slot = property.kind()?.slot();
        *self
            .properties_0c
            .get_mut(slot)
            .ok_or("screen polygon property state has no slot for this class")? = Slot::Installed(property);
        Ok(())
    }
    ///The `or byte ptr [polygon+20], 1` of 5CCB59 and 5CD836.
    fn mark_changed(&mut self) {
        self.flags_20 |= 1;
    }
}

///UI+58 tracker words 5CCA80 reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tracker {
    ///+4/+8
    pub current: [i32; 2],
    ///+C/+10
    pub left_press: [i32; 2],
}

///5CCA80 on the box polygon: x = column / B396D0 and y = 1 - row / B396D4 of
///the rectangle between the left press and the cursor.
pub fn update_5cca80(polygon: &mut ScreenPolygon, t: Tracker, viewport: [i32; 2], p: Precision) -> Result<()> {
    if polygon.vertices_14.len() < 4 {
        return Err("5CCA80 writes four screen vertices".into());
    }
    let [cx, cy] = t.current;
    let [px, py] = t.left_press;
    let min_x = if px >= cx { cx } else { px };
    let max_x = if px > cx { px } else { cx };
    let min_y = if py >= cy { cy } else { py };
    let max_y = if py > cy { py } else { cy };
    let e = |e: aa_numeric::Error| e.to_string();
    //FILD W, FILD min, FDIV ST(1): FSTP m32 A.
    let width = Binary::from_i32(viewport[0]);
    let a = Binary::from_i32(min_x).div(&width, p).map_err(e)?.store_f32().map_err(e)?;
    //FIDIVR max: ST0 = max / W, held until its stores.
    let b = Binary::from_i32(max_x).div(&width, p).map_err(e)?;
    //FILD H, FSTP m32: the height operand is binary32.
    let height = Binary::from_f32_bits(Binary::from_i32(viewport[1]).store_f32().map_err(e)?).map_err(e)?;
    let one = Binary::from_f32_bits(ONE).map_err(e)?;
    //FILD min, FDIV m32 H, FSUBR m32 1.0 (and the same for max).
    let c = one.sub(&Binary::from_i32(min_y).div(&height, p).map_err(e)?, p).map_err(e)?;
    let d = one.sub(&Binary::from_i32(max_y).div(&height, p).map_err(e)?, p).map_err(e)?;
    let (b, c, d) = (b.store_f32().map_err(e)?, c.store_f32().map_err(e)?, d.store_f32().map_err(e)?);
    let v = &mut polygon.vertices_14;
    v[0][0] = a; //+0
    v[1][0] = a; //+C
    v[0][1] = c; //+4
    v[1][1] = d; //+10
    v[2][0] = b; //+18
    v[2][1] = d; //+1C
    v[3][0] = b; //+24
    v[3][1] = c; //+28
    polygon.mark_changed();
    Ok(())
}

///The polygon part of 5CD850 after its +214 store: 887F40 on the seed
///vertices and box colors, 5CCA80, then the three installed properties.
///The material constructor draws the SAME process material id (B323C0).
#[allow(clippy::too_many_arguments)]
pub fn box_5cd850(
    seeds: [u32; 3],
    state_895550: Vec<Option<Reference>>,
    factory: &mut Factory,
    globals: &mut impl Globals,
    tracker: Tracker,
    viewport: [i32; 2],
    p: Precision,
) -> Result<ScreenPolygon> {
    let mut polygon = ScreenPolygon::from_887f40(&[seeds; 4], None, Some(&[BOX_COLOR; 4]), state_895550)?;
    update_5cca80(&mut polygon, tracker, viewport, p)?;
    let mut construct = |slot: usize| -> Result<Installed> {
        factory
            .construct(Kind::from_slot(slot)?, globals)
            .map(Installed::new)
            .map_err(|failure| failure.error)
    };
    //5CD9DA..5CDA3C: NiVertexColorProperty (constructor +24 = 0, +28 = 1).
    let mut vertex_color = construct(9)?;
    vertex_color.store_word(0x24, 1);
    vertex_color.store_word(0x28, 0);
    polygon.install(vertex_color)?;
    //5CDA5F..5CDABA: NiAlphaProperty (constructor +20 = 0xEC, +22 = 0).
    let mut alpha = construct(0)?;
    let flags = alpha.byte(0x20)?;
    alpha.store_byte(0x20, flags | 1);
    polygon.install(alpha)?;
    //5CDADD..5CDB36: NiMaterialProperty 87D280.
    let mut material = construct(3)?;
    let revision = material.word(0x5c)?.wrapping_add(1);
    material.store_word(0x58, 0x3f00_0000);
    material.store_word(0x5c, revision);
    polygon.install(material)?;
    Ok(polygon)
}

///Camera+168: NiTArray of NiScreenPolygon pointers (+4 data, +C size,
///+10 non-NULL count).
#[derive(Default)]
pub struct List {
    slots: Vec<Option<Polygon>>,
}
impl List {
    pub fn slots(&self) -> &[Option<Polygon>] {
        &self.slots
    }
    ///+10
    pub fn count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }
    ///5CD7E0: 5CD730 then polygon+20 |= 1. Returns the slot index.
    pub fn attach_5cd7e0(&mut self, polygon: &Polygon) -> Result<usize> {
        let index = match self.slots.iter().position(Option::is_none) {
            Some(i) => {
                self.slots[i] = Some(Arc::clone(polygon));
                i
            }
            //5CD775..5CD78F: grow (5CCFE0) when full, then 882300 at +C.
            None => {
                self.slots.push(Some(Arc::clone(polygon)));
                self.slots.len() - 1
            }
        };
        polygon.lock().map_err(|_| "screen polygon owner poisoned")?.mark_changed();
        Ok(index)
    }
    ///874350: the first slot holding this polygon becomes NULL (882300 keeps
    ///+C); a polygon not in the list leaves it unchanged.
    pub fn detach_874350(&mut self, polygon: &Polygon) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.as_ref().is_some_and(|q| Arc::ptr_eq(q, polygon))) {
            *slot = None;
        }
    }
}

#[cfg(test)]
#[path = "native_camera_screen_tests.rs"]
mod tests;
