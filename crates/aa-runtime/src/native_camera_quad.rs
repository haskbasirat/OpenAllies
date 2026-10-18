//! Camera7016A0's four-vertex auxiliary quad887F40 and retained properties.
//! Shared defaults and all replacements share actual property allocations.
//! This does not submit a GPU draw or authorize camera/view completion.
use crate::{
    native_render_defaults::SharedSet,
    native_render_properties::{self, CameraInputs, Kind, Reference},
};
type Result<T> = std::result::Result<T, String>;

pub struct Quad {
    vertices: Vec<[u32; 3]>,
    //887F40 receives NULL UV/color arguments; the actual empty owners are
    //retained rather than generating UVs/colors for this full-screen quad.
    texture_coordinates: Option<Vec<[u32; 2]>>,
    colors: Option<Vec<[u32; 4]>>,
    property_set: Option<Vec<Option<Reference>>>,
    pending_property: Option<Reference>,
    flags_20: u16,
    word_24: u32,
    complete: bool,
}
pub struct Failure {
    pub quad: Quad,
    pub error: String,
}
impl std::fmt::Debug for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CameraQuadFailure")
            .field("error", &self.error)
            .finish()
    }
}
impl Quad {
    ///The twelve words are copied from SAME Camera340..36C by the concrete
    ///constructor. They are arguments here, not a current-World/camera receipt.
    pub(crate) fn from_camera_vertices_887f40(
        vertices: [[u32; 3]; 4],
        defaults: &mut SharedSet,
        input: &mut impl CameraInputs,
    ) -> std::result::Result<Self, Failure> {
        let mut q = Self {
            vertices: vertices.into_iter().collect(),
            texture_coordinates: None,
            colors: None,
            property_set: None,
            pending_property: None,
            flags_20: 0,
            word_24: 0,
            complete: false,
        };
        let built = (|| -> Result<()> {
            q.property_set = Some(defaults.new_property_set_895550()?);
            //Native constructor order differs from property-slot order.
            for slot in [4, 9, 3, 8, 5, 2, 10, 11, 7, 6, 1, 0] {
                let kind = Kind::from_slot(slot)?;
                let (factory, globals) = defaults.factory_and_globals();
                let p = match factory.construct(kind, globals) {
                    Ok(p) => p,
                    Err(failure) => {
                        q.pending_property = Some(failure.property);
                        return Err(failure.error);
                    }
                };
                q.pending_property = Some(p);
                native_render_properties::apply_camera_7016a0(
                    q.pending_property.as_ref().unwrap(),
                    input,
                )?;
                //The new reference is installed only AFTER its actual changes.
                //Replacing drops just this set's old reference, not the shared
                //default allocation or another set's current reference.
                q.property_set.as_mut().unwrap()[slot] = q.pending_property.take();
            }
            q.complete = true;
            Ok(())
        })();
        match built {
            Ok(()) => Ok(q),
            Err(error) => Err(Failure { quad: q, error }),
        }
    }
    pub fn require_complete(&self) -> Result<()> {
        if self.complete {
            Ok(())
        } else {
            Err("camera auxiliary quad constructor is incomplete".into())
        }
    }
    pub fn vertices(&self) -> &[[u32; 3]] {
        &self.vertices
    }
    pub fn texture_coordinates(&self) -> Option<&[[u32; 2]]> {
        self.texture_coordinates.as_deref()
    }
    pub fn colors(&self) -> Option<&[[u32; 4]]> {
        self.colors.as_deref()
    }
    pub fn flags_20(&self) -> u16 {
        self.flags_20
    }
    pub fn word_24(&self) -> u32 {
        self.word_24
    }
    pub fn property(&self, slot: usize) -> Result<Option<&Reference>> {
        self.property_set
            .as_ref()
            .ok_or("camera quad property set has no reached producer")?
            .get(slot)
            .map(Option::as_ref)
            .ok_or("camera quad property slot outside native set".into())
    }
    pub fn pending_property(&self) -> Option<&Reference> {
        self.pending_property.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Inputs {
        cut: Option<&'static str>,
        calls: Vec<&'static str>,
    }
    impl CameraInputs for Inputs {
        fn stencil_shadow_color_c0(&mut self) -> Result<[u32; 3]> {
            self.calls.push("c0");
            if self.cut == Some("c0") {
                Err("required C0 source".into())
            } else {
                Ok([0x3e4ccccd, 0x3ecccccd, 0x3f19999a])
            }
        }
        fn renderer_word_24c(&mut self) -> Result<u32> {
            self.calls.push("renderer");
            if self.cut == Some("renderer") {
                Err("required renderer source".into())
            } else {
                Ok(2)
            }
        }
    }
    #[test]
    fn real_default_bootstrap_then_native_camera_quad_all_fields() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-native-camera/native-camera-quad.json"
        ))
        .unwrap();
        let sizes = [
            0x24, 0x24, 0x34, 0x64, 0x24, 0x24, 0x24, 0x40, 0x48, 0x2c, 0x24, 0x28,
        ];
        for row in fixture["rows"].as_array().unwrap() {
            let mut defaults = SharedSet::from_fresh_native_process().unwrap();
            defaults.initialize_properties_prefix_8c3ed0().unwrap();
            let original = defaults.new_property_set_895550().unwrap();
            let flat = serde_json::from_value::<Vec<u32>>(row["vertices"].clone()).unwrap();
            let vertices = std::array::from_fn(|i| std::array::from_fn(|j| flat[i * 3 + j]));
            let mut input = Inputs {
                cut: None,
                calls: Vec::new(),
            };
            let q = Quad::from_camera_vertices_887f40(vertices, &mut defaults, &mut input).unwrap();
            q.require_complete().unwrap();
            assert_eq!(q.vertices(), vertices);
            assert_eq!(q.flags_20(), 0);
            assert_eq!(q.word_24(), 0);
            assert!(q.texture_coordinates().is_none() && q.colors().is_none());
            assert_eq!(input.calls, ["c0", "renderer"]);
            for slot in 0..12 {
                let p = q.property(slot).unwrap().unwrap();
                assert!(!p.same_allocation(original[slot].as_ref().unwrap()));
                let r = &row["properties"][slot];
                let known = r["known"].as_array().unwrap();
                let words = r["words"].as_array().unwrap();
                assert_eq!(
                    u64::from(p.kind().unwrap().native_vtable_identity()),
                    words[0].as_u64().unwrap()
                );
                for offset in 8..sizes[slot] {
                    if slot == 8 && (0x28..0x34).contains(&offset) {
                        continue;
                    }
                    let mask = known[offset / 4].as_u64().unwrap();
                    let shift = 8 * (offset & 3);
                    if mask & (0xff << shift) != 0xff << shift {
                        continue;
                    }
                    assert_eq!(
                        u64::from(p.byte(offset as u32).unwrap()),
                        (words[offset / 4].as_u64().unwrap() >> shift) & 255,
                        "slot{slot}/offset{offset:X}"
                    )
                }
            }
            assert_eq!(
                defaults.material_next_id(),
                row["counter"].as_u64().unwrap() as u32
            );
            assert_eq!(original[3].as_ref().unwrap().word(0x20).unwrap(), 1);
            assert_eq!(q.property(3).unwrap().unwrap().word(0x20).unwrap(), 2);
        }
    }
    #[test]
    fn source_error_retains_pending_property_and_shared_replaced_prefix() {
        for cut in ["c0", "renderer"] {
            let mut defaults = SharedSet::from_fresh_native_process().unwrap();
            defaults.initialize_properties_prefix_8c3ed0().unwrap();
            let original = defaults.new_property_set_895550().unwrap();
            let mut input = Inputs {
                cut: Some(cut),
                calls: Vec::new(),
            };
            let failure = Quad::from_camera_vertices_887f40([[0; 3]; 4], &mut defaults, &mut input)
                .err()
                .unwrap();
            let q = failure.quad;
            assert!(q.require_complete().is_err());
            let pending = q.pending_property().unwrap();
            let slot = if cut == "c0" { 3 } else { 7 };
            assert_eq!(pending.kind().unwrap().slot(), slot);
            assert!(q
                .property(slot)
                .unwrap()
                .unwrap()
                .same_allocation(original[slot].as_ref().unwrap()));
            assert!(!q
                .property(4)
                .unwrap()
                .unwrap()
                .same_allocation(original[4].as_ref().unwrap()));
            assert!(!q
                .property(9)
                .unwrap()
                .unwrap()
                .same_allocation(original[9].as_ref().unwrap()));
            assert_eq!(defaults.material_next_id(), 3);
            if cut == "c0" {
                assert_eq!(input.calls, ["c0"]);
                assert_eq!(pending.word(0x20).unwrap(), 2);
                assert_eq!(pending.word(0x5c).unwrap(), 1)
            } else {
                assert_eq!(input.calls, ["c0", "renderer"]);
                assert_eq!(pending.byte(0x20).unwrap(), 1);
                assert_eq!(pending.word(0x24).unwrap(), 5);
                assert_eq!(pending.word(0x38).unwrap(), 3)
            }
        }
    }
}
