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
    //Diagnostic census (AA_STARTUP_CENSUS=<new JSON path>): record every
    //failing definition instead of stopping at the first. The attempt still
    //fails afterwards; no failed definition is treated as prepared.
    let census = std::env::var_os("AA_STARTUP_CENSUS").map(std::path::PathBuf::from);
    let mut failures = Vec::new();
    for entry in &catalog.actors.actors {
        //4BDC50 hands the allocation 4BCA00 just registered to vf0 LoadData.
        //A later Thing translation (chaparral_wall -> temperate_walls) changes
        //Find, not which allocation loads, so select it by registered IDS.
        let thing = aa_rules::actors::ThingRef::registered(&entry.id);
        let registered = catalog.actors.select(thing)
            .ok_or_else(||format!("registered source Data {} is not available by its identity",entry.id))?;
        if registered.id != entry.id || registered.source != entry.source {
            return Err(format!("native source registered identity {} ({}:{}) differs from its registry slot {}",
                entry.id,entry.source.path,entry.source.span.start,registered.id));
        }
        let category = match catalog.actors.native_category(thing) {
            Ok(category) => category,
            Err(error) => {
                let error = format!("native source category {} ({}): {error}", entry.id, entry.source.path);
                if census.is_none() { return Err(error) }
                failures.push(serde_json::json!({"id": entry.id, "source": entry.source.path,
                    "line": entry.source.span.start, "kind": null, "error": error}));
                continue;
            }
        };
        //The other ThingData factories have their own source loaders. This
        //does not manufacture ObjectData for VFX/Projectile/other descriptors.
        if !matches!(category.native_kind, 1 | 2 | 20 | 21) {
            continue;
        }
        let result = (|| -> Result<()> {
            //Resolve before the immutable Bank loan. ActorBindings is the
            //existing canonical cache, not a replacement RuntimeDefinition.
            let definition = catalog.resolve_registered(&entry.id, precision)?;
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
            if census.is_none() {
                return Err(format!("native Object/Actor source preparation stopped at {} ({}:{}; kind{}; completed{}): {error}",
                    entry.id, entry.source.path, entry.source.span.start, category.native_kind, completed));
            }
            log::warn!("startup census: {} ({}; kind{}): {error}", entry.id, entry.source.path, category.native_kind);
            failures.push(serde_json::json!({"id": entry.id, "source": entry.source.path,
                "line": entry.source.span.start, "kind": category.native_kind,
                "class": category.runtime_class, "category": category.name, "error": error}));
            continue;
        }
        completed += 1;
        log::debug!("native Object/Actor source prepared {} ({completed})", entry.id);
    }
    if let Some(path) = census {
        let report = serde_json::json!({"stage": "pre-World Object/Actor source preparation",
            "registered": catalog.actors.actors.len(), "prepared": completed,
            "failed": failures.len(), "failures": failures});
        let text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
        std::fs::write(&path, text).map_err(|e| format!("startup census report {}: {e}", path.display()))?;
        //AA_STARTUP_CENSUS_CONTINUE=1 is a look-ahead diagnostic only: later
        //stages then run with the failed definitions still unprepared, so
        //their errors may include consequences of these failures.
        let look_ahead = std::env::var_os("AA_STARTUP_CENSUS_CONTINUE").is_some();
        if !failures.is_empty() && look_ahead {
            log::warn!("startup census look-ahead: continuing past {} failed definitions; this attempt is diagnostic only",
                failures.len());
        } else if !failures.is_empty() {
            return Err(format!("startup census: {} of {} definitions failed source preparation; report {}",
                failures.len(), catalog.actors.actors.len(), path.display()));
        }
    }
    Ok(Report { definitions: completed })
}
