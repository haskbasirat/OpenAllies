//!Real-data exercise of the vf18/vf1C bindings (native kernels/coordinators are
//!replayed in data_release_fields_tests / data_release_kernels_tests).
use super::*;
use aa_rules::{actors::ActorCatalog, resources::ResourceCatalog, sources::{SourceLimits, V14Sources}, tgi::Limits};
use std::path::PathBuf;

struct LabModel;
impl crate::resource_objects::ModelPolicy for LabModel {
    fn use_hardware_model_73a560(&mut self) -> Result<bool> {
        Ok(false)
    }
}
struct LabTexture;
impl crate::resource_objects::TexturePolicy for LabTexture {
    fn data_compatibility_mode_current(&mut self) -> Result<u8> {
        Ok(1)
    }
    fn data_missing_resource_warning_current(&mut self) -> Result<u8> {
        Ok(0)
    }
    fn missing_texture_diagnostic_404fc0(&mut self, _: &[u16], _: u32) -> Result<()> {
        Ok(())
    }
    fn texture_cache_source_overrides(&mut self) -> Result<crate::load_view_variables::NativeOverrides> {
        Ok(crate::load_view_variables::NativeOverrides::ControlledNone)
    }
}
///Laboratory request services: tasks are queued, never decoded or loaded.
struct LabBackend;
impl crate::resource_task::Services for LabBackend {
    fn enter_request(&mut self, _: &Handle) -> Result<()> {
        Ok(())
    }
    fn leave_request(&mut self, _: &Handle) -> Result<()> {
        Ok(())
    }
    fn request_diagnostic(&mut self, _: &Handle, _: bool) -> Result<()> {
        Ok(())
    }
}
impl crate::resource_depot::Backend for LabBackend {}
///Laboratory-only: records each5AA550 call instead of releasing.
#[derive(Default)]
struct Recording(Vec<(String, u32)>);
impl ReleaseServices for Recording {
    fn release_14(&mut self, _: &mut Resources<'_>, h: &Handle, delta: u32) -> Result<()> {
        self.0.push((String::from_utf16_lossy(h.logical_ids()), delta));
        Ok(())
    }
}
#[test]
#[ignore = "requires actual V14 sources (AA_GAME_ROOT); bindings only, Resource release is a recording laboratory"]
fn actual_truck_release_reaches_its_deployment_building() {
    let game = PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(game.join("Data.rwd"), game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: u64::MAX }).unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap());
    catalog.bind_resource_catalog(Arc::new(ResourceCatalog::load_v14(&mut files).unwrap())).unwrap();
    let receipt = crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
    crate::combat_source_completion::install(&catalog, &mut files, receipt.context(), receipt.csv(), receipt.globals()).unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    crate::replacement_flags_source::complete_source_suffix(&catalog).unwrap();
    let pc = catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
    let variables = crate::creator_variables::State::from_fresh_sources(&catalog, &mut files,
        crate::load_view_variables::NativeOverrides::ControlledNone).unwrap();
    let creator = crate::resource_creator::Creator::from_load_constructor(&catalog, &variables).unwrap();
    let mut application = crate::resource_depot::Application::from_sources(files);
    application.install_fresh_cache_variable_scope(&catalog, crate::load_view_variables::NativeOverrides::ControlledNone).unwrap();
    {
        let bank = catalog.definition_bank.borrow();
        application.prepare_model_source_context(&catalog, &bank).unwrap();
    }
    let work = std::env::temp_dir().join(format!("aa-release-{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();
    application.configure_portable_work_depot(&work).unwrap();
    let truck = catalog.resolve_registered("rus_truck_corps", pc).unwrap();
    let building = catalog.resolve_registered("rus_corps", pc).unwrap();
    let bank = catalog.definition_bank.borrow();
    let (released, ready, counts) = application
        .with_data_pre_request_resources(|state, resources| {
            let mut words = Vec::new();
            let mut data = Vec::new();
            for d in [&truck, &building] {
                data.push(state.prepare_object_requests(&catalog, &bank, d, resources, &mut LabModel, &mut LabTexture)?);
            }
            state.complete_deferred_links_475230(&catalog, &bank)?;
            let biome = crate::vworld_resource_context::Biome::from_world_init_creator(&catalog, &bank, &creator)?;
            //45DD80 on the truck also requests rus_corps through+534 (mode 0).
            state.pre_request_object(&catalog, &bank, &truck, Some(&biome), resources, &mut LabModel, &mut LabBackend, 1, 0)?;
            for d in &data {
                assert_eq!(state.count_word(d)?, 1);
            }
            //Nothing has loaded: the first unloaded Model ends 469C50 false.
            let ready = state.object_ready(&catalog, &bank, &truck, resources)?;
            let mut recording = Recording::default();
            state.release_object(&catalog, &bank, &truck, resources, &mut recording, 1)?;
            for d in &data {
                words.push(state.count_word(d)?);
            }
            Ok((recording.0, ready, words))
        })
        .unwrap();
    assert!(!ready);
    //45DF40's +534 tail released the building's own Data+20 (1 - 1).
    assert_eq!(counts, vec![0, 0]);
    assert!(released.iter().all(|(_, d)| *d == 1));
    let ids: Vec<&str> = released.iter().map(|(id, _)| id.as_str()).collect();
    assert!(ids.iter().any(|id| id.to_ascii_lowercase().contains("rustruck")), "{ids:?}");
    assert!(ids.iter().any(|id| id.to_ascii_lowercase().contains("ruscorps")), "{ids:?}");
    println!("released {} Resource handles: {ids:?}", released.len());
}
