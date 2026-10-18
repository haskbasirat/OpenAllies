use super::*;

struct Input;
impl ReadOnlyInputs for Input {
    fn metagame_40cee0(&self) -> Result<bool> {
        Ok(false)
    }
    fn free_camera_without_directory_role(&self) -> Result<bool> {
        Ok(false)
    }
    fn avar(&self, o: u32) -> Result<u32> {
        let v: f32 = match o {
            0x1c => 1.,
            0x20 => 512.,
            0x24 => 0.1,
            0x28 => 1.5,
            0x2c => 1.,
            0x30 => 0.5,
            0x34 | 0x38 => 1.2,
            0x3c => 0.7,
            0x40 => 2.1,
            0x44 => 1.,
            0x48 => -45.,
            0x4c => 0.,
            0x50 => 40.,
            0x54 => 10.,
            0x58 => 100.,
            0x5c => 15.,
            _ => return Err(format!("unknown avar{o:X}")),
        };
        Ok(v.to_bits())
    }
    fn map_dimensions_74_78(&self) -> Result<Option<[i32; 2]>> {
        Ok(None)
    }
    fn fallback_dimensions_uvar_a8_ac(&self) -> Result<[i32; 2]> {
        Ok([256, 128])
    }
    fn visual_height_43d050(&self, _: [u32; 2], _: u32, _: Precision) -> Result<Binary> {
        Err("fresh visual map NULL must not reach43D050".into())
    }
    fn seed_word(&self, _: u32) -> Result<u32> {
        Ok(0)
    }
    fn free_yaw_uvar_b4(&self) -> Result<bool> {
        Ok(false)
    }
    fn world_editor_275(&self) -> Result<bool> {
        Ok(false)
    }
    fn session_state_e4(&self) -> Result<u32> {
        Ok(0)
    }
    fn map_height_550210(&self, _: [u32; 2], _: Precision) -> Result<Binary> {
        Binary::from_f32_bits(12.5f32.to_bits()).map_err(|e| e.to_string())
    }
}
struct Vars {
    calls: usize,
    fail: Option<usize>,
}
impl render_lists::Variables for Vars {
    type Reference = &'static str;
    fn bool_reference(&mut self, name: &'static str) -> Result<Self::Reference> {
        let n = self.calls;
        self.calls += 1;
        if self.fail == Some(n) {
            Err("current Bool lookup refused".into())
        } else {
            Ok(name)
        }
    }
}
struct Renderer {
    calls: usize,
    fail: Option<usize>,
}
impl RendererSource for Renderer {
    type Reference = usize;
    fn current_device_40(&mut self) -> Result<usize> {
        let n = self.calls;
        self.calls += 1;
        if self.fail == Some(n) {
            Err("renderer source refused".into())
        } else {
            Ok(91)
        }
    }
}
struct Properties {
    calls: Vec<&'static str>,
    fail: bool,
}
impl CameraInputs for Properties {
    fn stencil_shadow_color_c0(&mut self) -> Result<[u32; 3]> {
        self.calls.push("c0");
        if self.fail {
            Err("C0 refused".into())
        } else {
            Ok([0.2f32.to_bits(), 0.4f32.to_bits(), 0.6f32.to_bits()])
        }
    }
    fn renderer_word_24c(&mut self) -> Result<u32> {
        self.calls.push("24c");
        Ok(2)
    }
}
fn constructors() -> (Directory<&'static str, usize>, Vars, Renderer, Properties) {
    (
        Directory::from_fresh_native_process().unwrap(),
        Vars {
            calls: 0,
            fail: None,
        },
        Renderer {
            calls: 0,
            fail: None,
        },
        Properties {
            calls: Vec::new(),
            fail: false,
        },
    )
}
#[test]
fn all_native_constructor_publication_reset_rows_and_same_actual_properties() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-native-camera/native-camera-constructor.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 60);
    for row in fixture["rows"].as_array().unwrap() {
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => panic!(),
        };
        let lists = row["render_lists"] == 1;
        let (mut d, mut v, mut r, mut p) = constructors();
        let h = d
            .construct_camera(lists, &Input, &mut v, &mut r, &mut p, pc)
            .unwrap();
        assert!(d.camera(&h).unwrap().normalized_state().is_err());
        assert_eq!(d.cameras[0].states.len(), 2);
        d.reset_current_700e30(&Input, pc).unwrap();
        let c = d.camera(&h).unwrap();
        let s = c.normalized_state().unwrap();
        assert_eq!(c.states.len(), 1);
        assert_eq!(r.calls, 2);
        assert_eq!(v.calls, if lists { 14 } else { 0 });
        assert_eq!(p.calls, ["c0", "24c"]);
        assert_eq!(d.defaults.material_next_id(), 3);
        let skip = [
            0, 4, 0xa0, 0x164, 0x168, 0x16c, 0x180, 0x184, 0x198, 0x270, 0x2a0, 0x370, 0x374,
        ];
        for (i, word) in s.words.iter().enumerate() {
            let off = i as u32 * 4;
            if skip.contains(&off) {
                continue;
            }
            let expected = row["output"]["camera_words"][i].as_u64().unwrap() as u32;
            let known = row["output"]["camera_known"][i].as_u64().unwrap() as u32;
            assert_eq!(word.known & !known, 0, "unexpected numeric writer{off:X}");
            if known == u32::MAX {
                assert_eq!(
                    s.word(off)
                        .unwrap_or_else(|e| panic!("missing native writer {off:X}: {e}")),
                    expected,
                    "all reached word{off:X}"
                );
            } else {
                assert_eq!(
                    known & !word.known,
                    0,
                    "missing partial native writer{off:X}"
                );
            }
            assert_eq!(
                word.bits & word.known,
                expected & word.known,
                "{} lists{lists} offset{off:X}",
                row["precision"]
            );
        }
        for off in [4, 0x164, 0x198, 0x270, 0x2a0, 0x370] {
            assert!(s.word(off).is_err(), "typedpointer{off:X}")
        }
        assert!(s.target_rectangle_alias_present() && s.target_point_alias_present());
        let root = c.root.as_ref().unwrap();
        assert_eq!(root.properties.len(), 12);
        let defaults = d.defaults.new_property_set_895550().unwrap();
        for i in 0..12 {
            assert!(root.properties[i]
                .as_ref()
                .unwrap()
                .same_allocation(defaults[i].as_ref().unwrap()));
        }
        assert_eq!(
            root.node.local_transform().words(),
            root.node.world_transform().words()
        );
        assert_eq!(
            c.auxiliary_quad().unwrap().vertices()[3],
            [0, 1.0f32.to_bits(), 0]
        );
    }
}
#[test]
fn actual_external_effects_run_once_and_failed_prefixes_stay_owned() {
    let (mut d, mut v, mut r, mut p) = constructors();
    v.fail = Some(3);
    assert!(d
        .construct_camera(true, &Input, &mut v, &mut r, &mut p, Precision::Double53)
        .is_err());
    assert_eq!(r.calls, 1);
    assert_eq!(v.calls, 4);
    assert_eq!(d.camera_count(), 0);
    assert!(d.failed_camera().unwrap().camera.lists.is_some());
    assert!(p.calls.is_empty());
    let (mut d, mut v, mut r, mut p) = constructors();
    p.fail = true;
    assert!(d
        .construct_camera(true, &Input, &mut v, &mut r, &mut p, Precision::Double53)
        .is_err());
    assert!(d
        .failed_camera()
        .unwrap()
        .camera
        .quad
        .as_ref()
        .unwrap()
        .pending_property()
        .is_some());
    assert_eq!(p.calls, ["c0"]);
    let (mut d, mut v, mut r, mut p) = constructors();
    r.fail = Some(1);
    assert!(d
        .construct_camera(false, &Input, &mut v, &mut r, &mut p, Precision::Double53)
        .is_err());
    assert_eq!(d.camera_count(), 1);
    assert!(d.publication_error().is_some());
    assert!(d.reset_current_700e30(&Input, Precision::Double53).is_err());
    assert_eq!(d.cameras[0].states[0].word(0x204).unwrap(), 0);
}
#[test]
fn handles_and_real_bitwise_convergence_are_required() {
    let (mut d, mut v, mut r, mut p) = constructors();
    let h = d
        .construct_camera(false, &Input, &mut v, &mut r, &mut p, Precision::Double53)
        .unwrap();
    let (other, _, _, _) = constructors();
    assert!(other.camera(&h).is_err());
    d.cameras[0].states[1].put(0x390, 0x3f000000);
    assert!(d
        .reset_current_700e30(&Input, Precision::Double53)
        .err()
        .unwrap()
        .contains("did not converge"));
    assert!(d.camera(&h).unwrap().normalized_state().is_err());
    let (mut d, mut v, mut r, mut p) = constructors();
    let h = d
        .construct_camera(false, &Input, &mut v, &mut r, &mut p, Precision::Double53)
        .unwrap();
    d.reset_current_700e30(&Input, Precision::Double53).unwrap();
    let root = d.camera_mut(&h).unwrap().root_164().unwrap();
    assert_eq!(root.local_transform().words()[12], 1f32.to_bits());
}

struct Zoom {
    word: u32,
    calls: Vec<&'static str>,
    fail: bool,
}
impl ZoomVariable for Zoom {
    fn camera_scale_zoom_34(&mut self) -> Result<u32> {
        self.calls.push("read");
        Ok(self.word)
    }
    fn store_camera_scale_zoom_34(&mut self, w: u32) -> Result<()> {
        self.calls.push("write");
        if self.fail {
            return Err("actual UVar setter refused".into());
        }
        self.word = w;
        Ok(())
    }
}
#[test]
fn actual_7010d0_and_7005f0_native_zoom_and_ui_write_prefix() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-native-camera/native-camera-actions.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 72);
    for row in fixture["rows"].as_array().unwrap() {
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => panic!(),
        };
        let (mut d, mut v, mut r, mut p) = constructors();
        let h = d
            .construct_camera(row["render_lists"] == 1, &Input, &mut v, &mut r, &mut p, pc)
            .unwrap();
        d.reset_current_700e30(&Input, pc).unwrap();
        let ordinal = row["ordinal"].as_u64().unwrap() as u32;
        let disabled = row["disabled"].as_u64().unwrap() as u8;
        d.cameras[0].states[0].directory_index_702380(ordinal as i32);
        d.cameras[0].states[0].put_byte(0x214, disabled);
        let mut z = Zoom {
            word: 0.5f32.to_bits(),
            calls: Vec::new(),
            fail: false,
        };
        if row["action"] == "reset" {
            d.reset_current_7010d0(&Input, &mut z, pc).unwrap();
        } else {
            apply_zoom_7005f0(
                &mut d.cameras[0].states[0],
                ordinal != 0,
                row["zoomword"].as_u64().unwrap() as u32,
                &mut z,
                pc,
            )
            .unwrap();
        }
        assert_eq!(
            z.word,
            row["output"]["zoom_variable_word"].as_u64().unwrap() as u32
        );
        let s = &d.cameras[0].states[0];
        let skip = [
            0, 4, 0xa0, 0x164, 0x168, 0x16c, 0x180, 0x184, 0x198, 0x270, 0x2a0, 0x370, 0x374,
        ];
        for (i, w) in s.words.iter().enumerate() {
            let off = i as u32 * 4;
            if skip.contains(&off) {
                continue;
            }
            let expected = row["output"]["camera_words"][i].as_u64().unwrap() as u32;
            let known = row["output"]["camera_known"][i].as_u64().unwrap() as u32;
            if known == u32::MAX {
                assert_eq!(
                    s.word(off).unwrap(),
                    expected,
                    "{} {} ord{ordinal} disabled{disabled} off{off:X}",
                    row["precision"],
                    row["action"]
                )
            } else {
                assert_eq!(w.bits & w.known, expected & w.known)
            }
        }
    }
    let (mut d, mut v, mut r, mut p) = constructors();
    let h = d
        .construct_camera(false, &Input, &mut v, &mut r, &mut p, Precision::Double53)
        .unwrap();
    d.reset_current_700e30(&Input, Precision::Double53).unwrap();
    let mut z = Zoom {
        word: 0.5f32.to_bits(),
        calls: Vec::new(),
        fail: true,
    };
    assert!(d
        .set_zoom_captured_7005f0(&h, 1.5f32.to_bits(), &mut z, Precision::Double53)
        .is_err());
    assert_eq!(
        d.cameras[0].states[0].word(0x208).unwrap(),
        1.5f32.to_bits()
    );
    assert_eq!(z.word, 0.5f32.to_bits());
    assert!(d.camera(&h).unwrap().normalized_state().is_err());
}

struct Viewport {
    current: std::sync::atomic::AtomicBool,
    device: usize,
}
impl super::super::backend::Viewport for Viewport {
    fn pixels(&self) -> [u32; 2] {
        [1280, 720]
    }
    fn require_current(&self) -> Result<()> {
        if self.current.load(std::sync::atomic::Ordering::Acquire) {
            Ok(())
        } else {
            Err("actual backend viewport expired".into())
        }
    }
}
impl super::super::backend::ViewportFor<usize> for Viewport {
    fn require_device(&self, d: &usize) -> Result<()> {
        if self.device == *d {
            Ok(())
        } else {
            Err("foreign device".into())
        }
    }
}
impl Globals for Input {
    fn word(&mut self, a: u32) -> Result<u32> {
        self.seed_word(a)
    }
}
struct SceneCamera<'a> {
    directory: &'a mut Directory<&'static str, usize>,
    zoom: Zoom,
}
impl crate::view_scene_init::Camera for SceneCamera<'_> {
    fn reset_current_7010d0(&mut self) -> Result<()> {
        self.directory
            .reset_current_7010d0(&Input, &mut self.zoom, Precision::Double53)
            .map(|_| ())
    }
    fn current_root_164(&mut self) -> Result<&mut Node> {
        let h = self.directory.current_handle()?;
        self.directory.camera_mut(&h)?.root_164()
    }
}
fn installed_scene(
    d: &mut Directory<&'static str, usize>,
    world: &crate::world_revision::ConstructedWorldRevision,
) -> crate::view_scene_init::State {
    let mut scene = crate::view_scene_init::State::from_constructor_fields();
    let mut c = SceneCamera {
        directory: d,
        zoom: Zoom {
            word: 0.5f32.to_bits(),
            calls: Vec::new(),
            fail: false,
        },
    };
    scene
        .initialize(world, &mut Input, &mut c, Precision::Double53)
        .unwrap();
    scene
}
#[test]
fn render_snapshot_requires_real_scene_parent_world_camera_and_current_backend() {
    use super::super::backend::Viewport as _;
    let (mut d, mut v, mut r, mut p) = constructors();
    let h = d
        .construct_camera(true, &Input, &mut v, &mut r, &mut p, Precision::Double53)
        .unwrap();
    let mut counter = crate::world_revision::Counter::fresh_application();
    let world = counter.construct();
    let viewport = Arc::new(Viewport {
        current: std::sync::atomic::AtomicBool::new(true),
        device: 91,
    });
    let missing = crate::view_scene_init::State::from_constructor_fields();
    assert!(d
        .snapshot_current_for_scene(&world, &missing, Arc::clone(&viewport))
        .is_err());
    let scene = installed_scene(&mut d, &world);
    let snapshot = d
        .snapshot_current_for_scene(&world, &scene, Arc::clone(&viewport))
        .unwrap();
    snapshot.require_current().unwrap();
    assert_eq!(snapshot.viewport_pixels(), [1280, 720]);
    let mut other_counter = crate::world_revision::Counter::fresh_application();
    let other_world = other_counter.construct();
    assert!(snapshot.require_world(&other_world).is_err());
    assert!(d
        .snapshot_current_for_scene(&other_world, &scene, Arc::clone(&viewport))
        .is_err());
    let (mut other, mut v, mut r, mut p) = constructors();
    other
        .construct_camera(true, &Input, &mut v, &mut r, &mut p, Precision::Double53)
        .unwrap();
    other
        .reset_current_700e30(&Input, Precision::Double53)
        .unwrap();
    assert!(other
        .snapshot_current_for_scene(&world, &scene, Arc::clone(&viewport))
        .is_err());
    let wrong = Arc::new(Viewport {
        current: std::sync::atomic::AtomicBool::new(true),
        device: 92,
    });
    assert!(d.snapshot_current_for_scene(&world, &scene, wrong).is_err());
    viewport
        .current
        .store(false, std::sync::atomic::Ordering::Release);
    assert!(snapshot.require_current().is_err());
    viewport
        .current
        .store(true, std::sync::atomic::Ordering::Release);
    d.set_zoom_captured_7005f0(
        &h,
        1f32.to_bits(),
        &mut Zoom {
            word: 0,
            calls: Vec::new(),
            fail: false,
        },
        Precision::Double53,
    )
    .unwrap();
    assert!(snapshot.require_current().is_err());
    let snapshot = d
        .snapshot_current_for_scene(&world, &scene, Arc::clone(&viewport))
        .unwrap();
    drop(scene);
    assert!(snapshot.require_current().is_err());
    let scene = installed_scene(&mut d, &world);
    let snapshot = d
        .snapshot_current_for_scene(&world, &scene, Arc::clone(&viewport))
        .unwrap();
    drop(world);
    assert!(snapshot.require_current().is_err());
}

struct PanFixture {
    calls: Vec<u32>,
}
impl PanVariables for PanFixture {
    fn camera_pan_word(&mut self, o: u32) -> Result<u32> {
        self.calls.push(o);
        Ok(match o {
            0x28 => 10f32.to_bits(),
            0x18 => 2f32.to_bits(),
            _ => panic!(),
        })
    }
}
#[test]
fn native_700500_unsigned_milliseconds_and_ordered_planar_float_stores() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-native-camera/native-camera-pan.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 18);
    for row in fixture["rows"].as_array().unwrap() {
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => panic!(),
        };
        let mut s = State::unbound_allocation();
        let basis: Vec<u32> = serde_json::from_value(row["basis"].clone()).unwrap();
        let center: Vec<u32> = serde_json::from_value(row["center"].clone()).unwrap();
        for i in 0..2 {
            s.put(0x114 + i as u32 * 4, basis[i]);
            s.put(0x24c + i as u32 * 4, center[i]);
        }
        let mut vars = PanFixture { calls: Vec::new() };
        let value = pan_target_700500(
            &s,
            row["millis"].as_u64().unwrap() as u32,
            serde_json::from_value(row["direction"].clone()).unwrap(),
            &mut vars,
            pc,
        )
        .unwrap();
        assert_eq!(serde_json::json!(value), row["target"]);
        assert_eq!(vars.calls, [0x28, 0x18]);
    }
}
