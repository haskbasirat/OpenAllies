//! SummonEffect Apply 4CCA70 and creators against
//! research/phase8-combat/native-summon-fixtures.json.
use aa_body::F32;
use aa_combat::{
    Result, ability::Ability, performer::Context, special_ops::EffectTarget, summon::*,
};
use aa_numeric::Precision;
use serde_json::{Value, json};

const OBJECT: u32 = 1;
const CAMPAIGN: u32 = 2;
const TRANSPORT: u32 = 3;
const CREATED: u32 = 10;
const TARGET: u32 = 20;
const ATTACKER: u32 = 21;
const ALLEGIANCE: u32 = 30;
const SOURCE: u32 = 40;

fn bits(v: &Value) -> F32 {
    F32(v.as_u64().unwrap() as u32)
}
fn label(id: u32) -> Value {
    match id {
        OBJECT => json!("object"),
        CAMPAIGN => json!("campaign"),
        TRANSPORT => json!("transport_def"),
        CREATED => json!("created"),
        TARGET => json!("target"),
        ATTACKER => json!("attacker"),
        ALLEGIANCE => json!("allegiance"),
        0 => json!(0),
        n => json!(format!("m{}", n - 50)),
    }
}

struct World<'a> {
    case: &'a Value,
    events: Vec<Value>,
    flags8: u32,
    descriptors: Vec<(Creator, Descriptor)>,
}
impl EffectTarget for World<'_> {
    fn effect_target(&mut self, _: &Context) -> Result<Option<u32>> {
        Ok(self.case["target"].as_bool().unwrap().then_some(TARGET))
    }
}
impl SummonWorld for World<'_> {
    fn definition_flags(&self, _: u32) -> Result<u32> {
        Ok(if self.case["target_location"].as_bool().unwrap() {
            0x20
        } else {
            0
        })
    }
    fn campaign_mode(&self) -> Result<bool> {
        Ok(self.case["campaign"].as_bool().unwrap())
    }
    fn actor_position(&self, actor: u32) -> Result<[F32; 3]> {
        assert_eq!(actor, SOURCE);
        let p = &self.case["source_actor_position"];
        Ok([bits(&p[0]), bits(&p[1]), F32::ZERO])
    }
    fn actor_facing(&self, _: u32) -> Result<[F32; 2]> {
        let f = &self.case["facing"];
        Ok([bits(&f[0]), bits(&f[1])])
    }
    fn actor_extent(&self, _: u32) -> Result<F32> {
        Ok(bits(&self.case["extent"]))
    }
    fn random_direction(&mut self) -> Result<[F32; 2]> {
        self.events.push(json!({"random_rng": true}));
        self.events.push(json!({"random_direction": true}));
        let r = &self.case["random_direction"];
        Ok([bits(&r[0]), bits(&r[1])])
    }
    fn object_is_actor(&self, _: u32) -> Result<bool> {
        Ok(self.case["kind"] != "object")
    }
    fn object_ethereal(&self, _: u32) -> Result<bool> {
        Ok(self.case["ethereal"].as_bool().unwrap())
    }
    fn object_transport(&self, _: u32) -> Result<Option<u32>> {
        Ok((self.case["kind"] == "transported").then_some(TRANSPORT))
    }
    fn object_is_organization(&self, _: u32) -> Result<bool> {
        Ok(self.case["kind"] == "organization")
    }
    fn find_open_position(
        &mut self,
        position: &mut [F32; 2],
        direction: [F32; 2],
        object: u32,
    ) -> Result<bool> {
        self.events
            .push(json!({"open": [position[0].0, position[1].0],
            "dir": [direction[0].0, direction[1].0], "object": label(object),
            "radius": 0x4140_0000u32, "a4": 0, "a5": 1}));
        let ok = self.case["open_ok"].as_bool().unwrap();
        if ok {
            let s = &self.case["open_shift"];
            *position = [bits(&s[0]), bits(&s[1])];
        }
        Ok(ok)
    }
    fn log(&mut self) -> Result<()> {
        self.events.push(json!({"log": true}));
        Ok(())
    }
    fn summon_failed(&mut self, allegiance: Option<u32>) -> Result<()> {
        self.events
            .push(json!({"failed": label(allegiance.unwrap_or(0))}));
        Ok(())
    }
    fn create(&mut self, creator: Creator, descriptor: &Descriptor) -> Result<Option<u32>> {
        self.descriptors.push((creator, *descriptor));
        let name = match creator {
            Creator::Generic => "create_object",
            Creator::Actor => "create_actor",
            Creator::Organization => "create_organization",
        };
        // Compared field by field in the test body.
        self.events.push(json!({name: "descriptor", "world": true}));
        Ok(self.case["create_ok"].as_bool().unwrap().then_some(CREATED))
    }
    fn set_uncontrollable(&mut self, actor: u32) -> Result<()> {
        assert_eq!(actor, CREATED);
        self.flags8 |= 4;
        Ok(())
    }
    fn sai_watching(&self) -> Result<bool> {
        Ok(self.case["sai"].as_u64() == Some(1))
    }
    fn notify_sai(&mut self, actor: u32) -> Result<()> {
        self.events.push(json!({"sai": label(actor)}));
        Ok(())
    }
    fn actor_id(&self, _: u32) -> Result<u32> {
        Ok(self.case["created_gtid"].as_u64().unwrap() as u32)
    }
    fn apply_property(&mut self, actor: u32, _: u32) -> Result<()> {
        self.events
            .push(json!({"property": label(actor), "value": "property", "kind": 3, "source": 0}));
        Ok(())
    }
    fn organization_members(&mut self, _: u32) -> Result<Option<Vec<Option<u32>>>> {
        if self.case["kind"] != "organization" {
            return Ok(None);
        }
        Ok(Some(
            self.case["members"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(i, m)| (!m.is_null()).then_some(50 + i as u32))
                .collect(),
        ))
    }
    fn emit_fx(&mut self, _: &mut Ability, event: u32, context: &mut Context) -> Result<()> {
        self.events.push(json!({"fx": "ability", "event": event,
            "target": label(context.target.unwrap())}));
        Ok(())
    }
    fn target_attacks(&mut self, target: u32, summoned: u32) -> Result<bool> {
        assert_eq!(target, TARGET);
        self.events.push(json!({"attacks_query": label(summoned)}));
        Ok(self.case["target_attacks"].as_bool().unwrap())
    }
    fn body_attacker(&self, _: u32) -> Result<Option<u32>> {
        Ok((self.case["body_attacker"] == "live").then_some(ATTACKER))
    }
    fn order_attack(&mut self, summoned: u32, target: u32) -> Result<()> {
        self.events.push(json!({"command": label(target)}));
        self.events
            .push(json!({"issue": label(summoned), "command_words": "attack"}));
        self.events.push(json!({"command_done": true}));
        Ok(())
    }
    fn map_size(&self) -> Result<[F32; 2]> {
        let m = &self.case["map"];
        Ok([bits(&m[0]), bits(&m[1])])
    }
    fn clamp_map_position(&mut self, position: &mut [F32; 2]) -> Result<()> {
        self.events
            .push(json!({"clamp": [position[0].0, position[1].0]}));
        Ok(())
    }
    fn transport_altitude(&self, transport: u32) -> Result<F32> {
        assert_eq!(transport, TRANSPORT);
        Ok(F32(40.0f32.to_bits()))
    }
    fn load_transport(
        &mut self,
        transport: u32,
        object: u32,
        organization_data: bool,
        start: [F32; 2],
        drop: [F32; 2],
    ) -> Result<()> {
        assert_eq!(transport, CREATED);
        if organization_data {
            self.events.push(json!({"organization_data": 0x2F0F0}));
        }
        self.events.push(json!({"payload": label(object),
            "organization": if organization_data { 0x0D0D0 } else { 0 },
            "start": [start[0].0, start[1].0], "drop": [drop[0].0, drop[1].0], "component": true}));
        Ok(())
    }
    fn order_move(&mut self, transport: u32, point: [F32; 2]) -> Result<()> {
        self.events.push(json!({"command_kind": 5}));
        self.events.push(json!({"issue": label(transport),
            "command_words": ["move", point[0].0, point[1].0]}));
        self.events.push(json!({"command_done": true}));
        Ok(())
    }
}

/// Native descriptor record: [def, x, y, z, dirx, diry, +20, +24, +28 ...].
fn check_descriptor(native: &Value, creator: Creator, descriptor: &Descriptor) {
    let w: Vec<u64> = native.as_array().unwrap()[1..]
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    let object = match descriptor {
        Descriptor::Object(o) => o,
        Descriptor::Actor(a) => &a.object,
    };
    assert_eq!(native[0], label(object.definition));
    let fields = [
        object.position[0].0,
        object.position[1].0,
        object.position[2].0,
        object.direction[0].0,
        object.direction[1].0,
    ];
    assert_eq!(
        &w[..5],
        fields.map(u64::from).as_slice(),
        "{native} {creator:?}"
    );
    assert_eq!((w[5], w[6]), (0xffff_ffff, 0));
    if let Descriptor::Actor(a) = descriptor {
        assert_eq!(w[7] != 0, a.allegiance.is_some());
        assert_eq!((w[8], w[9], w[10]), (0, 0, 0xffff_ffff));
        assert!(w[11..17].iter().all(|&x| x == 0), "{native}");
        assert_eq!(w[17] & 0xff_ffff, u64::from(a.flag51) << 8);
    }
}

#[test]
fn summon_apply_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-summon-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 160);
    for case in cases {
        let effect = SummonEffect {
            object: OBJECT,
            dynamic_object: CAMPAIGN,
            property: case["property"].as_bool().unwrap().then_some(99),
            controllable: case["controllable"].as_bool().unwrap(),
        };
        let mut ability = Ability::new(7, None, F32::ZERO);
        let mut context = Context::new(0);
        context.source = case["source"].as_bool().unwrap().then_some(SOURCE);
        context.allegiance = case["allegiance"].as_bool().unwrap().then_some(ALLEGIANCE);
        let sp = &case["source_position"];
        let tp = &case["target_position"];
        context.source_position = [bits(&sp[0]), bits(&sp[1]), F32::ZERO];
        context.target_position = [bits(&tp[0]), bits(&tp[1]), F32::ZERO];
        context.target = Some(0xCAFE);
        let mut world = World {
            case,
            events: vec![],
            flags8: 0x100,
            descriptors: vec![],
        };
        let got = apply(
            &effect,
            &mut ability,
            &mut context,
            &mut world,
            Precision::Double53,
        )
        .unwrap();
        assert_eq!(u64::from(got), case["returned"].as_u64().unwrap(), "{case}");
        assert_eq!(context.target, Some(0xCAFE));
        let native = case["events"].as_array().unwrap();
        assert_eq!(world.events.len(), native.len(), "{case}");
        let mut descriptors = world.descriptors.iter();
        for (rust, native) in world.events.iter().zip(native) {
            if let Some((name, _)) = rust
                .as_object()
                .unwrap()
                .iter()
                .find(|(k, _)| k.starts_with("create_"))
            {
                let (creator, descriptor) = descriptors.next().unwrap();
                check_descriptor(&native[name], *creator, descriptor);
                assert_eq!(native["world"], true);
            } else if rust.get("issue").is_some() {
                assert_eq!(rust["issue"], native["issue"]);
                let words = &native["command_words"];
                if rust["command_words"] != "attack" {
                    assert_eq!(words[0], 0x1C, "{case}");
                    assert_eq!(
                        &words.as_array().unwrap()[1..3],
                        &rust["command_words"].as_array().unwrap()[1..3]
                    );
                }
            } else {
                assert_eq!(rust, native, "{case}");
            }
        }
        if case["created_flags8"].is_u64() {
            let expected = case["created_flags8"].as_u64().unwrap() as u32;
            let touched = got || case["kind"] == "transported";
            if touched {
                assert_eq!(world.flags8, expected, "{case}");
            }
        }
        let summoned: Vec<u64> = ability.summoned_ids.iter().map(|&i| u64::from(i)).collect();
        let native: Vec<u64> = case["summoned"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();
        assert_eq!(summoned, native, "{case}");
    }
}
