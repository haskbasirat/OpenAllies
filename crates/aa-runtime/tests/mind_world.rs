use aa_actors::{
    ActorId,
    components::ComponentSlot,
    creation::{ActorCreationRecord, CreationRecord, ThingDefinition},
};
use aa_body::F32;
use aa_mind::{Error, MindId, Result, StateId, World as _, command, construct::Kind};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use aa_runtime::{
    actor_world::{ActiveComponent, ActorAccess, ActorStore, RuntimeComponent},
    catalog::Catalog,
    mind_world::{self, Context, PlainCommand, Services, StateBinding},
    scheduler_owners::{Owners, Target},
};
use aa_simulation::NativeRng;
use serde_json::{Value, json};
fn missing<T>() -> Result<T> {
    Err(Error::World(
        "test reached an unprovided original service".into(),
    ))
}
#[derive(Default)]
struct Leaves {
    events: Vec<Value>,
    reenter: bool,
    fail_stance: bool,
}
impl Services for Leaves {
    fn can_activate_other(
        &mut self,
        _: StateId,
        _: StateBinding,
        _: &mut Context<'_, '_>,
    ) -> Result<bool> {
        missing()
    }
    fn enter_other(&mut self, _: StateId, _: StateBinding, _: &mut Context<'_, '_>) -> Result<()> {
        missing()
    }
    fn leave_other(&mut self, _: StateId, _: StateBinding, _: &mut Context<'_, '_>) -> Result<()> {
        missing()
    }
    fn actor_state_other(
        &mut self,
        _: StateId,
        _: StateBinding,
        _: &mut Context<'_, '_>,
    ) -> Result<u8> {
        missing()
    }
    fn set_actor_state(&mut self, _: ActorId, index: u8, _: &mut Context<'_, '_>) -> Result<()> {
        self.events.push(json!(["state", 0, index, 1]));
        Ok(())
    }
    fn rebuild_actions(&mut self, _: ActorId, _: &mut Context<'_, '_>) -> Result<()> {
        self.events.push(json!(["rebuild", 0]));
        Ok(())
    }
    fn default_stance(&mut self, actor: ActorId, c: &mut Context<'_, '_>) -> Result<()> {
        self.events.push(json!(["stance", 0]));
        if self.fail_stance {
            return missing();
        }
        if self.reenter {
            let Some(RuntimeComponent::Mind(m)) =
                c.actors.component(actor, ComponentSlot::Mind).unwrap()
            else {
                panic!()
            };
            let id = m.mind_id;
            let mut w = mind_world::World {
                actors: c.actors.reborrow(),
                runtime: c.runtime,
                queues: c.queues,
                owners: c.owners,
                services: self,
            };
            w.reset_default(id, c.arena)?;
        }
        Ok(())
    }
    fn reset_other(&mut self, _: MindId, _: Kind, _: &mut Context<'_, '_>) -> Result<()> {
        missing()
    }
    fn sync_state_other(&mut self, _: MindId, _: Kind, _: &mut Context<'_, '_>) -> Result<()> {
        missing()
    }
    fn facing(&self, _: ActorId, _: &ActorAccess<'_>) -> Result<[u32; 2]> {
        missing()
    }
    fn set_facing(&mut self, _: ActorId, _: [u32; 2], _: &mut Context<'_, '_>) -> Result<()> {
        missing()
    }
    fn actor_flags(&self, a: ActorId, s: &ActorAccess<'_>) -> Result<u32> {
        Ok(s.object(s.key(a).unwrap())
            .unwrap()
            .actor
            .as_ref()
            .unwrap()
            .words
            .get(&8)
            .copied()
            .unwrap_or(0))
    }
    fn has_live_group(&self, _: ActorId, _: &ActorAccess<'_>) -> Result<bool> {
        Ok(false)
    }
    fn finish_group_commands(&mut self, _: ActorId, _: &mut Context<'_, '_>) -> Result<()> {
        missing()
    }
    fn invoke_command(
        &mut self,
        _: MindId,
        _: u32,
        _: u32,
        _: &mut command::Context,
        _: &mut Context<'_, '_>,
    ) -> Result<bool> {
        missing()
    }
    fn deliver_stop(&mut self, _: ActorId, _: &mut Context<'_, '_>) -> Result<bool> {
        missing()
    }
    fn defers_other(
        &mut self,
        _: StateId,
        _: StateBinding,
        _: &mut Context<'_, '_>,
    ) -> Result<bool> {
        missing()
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
fn plain() -> PlainCommand {
    PlainCommand {
        definition_id: 1,
        definition: Some(command::Definition {
            component_slot: 4,
            mind_type: 6,
            behavior_flags: 0,
        }),
        flags: 0,
        word10: 0,
        object_definition: None,
        position_bits: [0; 2],
        auxiliary_bits: [0; 2],
        scalar_bits: 0,
        parameter_definition: None,
        extra_definition: None,
        boolean: 0,
        trailing: 0,
    }
}
#[test]
#[ignore = "requires AA_GAME_ROOT original sources"]
fn actual_actor_store_matches_original_mind_callbacks_and_queue_selection() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let catalog = Catalog::new(&actors);
    let pc = Precision::Double53;
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-mind-world/native-world.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in fixture["cases"].as_array().unwrap() {
        let c = &row["input"];
        let expected = &row["output"];
        let kind = c["kind"].as_str().unwrap();
        // Aircraft factories require their separate flight components; their two
        // native vtable checks are source proof, not claimed ground-store execution.
        if kind == "aircraft" {
            continue;
        }
        let mut store = ActorStore::new();
        let mut rng = NativeRng::new(0x12345678);
        let name = if kind == "regiment" {
            "regiment_usa_infantry"
        } else {
            "usa_infantry"
        };
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
        let Some(RuntimeComponent::Mind(m)) = store.component(actor, ComponentSlot::Mind).unwrap()
        else {
            panic!()
        };
        let id = m.mind_id;
        let bindings = m.state_bindings.clone();
        let mut runtime = mind_world::Runtime::default();
        let cmd = runtime.commands.allocate(plain()).unwrap();
        if c["current"].as_bool().unwrap() {
            runtime.commands.retain(cmd).unwrap();
            store.minds.mind_mut(id).unwrap().current_command = Some(cmd);
        }
        for &s in bindings.values() {
            store.minds.state_mut(s).unwrap().status = 17;
        }
        store.minds.mind_mut(id).unwrap().stack = c["stack"]
            .as_array()
            .unwrap()
            .iter()
            .map(|off| off.as_u64().map(|v| bindings[&(v as u32)]))
            .collect();
        store
            .object_mut(key)
            .unwrap()
            .components_mut()
            .unwrap()
            .state
            .high = if c["state33"].as_bool().unwrap() {
            2
        } else {
            0
        };
        store.object_mut(key).unwrap().active_component = match c["active"].as_str().unwrap() {
            "performer" => ActiveComponent::Slot(ComponentSlot::Performer),
            "mover" => ActiveComponent::Slot(ComponentSlot::Mover),
            _ => ActiveComponent::None,
        };
        if let Some(RuntimeComponent::Performer(p)) = store
            .component_mut(actor, ComponentSlot::Performer)
            .unwrap()
        {
            p.state.stage = c["stage"].as_i64().unwrap() as i32;
        }
        if let Some(RuntimeComponent::Mind(m)) =
            store.component_mut(actor, ComponentSlot::Mind).unwrap()
        {
            m.bytes.insert(0x144, 0);
        }
        let plan =
            aa_world::initialization::ClockPlan::from_tick_rate(10f32.to_bits(), pc).unwrap();
        let mut queues = plan.initialize_queues(plan.new_clock(), pc).unwrap();
        let owner = owners
            .get(Target::Component {
                object: key,
                slot: 4,
            })
            .unwrap();
        for data in c["pending"].as_array().unwrap() {
            queues
                .add_after(4, owner, data.as_u64().unwrap() as u32, 0)
                .unwrap();
        }
        if c["mover_pending"].as_bool().unwrap() {
            let ch = owners
                .get(Target::Component {
                    object: key,
                    slot: 3,
                })
                .unwrap();
            queues.add_after(4, ch, 88, 0).unwrap();
        }
        let mut leaves = Leaves {
            reenter: c["reenter"].as_bool().unwrap(),
            ..Default::default()
        };
        let (arena, access) = store.split_minds();
        let mut w = mind_world::World {
            actors: access,
            runtime: &mut runtime,
            queues: &mut queues,
            owners: &mut owners,
            services: &mut leaves,
        };
        let state = bindings
            .get(&(c["offset"].as_u64().unwrap() as u32))
            .copied();
        let op = c["operation"].as_str().unwrap();
        let mut answer = Value::Null;
        match op {
            "can" => answer = json!(w.can_activate(state.unwrap(), arena).unwrap()),
            "defers" => {
                answer = json!(
                    command::World::defers_rejected_command(&mut w, state.unwrap(), arena).unwrap()
                )
            }
            "enter" => w.enter(state.unwrap(), arena).unwrap(),
            "leave" => w.leave(state.unwrap(), arena).unwrap(),
            "sync" => {
                w.init_load(id, arena).unwrap();
            }
            "changed" => w.state_changed(id, arena).unwrap(),
            "init" => answer = json!(w.init_load(id, arena).unwrap()),
            "create" => answer = json!(w.init_create(id, arena).unwrap()),
            "default" => w.reset_default(id, arena).unwrap(),
            "schedule" => w
                .schedule(id, c["delay"].as_u64().unwrap() as u32, 77, arena)
                .unwrap(),
            _ => panic!(),
        }
        let actual_stack: Vec<_> = arena
            .mind(id)
            .unwrap()
            .stack
            .iter()
            .map(|s| s.map(|s| *bindings.iter().find(|(_, v)| **v == s).unwrap().0))
            .collect();
        assert_eq!(answer, expected["answer"], "{}", c["name"]);
        assert_eq!(json!(actual_stack), expected["stack"], "{}", c["name"]);
        if kind == "element" {
            let statuses: std::collections::BTreeMap<_, _> = bindings
                .iter()
                .map(|(off, s)| (off.to_string(), arena.state(*s).unwrap().status))
                .collect();
            assert_eq!(json!(statuses), expected["statuses"], "{}", c["name"]);
            let Some(RuntimeComponent::Mind(m)) =
                w.actors.component(actor, ComponentSlot::Mind).unwrap()
            else {
                panic!()
            };
            assert_eq!(
                json!(m.bytes[&0x144]),
                expected["engage_byte"],
                "{}",
                c["name"]
            );
        }
        assert_eq!(
            json!(arena.mind(id).unwrap().current_command.is_some()),
            expected["current"],
            "{}",
            c["name"]
        );
        // Native fixture reserves one external reference in addition to current.
        let refs = w.runtime.commands.references(cmd).unwrap();
        let original_refs = expected["references"].as_u64().unwrap() as u32;
        assert_eq!(
            refs,
            original_refs - u32::from(!c["current"].as_bool().unwrap()),
            "{}",
            c["name"]
        );
        let events: Vec<_> = expected["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e[0] != "cancel" && e[0] != "schedule")
            .cloned()
            .collect();
        assert_eq!(w.services.events, events, "{}", c["name"]);
        let scheduled = expected["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e[0] == "schedule");
        if let Some(e) = expected["events"]
            .as_array()
            .unwrap()
            .iter()
            .rev()
            .find(|e| e[0] == "schedule")
        {
            assert!(
                w.queues.has(owner, 4, e[3].as_u64().unwrap() as u32),
                "{}",
                c["name"]
            );
        } else {
            assert!(!w.queues.has(owner, 4, 77), "{}", c["name"]);
        }
        if scheduled {
            for data in c["pending"].as_array().unwrap() {
                assert!(
                    !w.queues.has(owner, 4, data.as_u64().unwrap() as u32),
                    "{}",
                    c["name"]
                );
            }
        }
        if c["name"] == "default_False_False" {
            // A failing default-stance service follows the committed status write.
            w.services.fail_stance = true;
            arena.mind_mut(id).unwrap().stack.clear();
            assert!(w.reset_default(id, arena).is_err());
            assert_eq!(arena.state(bindings[&0x68]).unwrap().status, 2);
            w.services.fail_stance = false;
            let before = w.services.events.len();
            w.reset_default(id, arena).unwrap();
            assert!(
                w.services.events.len() > before,
                "global reset guard must release on service error"
            );
            let stop = w.runtime.commands.allocate(plain()).unwrap();
            assert!(
                command::dispatch(id, stop, arena, &mut w, &mut command::Context::default())
                    .unwrap()
            );
            assert_eq!(arena.mind(id).unwrap().current_command, Some(stop));
            assert_eq!(
                arena.mind(id).unwrap().stack.front(),
                Some(&Some(bindings[&0x68]))
            );
            w.runtime.commands.release(stop).unwrap();
            assert_eq!(w.runtime.commands.references(stop).unwrap(), 1);
            command::set_current(id, None, arena, &mut w).unwrap();
            assert!(w.runtime.commands.value(stop).is_err());
        }
        if c["operation"] == "create" {
            let fixture: Value = serde_json::from_str(include_str!(
                "../../../research/phase7-mind-world/native-plain-command.json"
            ))
            .unwrap();
            let off = if kind == "element" { 0x7c } else { 0x68 };
            for value in fixture["moving"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| v["kind"] == kind)
            {
                arena.state_mut(bindings[&off]).unwrap().status =
                    value["status"].as_u64().unwrap() as u32;
                assert_eq!(
                    json!(mind_world::moving(&w.actors, arena, actor).unwrap()),
                    value["result"]
                );
            }
        }
        count += 1;
    }
    assert_eq!(count, 164);
}
#[test]
fn plain_command_ownership_is_unique_and_fallible() {
    let mut runtime = mind_world::Runtime::default();
    let a = runtime.commands.allocate(plain()).unwrap();
    let b = runtime.commands.clone_owned(a).unwrap();
    assert_ne!(a, b);
    assert_eq!(
        runtime.commands.value(a).unwrap(),
        runtime.commands.value(b).unwrap()
    );
    runtime.commands.retain(a).unwrap();
    runtime.commands.release(a).unwrap();
    assert_eq!(runtime.commands.references(a).unwrap(), 1);
    runtime.commands.release(a).unwrap();
    assert!(runtime.commands.value(a).is_err());
    assert!(runtime.commands.release(a).is_err());
    assert_eq!(runtime.commands.references(b).unwrap(), 1);
    let c = runtime.commands.allocate(plain()).unwrap();
    assert!(c > b);
}

#[test]
fn original_plain_command_copy_preserves_every_initialized_field() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-mind-world/native-plain-command.json"
    ))
    .unwrap();
    for row in fixture["cases"].as_array().unwrap() {
        let raw: Vec<u32> = row["words"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect();
        let value = PlainCommand {
            definition_id: raw[0],
            definition: plain().definition,
            flags: raw[1],
            word10: raw[2],
            object_definition: Some(raw[3]),
            position_bits: [raw[4], raw[5]],
            auxiliary_bits: [raw[6], raw[7]],
            scalar_bits: raw[8],
            parameter_definition: Some(raw[9]),
            extra_definition: Some(row["extra"].as_u64().unwrap() as u32),
            boolean: u8::from(row["boolean"].as_bool().unwrap()),
            trailing: u8::from(row["trailing"].as_bool().unwrap()),
        };
        let mut commands = mind_world::Commands::default();
        let a = commands.allocate(value).unwrap();
        let b = commands.clone_owned(a).unwrap();
        let v = commands.value(b).unwrap();
        assert_eq!(
            json!([
                v.definition_id,
                v.flags,
                v.word10,
                v.object_definition.unwrap(),
                v.position_bits[0],
                v.position_bits[1],
                v.auxiliary_bits[0],
                v.auxiliary_bits[1],
                v.scalar_bits,
                v.parameter_definition.unwrap()
            ]),
            row["copied"]
        );
        assert_eq!(json!(v.extra_definition.unwrap()), row["extra_copied"]);
        assert_eq!(json!(v.boolean != 0), row["boolean_copied"]);
        assert_eq!(json!(v.trailing != 0), row["trailing_copied"]);
        assert_eq!(commands.references(a).unwrap(), 1);
        assert_eq!(commands.references(b).unwrap(), 1);
    }
}
