//! Retained application owners used by the native VWorld startup prefix.
//! This composition moves the existing seed/AVar allocations; it neither
//! registers another variable manager nor authorizes a completed view/battle.
use crate::{
    catalog::Catalog, combat_source_receipt::VerifiedStartupReceipt, definitions::Bank,
    load_view_variables::NativeOverrides, view_grid_startup, view_grid_variables,
    view_renderer_variables,
};
use aa_rules::sources::V14Sources;

///One application retains this move-only bundle before its source backend is
///moved into the resource depot. Later reads borrow the same current cells.
pub struct Inputs {
    seeds: view_grid_startup::Seeds,
    grid: view_grid_variables::State,
    orientation: view_renderer_variables::State,
}
///Not an initialized Inputs. Previously constructed owners, including a
///partially applied source assignment pass, stay reachable on failure.
pub struct Failure {
    pub error: String,
    seeds: Option<view_grid_startup::Seeds>,
    grid: Option<view_grid_variables::State>,
    orientation: Option<view_renderer_variables::State>,
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
}
impl std::fmt::Debug for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewStartupInputFailure")
            .field("error", &self.error)
            .field("seeds", &self.seeds.is_some())
            .field("grid", &self.grid.is_some())
            .field("orientation", &self.orientation.is_some())
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
        })
    }
    ///The authoritative Bank can have moved into Staged; it must be lent by
    ///its current owner, never rebuilt from Catalog after that move.
    pub fn validate(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<(), String> {
        bank.combat_source()?.completed_means()?;
        self.seeds.validate_catalog(catalog)?;
        self.grid.validate(catalog)?;
        self.orientation.validate(catalog, bank)
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
