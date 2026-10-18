//!Concrete76C310 output over the retained film/application/live World fields.
//!The actual netcode Node owns recorder attachment; the required bridge only
//!lends that exact service and the native player diagnostic. This adapter
//!never declares a Session/World ready or substitutes an absent text backend.
use crate::{
    actor_world::{ActorStore, Source, SpecialSource},
    catalog::Catalog,
    definitions::Bank,
    film_map_strings,
    film_resource::{RecorderHandle, Resource},
    film_session,
    film_text::MapTextOrder,
    film_variables,
    localization::Localization,
    resource_depot::Application,
};
use aa_actors::ActorId;
pub trait NodeRecorder {
    fn diagnostic_player(&mut self, ai: u8, implicit: u8, id: u32) -> film_session::Result<()>;
    ///The concrete implementation invokes actual74D5F0 on its retained Node.
    ///This bridge does not mint a World/Session ready or activation witness.
    fn attach_recorder_74d5f0(&mut self, handle: RecorderHandle) -> film_session::Result<()>;
}
pub struct Context<'a, 'source, B: NodeRecorder + ?Sized> {
    pub catalog: &'a Catalog<'source>,
    pub bank: &'a Bank,
    pub application: &'a mut Application,
    pub resource: &'a mut Resource,
    pub variables: &'a film_variables::State,
    ///Read retained preflight errors only at4738C0, after actual Header/path
    ///prefixes, rather than failing while this context is constructed.
    pub map_text: &'a std::result::Result<MapTextOrder, String>,
    pub live_text: &'a Localization,
    pub actors: &'a ActorStore,
    pub node: &'a mut B,
}
pub struct Current<'a, 'source, B: NodeRecorder + ?Sized> {
    context: Context<'a, 'source, B>,
    prelude_failure: Option<film_map_strings::PreludeFailure>,
}
impl<'a, 'source, B: NodeRecorder + ?Sized> Current<'a, 'source, B> {
    pub fn new(context: Context<'a, 'source, B>) -> Self {
        Self {
            context,
            prelude_failure: None,
        }
    }
    pub fn prelude_failure(&self) -> Option<&film_map_strings::PreludeFailure> {
        self.prelude_failure.as_ref()
    }
    pub fn into_prelude_failure(self) -> Option<film_map_strings::PreludeFailure> {
        self.prelude_failure
    }
}
impl<B: NodeRecorder + ?Sized> film_session::Output for Current<'_, '_, B> {
    fn save_map_header_and_strings(&mut self) -> film_session::Result<()> {
        if self.prelude_failure.is_some() {
            return Err(
                "failed native film prelude must be retained/retired before another attempt".into(),
            );
        }
        let c = &mut self.context;
        match film_map_strings::save_current_map(
            c.catalog,
            c.bank,
            film_map_strings::Current {
                application: c.application,
                resource: c.resource,
                variables: c.variables,
                order: c.map_text,
                text: c.live_text,
            },
        ) {
            Ok(()) => Ok(()),
            Err(failure) => {
                let error = failure.error().to_owned();
                self.prelude_failure = Some(failure);
                Err(error)
            }
        }
    }
    fn compression_enabled(&mut self) -> film_session::Result<u8> {
        Ok(self.context.variables.compressed_byte())
    }
    fn open_chunk(&mut self, name: &str, compressed: u8) -> film_session::Result<()> {
        self.context.resource.open_chunk(name, compressed)
    }
    fn write(&mut self, bytes: &[u8]) -> film_session::Result<()> {
        self.context.resource.write(bytes)
    }
    fn kingdom_index_188(&mut self, id: ActorId) -> film_session::Result<u32> {
        let key = self
            .context
            .actors
            .key(id)
            .ok_or("film player Kingdom generation is absent")?;
        let entity = self.context.actors.entity(key).map_err(|e| e.to_string())?;
        let Source::Special(special) = &entity.source else {
            return Err("film player Kingdom is not a live special allocation".into());
        };
        if !matches!(special.source, SpecialSource::Kingdom(_)) || !special.derived_payload_loaded {
            return Err("film player Kingdom derived fields are not completed".into());
        }
        special
            .derived
            .as_ref()
            .ok_or("film player Kingdom current storage is absent")?
            .words
            .get(&0x188)
            .copied()
            .ok_or("film Kingdom+188 current index is unbound".into())
    }
    fn diagnostic_player(&mut self, ai: u8, implicit: u8, id: u32) -> film_session::Result<()> {
        self.context.node.diagnostic_player(ai, implicit, id)
    }
    fn flush(&mut self) -> film_session::Result<()> {
        self.context.resource.flush()
    }
    fn attach_recorder(&mut self) -> film_session::Result<()> {
        self.context
            .node
            .attach_recorder_74d5f0(self.context.resource.recorder_handle()?)
    }
}
