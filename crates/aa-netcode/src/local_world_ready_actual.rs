//!Concrete7757F0 services. Only authentic completed task/mission owners enter
//!this procedure. This is still before7738B0 and grants no product activation.
use super::*;
use aa_runtime::{
    catalog::Catalog, load_view_variables::NativeOverrides, world_frame_inputs::WorldFrameInputs,
    world_ready_services::Application,
};

#[derive(Debug, PartialEq, Eq)]
pub enum Diagnostic {
    OpeningFilm(Vec<u16>),
    FilmPlayer { ai: u8, implicit: u8, id: u32 },
    Prepare(super::super::begin_play_live::Diagnostic),
}
enum Owners {
    Task(super::super::load_task::SuccessfulTask),
    Mission {
        mission: aa_runtime::load_mission_start_world::StartedMission,
        ticket: super::super::load_task::CallbacksTicket,
    },
    Frame {
        frame: WorldFrameInputs,
        ticket: super::super::load_task::CallbacksTicket,
    },
}
pub struct Failure {
    error: String,
    owners: Owners,
    film: Option<aa_runtime::film_create::Failure<String>>,
    prelude: Option<aa_runtime::film_map_strings::PreludeFailure>,
}
impl Failure {
    pub fn error(&self) -> &str {
        &self.error
    }
}
impl std::fmt::Debug for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorldReadyFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}
pub(crate) struct Ready {
    pub(super) frame: WorldFrameInputs,
    pub(super) ticket: super::super::load_task::CallbacksTicket,
}
impl LocalBootstrap {
    ///Concrete pre-success callback. No raw Staged, scalar-ready bit or
    ///caller-provided successful game-service implementation is accepted.
    pub(crate) fn finish_world_ready_task<C: FnMut() -> std::result::Result<[u32; 2], String>>(
        &mut self,
        task: super::super::load_task::SuccessfulTask,
        catalog: &Catalog<'_>,
        services: &mut Application,
        current_local_filetime: &mut C,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> std::result::Result<Ready, Failure> {
        if let Err(error) = services.validate(catalog) {
            return Err(Failure {
                error,
                owners: Owners::Task(task),
                film: None,
                prelude: None,
            });
        }
        let (mission, ticket) = task.into_parts(self).map_err(|(task, error)| Failure {
            error,
            owners: Owners::Task(task),
            film: None,
            prelude: None,
        })?;
        let mut frame = match WorldFrameInputs::from_started(mission) {
            Ok(frame) => frame,
            Err((mission, error)) => {
                return Err(Failure {
                    error,
                    owners: Owners::Mission { mission, ticket },
                    film: None,
                    prelude: None,
                });
            }
        };
        let mut actual = Current {
            owner: self,
            frame: &mut frame,
            catalog,
            services,
            current_local_filetime,
            diagnostics,
            film_failure: None,
            prelude_failure: None,
        };
        let result = run(&mut actual).map_err(|e| e.to_string());
        let film = actual.film_failure.take();
        let prelude = actual.prelude_failure.take();
        drop(actual);
        if let Err(error) = result.and_then(|()| frame.require_sai_complete(catalog)) {
            return Err(Failure {
                error,
                owners: Owners::Frame { frame, ticket },
                film,
                prelude,
            });
        }
        Ok(Ready { frame, ticket })
    }
}
struct Current<'a, 'source, C> {
    owner: &'a mut LocalBootstrap,
    frame: &'a mut WorldFrameInputs,
    catalog: &'a Catalog<'source>,
    services: &'a mut Application,
    current_local_filetime: &'a mut C,
    diagnostics: &'a mut Vec<Diagnostic>,
    film_failure: Option<aa_runtime::film_create::Failure<String>>,
    prelude_failure: Option<aa_runtime::film_map_strings::PreludeFailure>,
}
impl<C: FnMut() -> std::result::Result<[u32; 2], String>> World for Current<'_, '_, C> {
    type Film = aa_runtime::film_resource::Resource;
    fn reset_name(&mut self) -> Result<()> {
        self.owner
            .reset_world_ready_film_name(self.services.application());
        Ok(())
    }
    fn state(&mut self) -> Result<u32> {
        SessionPhase::application_state(self.owner).map_err(missing)
    }
    fn phase(&mut self) -> Result<u32> {
        SessionPhase::current_phase(self.owner).map_err(missing)
    }
    fn old_film(&mut self) -> Result<bool> {
        Ok(self.owner.film_1c.is_some())
    }
    fn detach(&mut self) -> Result<()> {
        self.owner
            .host_node
            .as_mut()
            .ok_or_else(|| missing("film detach actual Node absent"))?
            .detach_recorder_74b310();
        Ok(())
    }
    fn close(&mut self) -> Result<()> {
        self.owner
            .film_1c
            .as_mut()
            .ok_or_else(|| missing("film close current Session1C absent"))?
            .close()
            .map(|_| ())
            .map_err(missing)
    }
    fn clear_film(&mut self) -> Result<()> {
        self.owner.film_1c = None;
        Ok(())
    }
    fn prepare(&mut self) -> Result<()> {
        let parts = self.services.parts(self.catalog).map_err(missing)?;
        let mut diagnostics = Vec::new();
        let result = self.owner.prepare_retained_players(
            self.frame,
            self.catalog,
            parts.application,
            parts.glyph,
            &mut diagnostics,
        );
        self.diagnostics
            .extend(diagnostics.into_iter().map(Diagnostic::Prepare));
        result
    }
    fn rotation(&mut self) -> Result<i32> {
        Ok(self
            .services
            .parts(self.catalog)
            .map_err(missing)?
            .film
            .rotation_queue_size())
    }
    fn filename(&mut self) -> Result<Vec<u16>> {
        let world = self.frame.recording_parts(self.catalog).map_err(missing)?;
        let parts = self.services.parts(self.catalog).map_err(missing)?;
        Ok(parts
            .film
            .filename(&mut self.owner.session.film_name_150, 1, world.live_text)
            .map_err(missing)?
            .to_vec())
    }
    fn opening(&mut self, name: &[u16]) -> Result<()> {
        self.diagnostics
            .push(Diagnostic::OpeningFilm(name.to_vec()));
        Ok(())
    }
    fn extension(&mut self, name: &mut Vec<u16>) -> Result<()> {
        aa_runtime::film_paths::append_extension(name, &['R' as u16, 'F' as u16, 'M' as u16]);
        Ok(())
    }
    fn path(&mut self) -> Result<Vec<u16>> {
        Ok(self
            .services
            .parts(self.catalog)
            .map_err(missing)?
            .film
            .path_films()
            .to_vec())
    }
    fn join(&mut self, path: &[u16], name: &[u16]) -> Result<Vec<u16>> {
        Ok(aa_runtime::film_paths::join(path, name))
    }
    fn construct(&mut self, ids: &[u16]) -> Result<Self::Film> {
        let current_world=self.frame.revision().witness();
        let services = self.services.parts(self.catalog).map_err(missing)?;
        let world = self
            .frame
            .film_capture_parts(self.catalog)
            .map_err(missing)?;
        let (_, mut players, _, sim) = self.owner.recording_parts()?;
        let mut roster = players.glyph_roster_in_created_world(sim, current_world, world.standalone);
        let context = aa_runtime::film_world_capture::Context {
            catalog: self.catalog,
            bank: world.bank,
            world: world.world,
            actors: world.actors,
            live_text: world.live_text,
            application: services.application,
            ai_manager: world.ai_manager,
            ai_players: world.ai_players,
            formatter: services.formatter,
            glyph_variables: services.glyph,
            players: &mut roster,
            current_local_filetime: &mut *self.current_local_filetime,
        };
        let (mut depot, process) = services
            .depot
            .film_parts(self.catalog, NativeOverrides::ControlledNone)
            .map_err(missing)?;
        let mut factory = aa_runtime::film_resource_factory::Current::new(ids, process, context);
        match aa_runtime::film_create::create_with(&mut depot, ids, &mut factory) {
            Ok(resource) => Ok(resource),
            Err(failure) => {
                let error = failure.to_string();
                self.film_failure = Some(failure);
                Err(missing(error))
            }
        }
    }
    fn publish(&mut self, film: Self::Film) -> Result<()> {
        self.owner.publish_film_resource(film)
    }
    fn open(&mut self) -> Result<()> {
        let resource = self
            .owner
            .film_resource_mut()
            .ok_or_else(|| missing("7757F0 published film absent"))?;
        self.services
            .depot()
            .open_film_file(resource)
            .map_err(|e| missing(e.to_string()))
    }
    fn record(&mut self) -> Result<()> {
        let current_world=self.frame.revision().witness();
        let world = self.frame.recording_parts(self.catalog).map_err(missing)?;
        let services = self.services.parts(self.catalog).map_err(missing)?;
        let (film, mut players, node, sim) = self.owner.recording_parts()?;
        let mut roster = players.film_roster_in_created_world(sim, current_world);
        let mut bridge = NodeBridge {
            node,
            diagnostics: self.diagnostics,
        };
        let mut output = aa_runtime::film_session_output::Current::new(
            aa_runtime::film_session_output::Context {
                catalog: self.catalog,
                bank: world.bank,
                application: services.depot,
                resource: film
                    .as_mut()
                    .ok_or("7757F0 film preamble Session1C absent")
                    .map_err(missing)?,
                variables: services.film,
                map_text: world.map_text,
                live_text: world.live_text,
                actors: world.actors,
                node: &mut bridge,
            },
        );
        let result = aa_runtime::film_session::open(&mut roster, &mut output).map_err(missing);
        self.prelude_failure = output.into_prelude_failure();
        result
    }
    fn sai(&mut self) -> Result<()> {
        self.frame.initialize_sai(self.catalog).map_err(missing)
    }
    fn playback(&mut self) -> Result<()> {
        Err(missing("7757F0 actual76E1D0 playback owner required"))
    }
    fn playback_name_present(&mut self) -> Result<bool> {
        Err(missing(
            "7757F0 actual playback LaunchSettings68+3C owner required",
        ))
    }
    fn store_playback_flag(&mut self, value: bool) -> Result<()> {
        self.owner.session.flags_168_16e[0] = Some(value);
        Ok(())
    }
}
struct NodeBridge<'a> {
    node: &'a mut crate::local_host::Node,
    diagnostics: &'a mut Vec<Diagnostic>,
}
impl aa_runtime::film_session_output::NodeRecorder for NodeBridge<'_> {
    fn diagnostic_player(
        &mut self,
        ai: u8,
        implicit: u8,
        id: u32,
    ) -> std::result::Result<(), String> {
        self.diagnostics
            .push(Diagnostic::FilmPlayer { ai, implicit, id });
        Ok(())
    }
    fn attach_recorder_74d5f0(
        &mut self,
        handle: aa_runtime::film_resource::RecorderHandle,
    ) -> std::result::Result<(), String> {
        self.node
            .attach_recorder_74d5f0(handle)
            .map_err(|e| e.to_string())
    }
}
