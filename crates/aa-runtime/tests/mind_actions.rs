use aa_actors::{
    ActorId, ability_flags,
    components::ComponentSlot,
    creation::{ActorCreationRecord, CreationRecord, ThingDefinition},
};
use aa_body::F32;
use aa_mind::{Error, Result};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use aa_runtime::{
    actor_world::{ActorAccess, ActorStore, PlacementState, RuntimeComponent, Spatial},
    catalog::Catalog,
    mind_actions::{self, Context, Services},
    scheduler_owners::Owners,
};
use aa_simulation::NativeRng;
use serde_json::{Value, json};
struct NoPublishedSpatial;
impl Spatial for NoPublishedSpatial {
    fn placement(&self, _: ActorId) -> aa_runtime::actor_world::Result<PlacementState> {
        Err(aa_runtime::actor_world::Error::Missing(
            "test only owns staged transforms",
        ))
    }
    fn publish(&mut self, _: ActorId, _: PlacementState) -> aa_runtime::actor_world::Result<()> {
        Err(aa_runtime::actor_world::Error::Missing(
            "test does not publish",
        ))
    }
}
fn unprovided<T>() -> Result<T> {
    Err(Error::World(
        "native branch requires an unprovided test service".into(),
    ))
}
struct Leaves;
impl Services<NoPublishedSpatial> for Leaves {
    fn setting_cache108(&self) -> Result<bool> {
        unprovided()
    }
    fn setting_cached8(&self) -> Result<u32> {
        unprovided()
    }
    fn state_resupply(&self, _: u8) -> Result<bool> {
        unprovided()
    }
    fn organization_member_44(&self, _: ActorId, _: &ActorAccess<'_>) -> Result<Option<ActorId>> {
        unprovided()
    }
    fn organization_groups(
        &self,
        _: ActorId,
        _: &ActorAccess<'_>,
    ) -> Result<Vec<ability_flags::AttackGroup>> {
        unprovided()
    }
    fn owner_can_recruit(
        &mut self,
        _: ActorId,
        _: ActorId,
        _: &mut Context<'_, '_, '_, NoPublishedSpatial>,
    ) -> Result<bool> {
        unprovided()
    }
    fn owner_recruit_count(
        &mut self,
        _: ActorId,
        _: ActorId,
        _: &mut Context<'_, '_, '_, NoPublishedSpatial>,
    ) -> Result<i32> {
        unprovided()
    }
    fn mobile_deployment(
        &mut self,
        _: ActorId,
        _: u32,
        _: &mut Context<'_, '_, '_, NoPublishedSpatial>,
    ) -> Result<bool> {
        unprovided()
    }
    fn other_contributor(
        &mut self,
        _: ActorId,
        _: ComponentSlot,
        _: &mut Context<'_, '_, '_, NoPublishedSpatial>,
    ) -> Result<()> {
        unprovided()
    }
    fn refresh_owner_views(
        &mut self,
        _: Option<ActorId>,
        _: &mut Context<'_, '_, '_, NoPublishedSpatial>,
    ) -> Result<()> {
        unprovided()
    }
    fn renderer_flags(&self, a: ActorId, actors: &ActorAccess<'_>) -> Result<Option<u32>> {
        let object = actors.object(actors.key(a).unwrap()).unwrap();
        if object.thing.word_c == 0 {
            Ok(None)
        } else {
            unprovided()
        }
    }
    fn refresh_selected_view(
        &mut self,
        _: ActorId,
        _: &mut Context<'_, '_, '_, NoPublishedSpatial>,
    ) -> Result<()> {
        unprovided()
    }
}
struct Assembly;
impl aa_runtime::actor_world::AssemblyEffects for Assembly {
    fn completed(
        &mut self,
        _: &mut ActorStore,
        _: ActorId,
        _: &[ComponentSlot],
    ) -> aa_runtime::actor_world::Result<()> {
        Ok(())
    }
}
fn record(name: &str) -> ActorCreationRecord<String> {
    ActorCreationRecord {
        base: CreationRecord {
            definition: ThingDefinition {
                key: name.into(),
                flags: 0,
            },
            position_bits: [0; 3],
            direction_bits: [0, (-1f32).to_bits()],
            name_utf16: vec![],
            saved_id: ActorId::NULL,
        },
        variant: -1,
        owner_word: 0,
        word_30: 0,
        request_nonblocking: false,
    }
}
#[test]
#[ignore = "requires AA_GAME_ROOT original source and generated native action corpus"]
fn five_faction_infantry_live_actions_match_full_original_chain() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut source = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut source).unwrap();
    let catalog = Catalog::new(&actors);
    let pc = Precision::Double53;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("research/phase7-mind-world/native-live-actions.json");
    let export = std::env::var_os("AA_EXPORT_MIND_ACTIONS").is_some();
    let fixture: Value = if export {
        Value::Null
    } else {
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    };
    let mut output = Vec::new();
    for name in [
        "usa_infantry",
        "gbr_infantry",
        "ger_infantry",
        "jpn_infantry",
        "rus_infantry",
    ] {
        let mut store = ActorStore::new();
        let mut rng = NativeRng::new(0x12345678);
        let key = store
            .construct_new(&catalog, record(name), 0, &mut rng, pc, pc)
            .unwrap();
        let actor = store.initialize_object(key, &mut rng, pc).unwrap();
        let mut owners = Owners::new();
        let mut definitions = aa_runtime::definitions::Bank::default();
        store
            .assemble_ground(
                key,
                &catalog,
                F32::ZERO,
                &mut rng,
                &mut owners,
                &mut definitions,
                pc,
                &mut Assembly,
            )
            .unwrap();
        let binding = catalog.actor_bindings(name, pc).unwrap();
        let authored = binding.performer.as_ref().unwrap().abilities();
        let handles = match store.component(actor, ComponentSlot::Performer).unwrap() {
            Some(RuntimeComponent::Performer(p)) => p.state.abilities.clone(),
            _ => panic!(),
        };
        assert_eq!(authored.len(), handles.len());
        if !export {
            for row in fixture["disabled"].as_array().unwrap() {
                store.staged_placement_mut(key).unwrap().flags =
                    row["flags"].as_u64().unwrap() as u32;
                let bits = row["state"].as_u64().unwrap();
                let states = &mut store
                    .object_mut(key)
                    .unwrap()
                    .components_mut()
                    .unwrap()
                    .state;
                states.low = bits as u32;
                states.high = (bits >> 32) as u32;
                let (_, access) = store.split_minds();
                assert_eq!(
                    json!(mind_actions::disabled(&access, &NoPublishedSpatial, actor).unwrap()),
                    row["result"]
                );
            }
        }
        for flags in [0u32, 1 << 29, 1] {
            for modifier in [0u32, 1 << 11] {
                for enabled in [0u32, 1, 2, 3] {
                    store.staged_placement_mut(key).unwrap().flags = flags;
                    store
                        .object_mut(key)
                        .unwrap()
                        .components_mut()
                        .unwrap()
                        .state = Default::default();
                    store.set_action_bits(actor, 0xdeadbeef55555555).unwrap();
                    let Some(RuntimeComponent::Properties(p)) = store
                        .component_mut(actor, ComponentSlot::Properties)
                        .unwrap()
                    else {
                        panic!()
                    };
                    p.modifier.flags = modifier;
                    for (i, id) in handles.iter().enumerate() {
                        store.abilities.get_mut(id).unwrap().enabled = enabled & (1 << i) != 0;
                    }
                    let abilities:Vec<_>=authored.iter().enumerate().map(|(i,a)|json!({"enabled":enabled&(1<<i)!=0,"flags":a.flags,"required_targets":a.targets.required,"effect_kinds":a.effects.iter().map(|e|e.kind).collect::<Vec<_>>()})).collect();
                    let input = json!({"name":format!("{name}_{flags}_{modifier}_{enabled}"),"flags":flags,"modifier":modifier,"abilities":abilities});
                    if !export {
                        let native = fixture["cases"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|v| v["input"]["name"] == input["name"])
                            .unwrap();
                        assert_eq!(input, native["input"]);
                        let (arena, mut access) = store.split_minds();
                        let mut spatial = NoPublishedSpatial;
                        let mut leaves = Leaves;
                        let mut world = mind_actions::World {
                            context: Context {
                                actors: &mut access,
                                arena,
                                spatial: &mut spatial,
                                definitions: &definitions,
                                catalog: &catalog,
                                precision: pc,
                            },
                            services: &mut leaves,
                        };
                        world.rebuild(actor).unwrap();
                        let bits = world
                            .context
                            .actors
                            .action_fields(actor, world.context.spatial)
                            .unwrap()
                            .action_bits;
                        assert_eq!(json!(bits), native["actions"], "{}", input["name"]);
                    }
                    output.push(input);
                }
            }
        }
    }
    assert_eq!(output.len(), 120);
    if export {
        std::fs::write(
            root.join("research/phase7-mind-world/live-action-inputs.json"),
            serde_json::to_vec_pretty(&output).unwrap(),
        )
        .unwrap();
    }
}
