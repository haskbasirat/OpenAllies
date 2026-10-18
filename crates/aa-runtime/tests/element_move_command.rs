//! Source-bound stores replay native538340 fixtures; activation leaves are
//! explicit oracle seams. No fixture scene is published or claimed playable.
use aa_actors::{
    ActorId,
    components::ComponentSlot,
    creation::{ActorCreationRecord, CreationRecord, ThingDefinition},
};
use aa_body::F32;
use aa_mind::{Error, MindId, Result, StateId, command, construct::Kind};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use aa_runtime::{
    actor_world::{ActorAccess, ActorStore, RuntimeComponent},
    catalog::Catalog,
    element_move_command::{self as adapter, Context as MoveContext},
    mind_world::{self, Context, PlainCommand, StateBinding},
    scheduler_owners::{Owners, Target},
    spatial_world::SpatialWorld,
};
use aa_simulation::NativeRng;
use aa_spatial::collision::Grid;
use serde_json::{Value, json};

fn missing<T>() -> Result<T> {
    Err(Error::World("explicit test service is unbound".into()))
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
fn record() -> ActorCreationRecord<String> {
    ActorCreationRecord {
        base: CreationRecord {
            definition: ThingDefinition {
                key: "usa_infantry".into(),
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
fn command(c: &Value, target: ActorId) -> PlainCommand {
    let source = aa_rules::native::actor_command::predefined_actor_command("move").unwrap();
    PlainCommand {
        definition_id: 1,
        definition: Some(command::Definition {
            component_slot: source.component_slot,
            mind_type: source.mind_type,
            behavior_flags: source.behavior_flags,
        }),
        flags: c["flags"].as_u64().unwrap() as u32,
        word10: match c["target"].as_u64().unwrap() as u32 {
            0x10002 => target.raw(),
            other => other,
        },
        object_definition: None,
        position_bits: serde_json::from_value(c["position_bits"].clone()).unwrap(),
        auxiliary_bits: serde_json::from_value(c["auxiliary_bits"].clone()).unwrap(),
        scalar_bits: c["speed_bits"].as_u64().unwrap() as u32,
        parameter_definition: None,
        extra_definition: None,
        boolean: 0,
        trailing: 0,
    }
}
struct Leaves {
    input: Value,
    events: Vec<Value>,
    accept_count: usize,
    command: u32,
    actor: ActorId,
    target: ActorId,
    states: std::collections::BTreeMap<StateId, u32>,
    refuse_activation: bool,
    commit_before_refusal: bool,
}
impl Leaves {
    fn outcome(&self, name: &str) -> bool {
        self.input[name].as_bool().unwrap()
    }
    fn event(&mut self, name: &str, args: &[u32], context: &mut MoveContext<'_, '_>) {
        let mut entry = vec![json!(name)];
        entry.extend(args.iter().map(|x| json!(x)));
        self.events.push(json!(entry));
        if self.input["mutations"]
            .as_array()
            .unwrap()
            .first()
            .is_some_and(|m| m["event"] == name)
        {
            let m = self.input["mutations"].as_array_mut().unwrap().remove(0);
            let c = context
                .mind
                .runtime
                .commands
                .value_mut(self.command)
                .unwrap();
            for (k, v) in m["command"].as_object().unwrap() {
                match k.as_str() {
                    "auxiliary_bits" => {
                        c.auxiliary_bits = serde_json::from_value(v.clone()).unwrap()
                    }
                    "speed_bits" => c.scalar_bits = v.as_u64().unwrap() as u32,
                    _ => panic!("unmapped native mutation {k}"),
                }
            }
        }
    }
    fn state(&self, state: StateId) -> u32 {
        self.states[&state]
    }
}
impl mind_world::Services for Leaves {
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
    fn set_actor_state(&mut self, _: ActorId, _: u8, _: &mut Context<'_, '_>) -> Result<()> {
        missing()
    }
    fn rebuild_actions(&mut self, _: ActorId, _: &mut Context<'_, '_>) -> Result<()> {
        missing()
    }
    fn default_stance(&mut self, _: ActorId, _: &mut Context<'_, '_>) -> Result<()> {
        missing()
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
    fn actor_flags(&self, _: ActorId, _: &ActorAccess<'_>) -> Result<u32> {
        missing()
    }
    fn has_live_group(&self, _: ActorId, _: &ActorAccess<'_>) -> Result<bool> {
        missing()
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
impl adapter::Services for Leaves {
    fn indirect_fire(&mut self, actor: ActorId, c: &mut MoveContext<'_, '_>) -> Result<bool> {
        assert_eq!(actor, self.actor);
        self.event("indirect", &[5], c);
        Ok(self.outcome("indirect"))
    }
    fn target_accepts(
        &mut self,
        target: ActorId,
        actor: ActorId,
        c: &mut MoveContext<'_, '_>,
    ) -> Result<bool> {
        assert_eq!(target, self.target);
        assert_eq!(actor, self.actor);
        self.event("accept", &[2, 1], c);
        let v = self.input["accept"].as_array().unwrap();
        let r = v[self.accept_count.min(v.len() - 1)].as_bool().unwrap();
        self.accept_count += 1;
        Ok(r)
    }
    fn activate_sleep(
        &mut self,
        s: StateId,
        mode: u32,
        c: &mut MoveContext<'_, '_>,
    ) -> Result<bool> {
        self.event("sleep", &[self.state(s), mode], c);
        if self.refuse_activation {
            return missing();
        }
        Ok(self.outcome("sleep_result"))
    }
    fn activate_move(
        &mut self,
        s: StateId,
        r: aa_navigation::Activation,
        c: &mut MoveContext<'_, '_>,
    ) -> Result<bool> {
        self.event(
            "move",
            &[
                self.state(s),
                r.activation_mode,
                r.destination_bits[0],
                r.destination_bits[1],
                r.movement_mode,
                r.requested_speed_bits,
                r.auxiliary_bits[0],
                r.auxiliary_bits[1],
            ],
            c,
        );
        if self.commit_before_refusal {
            c.mind
                .runtime
                .movement
                .handle(s)?
                .write()?
                .plan
                .movement
                .destination_bits = r.destination_bits;
        }
        if self.refuse_activation {
            return missing();
        }
        Ok(self.outcome("move_result"))
    }
    fn activate_kill(
        &mut self,
        s: StateId,
        mode: u32,
        target: ActorId,
        c: &mut MoveContext<'_, '_>,
    ) -> Result<bool> {
        assert_eq!(target, self.target);
        self.event("kill", &[self.state(s), mode, 2], c);
        if self.refuse_activation {
            return missing();
        }
        Ok(self.outcome("kill_result"))
    }
    fn activate_bombard(
        &mut self,
        s: StateId,
        mode: u32,
        target: ActorId,
        c: &mut MoveContext<'_, '_>,
    ) -> Result<bool> {
        assert_eq!(target, self.target);
        self.event("bombard", &[self.state(s), mode, 2], c);
        if self.refuse_activation {
            return missing();
        }
        Ok(self.outcome("bombard_result"))
    }
    fn activate_follow(
        &mut self,
        s: StateId,
        mode: u32,
        target: ActorId,
        movement_mode: u32,
        speed: u32,
        aux: [u32; 2],
        c: &mut MoveContext<'_, '_>,
    ) -> Result<bool> {
        assert_eq!(target, self.target);
        self.event(
            "follow",
            &[self.state(s), mode, 2, movement_mode, speed, aux[0], aux[1]],
            c,
        );
        if self.refuse_activation {
            return missing();
        }
        Ok(self.outcome("follow_result"))
    }
}
fn source() -> ActorCatalog {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    ActorCatalog::load_v14(&mut files).unwrap()
}
fn spatial(pc: Precision) -> SpatialWorld {
    SpatialWorld::new(
        Grid {
            dimensions: [64, 64],
            coordinate_shift: 2,
            cell_size_bits: 4f32.to_bits(),
        },
        vec![0; 4096],
        pc,
    )
    .unwrap()
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source bindings"]
fn actual_element_store_replays_all_123_native_rows_and_mutations() {
    let definitions = source();
    let catalog = Catalog::new(&definitions);
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-playable-path/native-element-move.json"
    ))
    .unwrap();
    let mut mutation_count = 0;
    let mut wake_count = 0;
    for row in fixture["cases"].as_array().unwrap() {
        let c = &row["input"];
        let pc: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut store = ActorStore::new();
        let mut rng = NativeRng::new(0x12345678);
        let mut owners = Owners::new();
        let mut defs = aa_runtime::definitions::Bank::default();
        let key = store
            .construct_new(&catalog, record(), 0, &mut rng, pc, pc)
            .unwrap();
        let actor = store.initialize_object(key, &mut rng, pc).unwrap();
        store
            .assemble_ground(
                key,
                &catalog,
                F32::ZERO,
                &mut rng,
                &mut owners,
                &mut defs,
                pc,
                &mut Assembly,
            )
            .unwrap();
        let target_key = store
            .construct_new(&catalog, record(), 0, &mut rng, pc, pc)
            .unwrap();
        let target = store.initialize_object(target_key, &mut rng, pc).unwrap();
        let target_live = c["registry"]["65538"].as_u64().is_some();
        let mut live_command = command(&c["command"], target);
        if !target_live && live_command.word10 == target.raw() {
            live_command.word10 ^= 0x10000;
        }
        if c["name"] == "target_stale" {
            live_command.word10 = target.raw() ^ 0x10000;
        }
        let placement = store.staged_placement_mut(key).unwrap();
        placement.flags = c["actor"]["flags"].as_u64().unwrap() as u32;
        let pos: [u32; 2] = serde_json::from_value(c["actor"]["position_bits"].clone()).unwrap();
        placement.position_bits = [pos[0], pos[1], 0];
        if c["actor"]["performer"].is_null() {
            store
                .object_mut(key)
                .unwrap()
                .components_mut()
                .unwrap()
                .remove(ComponentSlot::Performer);
        }
        let (mind, bindings) = match store
            .component(actor, ComponentSlot::Mind)
            .unwrap()
            .unwrap()
        {
            RuntimeComponent::Mind(m) => (m.mind_id, m.state_bindings.clone()),
            _ => panic!(),
        };
        let mut runtime = mind_world::Runtime::default();
        if let Some(RuntimeComponent::Mind(m)) =
            store.component_mut(actor, ComponentSlot::Mind).unwrap()
        {
            runtime.movement.bind_element(m).unwrap();
            m.update_head = Some(0xdeadbeef); // Deliberately not authoritative.
        }
        let movement = runtime.movement.handle(bindings[&0x7c]).unwrap();
        {
            let mut payload = movement.write().unwrap();
            payload.plan.movement.destination_bits =
                serde_json::from_value(c["element"]["move_destination_bits"].clone()).unwrap();
            payload.plan.movement.target_id = if target_live {
                target.raw()
            } else {
                target.raw() ^ 0x10000
            };
        }
        let active = c["active"].as_bool().unwrap();
        store.minds.state_mut(bindings[&0x7c]).unwrap().status = if active { 17 } else { 2 };
        if active {
            store
                .minds
                .mind_mut(mind)
                .unwrap()
                .stack
                .push_front(Some(bindings[&0x7c]));
        }
        let plan =
            aa_world::initialization::ClockPlan::from_tick_rate(10f32.to_bits(), pc).unwrap();
        let mut queues = plan.initialize_queues(plan.new_clock(), pc).unwrap();
        let owner = owners
            .get(Target::Component {
                object: key,
                slot: ComponentSlot::Mind as u8,
            })
            .unwrap();
        if c["element"]["update_pending"].as_bool().unwrap() {
            queues.add_after(4, owner, 99, 0).unwrap();
        }
        let cmd = runtime.commands.allocate(live_command).unwrap();
        let labels = [(0x7c, 2), (0x68, 3), (0xf8, 4), (0x10c, 5)]
            .into_iter()
            .map(|(off, label)| (bindings[&off], label))
            .collect();
        let mut leaves = Leaves {
            input: c.clone(),
            events: vec![],
            accept_count: 0,
            command: cmd,
            actor,
            target,
            states: labels,
            refuse_activation: false,
            commit_before_refusal: false,
        };
        let mut spatial = spatial(pc);
        let (arena, mut access) = store.split_minds();
        let mut world = adapter::World {
            context: MoveContext {
                mind: Context {
                    actors: &mut access,
                    arena,
                    runtime: &mut runtime,
                    queues: &mut queues,
                    owners: &mut owners,
                },
                spatial: &mut spatial,
            },
            services: &mut leaves,
        };
        let answer = world.execute(mind, cmd, pc).unwrap();
        let mut events = world.services.events.clone();
        let native_events = row["output"]["events"].as_array().unwrap();
        if native_events.iter().any(|e| e[0] == "schedule") {
            assert!(world.context.mind.queues.has(owner, 4, 0));
            assert!(!world.context.mind.queues.has(owner, 4, 99));
            events.push(json!(["schedule", 1, 0x3f19999au32, 0]));
            wake_count += 1;
        } else {
            assert!(!world.context.mind.queues.has(owner, 4, 0));
        }
        assert_eq!(
            json!({"accepted": answer, "events": events}),
            row["output"],
            "{} {:?}",
            c["name"],
            pc
        );
        assert!(
            world.services.input["mutations"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        if !c["mutations"].as_array().unwrap().is_empty() {
            mutation_count += 1;
            assert_eq!(
                world
                    .context
                    .mind
                    .runtime
                    .commands
                    .value(cmd)
                    .unwrap()
                    .scalar_bits,
                99f32.to_bits()
            );
        }
        assert_eq!(
            world.context.mind.runtime.commands.references(cmd).unwrap(),
            1
        );
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 123);
    assert_eq!(mutation_count, 3);
    assert_eq!(wake_count, 6);
}

struct FailAfterPosition;
impl aa_spatial::index::TransformCallbacks for FailAfterPosition {
    fn position_changed(
        &mut self,
        _: &mut aa_spatial::index::SpatialIndex,
        _: u32,
    ) -> aa_spatial::Result<()> {
        Err(aa_spatial::Error::World(
            "deliberate effect failure after canonical position commit".into(),
        ))
    }
    fn world_phase(&self) -> u32 {
        panic!("position effect failed first")
    }
    fn presentation_available(&self) -> bool {
        panic!("position effect failed first")
    }
    fn presentation_position_changed(
        &mut self,
        _: &mut aa_spatial::index::SpatialIndex,
        _: u32,
    ) -> aa_spatial::Result<()> {
        panic!("position effect failed first")
    }
    fn presentation_facing_changed(
        &mut self,
        _: &mut aa_spatial::index::SpatialIndex,
        _: u32,
    ) -> aa_spatial::Result<()> {
        panic!("not called")
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source bindings"]
fn actual_early_returns_do_not_read_unreached_payloads_and_partial_effects_survive() {
    let definitions = source();
    let catalog = Catalog::new(&definitions);
    let pc = Precision::Double53;
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-playable-path/native-element-move.json"
    ))
    .unwrap();
    let base = &fixture["cases"][0]["input"];
    for scenario in [
        "inactive",
        "immobile",
        "stale_target",
        "fresh_unbound",
        "published_near",
        "activation_partial",
    ] {
        let mut store = ActorStore::new();
        let mut rng = NativeRng::new(0x12345678);
        let mut owners = Owners::new();
        let mut defs = aa_runtime::definitions::Bank::default();
        let key = store
            .construct_new(&catalog, record(), 0, &mut rng, pc, pc)
            .unwrap();
        let actor = store.initialize_object(key, &mut rng, pc).unwrap();
        store
            .assemble_ground(
                key,
                &catalog,
                F32::ZERO,
                &mut rng,
                &mut owners,
                &mut defs,
                pc,
                &mut Assembly,
            )
            .unwrap();
        let (mind, bindings) = match store
            .component(actor, ComponentSlot::Mind)
            .unwrap()
            .unwrap()
        {
            RuntimeComponent::Mind(m) => (m.mind_id, m.state_bindings.clone()),
            _ => panic!(),
        };
        let p = store.staged_placement_mut(key).unwrap();
        p.flags = match scenario {
            "inactive" => 1,
            "immobile" => 1 << 29,
            _ => 0,
        };
        p.position_bits = [10f32.to_bits(), 10f32.to_bits(), 0];
        let mut runtime = mind_world::Runtime::default();
        if scenario == "activation_partial" {
            if let Some(RuntimeComponent::Mind(m)) =
                store.component_mut(actor, ComponentSlot::Mind).unwrap()
            {
                runtime.movement.bind_element(m).unwrap();
            }
        }
        if matches!(scenario, "inactive" | "immobile" | "stale_target") {
            if let Some(RuntimeComponent::Mind(m)) =
                store.component_mut(actor, ComponentSlot::Mind).unwrap()
            {
                m.state_bindings.remove(&0x7c);
            }
        }
        let mut value = command(&base["command"], actor);
        if scenario == "stale_target" {
            value.flags |= 1;
            value.word10 = actor.raw() ^ 0x10000;
        }
        let cmd = runtime.commands.allocate(value).unwrap();
        let labels = [(0x7c, 2), (0x68, 3), (0xf8, 4), (0x10c, 5)]
            .into_iter()
            .map(|(off, label)| (bindings[&off], label))
            .collect();
        let mut leaves = Leaves {
            input: base.clone(),
            events: vec![],
            accept_count: 0,
            command: cmd,
            actor,
            target: actor,
            states: labels,
            refuse_activation: true,
            commit_before_refusal: scenario == "activation_partial",
        };
        let plan =
            aa_world::initialization::ClockPlan::from_tick_rate(10f32.to_bits(), pc).unwrap();
        let mut queues = plan.initialize_queues(plan.new_clock(), pc).unwrap();
        let mut spatial = spatial(pc);
        if scenario == "published_near" {
            spatial.publish_actor(&mut store, &catalog, key).unwrap();
            assert!(store.staged_placement_mut(key).is_err());
            assert!(
                spatial
                    .commit_position(
                        key,
                        [20f32.to_bits(), 20f32.to_bits(), 0],
                        &mut FailAfterPosition
                    )
                    .is_err()
            );
        }
        let (arena, mut access) = store.split_minds();
        let mut world = adapter::World {
            context: MoveContext {
                mind: Context {
                    actors: &mut access,
                    arena,
                    runtime: &mut runtime,
                    queues: &mut queues,
                    owners: &mut owners,
                },
                spatial: &mut spatial,
            },
            services: &mut leaves,
        };
        let answer = world.execute(
            mind,
            if matches!(scenario, "inactive" | "immobile") {
                u32::MAX
            } else {
                cmd
            },
            pc,
        );
        if matches!(scenario, "inactive" | "immobile" | "stale_target") {
            assert!(!answer.unwrap());
            assert!(world.services.events.is_empty());
        } else {
            assert!(
                answer
                    .unwrap_err()
                    .to_string()
                    .contains("explicit test service is unbound"),
                "{scenario}"
            );
            assert_eq!(
                world.services.events[0][0],
                if scenario == "published_near" {
                    "sleep"
                } else {
                    "move"
                }
            );
        }
        if scenario == "activation_partial" {
            assert_eq!(
                world
                    .context
                    .mind
                    .runtime
                    .movement
                    .handle(bindings[&0x7c])
                    .unwrap()
                    .read()
                    .unwrap()
                    .plan
                    .movement
                    .destination_bits,
                [20f32.to_bits(), 20f32.to_bits()]
            );
        }
        world.context.mind.runtime.commands.release(cmd).unwrap();
        assert!(world.context.mind.runtime.commands.value_mut(cmd).is_err());
    }
}
