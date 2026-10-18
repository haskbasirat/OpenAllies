//!54BDE0→540A60→546840/542A00 and the43FF20 VFX request loop.
//! Actual trigger definitions and canonical VFX references are borrowed; each
//! list occurrence is retained, including duplicates of the same definition.
use crate::{
    catalog::Catalog,
    definitions::Bank,
    resource_depot::Application,
    view_vfx_source::{self, Effect},
};
use aa_triggers::manager::Manager;
use std::collections::VecDeque;
type Result<T> = std::result::Result<T, String>;
#[path="view_trigger_vfx_collect.rs"]
mod ordered;
pub struct List {
    entries: VecDeque<Effect>,
    attempted: bool,
    collected: bool,
    requested: usize,
    complete: bool,
    stopped: bool,
}
impl List {
    ///442C00's B0/B4 initially null, not an inferred source/trigger census.
    pub fn from_constructor_fields() -> Self {
        Self {
            entries: VecDeque::new(),
            attempted: false,
            collected: false,
            requested: 0,
            complete: false,
            stopped: false,
        }
    }
    pub fn entries(&self) -> impl Iterator<Item = &Effect> {
        self.entries.iter()
    }
    pub fn requested_occurrences(&self) -> usize {
        self.requested
    }
    pub fn is_complete(&self) -> bool {
        self.complete && !self.stopped
    }
    pub fn collect(&mut self, catalog: &Catalog<'_>, bank: &Bank, manager: &Manager) -> Result<()> {
        if self.attempted {
            return Err("VWorld trigger VFX collection already attempted; actual teardown/release is required".into());
        }
        self.attempted = true;
        //546840 Find precedes vf48. find() executes the SAME one-hop
        //canonical source selection and exact class check.
        let result = ordered::collect(&mut self.entries,manager,|ids|view_vfx_source::find(catalog,bank,ids));
        self.collected = result.is_ok();
        self.stopped = result.is_err();
        result
    }
    ///Concrete native base-count request path. The same application owns all
    ///Data+20 counters. Resource-owning classes stop AFTER their base prefix;
    ///no constructor-null or ignored positive child success is manufactured.
    pub fn request(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        application: &Application,
    ) -> Result<()> {
        if self.stopped || !self.collected || self.complete {
            return Err("VWorld trigger VFX request is outside its fresh collected interval".into());
        }
        let result = ordered::request(&self.entries,&mut self.requested,|effect|{
                application.with_data_pre_request(|state|state.pre_request_vfx(
                    catalog,bank,effect,1,0,&mut crate::data_pre_request::RequiredVfxResources,
                ))
        });
        self.complete = result.is_ok();
        self.stopped = result.is_err();
        result
    }
}
