//! Actual canonical source/record ownership acceptance. Restoration here omits
//! Init/PostCreate and uses an explicit E4=3 selection fixture; this does not
//! publish a battle or impersonate a LocalBootstrap player.
use aa_actors::{ActorId, components::ComponentSlot};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{
    actor_world::{ActorStore, PlacementState, Spatial},
    catalog::Catalog,
    combat_source_completion as completion,
    combat_source_receipt::VerifiedStartupReceipt,
    load_world_globals::SessionPhase,
    localization::Localization,
    mind_world, player_order_world as orders,
    player_selection::{List, View},
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use std::sync::Arc;
struct StagedOnly;
impl Spatial for StagedOnly {
    fn placement(&self, _: ActorId) -> aa_runtime::actor_world::Result<PlacementState> {
        panic!("actual fixture actors remain staged; no spatial owner invented")
    }
    fn publish(&mut self, _: ActorId, _: PlacementState) -> aa_runtime::actor_world::Result<()> {
        panic!("no publication")
    }
}
struct SessionFixture;
impl SessionPhase for SessionFixture {
    fn current_phase(&self) -> Result<u32, String> {
        panic!("selection uses E4, not F8")
    }
    fn application_state(&self) -> Result<u32, String> {
        Ok(3)
    }
}
struct PlayerFixture {
    list: List,
    masks: [u32; 4],
    look: [u32; 2],
    kingdom: Option<ActorId>,
}
impl View for PlayerFixture {
    fn player_id(&self) -> u32 {
        17
    }
    fn list(&self) -> &List {
        &self.list
    }
    fn list_mut(&mut self) -> &mut List {
        &mut self.list
    }
    fn kingdom(&self) -> Option<ActorId> {
        self.kingdom
    }
    fn mask_words_mut(&mut self) -> &mut [u32; 4] {
        &mut self.masks
    }
    fn look_at_mut(&mut self) -> &mut [u32; 2] {
        &mut self.look
    }
    fn group_is_null(&self) -> Result<bool, String> {
        Ok(true)
    }
    fn clear_group(&mut self) {}
}
struct GlobalsFixture;
impl orders::Globals for GlobalsFixture {
    fn can_process_40eaa0(&mut self) -> orders::Result<bool> {
        Ok(false)
    }
    fn session_phase_76ac90(&mut self) -> orders::Result<u32> {
        panic!("false gate must skip phase")
    }
}
// Every world/subtype leaf is a required error. Only invoke records the
// reached native boundary; this is an ownership-prefix probe, never fake AI.
#[derive(Default)]
struct RequiredMind {
    reached: Option<(aa_mind::MindId, u32, u32)>,
}
impl mind_world::Services for RequiredMind {
    fn can_activate_other(
        &mut self,
        _: aa_mind::StateId,
        _: mind_world::StateBinding,
        _: &mut mind_world::Context<'_, '_>,
    ) -> aa_mind::Result<bool> {
        Err(aa_mind::Error::World("required can_activate_other".into()))
    }
    fn enter_other(
        &mut self,
        _: aa_mind::StateId,
        _: mind_world::StateBinding,
        _: &mut mind_world::Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        Err(aa_mind::Error::World("required enter_other".into()))
    }
    fn leave_other(
        &mut self,
        _: aa_mind::StateId,
        _: mind_world::StateBinding,
        _: &mut mind_world::Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        Err(aa_mind::Error::World("required leave_other".into()))
    }
    fn actor_state_other(
        &mut self,
        _: aa_mind::StateId,
        _: mind_world::StateBinding,
        _: &mut mind_world::Context<'_, '_>,
    ) -> aa_mind::Result<u8> {
        Err(aa_mind::Error::World("required actor_state_other".into()))
    }
    fn set_actor_state(
        &mut self,
        _: ActorId,
        _: u8,
        _: &mut mind_world::Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        Err(aa_mind::Error::World("required set_actor_state".into()))
    }
    fn rebuild_actions(
        &mut self,
        _: ActorId,
        _: &mut mind_world::Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        Err(aa_mind::Error::World("required rebuild_actions".into()))
    }
    fn default_stance(
        &mut self,
        _: ActorId,
        _: &mut mind_world::Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        Err(aa_mind::Error::World("required default_stance".into()))
    }
    fn reset_other(
        &mut self,
        _: aa_mind::MindId,
        _: aa_mind::construct::Kind,
        _: &mut mind_world::Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        Err(aa_mind::Error::World("required reset_other".into()))
    }
    fn sync_state_other(
        &mut self,
        _: aa_mind::MindId,
        _: aa_mind::construct::Kind,
        _: &mut mind_world::Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        Err(aa_mind::Error::World("required sync_state_other".into()))
    }
    fn facing(
        &self,
        _: ActorId,
        _: &aa_runtime::actor_world::ActorAccess<'_>,
    ) -> aa_mind::Result<[u32; 2]> {
        Err(aa_mind::Error::World("required facing".into()))
    }
    fn set_facing(
        &mut self,
        _: ActorId,
        _: [u32; 2],
        _: &mut mind_world::Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        Err(aa_mind::Error::World("required set_facing".into()))
    }
    fn actor_flags(
        &self,
        _: ActorId,
        _: &aa_runtime::actor_world::ActorAccess<'_>,
    ) -> aa_mind::Result<u32> {
        Err(aa_mind::Error::World("required actor_flags".into()))
    }
    fn has_live_group(
        &self,
        _: ActorId,
        _: &aa_runtime::actor_world::ActorAccess<'_>,
    ) -> aa_mind::Result<bool> {
        Err(aa_mind::Error::World("required has_live_group".into()))
    }
    fn finish_group_commands(
        &mut self,
        _: ActorId,
        _: &mut mind_world::Context<'_, '_>,
    ) -> aa_mind::Result<()> {
        Err(aa_mind::Error::World(
            "required finish_group_commands".into(),
        ))
    }
    fn deliver_stop(
        &mut self,
        _: ActorId,
        _: &mut mind_world::Context<'_, '_>,
    ) -> aa_mind::Result<bool> {
        Err(aa_mind::Error::World("required deliver_stop".into()))
    }
    fn defers_other(
        &mut self,
        _: aa_mind::StateId,
        _: mind_world::StateBinding,
        _: &mut mind_world::Context<'_, '_>,
    ) -> aa_mind::Result<bool> {
        Err(aa_mind::Error::World("required defers_other".into()))
    }

    fn invoke_command(
        &mut self,
        mind: aa_mind::MindId,
        slot: u32,
        command: u32,
        depth: &mut aa_mind::command::Context,
        ctx: &mut mind_world::Context<'_, '_>,
    ) -> aa_mind::Result<bool> {
        assert_eq!(slot, 0xbc);
        assert_eq!(depth.depth, 1);
        assert_eq!(ctx.arena.mind(mind)?.current_command, Some(command));
        assert_eq!(ctx.runtime.commands.references(command)?, 2);
        self.reached = Some((mind, slot, command));
        Err(aa_mind::Error::World(
            "required actual native subtype move handler".into(),
        ))
    }
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn actual_sources_saved_actors_and_singleton_selection_share_the_owned_fields() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let mut attempt = Attempt::load_with_source_setup(
        &map,
        &actors,
        settings,
        text,
        Precision::Double53,
        u64::MAX,
        aa_mapworld::restore::RestoreMode::InspectPartial,
        LoadBindings {
            native_load_mode: Some(1),
            inactive_load: Some(inactive),
            resources: Some(resources),
        },
        |catalog| {
            let proof = VerifiedStartupReceipt::verify_embedded_reference(catalog, &mut sources)?;
            completion::install(
                catalog,
                &mut sources,
                proof.context(),
                proof.csv(),
                proof.globals(),
            )?;
            completion::compute(catalog)
        },
    )
    .unwrap();
    assert!(!attempt.report.runtime_publication_ready);
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(attempt.settings.clone());
    catalog.definition_bank = std::cell::RefCell::new(std::mem::take(&mut attempt.definitions));
    let id = attempt
        .registry
        .iter()
        .find_map(|e| {
            let v = e.value.loaded()?;
            let a = v.shell.actor.as_ref()?;
            (v.definition.native_class == 3 && a.components.present(ComponentSlot::Mind))
                .then_some(e.id)
        })
        .expect("actual saved ordinary actor with Mind");
    let mut store =
        ActorStore::from_restored_with_abilities(attempt.registry, attempt.abilities).unwrap();
    let mut runtime = mind_world::Runtime::default();
    runtime.bind_restored_commands(&mut store).unwrap();
    let command = orders::allocate(
        &catalog,
        &mut runtime,
        orders::Input::Move {
            destination: [0x3f800000, 0x40000000],
            auxiliary: [0, 0],
            speed: 0,
            trailing: 0,
        },
    )
    .unwrap();
    let (arena, mut access) = store.split_minds();
    let key = access.key(id).unwrap();
    let object_address = access.object(key).unwrap() as *const _ as usize;
    let byte_before = access.object(key).unwrap().object.as_ref().unwrap().byte_4c;
    let kingdom = access.object(key).unwrap().actor.as_ref().unwrap().words[&0xe8];
    let mut view = PlayerFixture {
        list: List::from_vworld_constructor(),
        masks: [u32::MAX; 4],
        look: [0; 2],
        kingdom: (kingdom != 0).then_some(ActorId::from_raw(kingdom)),
    };
    assert_eq!(
        orders::select(&catalog, &mut access, &mut view, &SessionFixture, Some(id)).unwrap(),
        orders::SelectionOutcome::Selected
    );
    let head = view.list.head().unwrap();
    assert_eq!(view.list.node(head).unwrap().actor, id);
    assert_eq!(
        access.object(key).unwrap().object.as_ref().unwrap().byte_4c,
        byte_before.wrapping_add(1)
    );
    assert_eq!(view.masks, [0; 4]);
    assert_eq!(
        orders::select(&catalog, &mut access, &mut view, &SessionFixture, Some(id)).unwrap(),
        orders::SelectionOutcome::AlreadySelected
    );
    assert!(orders::deselect(&catalog, &mut access, &mut view, id).unwrap());
    assert!(view.list.is_empty());
    assert_eq!(
        access.object(key).unwrap().object.as_ref().unwrap().byte_4c,
        byte_before
    );
    orders::select(&catalog, &mut access, &mut view, &SessionFixture, Some(id)).unwrap();
    orders::clear_selection(&catalog, &mut access, &mut view).unwrap();
    assert_eq!(
        access.object(key).unwrap() as *const _ as usize,
        object_address
    );
    assert_eq!(
        access.object(key).unwrap().object.as_ref().unwrap().byte_4c,
        byte_before
    );
    let allowed = orders::authorize_selected(
        &catalog,
        &access,
        &StagedOnly,
        id,
        view.kingdom,
        runtime.commands.value(command).unwrap(),
        &mut GlobalsFixture,
    )
    .unwrap();
    let native_flags = access.placement(key, &StagedOnly).unwrap().flags;
    let component_names = access
        .object(key)
        .unwrap()
        .components()
        .unwrap()
        .iter()
        .map(|(s, _)| format!("{s:?}"))
        .collect::<Vec<_>>();
    // Controlled native action-mask probe after genuine source restoration.
    // It does not claim diagnostic restoration performed Actor Init/rebuild.
    let action_before = access.object(key).unwrap().actor.as_ref().unwrap().words[&0xf0];
    access
        .object_mut(key)
        .unwrap()
        .actor
        .as_mut()
        .unwrap()
        .words
        .insert(0xf0, action_before | (1 << 5));
    let mut queues = aa_world::queues::Queues::new(
        aa_world::queues::Clock {
            begin_bits: 0,
            end_bits: 0,
            step_bits: 0x3dcccccd,
        },
        Precision::Double53,
    )
    .unwrap();
    let mut owners = aa_runtime::scheduler_owners::Owners::new();
    let mut services = RequiredMind::default();
    let error = orders::deliver(
        &catalog,
        &mut mind_world::World {
            actors: access.reborrow(),
            runtime: &mut runtime,
            queues: &mut queues,
            owners: &mut owners,
            services: &mut services,
        },
        arena,
        command,
        Some(id),
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("required actual native subtype move handler")
    );
    let (mind, slot, copied) = services.reached.unwrap();
    assert_eq!(slot, 0xbc);
    assert_ne!(copied, command);
    assert_eq!(arena.mind(mind).unwrap().current_command, Some(copied));
    assert_eq!(runtime.commands.references(copied).unwrap(), 1);
    assert_eq!(runtime.commands.references(command).unwrap(), 1);
    access
        .object_mut(key)
        .unwrap()
        .actor
        .as_mut()
        .unwrap()
        .words
        .insert(0xf0, action_before);
    let result = serde_json::json!({"id":id.raw(),"definition":access.object(key).unwrap().definition.proof.id,
  "flags":native_flags,"kingdom":kingdom,"components":component_names,"allowed_with_explicit_false_gate":allowed,
  "same_runtime_object_allocation":true,"positive_4D9B60_52C320_prefix_reached":true,"controlled_action_bit5_probe":true,"actual_Move_subtype":"required_error_retains_current_clone","same_canonical_bank":true,"same_owned_4c_byte":true,
  "selection_fixture_application_state":3,"actual_session_player_binding":false,
  "Init_PostCreate_publication_omitted":true,"battle_ready":false});
    if let Some(path) = std::env::var_os("AA_PLAYER_ORDER_CAPTURE") {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        f.write_all(serde_json::to_string_pretty(&result).unwrap().as_bytes())
            .unwrap();
    }
    runtime.commands.release(command).unwrap();
    assert!(runtime.commands.value(command).is_err());
}
