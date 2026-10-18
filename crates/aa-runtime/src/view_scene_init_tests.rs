use super::*;
use serde_json::{Value, json};
struct GlobalsInput([u32; 3]);
impl Globals for GlobalsInput {
    fn word(&mut self, a: u32) -> Result<u32> {
        Ok(self.0[((a - 0xb43cf8) / 4) as usize])
    }
}
struct CurrentCamera {
    root: Node,
    fail_reset: bool,
    events: Vec<String>,
}
impl Camera for CurrentCamera {
    fn reset_current_7010d0(&mut self) -> Result<()> {
        if self.fail_reset {
            return Err("camera reset child failed".into());
        }
        self.events.push("camera_reset".into());
        Ok(())
    }
    fn current_root_164(&mut self) -> Result<&mut Node> {
        Ok(&mut self.root)
    }
}
fn words<const N: usize>(v: &Value) -> [u32; N] {
    assert_eq!(v.as_array().unwrap().len(), N);
    std::array::from_fn(|i| v[i].as_u64().unwrap() as u32)
}
fn snapshot(state: &State) -> Value {
    let root = state.root.as_ref().unwrap();
    let context = state.light.as_ref().unwrap();
    let light = context.light.borrow();
    let effect = state.effect.as_ref();
    json!({"present":[state.root.is_some(),state.wireframe.is_some(),state.texturing.is_some(),state.light.is_some(),effect.is_some()],
        "root_local":root.local_transform().0,"root_world":root.world_transform().0,"root_parent":root.has_parent(),
        "root_children":root.child_count(),"root_properties":root.property_count(),"root_effects":root.effect_count(),
        "wire_flags":state.wireframe.as_ref().unwrap().flags,
        "texturing":[state.texturing().unwrap().apply_mode,7,7,state.texturing().unwrap().base_filter_word().unwrap()],
        "context":[context.nesting,context.flags],"light_local":light.local.0,"light_world":light.world.0,"light_direction":light.direction,
        "light_values":[light.dimmer,light.colors[0][0],light.colors[0][1],light.colors[0][2],light.colors[1][0],light.colors[1][1],light.colors[1][2],light.colors[2][0],light.colors[2][1],light.colors[2][2]],
        "planar_direction":state.planar_direction.unwrap_or([0x40800000,0]),
        "effect":effect.map(|v|vec![u32::from(v.source_texture.is_some()),v.clamp,v.filter,v.kind,u32::from(v.enabled)]),
        "effect_world":effect.map(|v|v.world.0)})
}
#[test]
fn native_scene_constructors_attachments_transforms_and_camera_failure() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/development10/view-scene-init/native-scene.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in fixture["rows"].as_array().unwrap() {
        let c = &row["input"];
        let stop = c["stop"].as_str();
        if stop.is_some_and(|s| s != "camera_reset") {
            continue;
        }
        let p = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => panic!(),
        };
        let mut g = GlobalsInput(words(&c["seed"]));
        let mut root = Node::from_native_constructor(&mut GlobalsInput([0; 3])).unwrap();
        root.set_world_transform(Transform::from_words(words(&c["parent"])).unwrap());
        let mut camera = CurrentCamera {
            root,
            fail_reset: stop == Some("camera_reset"),
            events: Vec::new(),
        };
        let mut state = State::from_constructor_fields();
        if c["existing_effect"] == true {
            let mut e = TextureEffect::constructed(&mut GlobalsInput(words(&c["seed"]))).unwrap();
            e.filter = 5;
            e.enabled = 0;
            state.effect = Some(e);
        }
        let mut application = crate::world_revision::Counter::fresh_application();
        let world = application.construct();
        let result = state.initialize(&world, &mut g, &mut camera, p);
        assert_eq!(result.is_ok(), stop.is_none(), "{row}");
        let got = snapshot(&state);
        let expected = &row["output"];
        for (key, value) in got.as_object().unwrap() {
            assert_eq!(value, &expected[key], "{key}, {row}");
        }
        if result.is_ok() {
            state.require_cpu_complete(&world).unwrap();
            assert!(state.require_renderer(&world).is_err());
        } else {
            assert!(state.require_cpu_complete(&world).is_err());
            assert!(state.initialize(&world, &mut g, &mut camera, p).is_err());
        }
        count += 1;
    }
    assert_eq!(count, 39);
}
#[test]
fn same_scene_and_world_witnesses_expire_without_minting_view_ready() {
    fn send<T: Send>() {}
    send::<SceneWitness>();
    let mut counter = crate::world_revision::Counter::fresh_application();
    let world = counter.construct();
    let mut state = State::from_constructor_fields();
    state.world = Some(world.witness());
    let witness = state.witness().unwrap();
    assert!(witness.is_for(&state.witness().unwrap()));
    witness.require_world(&world).unwrap();
    let different = counter.construct();
    assert!(witness.require_world(&different).is_err());
    drop(state);
    assert!(!witness.is_for(&witness));
    assert!(witness.require_world(&world).is_err());
}
#[test]
fn native_transform_summation_stores_and_planar_threshold() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/development10/view-scene-init/native-scene.json"
    ))
    .unwrap();
    for row in fixture["numeric"].as_array().unwrap() {
        let p = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => panic!(),
        };
        let got = if row["kind"] == "transform" {
            json!(
                compose(
                    Transform(words(&row["parent"])),
                    Transform(words(&row["local"])),
                    p
                )
                .unwrap()
                .0
            )
        } else {
            json!(planar(words(&row["direction"]), p).unwrap())
        };
        assert_eq!(got, row["result"], "{row}");
    }
    assert_eq!(fixture["numeric"].as_array().unwrap().len(), 72);
}
