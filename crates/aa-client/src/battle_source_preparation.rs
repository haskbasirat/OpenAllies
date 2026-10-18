//! Pre-World Object/Actor source children on the retained simulation execution.
//! The source catalog's Thing vector is the native48D660/4BDC50 first-wins
//! registration order. Existing49DC40 producers retain every partial Aspect,
//! Resource and Graph in the SAME application; this is not World/View readiness.
use crate::{battle_texture_services, battle_ui_bridge};
use aa_mapworld::restore::RestoreDefinitions;
use aa_runtime::{
    catalog::Catalog,
    resource_objects::{ModelPolicy, TexturePolicy},
    simulation_execution::Execution,
    world_ready_services::Application,
};
use std::cell::RefCell;

type Result<T> = std::result::Result<T, String>;

///Diagnostic counts only. No constructor/readiness capability is minted here.
pub(crate) struct Report {
    pub definitions: usize,
}

///Two interfaces borrow ONE actual execution for one reached native read.
///No UI value, GPU policy or execution identity is copied into a source default.
struct Current<'a, 'execution, 'run> {
    bridge: &'a battle_ui_bridge::Client,
    execution: RefCell<&'execution mut Execution<'run>>,
}
impl<'run> Current<'_, '_, 'run> {
    fn with<T>(&self, call: impl FnOnce(&battle_ui_bridge::Client, &mut Execution<'run>) -> Result<T>) -> Result<T> {
        let mut execution = self.execution.try_borrow_mut()
            .map_err(|_| "reentrant native source policy execution loan")?;
        call(self.bridge, &mut **execution)
    }
}
struct Model<'a, 'b, 'c, 'run>(&'a Current<'b, 'c, 'run>);
struct Texture<'a, 'b, 'c, 'run>(&'a Current<'b, 'c, 'run>);
impl ModelPolicy for Model<'_, '_, '_, '_> {
    fn use_hardware_model_73a560(&mut self) -> Result<bool> {
        self.0.with(|bridge, execution| {
            crate::live_scene::model_policy::UiPolicy::new(bridge, execution)
                .use_hardware_model_73a560()
        })
    }
    fn hardware_model_suffix_40f7b0(&mut self) -> Result<Vec<u16>> {
        self.0.with(|bridge, execution| {
            crate::live_scene::model_policy::UiPolicy::new(bridge, execution)
                .hardware_model_suffix_40f7b0()
        })
    }
}
impl TexturePolicy for Texture<'_, '_, '_, '_> {
    fn data_compatibility_mode_current(&mut self) -> Result<u8> {
        self.0.with(|bridge, execution| {
            battle_texture_services::Current::for_controlled_fresh_start(bridge, execution)
                .data_compatibility_mode_current()
        })
    }
    fn data_missing_resource_warning_current(&mut self) -> Result<u8> {
        self.0.with(|bridge, execution| {
            battle_texture_services::Current::for_controlled_fresh_start(bridge, execution)
                .data_missing_resource_warning_current()
        })
    }
    fn missing_texture_diagnostic_404fc0(&mut self, ids: &[u16], level: u32) -> Result<()> {
        self.0.with(|bridge, execution| {
            battle_texture_services::Current::for_controlled_fresh_start(bridge, execution)
                .missing_texture_diagnostic_404fc0(ids, level)
        })
    }
    fn texture_cache_source_overrides(&mut self) -> Result<aa_runtime::load_view_variables::NativeOverrides> {
        self.0.with(|bridge, execution| {
            battle_texture_services::Current::for_controlled_fresh_start(bridge, execution)
                .texture_cache_source_overrides()
        })
    }
}

pub(crate) fn prepare(
    catalog: &Catalog<'_>,
    application: &mut Application,
    bridge: &battle_ui_bridge::Client,
    execution: &mut Execution<'_>,
) -> Result<Report> {
    application.validate(catalog)?;
    let precision = catalog.definition_bank.try_borrow()
        .map_err(|_| "native source precision Bank is already borrowed")?
        .combat_source()?.source_precision();
    let current = Current { bridge, execution: RefCell::new(execution) };
    let (mut model, mut texture) = (Model(&current), Texture(&current));
    let mut completed = 0;
    for entry in &catalog.actors.actors {
        let canonical = catalog.actors.find(&entry.id)
            .ok_or_else(||format!("registered source Data {} is not available by its identity",entry.id))?;
        if canonical.id != entry.id || canonical.source != entry.source {
            return Err(format!("native source registered identity {} ({}:{}) is shadowed by final Thing lookup to {}; registered-identity source projection is required",
                entry.id,entry.source.path,entry.source.span.start,canonical.id));
        }
        let category = catalog.actors.native_category(&entry.id)
            .map_err(|error| format!("native source category {} ({}): {error}", entry.id, entry.source.path))?;
        //The other ThingData factories have their own source loaders. This
        //does not manufacture ObjectData for VFX/Projectile/other descriptors.
        if !matches!(category.native_kind, 1 | 2 | 20 | 21) {
            continue;
        }
        let result = (|| -> Result<()> {
            //Resolve before the immutable Bank loan. ActorBindings is the
            //existing canonical cache, not a replacement RuntimeDefinition.
            let definition = catalog.resolve(&entry.id.encode_utf16().collect::<Vec<_>>(), precision)?;
            if definition.proof.id != entry.id || definition.proof.source != entry.source {
                return Err("native source resolver changed the actual registered Data allocation".into());
            }
            let bank = catalog.definition_bank.try_borrow()
                .map_err(|_| "native source Object/Actor Bank is already mutably borrowed")?;
            application.depot_mut().with_data_pre_request_resources(|state, resources| {
                state.prepare_object_requests(catalog, &bank, &definition, resources, &mut model, &mut texture)?;
                Ok(())
            })
        })();
        if let Err(error) = result {
            return Err(format!("native Object/Actor source preparation stopped at {} ({}:{}; kind{}; completed{}): {error}",
                entry.id, entry.source.path, entry.source.span.start, category.native_kind, completed));
        }
        completed += 1;
        log::debug!("native Object/Actor source prepared {} ({completed})", entry.id);
    }
    Ok(Report { definitions: completed })
}
