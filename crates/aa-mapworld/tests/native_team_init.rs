use aa_actors::{
    ActorId,
    loading::{LoadKind, ThingShell, construct_shell},
};
use aa_mapworld::team_init::{self, Error, Result, World};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};

struct Services<'a> {
    thing: &'a mut ThingShell<String>,
    input: &'a Value,
    events: Vec<Value>,
    height_stores: Vec<u32>,
}
impl Services<'_> {
    fn fail(&self, name: &str) -> Result<()> {
        if self.input["failure"] == name {
            Err(Error::World(name.into()))
        } else {
            Ok(())
        }
    }
}
impl World for Services<'_> {
    fn flags(&self, _: ActorId) -> Result<u32> {
        Ok(self.thing.flags)
    }
    fn tile_size_bits(&self) -> Result<u32> {
        Ok(self.input["tile"].as_u64().unwrap() as u32)
    }
    fn snap_to_grid(&mut self, _: ActorId, tile: u32, _: Precision) -> Result<()> {
        self.events.push(json!(["snap", tile]));
        for (i, b) in self.input["snapped"].as_array().unwrap().iter().enumerate() {
            self.thing.position_bits[i] = b.as_u64().unwrap() as u32;
        }
        self.fail("snap")
    }
    fn check_placement_null(&mut self, _: ActorId, _: Precision) -> Result<bool> {
        self.events
            .push(json!(["placement", self.thing.position_bits]));
        self.fail("placement")?;
        Ok(self.input["placement_return"].as_u64().unwrap() as u8 != 0)
    }
    fn update_index_membership(&mut self, _: ActorId) -> Result<()> {
        self.events.push(json!(["index"]));
        if let Some(flags) = self.input["index_flags"].as_u64() {
            self.thing.flags = flags as u32
        }
        self.fail("index")
    }
    fn position_bits(&self, _: ActorId) -> Result<[u32; 3]> {
        Ok(self.thing.position_bits)
    }
    fn terrain_height(&mut self, position: [u32; 3], flags: u32, _: Precision) -> Result<Binary> {
        self.events.push(json!(["height", position, flags]));
        self.fail("height")?;
        Ok(Binary::from_f64_bits(
            self.input["height_f64_bits"].as_u64().unwrap(),
        )?)
    }
    fn set_height_bits(&mut self, _: ActorId, bits: u32) -> Result<()> {
        self.thing.position_bits[2] = bits;
        self.height_stores.push(bits);
        Ok(())
    }
    fn name_utf16(&self, _: ActorId) -> Result<&[u16]> {
        Ok(&self.thing.name_utf16)
    }
    fn log_team_initialized(&mut self, _: ActorId, name: &[u16]) -> Result<()> {
        self.events.push(json!(["log", name]));
        self.fail("log")
    }
}
#[test]
fn original_and_v14_saved_team_init_and_dependency_prefixes_match() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-team-init/native-team-init.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 240);
    for row in cases {
        let c = &row["input"];
        let mut shell = construct_shell(LoadKind::Thing, "explicit test Team".to_owned());
        shell.thing.flags = c["flags"].as_u64().unwrap() as u32;
        shell.thing.position_bits = serde_json::from_value(c["position"].clone()).unwrap();
        shell.thing.name_utf16 = serde_json::from_value(c["name"].clone()).unwrap();
        let mut world = Services {
            thing: &mut shell.thing,
            input: c,
            events: vec![],
            height_stores: vec![],
        };
        let pc: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let result = team_init::init_saved(&mut world, ActorId::from_raw(1), pc);
        let output = &row["output"];
        assert_eq!(result.is_ok(), output["outcome"] == "ok", "{row}");
        if let Ok(value) = result {
            assert_eq!(Some(value), output["result"].as_bool(), "{row}")
        }
        assert_eq!(json!(world.events), output["events"], "{row}");
        assert_eq!(json!(world.height_stores), output["height_stores"], "{row}");
        assert_eq!(
            json!(world.thing.position_bits),
            output["position"],
            "{row}"
        );
        assert_eq!(json!(world.thing.flags), output["flags"], "{row}");
        assert_eq!(json!(world.thing.name_utf16), output["name"], "{row}");
    }
}

/// No actual spatial service is supplied by this source-only test. Every
/// dependency is a required error; reading real Team fields must never turn
/// an absent placement service into Init=true.
struct ActualTeam<'a>(&'a mut ThingShell<String>);
fn unbound<T>(name: &str) -> Result<T> {
    Err(Error::World(name.into()))
}
impl World for ActualTeam<'_> {
    fn flags(&self, _: ActorId) -> Result<u32> {
        Ok(self.0.flags)
    }
    fn tile_size_bits(&self) -> Result<u32> {
        unbound("world tile size")
    }
    fn snap_to_grid(&mut self, _: ActorId, _: u32, _: Precision) -> Result<()> {
        unbound("snap")
    }
    fn check_placement_null(&mut self, _: ActorId, _: Precision) -> Result<bool> {
        unbound("4EF6A0 actual world placement")
    }
    fn update_index_membership(&mut self, _: ActorId) -> Result<()> {
        unbound("4EE4C0 actual spatial membership")
    }
    fn position_bits(&self, _: ActorId) -> Result<[u32; 3]> {
        Ok(self.0.position_bits)
    }
    fn terrain_height(&mut self, _: [u32; 3], _: u32, _: Precision) -> Result<Binary> {
        unbound("terrain height")
    }
    fn set_height_bits(&mut self, _: ActorId, bits: u32) -> Result<()> {
        self.0.position_bits[2] = bits;
        Ok(())
    }
    fn name_utf16(&self, _: ActorId) -> Result<&[u16]> {
        Ok(&self.0.name_utf16)
    }
    fn log_team_initialized(&mut self, _: ActorId, _: &[u16]) -> Result<()> {
        unbound("Team diagnostic")
    }
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source assets"]
fn actual_skirmish_and_tutorial_team_fields_require_real_placement_service() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
    };
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let pc = Precision::Double53;
    let mut count = 0;
    for name in [
        "DeveloperMaps/(3v3)Firefight.RMP",
        "Campaign/tutorial02.RMP",
    ] {
        let map = aa_mapworld::MapArchive::from_bytes(
            std::fs::read(
                root.join("reconstruction/assets/original/Data/Maps")
                    .join(name),
            )
            .unwrap(),
        )
        .unwrap();
        let chunk = map.decode_chunk("TEAMS", u64::MAX).unwrap();
        let teams = aa_mapworld::kingdoms::Teams::decode(&chunk).unwrap();
        for source in teams.slots.iter().filter_map(|slot| slot.record.as_ref()) {
            let ids = String::from_utf16(&source.definition_id_utf16).unwrap();
            let flags = actors.native_final_flags(&ids, pc).unwrap();
            let mut shell = construct_shell(LoadKind::Thing, ids.clone());
            shell.thing.flags = source.thing.runtime_flags(shell.thing.flags, flags);
            shell.thing.word_1c = Some(source.thing.word_1c);
            shell.thing.position_bits = source.thing.position_bits;
            shell.thing.facing_bits = source.thing.facing_bits;
            assert!(
                source.thing.name_utf16.len() <= 1000,
                "native name-length branch differs"
            );
            shell.thing.name_utf16 = source.thing.name_utf16.clone();
            assert_eq!(
                shell.thing.flags & (1 << 16),
                0,
                "real Team has no tile-lock snap"
            );
            let before = (
                shell.thing.flags,
                shell.thing.position_bits,
                shell.thing.name_utf16.clone(),
            );
            let mut world = ActualTeam(&mut shell.thing);
            let actor = ActorId::from_raw(source.serialized_id);
            let error = team_init::init_saved(&mut world, actor, pc).unwrap_err();
            assert!(error.to_string().contains("4EF6A0"), "{error}");
            assert_eq!(
                (
                    world.0.flags,
                    world.0.position_bits,
                    world.0.name_utf16.clone()
                ),
                before
            );
            println!(
                "{name}: Team slot={} id={:#x} definition={ids} flags={:#x} name={:?} source={}",
                source.slot_index,
                source.serialized_id,
                world.0.flags,
                String::from_utf16_lossy(&world.0.name_utf16),
                teams.source.sha256
            );
            count += 1;
        }
    }
    assert!(count > 0);
    println!("actual Team records checked: {count}");
}
