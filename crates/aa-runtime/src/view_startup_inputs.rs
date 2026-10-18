//! Retained application owners used by the native VWorld startup prefix.
//! This composition moves the existing seed/AVar allocations; it neither
//! registers another variable manager nor authorizes a completed view/battle.
use crate::{
    catalog::Catalog, combat_source_receipt::VerifiedStartupReceipt, definitions::Bank,
    load_view_variables::NativeOverrides, native_camera_render_source, native_camera_variables,
    view_asset_aliases, view_grid_startup, view_grid_variables, view_renderer_variables,view_replacement_color,
};
use aa_rules::sources::V14Sources;

///One application retains this move-only bundle before its source backend is
///moved into the resource depot. Later reads borrow the same current cells.
pub struct Inputs {
    seeds: view_grid_startup::Seeds,
    grid: view_grid_variables::State,
    orientation: view_renderer_variables::State,
    replacement_color:view_replacement_color::State,
    camera: native_camera_variables::State,
    camera_render: native_camera_render_source::State,
    aliases: view_asset_aliases::State,
    helper_globals: crate::view_init_globals::State,
}
///Not an initialized Inputs. Previously constructed owners, including a
///partially applied source assignment pass, stay reachable on failure.
pub struct Failure {
    pub error: String,
    seeds: Option<view_grid_startup::Seeds>,
    grid: Option<view_grid_variables::State>,
    orientation: Option<view_renderer_variables::State>,
    replacement_color:Option<view_replacement_color::State>,
    camera: Option<native_camera_variables::State>,
    camera_render: Option<native_camera_render_source::State>,
    aliases: Option<view_asset_aliases::State>,
}
impl Failure {
    pub fn seeds(&self) -> Option<&view_grid_startup::Seeds> {
        self.seeds.as_ref()
    }
    pub fn grid(&self) -> Option<&view_grid_variables::State> {
        self.grid.as_ref()
    }
    pub fn orientation(&self) -> Option<&view_renderer_variables::State> {
        self.orientation.as_ref()
    }
    pub fn replacement_color(&self)->Option<&view_replacement_color::State>{self.replacement_color.as_ref()}
    pub fn camera(&self) -> Option<&native_camera_variables::State> {
        self.camera.as_ref()
    }
    pub fn camera_render(&self) -> Option<&native_camera_render_source::State> {
        self.camera_render.as_ref()
    }
    pub fn aliases(&self) -> Option<&view_asset_aliases::State> {
        self.aliases.as_ref()
    }
}
impl std::fmt::Debug for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewStartupInputFailure")
            .field("error", &self.error)
            .field("seeds", &self.seeds.is_some())
            .field("grid", &self.grid.is_some())
            .field("orientation", &self.orientation.is_some())
            .field("camera", &self.camera.is_some())
            .field("camera_render", &self.camera_render.is_some())
            .field("aliases", &self.aliases.is_some())
            .finish()
    }
}
impl Inputs {
    ///This is source-owner composition, not an additional native call-order
    ///claim. Member constructors retain their separately proved semantics.
    ///No raw-word, caller-PC, default, Clone or Deserialize constructor exists.
    pub fn from_fresh_sources(
        catalog: &Catalog<'_>,
        receipt: &VerifiedStartupReceipt,
        files: &mut V14Sources,
        overrides: NativeOverrides,
    ) -> Result<Self, Failure> {
        let mut partial = Failure {
            error: String::new(),
            seeds: None,
            grid: None,
            orientation: None,
            replacement_color:None,
            camera: None,
            camera_render: None,
            aliases: None,
        };
        let build = (|| -> Result<(), String> {
            catalog.require_source_owner()?;
            if !matches!(overrides, NativeOverrides::ControlledNone) {
                return Err("view startup current profile/console owner is unbound".into());
            }
            let bank = catalog
                .definition_bank
                .try_borrow()
                .map_err(|_| "view startup source Bank is already mutably borrowed")?;
            let source = bank.combat_source()?;
            source.completed_means()?;
            if source.source_identities() != catalog.actors.sources.as_slice()
                || source.context().reference_startup() != receipt.context().reference_startup()
                || source.source_precision() != receipt.context().precision()
            {
                return Err("view startup completed source Bank/receipt differs".into());
            }
            partial.seeds = Some(view_grid_startup::Seeds::from_verified_startup_reference(
                catalog, receipt,
            )?);
            match view_grid_variables::State::from_fresh_sources_retained(
                catalog,
                files,
                NativeOverrides::ControlledNone,
            ) {
                Ok(state) => partial.grid = Some(state),
                Err(failure) => {
                    partial.grid = failure.state;
                    return Err(failure.error);
                }
            }
            match view_renderer_variables::State::from_fresh_sources_retained(
                catalog,
                &bank,
                files,
                NativeOverrides::ControlledNone,
            ) {
                Ok(state) => partial.orientation = Some(state),
                Err(failure) => {
                    partial.orientation = failure.state;
                    return Err(failure.error);
                }
            }
            match view_replacement_color::State::from_fresh_sources_retained(catalog,&bank,files,NativeOverrides::ControlledNone){
                Ok(state)=>partial.replacement_color=Some(state),
                Err(failure)=>{partial.replacement_color=failure.state;return Err(failure.error);}
            }
            match native_camera_variables::State::from_fresh_sources_retained(
                catalog,
                &bank,
                files,
                NativeOverrides::ControlledNone,
            ) {
                Ok(state) => partial.camera = Some(state),
                Err(failure) => {
                    partial.camera = failure.state;
                    return Err(failure.error);
                }
            }
            match native_camera_render_source::State::from_fresh_sources_retained(
                catalog,
                &bank,
                files,
                NativeOverrides::ControlledNone,
            ) {
                Ok(state) => partial.camera_render = Some(state),
                Err(failure) => {
                    partial.camera_render = failure.state;
                    return Err(failure.error);
                }
            }
            match view_asset_aliases::State::from_fresh_sources_retained(
                catalog,
                files,
                NativeOverrides::ControlledNone,
            ) {
                Ok(state) => partial.aliases = Some(state),
                Err(failure) => {
                    partial.aliases = failure.state;
                    return Err(failure.error);
                }
            }
            Ok(())
        })();
        if let Err(error) = build {
            partial.error = error;
            return Err(partial);
        }
        Ok(Self {
            seeds: partial.seeds.expect("successful seed construction"),
            grid: partial.grid.expect("successful grid variable construction"),
            orientation: partial
                .orientation
                .expect("successful orientation construction"),
            replacement_color:partial.replacement_color.expect("successful replacement-color construction"),
            camera: partial
                .camera
                .expect("successful camera variable construction"),
            camera_render: partial
                .camera_render
                .expect("successful camera render source construction"),
            aliases: partial.aliases.expect("successful alias construction"),
            helper_globals: crate::view_init_globals::State::fresh_application(),
        })
    }
    ///The authoritative Bank can have moved into Staged; it must be lent by
    ///its current owner, never rebuilt from Catalog after that move.
    pub fn validate(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<(), String> {
        bank.combat_source()?.completed_means()?;
        self.seeds.validate_catalog(catalog)?;
        self.grid.validate(catalog)?;
        self.orientation.validate(catalog, bank)?;
        self.replacement_color.validate(catalog,bank)?;
        self.camera.validate(catalog, bank)?;
        self.camera_render.validate(catalog, bank)?;
        self.aliases.validate_in_bank(catalog, bank)
    }
    pub fn seeds(&self) -> &view_grid_startup::Seeds {
        &self.seeds
    }
    pub fn grid(&self) -> &view_grid_variables::State {
        &self.grid
    }
    pub fn grid_mut(&mut self) -> &mut view_grid_variables::State {
        &mut self.grid
    }
    pub fn orientation(&self) -> &view_renderer_variables::State {
        &self.orientation
    }
    pub fn replacement_color(&self)->&view_replacement_color::State{&self.replacement_color}
    pub fn camera(&self) -> &native_camera_variables::State {
        &self.camera
    }
    pub fn camera_mut(&mut self) -> &mut native_camera_variables::State {
        &mut self.camera
    }
    pub fn camera_render(&self) -> &native_camera_render_source::State {
        &self.camera_render
    }
    pub fn camera_render_mut(&mut self) -> &mut native_camera_render_source::State {
        &mut self.camera_render
    }
    pub fn aliases(&self) -> &view_asset_aliases::State {
        &self.aliases
    }
    pub fn aliases_mut(&mut self) -> &mut view_asset_aliases::State {
        &mut self.aliases
    }
    pub fn helper_globals(&self) -> &crate::view_init_globals::State {
        &self.helper_globals
    }
    pub fn helper_globals_mut(&mut self) -> &mut crate::view_init_globals::State {
        &mut self.helper_globals
    }
}

///Duplicate/foreign installation never consumes the caller's incoming owner.
pub struct InstallFailure {
    pub inputs: Inputs,
    pub error: String,
}
impl std::fmt::Debug for InstallFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewStartupInstallFailure")
            .field("error", &self.error)
            .finish()
    }
}

#[cfg(test)]
#[path = "view_startup_inputs_tests.rs"]
mod tests;
