use super::*;
use crate::{
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use serde_json::{Value, json};
use std::{cell::RefCell, path::PathBuf, sync::Arc};

fn evidence(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-member-properties")
        .join(name)
}

struct Probe {
    input: Value,
    entries: Vec<Value>,
    trace: RefCell<Vec<Value>>,
    applies: usize,
    owner: u32,
}
impl Probe {
    fn event(&self, value: Value) {
        self.trace.borrow_mut().push(value);
    }
    fn instance(&self, id: Object) -> Result<&Value> {
        self.entries
            .iter()
            .find(|row| row["handle"] == id.0)
            .ok_or_else(|| "instance_required".into())
    }
}
impl LookupWorld for Probe {
    type Properties = u32;
    fn properties(&self, actor: ActorId) -> Result<Option<u32>> {
        let present = if actor.raw() == 4099 {
            !self.input["properties"].is_null()
        } else {
            self.input["member"] != "absent"
        };
        self.event(json!(["properties", actor.raw(), present]));
        Ok(present.then_some(actor.raw()))
    }
    fn count(&self, _: u32) -> Result<i32> {
        let count = self.entries.len() as i32;
        self.event(json!(["count", count]));
        Ok(count)
    }
    fn slot(&self, _: u32, index: i32) -> Result<Object> {
        let handle = self.entries[index as usize]["handle"].as_u64().unwrap() as u32;
        self.event(json!(["slot", index, handle]));
        Ok(Object(handle))
    }
    fn instance_definition(&self, id: Object) -> Result<Object> {
        let def = self.instance(id)?["definition"].as_u64().unwrap() as u32;
        self.event(json!(["definition", id.0, def]));
        Ok(Object(def))
    }
    fn instance_source(&self, id: Object) -> Result<ActorId> {
        let value = self.instance(id)?["resolved_source"].as_u64().unwrap() as u32;
        self.event(json!(["source", id.0, value]));
        Ok(ActorId::from_raw(value))
    }
}
impl World for Probe {
    fn owner(&self) -> Result<ActorId> {
        self.event(json!(["owner", self.owner]));
        Ok(ActorId::from_raw(self.owner))
    }
    fn pass_down_members(&self, def: Object) -> Result<bool> {
        let value = self
            .entries
            .iter()
            .find(|row| row["definition"] == def.0)
            .unwrap()["pass_down"]
            .as_bool()
            .unwrap();
        self.event(json!(["pass_down", def.0, value]));
        Ok(value)
    }
    fn has_experience(&self, actor: ActorId) -> Result<bool> {
        let value = self.input["member"] == "experience";
        self.event(json!(["experience", actor.raw(), value]));
        Ok(value)
    }
    fn instance_kind(&self, id: Object) -> Result<u32> {
        let value = self.instance(id)?["kind"].as_u64().unwrap() as u32;
        self.event(json!(["kind", id.0, value]));
        Ok(value)
    }
    fn instance_disabled(&self, id: Object) -> Result<bool> {
        let value = self.instance(id)?["disabled"].as_bool().unwrap();
        self.event(json!(["disabled", id.0, value]));
        Ok(value)
    }
    fn apply(
        &mut self,
        actor: ActorId,
        property: Object,
        kind: u32,
        source: ActorId,
    ) -> Result<()> {
        self.event(json!([
            "apply",
            actor.raw(),
            property.0,
            kind,
            source.raw()
        ]));
        self.applies += 1;
        match self.input["callback"].as_str().unwrap() {
            "required" => return Err("apply_required".into()),
            "truncate" => self.entries.truncate(1),
            "append" if self.applies == 1 => {
                let extra = self.input["extra"].clone();
                self.entries.push(extra);
            }
            "replace_next" if self.applies == 1 && self.entries.len() > 1 => {
                self.entries[1] = self.input["extra"].clone();
            }
            "replace_owner" if self.applies == 1 => self.owner = 4101,
            _ => {}
        }
        Ok(())
    }
}

#[test]
fn coordinator_live_reads_and_guard_order_match_original_and_v14() {
    let fixture: Value =
        serde_json::from_slice(&std::fs::read(evidence("native-member-properties.json")).unwrap())
            .unwrap();
    let mut cases = 0;
    for row in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["scope"] == "coordinator")
    {
        let input = row["input"].clone();
        let entries = input["properties"].as_array().cloned().unwrap_or_default();
        let member = if input["member"] == "null" {
            ActorId::NULL
        } else {
            ActorId::from_raw(4100)
        };
        let mut world = Probe {
            input,
            entries,
            trace: RefCell::new(vec![]),
            applies: 0,
            owner: 4099,
        };
        let result = propagate(&mut world, member);
        let outcome = result.err().unwrap_or_else(|| "ok".into());
        assert_eq!(
            json!({"outcome":outcome,"trace":world.trace.into_inner(),"entries":world.entries}),
            row["output"],
            "{}",
            row["name"]
        );
        cases += 1;
    }
    assert!(
        cases >= 90,
        "replay both images/all PCs and all guard shapes"
    );
}

#[test]
fn lookup_guards_disabled_records_and_source_generation_match_original_and_v14() {
    let fixture: Value =
        serde_json::from_slice(&std::fs::read(evidence("native-member-properties.json")).unwrap())
            .unwrap();
    let mut count = 0;
    for row in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["scope"] == "lookup")
    {
        let input = row["input"].clone();
        let entries = input["properties"].as_array().cloned().unwrap_or_default();
        let world = Probe {
            input: input.clone(),
            entries,
            trace: RefCell::new(vec![]),
            applies: 0,
            owner: 4099,
        };
        let found = lookup(
            &world,
            ActorId::from_raw(4099),
            Object(input["property"].as_u64().unwrap() as u32),
            ActorId::from_raw(input["source"].as_u64().unwrap() as u32),
        )
        .unwrap();
        assert_eq!(
            json!({"found":found.map(|id|id.0),"trace":world.trace.into_inner()}),
            row["output"],
            "{}",
            row["name"]
        );
        count += 1;
    }
    assert!(count >= 48);
}

fn actual_input(world: &Borrowed<'_, '_, '_>, name: &str, member: ActorId) -> Value {
    let owner = world.context.owner;
    let properties = world
        .properties_state(owner)
        .unwrap()
        .map(|s| s.effects.iter().map(|id| id.0).collect::<Vec<_>>());
    let member_state = if member.is_null() {
        "null"
    } else if world.shell(member).is_err() {
        "stale"
    } else if world.properties_state(member).unwrap().is_none() {
        "absent"
    } else {
        "present"
    };
    json!({"name":name,"owner":owner.raw(),"member_id":member.raw(),"member":member_state,
        "properties":properties,"definition":world.context.owner_definition.proof.id,
        "actual_organization_owner":world.context.organization.header.owner.raw(),
        "instance_arena":"unbound","source_owner_changed":world.catalog.native_load_mode!=Some(1)})
}
fn check_actual(
    world: Borrowed<'_, '_, '_>,
    name: &str,
    member: ActorId,
    oracle: Option<&Value>,
    captured: &mut Vec<Value>,
) {
    let input = actual_input(&world, name, member);
    let before_org = json!(&world.context.organization);
    let before_actor = format!("{:?}", &world.context.owner_actor);
    let properties_address = world
        .properties_state(world.context.owner)
        .unwrap()
        .map(|s| s as *const _ as usize);
    let pc = world.context.precision;
    let result = member_added(
        world.catalog,
        OrganizationAttachContext {
            inventory: world.context.inventory,
            owner: world.context.owner,
            owner_actor: &mut *world.context.owner_actor,
            owner_definition: world.context.owner_definition,
            owner_thing_flags: world.context.owner_thing_flags,
            owner_thing_word_c: world.context.owner_thing_word_c,
            owner_object_byte_4c: world.context.owner_object_byte_4c,
            organization: &mut *world.context.organization,
            registry: &mut *world.context.registry,
            precision: pc,
        },
        member,
    );
    let outcome = match result {
        Ok(()) => "ok",
        Err(ref e) if e.contains("property-instance arena") => "instance_required",
        Err(ref e) if e.contains("not a current restored Actor") => "actor_required",
        Err(ref e) => panic!("unexpected boundary {e}"),
    };
    assert_eq!(format!("{:?}", &world.context.owner_actor), before_actor);
    assert_eq!(json!(&world.context.organization), before_org);
    assert_eq!(
        world
            .properties_state(world.context.owner)
            .unwrap()
            .map(|s| s as *const _ as usize),
        properties_address
    );
    let mut finds = vec![];
    for (property, source) in [(0, ActorId::NULL), (0x7fffffe0, world.context.owner)] {
        let result = find(
            world.catalog,
            OrganizationAttachContext {
                inventory: world.context.inventory,
                owner: world.context.owner,
                owner_actor: &mut *world.context.owner_actor,
                owner_definition: world.context.owner_definition,
                owner_thing_flags: world.context.owner_thing_flags,
                owner_thing_word_c: world.context.owner_thing_word_c,
                owner_object_byte_4c: world.context.owner_object_byte_4c,
                organization: &mut *world.context.organization,
                registry: &mut *world.context.registry,
                precision: pc,
            },
            property,
            source,
        );
        assert_eq!(
            result,
            find_actual(
                LookupContext {
                    owner: world.context.owner,
                    owner_actor: world.context.owner_actor,
                    registry: world.context.registry,
                },
                property,
                source
            ),
            "minimal read-only query uses the same coordinator"
        );
        finds.push(match result {
            Ok(value) => json!(value),
            Err(e) if e.contains("property-instance arena") => json!("instance_required"),
            Err(e) => panic!("find boundary {e}"),
        });
    }
    assert_eq!(format!("{:?}", &world.context.owner_actor), before_actor);
    assert_eq!(json!(&world.context.organization), before_org);
    let output = json!({"outcome":outcome,"properties":input["properties"],"find":finds});
    if let Some(oracle) = oracle {
        let rows: Vec<_> = oracle["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["scope"] == "actual_saved"
                    && row["input"] == input
                    && row["precision"] == format!("{pc:?}")
            })
            .collect();
        assert_eq!(rows.len(), 2, "original/v14 {name}");
        for row in rows {
            assert_eq!(row["output"], output, "{name} {pc:?}");
        }
    }
    captured.push(json!({"precision":format!("{pc:?}"),"input":input}));
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 data"]
fn actual_tutorial02_owner_properties_and_lazy_missing_instance_boundary_match_native() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut src = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut src, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut src, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut src).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut src).unwrap());
    let token = ScopedManagedLoad::from_sources(&mut src, 375)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            root.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut attempt = Attempt::load_with_bindings(
        &map,
        &actors,
        settings,
        text,
        Precision::Double53,
        u64::MAX,
        aa_mapworld::restore::RestoreMode::StopOnError,
        LoadBindings {
            native_load_mode: None,
            inactive_load: Some(token),
            resources: Some(Arc::clone(&resources)),
        },
    )
    .unwrap();
    assert!(!attempt.report.runtime_publication_ready);
    let owner = ActorId::from_raw(4099);
    let member = ActorId::from_raw(4100);
    let record = std::mem::replace(
        attempt.registry.value_mut(owner).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Actor(mut entity) = record else {
        panic!("actual tutorial owner")
    };
    let actor = entity.shell.actor.as_mut().unwrap();
    let Some(RestoredComponent::Organization(mut org)) =
        actor.components.remove(ComponentSlot::Organization)
    else {
        panic!("held Organization")
    };
    assert_eq!(
        org.members[4], member,
        "naturally reached attachment prefix"
    );
    assert_eq!(org.header.owner, owner);
    let Some(RestoredComponent::Properties(props)) =
        actor.components.get(ComponentSlot::Properties)
    else {
        panic!("Properties")
    };
    assert!(props.effects.is_empty(), "actual saved constructor list");
    let mut catalog = Catalog::new(&actors);
    catalog.native_load_mode = attempt.native_load_mode;
    catalog.settings = Some(attempt.settings.clone());
    catalog.bind_resource_catalog(resources).unwrap();
    catalog
        .bind_inactive_load(attempt.inactive_load.as_ref().unwrap())
        .unwrap();
    let capture_mode = std::env::var_os("AA_MEMBER_PROPERTIES_CAPTURE").is_some();
    let oracle: Option<Value> = (!capture_mode).then(|| {
        serde_json::from_slice(&std::fs::read(evidence("native-member-properties.json")).unwrap())
            .unwrap()
    });
    let inventory = &attempt.inventory;
    let mut captures = vec![];
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let mut invoke =
            |name: &str,
             target: ActorId,
             actor: &mut ActorShell<RestoredComponent>,
             org: &mut aa_actors::ground_states::OrganizationState<String>,
             registry: &mut aa_actors::registry::IdentityRegistry<RestoredRecord>| {
                check_actual(
                    Borrowed {
                        catalog: &catalog,
                        context: OrganizationAttachContext {
                            inventory,
                            owner,
                            owner_actor: actor,
                            owner_definition: &entity.definition,
                            owner_thing_flags: entity.shell.thing.flags,
                            owner_thing_word_c: entity.shell.thing.word_c,
                            owner_object_byte_4c: entity
                                .shell
                                .object
                                .as_ref()
                                .map(|s| s.byte_4c)
                                .unwrap_or(0),
                            organization: org,
                            registry,
                            precision: pc,
                        },
                    },
                    name,
                    target,
                    oracle.as_ref(),
                    &mut captures,
                );
            };
        invoke("actual", member, actor, &mut org, &mut attempt.registry);
        invoke(
            "probe_empty_stale_member",
            ActorId::from_raw(0xdead1004),
            actor,
            &mut org,
            &mut attempt.registry,
        );
        let absent = actor.components.remove(ComponentSlot::Properties).unwrap();
        invoke(
            "probe_absent_owner_stale_member",
            ActorId::from_raw(0xdead1004),
            actor,
            &mut org,
            &mut attempt.registry,
        );
        actor.components.insert(ComponentSlot::Properties, absent);
        let Some(RestoredComponent::Properties(props)) =
            actor.components.get_mut(ComponentSlot::Properties)
        else {
            panic!("Properties")
        };
        props.effects.push(Object(0x1badbeef));
        invoke(
            "probe_unbound_instance",
            member,
            actor,
            &mut org,
            &mut attempt.registry,
        );
        invoke(
            "probe_unbound_instance_null_member",
            ActorId::NULL,
            actor,
            &mut org,
            &mut attempt.registry,
        );
        let removed = attempt
            .registry
            .value_mut(member)
            .unwrap()
            .loaded_mut()
            .unwrap()
            .shell
            .actor
            .as_mut()
            .unwrap()
            .components
            .remove(ComponentSlot::Properties)
            .unwrap();
        invoke(
            "probe_unbound_instance_missing_member_properties",
            member,
            actor,
            &mut org,
            &mut attempt.registry,
        );
        attempt
            .registry
            .value_mut(member)
            .unwrap()
            .loaded_mut()
            .unwrap()
            .shell
            .actor
            .as_mut()
            .unwrap()
            .components
            .insert(ComponentSlot::Properties, removed);
        invoke(
            "probe_nonempty_stale_member",
            ActorId::from_raw(0xdead1004),
            actor,
            &mut org,
            &mut attempt.registry,
        );
        let Some(RestoredComponent::Properties(props)) =
            actor.components.get_mut(ComponentSlot::Properties)
        else {
            panic!("Properties")
        };
        props.effects.clear();
    }
    actor.components.insert(
        ComponentSlot::Organization,
        RestoredComponent::Organization(org),
    );
    *attempt.registry.value_mut(owner).unwrap() = RestoredRecord::Actor(entity);
    if capture_mode {
        std::fs::create_dir_all(evidence("")).unwrap();
        std::fs::write(evidence("actual-saved-inputs.json"),serde_json::to_vec_pretty(&json!({"schema":1,"context":{"map":"Campaign/tutorial02","mode":1,"clock_fixture_millis":375,"immediate_Init_PostCreate":false},"cases":captures})).unwrap()).unwrap();
    }
}
