//! Real source actor/model plus live owner-binding checks. This direct-owner
//! test does not claim that the map's full managed lifecycle/adoption passed.
use super::*;
use crate::{gpu::InstanceSlot, inspection::SpawnObject, loading::LoadSettings};
use aa_actors::creation::{ActorCreationRecord, CreationRecord, ThingDefinition};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use aa_runtime::catalog::Catalog;
use aa_simulation::NativeRng;
use aa_spatial::collision::Grid;

#[test]
fn owner_witnesses_survive_moves_and_reject_replacement_owners_after_drop() {
    let actors = ActorStore::new();
    let actor_witness = actors.identity_witness();
    let moved = Box::new(actors);
    assert!(actor_witness.is_for(&moved));
    assert!(!actor_witness.is_for(&ActorStore::new()));
    drop(moved);
    assert!(!actor_witness.is_for(&ActorStore::new()));
    let make_world = || {
        SpatialWorld::new(
            Grid {
                dimensions: [1, 1],
                coordinate_shift: 2,
                cell_size_bits: 4f32.to_bits(),
            },
            vec![0],
            Precision::Double53,
        )
        .unwrap()
    };
    let world = make_world();
    let world_witness = world.identity_witness();
    let moved = Box::new(world);
    assert!(world_witness.is_for(&moved));
    assert!(!world_witness.is_for(&make_world()));
    drop(moved);
    assert!(!world_witness.is_for(&make_world()));
}

fn actor(store: &mut ActorStore, catalog: &Catalog<'_>, saved: ActorId) -> ObjectKey {
    let mut rng = NativeRng::new(0x12345678);
    let key = store
        .construct_new(
            catalog,
            ActorCreationRecord {
                base: CreationRecord {
                    definition: ThingDefinition {
                        key: "usa_infantry".into(),
                        flags: 0,
                    },
                    position_bits: [10f32.to_bits(), 12f32.to_bits(), 3f32.to_bits()],
                    direction_bits: [0, (-1f32).to_bits()],
                    name_utf16: vec![],
                    saved_id: saved,
                },
                variant: -1,
                owner_word: 0,
                word_30: 0,
                request_nonblocking: false,
            },
            0,
            &mut rng,
            Precision::Double53,
            Precision::Double53,
        )
        .unwrap();
    store
        .initialize_object(key, &mut rng, Precision::Double53)
        .unwrap();
    key
}
struct ExplicitPresentationProbe {
    visible: bool,
}
struct TransformProbe;
impl aa_spatial::index::TransformCallbacks for TransformProbe {
    fn position_changed(
        &mut self,
        _: &mut aa_spatial::index::SpatialIndex,
        _: u32,
    ) -> aa_spatial::Result<()> {
        Err(aa_spatial::Error::World(
            "explicit diagnostic stop after native pose commit".into(),
        ))
    }
    fn world_phase(&self) -> u32 {
        6
    } // Explicit probe input to the already-proved kernel.
    fn presentation_available(&self) -> bool {
        panic!("phase6 short-circuits presentation")
    }
    fn presentation_position_changed(
        &mut self,
        _: &mut aa_spatial::index::SpatialIndex,
        _: u32,
    ) -> aa_spatial::Result<()> {
        panic!("stopped before this boundary")
    }
    fn presentation_facing_changed(
        &mut self,
        _: &mut aa_spatial::index::SpatialIndex,
        _: u32,
    ) -> aa_spatial::Result<()> {
        panic!("phase6 skips this boundary")
    }
}
impl Presentation for ExplicitPresentationProbe {
    fn appearance(
        &self,
        _: &ActorStore,
        _: &SpatialWorld,
        _: ActorId,
        _: &RuntimeObject,
        _: PlacementState,
    ) -> Result<Appearance> {
        Ok(Appearance {
            visible: self.visible,
            team_color: Some([0.2, 0.4, 0.8]),
        })
    }
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source and original converted model"]
fn real_source_actor_and_model_read_current_spatial_pose_scale_and_identity() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let definitions = ActorCatalog::load_v14(&mut sources).unwrap();
    let catalog = Catalog::new(&definitions);
    let mut actors = ActorStore::new();
    let key = actor(&mut actors, &catalog, ActorId::NULL);
    let id = actors.id(key).unwrap();
    let mut spatial = SpatialWorld::new(
        Grid {
            dimensions: [64, 64],
            coordinate_shift: 2,
            cell_size_bits: 4f32.to_bits(),
        },
        vec![0; 4096],
        Precision::Double53,
    )
    .unwrap();
    spatial.publish_actor(&mut actors, &catalog, key).unwrap();
    assert!(actors.staged_placement(key).is_err());
    let placement = actors.placement(key, &spatial).unwrap();
    let visual = ActorVisual {
        actor_id: id.raw(),
        native_position: placement.position_bits.map(f32::from_bits),
        native_facing: placement.facing_bits.map(f32::from_bits),
        scale: f32::from_bits(
            actors
                .object(key)
                .unwrap()
                .object
                .as_ref()
                .unwrap()
                .scale_bits,
        ),
        visible: true,
        team_color: None,
    };
    let asset = aa_assets::AssetStore::open(game.join("reconstruction")).unwrap();
    let settings = LoadSettings::new(&asset, 4096, aa_resources::MemoryPolicy::default()).unwrap();
    let binding = super::super::spawn::bind(
        &definitions,
        &super::super::spawn::SpawnActor {
            thing_id: "usa_infantry".into(),
            object_variant: -1,
            resolved_aspect: None,
            visual,
        },
        Precision::Double53,
    )
    .unwrap();
    let (mut model, _) = crate::loading::load_model(&asset, &binding.model_path, settings).unwrap();
    for draw in &mut model.draws {
        draw.instances = vec![visual.instance().unwrap()];
    }
    let mut objects = SceneObjects::empty().unwrap();
    let prepared = objects
        .prepare_append(
            &model.draws,
            SpawnObject {
                actor_id: id.raw(),
                thing_id: binding.thing_id,
                model_source: binding.model_path,
                rule_source: binding.source.path,
                instance: visual.instance().unwrap(),
                native_position: visual.native_position,
                visible: true,
            },
        )
        .unwrap();
    let object = objects.append(prepared);
    let mut live = LiveScene::empty();
    let slots = model
        .draws
        .iter()
        .filter(|d| !d.indices.is_empty() && !d.instances.is_empty())
        .enumerate()
        .map(|(draw, _)| InstanceSlot { draw, instance: 0 })
        .collect::<Vec<_>>();
    assert!(!slots.is_empty());
    live.bindings.insert(
        id.raw(),
        super::super::Binding {
            slots,
            object,
            original: visual,
        },
    );
    let bridge = BattleScene::from_owners(
        &actors,
        &spatial,
        SourceIdentity::of(b"explicit source/model bridge probe"),
        &live,
        &objects,
    )
    .unwrap();
    assert_eq!(bridge.actor_count(), 1);
    let probe = ExplicitPresentationProbe { visible: false };
    let first = bridge.collect_owners(&actors, &spatial, &probe).unwrap();
    assert_eq!(first[0].native_position, visual.native_position);
    assert!(!first[0].visible);
    // Write the actual authoritative fields; no extracted placement is updated.
    assert!(
        spatial
            .commit_position(
                key,
                [21f32.to_bits(), 23f32.to_bits(), 7f32.to_bits()],
                &mut TransformProbe
            )
            .is_err()
    );
    spatial
        .set_facing(key, [1f32.to_bits(), 0], &mut TransformProbe)
        .unwrap();
    actors
        .object_mut(key)
        .unwrap()
        .object
        .as_mut()
        .unwrap()
        .scale_bits = 2.25f32.to_bits();
    let moved = bridge.collect_owners(&actors, &spatial, &probe).unwrap();
    assert_eq!(moved[0].native_position, [21., 23., 7.]);
    assert_eq!(moved[0].native_facing, [1., 0., 0.]);
    assert_eq!(moved[0].scale, 2.25);
    assert_eq!(moved[0].team_color, Some([0.2, 0.4, 0.8]));
    assert_eq!(
        objects.objects[object].native_position, visual.native_position,
        "collection must not publish picking changes"
    );
    actors
        .object_mut(key)
        .unwrap()
        .object
        .as_mut()
        .unwrap()
        .variant = 4;
    assert!(
        bridge
            .collect_owners(&actors, &spatial, &probe)
            .unwrap_err()
            .to_string()
            .contains("variant")
    );
    actors
        .object_mut(key)
        .unwrap()
        .object
        .as_mut()
        .unwrap()
        .variant = -1;
    let mut foreign = ActorStore::new();
    let other = actor(&mut foreign, &catalog, id);
    assert_eq!(foreign.id(other), Some(id));
    assert!(
        bridge
            .collect_owners(&foreign, &spatial, &probe)
            .unwrap_err()
            .to_string()
            .contains("allocation")
    );
    let mut foreign_spatial = SpatialWorld::new(
        Grid {
            dimensions: [64, 64],
            coordinate_shift: 2,
            cell_size_bits: 4f32.to_bits(),
        },
        vec![0; 4096],
        Precision::Double53,
    )
    .unwrap();
    foreign_spatial
        .publish_actor(&mut foreign, &catalog, other)
        .unwrap();
    assert_eq!(
        foreign_spatial.key(other).unwrap(),
        spatial.key(key).unwrap()
    );
    assert!(
        bridge
            .collect_owners(&actors, &foreign_spatial, &probe)
            .unwrap_err()
            .to_string()
            .contains("spatial owner")
    );
    assert!(
        spatial
            .set_facing(key, [0, 0], &mut TransformProbe)
            .is_err()
    );
    assert!(bridge.collect_owners(&actors, &spatial, &probe).is_err());
}

#[test]
#[ignore = "requires AA_GAME_ROOT recovered tutorial02 map and extraction manifest"]
fn actual_rendered_map_receipt_rejects_foreign_hash_and_stale_converted_inputs() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let store = aa_assets::AssetStore::open(game.join("reconstruction")).unwrap();
    let mut scene = store
        .load_scene(
            "Campaign/tutorial02",
            aa_assets::HeightMode::NativeConditional,
        )
        .unwrap();
    let raw = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(
        RenderedMapSource::load(&store, &scene).is_err(),
        "the old converter's rounded samples must not be certified as exact"
    );
    let old_positions: Vec<_> = scene.terrain.vertices.iter().map(|v| v.position).collect();
    let receipt = RenderedMapSource::bind_exact(&store, &mut scene).unwrap();
    RenderedMapSource::load(&store, &scene).unwrap();
    let expected = scene
        .map
        .terrain
        .prepare(scene.terrain.height_mode)
        .unwrap();
    assert_eq!(
        scene.terrain, expected,
        "the actual terrain mesh must be regenerated"
    );
    assert!(
        old_positions
            .iter()
            .zip(&scene.terrain.vertices)
            .any(|(old, vertex)| old.map(f32::to_bits) != vertex.position.map(f32::to_bits)),
        "installing exact native samples must update actual tutorial terrain vertices"
    );
    let mut live = LiveScene::empty();
    live.map_id = Some("Campaign/tutorial02".into());
    live.seal_map_source(receipt).unwrap();
    BattleScene::require_map_source(&live, raw.source()).unwrap();
    // Same actor bindings/GTIDs remain intact; only the raw map identity differs.
    let foreign = SourceIdentity::of(b"another raw map with potentially repeated GTIDs");
    assert!(
        BattleScene::require_map_source(&live, &foreign)
            .unwrap_err()
            .to_string()
            .contains("source hash")
    );
    assert!(BattleScene::require_map_source(&LiveScene::empty(), raw.source()).is_err());
    let original = scene.map.terrain.heights[0];
    scene.map.terrain.heights[0] = f32::from_bits(original.to_bits() ^ 1);
    assert!(
        RenderedMapSource::load(&store, &scene)
            .unwrap_err()
            .to_string()
            .contains("HEIGHT_MAP")
    );
    scene.map.terrain.heights[0] = original;
    scene.map.placements.actors[0].x += 1.0;
    assert!(
        RenderedMapSource::load(&store, &scene)
            .unwrap_err()
            .to_string()
            .contains("placement")
    );
}
