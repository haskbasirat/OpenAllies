//!Composition of the actual retained replacement queue with fresh Actor
//!creation. Each method receives the same Staged owners and original request.
//!Required destruction/diagnostic/property children retain their exact errors.
use crate::{
    actor_creation_startup::StartupInputs,
    actor_world::ObjectKey,
    catalog::Catalog,
    map_replacement_drain::{self, Effects, Result},
    map_replacement_queue::{ActorRequest, DefinitionRef, ObjectRequest, RequestRef},
    restoration::Staged,
};
use aa_numeric::Precision;
pub(crate) struct Creation<'a> {
    pub startup: &'a StartupInputs,
}
impl Effects for Creation<'_> {
    fn remove(
        &mut self,
        s: &mut Staged,
        c: &Catalog<'_>,
        old: ObjectKey,
        p: Precision,
    ) -> Result<()> {
        map_replacement_drain::Required.remove(s, c, old, p)
    }
    fn virtual_160(
        &mut self,
        s: &mut Staged,
        c: &Catalog<'_>,
        old: ObjectKey,
        arg: u32,
        p: Precision,
    ) -> Result<()> {
        map_replacement_drain::Required.virtual_160(s, c, old, arg, p)
    }
    fn create_object(
        &mut self,
        s: &mut Staged,
        c: &Catalog<'_>,
        r: &mut ObjectRequest,
        p: Precision,
    ) -> Result<Option<ObjectKey>> {
        map_replacement_drain::Required.create_object(s, c, r, p)
    }
    fn create_actor(
        &mut self,
        s: &mut Staged,
        c: &Catalog<'_>,
        r: &mut ActorRequest,
        p: Precision,
    ) -> Result<Option<ObjectKey>> {
        crate::map_actor_creation::create(s, c, &mut r.params, self.startup, p)
    }
    fn property(
        &mut self,
        s: &mut Staged,
        c: &Catalog<'_>,
        new: ObjectKey,
        d: Option<&DefinitionRef>,
        p: Precision,
    ) -> Result<()> {
        map_replacement_drain::Required.property(s, c, new, d, p)
    }
    fn created_log(&mut self, s: &mut Staged, r: RequestRef<'_>) -> Result<()> {
        map_replacement_drain::Required.created_log(s, r)
    }
    fn failure_log(&mut self, s: &mut Staged, r: RequestRef<'_>) -> Result<()> {
        map_replacement_drain::Required.failure_log(s, r)
    }
    fn failure_text_ui(
        &mut self,
        s: &mut Staged,
        c: &Catalog<'_>,
        r: RequestRef<'_>,
        p: Precision,
    ) -> Result<()> {
        map_replacement_drain::Required.failure_text_ui(s, c, r, p)
    }
}
