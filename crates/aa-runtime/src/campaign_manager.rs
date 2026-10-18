//! Same fresh Application Campaign table read by411790/4F1310.
//! B397CC/D0/D4 begin in the PE-loader zero tail;9C79C0/40CAB0 do not
//! populate them. This owner is created once with the fresh application state,
//! never from an arbitrary existing mode or a freshly captured map Header.
use crate::application_mode::GameAppState;
use std::sync::Weak;

pub type Result<T> = std::result::Result<T, String>;

/// Opaque current Campaign reference. Positive allocation/resource loading is
/// still mandatory, so this slice provides no public constructor or cloning.
#[derive(Debug)]
pub struct Reference {
    origin: Weak<()>,
    slot: usize,
}

/// The actual empty growable table has one owner. No raw/Default/Clone/reset
/// API permits replacing a populated or existing manager with a new empty one.
#[derive(Debug)]
pub struct State {
    application: Weak<()>,
    entries: Vec<Reference>,
}
impl State {
    ///Only GameAppState's fresh application constructor supplies its private
    ///identity. No accessor exposes that capability for later reconstruction.
    pub(crate) fn from_application_constructor(application: Weak<()>) -> Self {
        Self {
            application,
            entries: Vec::new(),
        }
    }
    fn require_application(&self, application: &GameAppState) -> Result<()> {
        if !application.campaign_identity_matches(&self.application) {
            return Err("Campaign table belongs to another or destroyed application".into());
        }
        Ok(())
    }
    pub fn active_count(&self) -> usize {
        self.entries.len()
    }

    ///Actual411790 tests its current signed count before reading the name.
    ///The constructor-empty false lookup therefore needs no UserSettings
    ///CString contents, current CampaignData or invented resource identity.
    pub fn lookup_411790(
        &self,
        application: &GameAppState,
        create: bool,
    ) -> Result<Option<Reference>> {
        self.require_application(application)?;
        if !self.entries.is_empty() {
            return Err(
                "411790 positive Campaign rows require actual comparison/resource owners".into(),
            );
        }
        if !create {
            return Ok(None);
        }
        //411825 pushes20, then411827 calls the real allocation. Stop BEFORE
        //that required producer: no append, resource lookup or no-op creation.
        Err("411790 requires actual6F7810/470970 Campaign allocation and resource producer".into())
    }
    ///4F1310 calls40CEB0(0xC) and reads that SAME GameAppState+C. Nonzero
    ///returns null without UserSettings. Mode0 reaches the required positive
    ///lookup; this is not inferred from a local Session player or MapResource.
    pub fn current_4f1310(&self, application: &GameAppState) -> Result<Option<Reference>> {
        self.require_application(application)?;
        if application.mode() != 0 {
            return Ok(None);
        }
        self.lookup_411790(application, true)
    }
    fn require_reference(&self, application: &GameAppState, reference: &Reference) -> Result<()> {
        self.require_application(application)?;
        if !application.campaign_identity_matches(&reference.origin)
            || reference.slot >= self.entries.len()
        {
            return Err("current Campaign reference has no actual retained allocation".into());
        }
        Ok(())
    }
    ///Mandatory positive getters for the generic Header source. Fresh null
    ///branches never call these; no default name or campaign index is supplied.
    pub fn name_08(&self, application: &GameAppState, reference: &Reference) -> Result<&[u16]> {
        self.require_reference(application, reference)?;
        Err("current CampaignData+8 name owner is unbound".into())
    }
    pub fn index_14(&self, application: &GameAppState, reference: &Reference) -> Result<u32> {
        self.require_reference(application, reference)?;
        Err("current CampaignState+14 index owner is unbound".into())
    }
}

#[cfg(test)]
#[path = "campaign_manager_tests.rs"]
mod tests;
