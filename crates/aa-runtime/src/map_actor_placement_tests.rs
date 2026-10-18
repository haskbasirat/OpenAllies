use super::*;
use crate::{
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use aa_actors::creation::{ActorCreationRecord, CreationRecord, ThingDefinition};
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};

#[test]
#[ignore = "requires AA_GAME_ROOT; explicit fresh-constructor placement in a genuinely stopped load, not a started mission"]
fn actual_fresh_allocation_uses_same_moved_bank_params_and_index() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let source = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = std::sync::Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    struct Session(aa_netcode::lobby::Session);
    impl crate::load_world_globals::SessionPhase for Session {
        fn current_phase(&self) -> std::result::Result<u32, String> {
            Ok(self.0.field_f8)
        }
        fn application_state(&self) -> std::result::Result<u32, String> {
            Ok(self.0.state)
        }
    }
    let session = Session(aa_netcode::lobby::Session::default());
    let mut catalog = Catalog::new(&source);
    catalog.settings = Some(settings.clone());
    catalog.replace_current_text(text.clone());
    catalog.definition_text = Some(text.clone());
    let verified = crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(
        &catalog,
        &mut sources,
    )
    .unwrap();
    let startup =
        crate::actor_creation_startup::StartupInputs::from_verified_startup_reference(&verified)
            .unwrap();
    let mut rows = Vec::new();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
            .unwrap()
            .begin(TaskKind::Normal)
            .unwrap();
        let attempt = Attempt::load_immediate(
            &map,
            &source,
            settings.clone(),
            text.clone(),
            pc,
            u64::MAX,
            aa_mapworld::restore::RestoreMode::StopOnError,
            LoadBindings {
                native_load_mode: Some(1),
                inactive_load: Some(inactive),
                resources: Some(std::sync::Arc::clone(&resources)),
            },
            &mut sources,
            &session,
            false,
        )
        .unwrap();
        assert!(attempt.report.lifecycle.iter().any(|e| e.attempted));
        let mut staged = attempt.into_staged().unwrap();
        let world = staged.load_world.as_mut().unwrap();
        world.adopt_spatial(&mut staged.actors).unwrap();
        let source_pc = staged
            .definitions
            .combat_source()
            .unwrap()
            .source_precision();
        assert_eq!(source_pc, Precision::Double53);
        let binding = catalog.actor_bindings("gbr_corps", source_pc).unwrap();
        let token = staged
            .definitions
            .token("ActorDefinition", "gbr_corps")
            .unwrap();
        //These are explicit input arguments to an isolated constructor. Native
        //Nation selection, InitCreate, startup/mission and combat are not bypassed
        //or claimed by this test; only the actual placement binding is exercised.
        let mut params = Params::new(crate::map_replacement_queue::DefinitionRef {
            token,
            canonical_ids: binding.definition.proof.id.clone(),
        });
        params.position_bits = [100.0f32.to_bits(), 100.0f32.to_bits(), 0];
        params.direction_bits = [0, 0xbf800000];
        params.mode = 5;
        params.byte_52 = 0;
        let key = staged
            .actors
            .construct_new_bound(
                &catalog,
                std::sync::Arc::clone(&binding),
                ActorCreationRecord {
                    base: CreationRecord {
                        definition: ThingDefinition {
                            key: binding.definition.proof.id.clone(),
                            flags: binding.definition.flags,
                        },
                        position_bits: params.position_bits,
                        direction_bits: params.direction_bits,
                        name_utf16: vec![],
                        saved_id: ActorId::NULL,
                    },
                    variant: -1,
                    owner_word: 0,
                    word_30: 0,
                    request_nonblocking: false,
                },
                staged.clock.begin.0,
                &mut staged.rng,
                startup.startup_precision(),
                pc,
            )
            .unwrap();
        let id = staged
            .actors
            .initialize_object(key, &mut staged.rng, pc)
            .unwrap();
        //Explicit descriptor mutation for this direct placement branch; the
        //constructor used its genuine blocking-default branch. Full caller
        //interleaving/InitCreate is outside this fixture.
        params.byte_52 = 1;
        params.word_3c = 1;
        let address = std::ptr::from_ref(staged.actors.object(key).unwrap()) as usize;
        let before = staged.actors.staged_placement(key).unwrap();
        let seed = staged.rng.state;
        let wrong = source
            .native_source_requirements(Precision::Single24)
            .unwrap();
        let world = staged.load_world.as_mut().unwrap();
        let original = std::mem::replace(&mut world.source_fields, wrong);
        assert!(
            check(&mut staged, &catalog, key, &params, pc)
                .unwrap_err()
                .to_string()
                .contains("terrain/source fields differ")
        );
        staged.load_world.as_mut().unwrap().source_fields = original;
        assert_eq!(staged.actors.staged_placement(key).unwrap(), before);
        assert_eq!(staged.rng.state, seed);
        let result = check(&mut staged, &catalog, key, &params, pc);
        assert_eq!(result.unwrap(), 0);
        assert!(
            staged.actors.staged_placement(key).is_err(),
            "same pose loan moved exactly once"
        );
        let world = staged.load_world.as_ref().unwrap();
        let spatial = world.live_spatial().unwrap();
        let index_key = spatial.key(key).unwrap();
        let member = spatial.index().member(index_key).unwrap();
        assert_eq!(member.reference, id.raw());
        assert_eq!(member.actor.flags, before.flags);
        assert_eq!(
            member.actor.position,
            [before.position_bits[0], before.position_bits[1]]
        );
        let member_position = member.actor.position;
        assert_eq!(
            member.actor.flags & (1 << 26),
            0,
            "placement query must not insert native membership"
        );
        assert_eq!(
            std::ptr::from_ref(staged.actors.object(key).unwrap()) as usize,
            address
        );
        assert_eq!(staged.rng.state, seed);
        assert!(catalog.definition_bank.borrow().combat_source().is_err());
        //A changed actual descriptor is read at the next original boundary. This
        //does not move the Actor or create a synthetic/saved descriptor copy.
        params.position_bits[0] = (-100.0f32).to_bits();
        assert_eq!(check(&mut staged, &catalog, key, &params, pc).unwrap(), 3);
        let world = staged.load_world.as_mut().unwrap();
        let crate::load_world::SpatialOwner::Adopted(adopted) = &mut world.spatial else {
            unreachable!()
        };
        adopted
            .world
            .set_initialization_scalar(key, 0x28, 17.0f32.to_bits())
            .unwrap();
        let after = adopted.world.index().member(index_key).unwrap();
        assert_eq!(after.height_bits, 17.0f32.to_bits());
        assert_eq!(after.actor.position, member_position);
        assert!(
            adopted
                .world
                .set_initialization_scalar(key, 0x20, 0)
                .is_err()
        );
        rows.push(serde_json::json!({"pc":format!("{pc:?}"),"source_pc":format!("{source_pc:?}"),"id":id.raw(),"definition":params.definition.canonical_ids,"constructor_flags":before.flags,"placement_result":0,"changed_descriptor_result":3,"same_allocation":true,"native_membership_inserted":false,"rng_unchanged":true}));
    }
    if let Some(path) = std::env::var_os("AA_FRESH_PLACEMENT_CAPTURE") {
        use std::io::Write;
        let bytes=serde_json::to_vec_pretty(&serde_json::json!({"scope":"explicit fresh constructor plus4EF6A0 placement, stopped actual Tutorial load; no completed4D6F10/mission claim","cases":rows})).unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap()
            .write_all(&bytes)
            .unwrap();
    }
}
