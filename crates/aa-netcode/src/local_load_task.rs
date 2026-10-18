//!775EE0 ->76BE80 actual single-session load task. Session124 and the queued
//!job share one move-only task allocation. The resource worker is allowed to
//!remain unscheduled until77A950 has performed its main-thread state stores.
//!No polling/queued flag can manufacture a StartedMission or finish775DB0.
use super::*;
use aa_runtime::{
    execution_context::{CompletedWorker, WorkerIdentity, WorkerWitness},
    load_game_task::Inputs,
    load_mission_start_world::StartedMission,
};

enum Run {
    Queued,
    Registered(WorkerWitness),
    Running(WorkerWitness),
    Completed(CompletedWorker<StartedMission>),
    Rejected {
        receipt: CompletedWorker<StartedMission>,
        error: String,
    },
    InCallbacks,
}
pub(super) struct Task {
    owner: Weak<()>,
    inputs: Inputs,
    state: Mutex<Run>,
    ///Native+C is unbound until this SAME task's actual5AE160 publication.
    ///The opaque slot belongs to the application's one ResourceManager queue.
    sequence_c: Mutex<Option<aa_runtime::resource_task::ExternalSequence>>,
}
impl Task {
    fn sequence_c(&self) -> Result<u32> {
        let slot = self
            .sequence_c
            .lock()
            .map_err(|_| missing("task sequence field lock poisoned"))?;
        slot.as_ref()
            .ok_or_else(|| missing("actual LoadGameTask sequence+C is not published"))?
            .value()
            .map_err(missing)
    }
    ///The SAME retained task's native+14 projection; no copied status owner.
    pub(super) fn status_14(&self) -> Result<u32> {
        let run = self
            .state
            .lock()
            .map_err(|_| missing("task state lock poisoned"))?;
        match &*run {
            Run::Queued | Run::Registered(_) => Ok(0),
            Run::Running(_) => Ok(1),
            Run::Completed(receipt) => match receipt.outcome() {
                Ok(_) => Ok(2),
                Err(error) => Err(missing(format!(
                    "actual worker stopped: {error}; native failure-status producer remains required"
                ))),
            },
            Run::InCallbacks => Ok(2),
            Run::Rejected { error, .. } => Err(missing(error.clone())),
        }
    }
}
///Opaque actual queued work item. It cannot be cloned, filled with an
///arbitrary graph or constructed from a status number by a caller.
pub struct Job {
    task: Arc<Task>,
}
///Successful native worker result scoped to the SAME Session124. Only the
///concrete775DB0 callback bridge can consume its mission and finalization loan.
pub struct SuccessfulTask {
    mission: StartedMission,
    task: Arc<Task>,
}
pub(crate) struct CallbacksTicket {
    task: Arc<Task>,
}
impl SuccessfulTask {
    pub fn validate_owner(&self, owner: &LocalBootstrap) -> std::result::Result<(), String> {
        if !self.task.owner.ptr_eq(&Arc::downgrade(&owner.identity)) {
            return Err("finished task belongs to another local Session".into());
        }
        let leave_step::TaskSlot::Load(current) = &owner.session.task_124 else {
            return Err("finished Session124 task was removed".into());
        };
        if !Arc::ptr_eq(current, &self.task) {
            return Err("finished Session124 is another task allocation".into());
        }
        if !matches!(
            *self.task.state.lock().map_err(|_| "task lock poisoned")?,
            Run::InCallbacks
        ) {
            return Err("actual775DB0 callback interval is not live".into());
        }
        Ok(())
    }
    pub fn mission(&self) -> &StartedMission {
        &self.mission
    }
    pub(crate) fn into_parts(
        self,
        owner: &LocalBootstrap,
    ) -> std::result::Result<(StartedMission, CallbacksTicket), (Self, String)> {
        if let Err(error) = self.validate_owner(owner) {
            return Err((self, error));
        }
        Ok((self.mission, CallbacksTicket { task: self.task }))
    }
}
impl Job {
    ///The actual manager-assigned native+C; never a queue/ready substitute.
    pub fn sequence_c(&self) -> std::result::Result<u32, String> {
        self.task.sequence_c().map_err(|e| e.to_string())
    }
    pub fn inputs(&self) -> &Inputs {
        &self.task.inputs
    }
    pub fn on_created(&self, identity: &WorkerIdentity) -> std::result::Result<(), String> {
        let mut run = self
            .task
            .state
            .lock()
            .map_err(|_| "task state lock poisoned")?;
        if !matches!(*run, Run::Queued) {
            return Err("actual load job was already started".into());
        }
        *run = Run::Registered(identity.witness());
        Ok(())
    }
    ///5B1910 writes status1 at actual Run entry. OnCreated alone leaves the
    ///queued status0 intact until the child has been released and starts work.
    pub fn on_run_start(&self, identity: &WorkerIdentity) -> std::result::Result<(), String> {
        let mut run = self
            .task
            .state
            .lock()
            .map_err(|_| "task state lock poisoned")?;
        let Run::Registered(worker) = &*run else {
            return Err("actual load job has no registered worker".into());
        };
        if !worker.is_for(identity) {
            return Err("Run belongs to another actual worker job".into());
        }
        *run = Run::Running(worker.clone());
        Ok(())
    }
    pub fn accept_completed(
        self,
        receipt: CompletedWorker<StartedMission>,
    ) -> std::result::Result<(), String> {
        let mut run = self
            .task
            .state
            .lock()
            .map_err(|_| "task state lock poisoned")?;
        let Run::Running(worker) = &*run else {
            let error = "completed worker has no running actual task".to_owned();
            *run = Run::Rejected {
                receipt,
                error: error.clone(),
            };
            return Err(error);
        };
        if !worker.is_for(receipt.identity()) {
            let error = "completed worker belongs to another actual task job".to_owned();
            *run = Run::Rejected {
                receipt,
                error: error.clone(),
            };
            return Err(error);
        }
        if let Ok(mission) = receipt.outcome() {
            if let Err(error) = mission.validate_task_inputs(&self.task.inputs) {
                *run = Run::Rejected {
                    receipt,
                    error: error.clone(),
                };
                return Err(error);
            }
        }
        *run = Run::Completed(receipt);
        Ok(())
    }
}
pub trait Effects {
    fn checking_old_task(&mut self) -> Result<()>;
    fn launching_map_type(&mut self, mode: u32) -> Result<()>;
    fn creating_load_game_task(&mut self) -> Result<()>;
    fn begin_task_diagnostic(&mut self, vworld_required: bool) -> Result<()>;
}
impl LocalBootstrap {
    ///Actual775EE0 fresh ordinary path, including A/S suspension, real
    ///no-world EndSession notification, constructor publication and enqueue.
    ///The returned bool means native CreateLaunchTask produced a task only.
    pub fn create_host_load_task_775ee0(
        &mut self,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        bank: &aa_runtime::definitions::Bank,
        depot: &aa_runtime::resource_depot::Application,
        notifications: &Notifications,
        effects: &mut impl Effects,
    ) -> Result<bool> {
        if self.host_input_stage()? != launch::InputStage::Finalized
            || self.session.launch_mode != 2
            || self.session.launch_game_type != 2
        {
            return Err(missing(
                "775EE0 concrete path requires finalized ordinary selected-map Host",
            ));
        }
        effects.checking_old_task()?;
        self.check_finished_task_775db0(true)?;
        self.sync_a.get_mut().suspend = self.sync_a.get_mut().suspend.wrapping_add(1);
        self.sync_s.get_mut().suspend = self.sync_s.get_mut().suspend.wrapping_add(1);
        if self.film_1c.is_some() {
            return Err(Error::Unsupported(
                "775EE0 EndSession requires actual existing film close",
            ));
        }
        {
            let _guard = self
                .world_lock
                .lock()
                .map_err(|_| missing("actual World lock poisoned"))?;
            if !self.sim.world_is_null() {
                return Err(Error::Unsupported(
                    "775EE0 EndSession actual existing World destructor",
                ));
            }
        }
        notifications.push(Notification::end_session())?;
        self.sync_a.get_mut().suspend = self.sync_a.get_mut().suspend.wrapping_sub(1);
        self.sync_s.get_mut().suspend = self.sync_s.get_mut().suspend.wrapping_sub(1);
        //Native captures54 before this diagnostic. Our admitted ordinary2
        //path therefore selects kind0 without inventing editor/film choices.
        effects.launching_map_type(self.session.launch_mode)?;
        let creator = self.selected_load_creator()?;
        if creator.source_resource().is_none() {
            return Err(missing("775EE0 current CreatorLoad source+50 is absent"));
        }
        effects.creating_load_game_task()?;
        let inputs = Inputs::from_selected_creator(
            catalog,
            bank,
            creator,
            aa_runtime::map_resource_mode::LoadGameTaskKind::from_native_word(0),
        )
        .map_err(missing)?;
        let task = Arc::new(Task {
            owner: Arc::downgrade(&self.identity),
            inputs,
            state: Mutex::new(Run::Queued),
            sequence_c: Mutex::new(None),
        });
        self.session.task_124 = leave_step::TaskSlot::Load(Arc::clone(&task));
        {
            //Acquire the fresh field loan before advancing the real counter,
            //so no later fallible lock can discard its just-published value.
            let mut sequence = task
                .sequence_c
                .lock()
                .map_err(|_| missing("task sequence field lock poisoned"))?;
            let assigned = depot
                .publish_load_task_sequence(
                    catalog,
                    bank,
                    self.selected_load_creator()?,
                    &task.inputs,
                )
                .map_err(missing)?;
            *sequence = Some(assigned);
        }
        //Native5AED10 retains this SAME allocation in its queue before the
        //Node's BeginTask writes. VecDeque is growable; no queue cap is added.
        self.load_jobs.push_back(Job { task });
        self.begin_task_74daf0(true, effects)?;
        Ok(true)
    }
    fn begin_task_74daf0(&mut self, required: bool, effects: &mut impl Effects) -> Result<()> {
        let node = self
            .host_node
            .as_mut()
            .ok_or_else(|| missing("74DAF0 actual Node is absent"))?;
        node.vworld_init_required_8c = Some(u8::from(required));
        effects.begin_task_diagnostic(required)?;
        for peer in &self.peer_list {
            match peer {
                PeerLink::Node => node.peer_task_34 = 1,
                PeerLink::Null => return Err(missing("74DAF0 actual peer is NULL")),
                PeerLink::Conn(_) => {
                    return Err(Error::Unsupported("74DAF0 actual remote peer task34 owner"));
                }
            }
        }
        Ok(())
    }
    ///Drain actual retained work after77A950's state stores. This is a valid
    ///delayed native worker schedule; it never treats an empty queue as ready.
    pub fn take_queued_load_job(&mut self) -> Result<Job> {
        if self.host_input_stage()? != launch::InputStage::MainLaunchComplete {
            return Err(missing(
                "actual delayed worker must wait until77A950 main-thread state stores complete",
            ));
        }
        self.load_jobs
            .pop_front()
            .ok_or_else(|| missing("no actual queued load task"))
    }
    pub fn task_status_14(&self) -> Result<Option<u32>> {
        let leave_step::TaskSlot::Load(task) = &self.session.task_124 else {
            return Ok(None);
        };
        task.status_14().map(Some)
    }
    pub fn task_sequence_c(&self) -> Result<Option<u32>> {
        let leave_step::TaskSlot::Load(task) = &self.session.task_124 else {
            return Ok(None);
        };
        task.sequence_c().map(Some)
    }
    ///Native775DB0 keeps124 nonnull throughout7757F0/7738B0. A caller may
    ///move out the genuine mission to those callbacks, but cannot clear the
    ///task slot or mint an activation result by consuming this value.
    pub fn begin_finished_task_callbacks(&mut self) -> Result<SuccessfulTask> {
        let leave_step::TaskSlot::Load(task) = &self.session.task_124 else {
            return Err(missing("775DB0 has no actual task"));
        };
        let mut run = task
            .state
            .lock()
            .map_err(|_| missing("task state lock poisoned"))?;
        match &*run {
            Run::Completed(receipt) if receipt.outcome().is_ok() => {}
            Run::Completed(_) => {
                return Err(Error::Unsupported(
                    "775DB0 Rust worker refusal is not a proved native status3/76DF30 failure",
                ));
            }
            _ => return Err(missing("775DB0 actual worker has not completed")),
        }
        let Run::Completed(receipt) = std::mem::replace(&mut *run, Run::InCallbacks) else {
            unreachable!()
        };
        let mission = receipt.into_outcome().map_err(missing)?;
        Ok(SuccessfulTask {
            mission,
            task: Arc::clone(task),
        })
    }
    ///Only a concrete crate-owned7757F0/7738B0 bridge can possess this loan.
    ///No public setter changes124 or acknowledges arbitrary callback success.
    pub(crate) fn finish_finished_task_callbacks(
        &mut self,
        ticket: CallbacksTicket,
    ) -> std::result::Result<(), (CallbacksTicket, Error)> {
        let validate = (|| -> Result<()> {
            let leave_step::TaskSlot::Load(task) = &self.session.task_124 else {
                return Err(missing("775DB0 task disappeared during callbacks"));
            };
            if !Arc::ptr_eq(task, &ticket.task)
                || !task.owner.ptr_eq(&Arc::downgrade(&self.identity))
            {
                return Err(missing(
                    "775DB0 callback finalization belongs to another task/Session",
                ));
            }
            if !matches!(
                *task
                    .state
                    .lock()
                    .map_err(|_| missing("task lock poisoned"))?,
                Run::InCallbacks
            ) {
                return Err(missing("775DB0 callback interval is not current"));
            }
            Ok(())
        })();
        if let Err(error) = validate {
            return Err((ticket, error));
        }
        self.session.task_124 = leave_step::TaskSlot::NullFromNativeConstructor;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_constructor_publication_and_separate_status_leaf_evidence() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-local-bootstrap/load-task/native-task.json"
        ))
        .unwrap();
        assert_eq!(fixture["passed"], true);
        assert_eq!(fixture["cases"].as_array().unwrap().len(), 36);
        for row in fixture["cases"].as_array().unwrap() {
            assert_eq!(row["constructor_fields"][1], 2);
            assert_eq!(row["constructor_fields"][4], 0);
            assert_eq!(row["enqueue"]["task_is_session124"], true);
            assert_eq!(row["enqueue"]["references4"], 1);
            assert_eq!(
                row["after_begin_task"],
                serde_json::json!({"node8c":1,"peer34":1})
            );
            assert_eq!(row["separate_status_stores"], serde_json::json!([2, 3]));
        }
    }
    struct BeginEffects {
        fail: bool,
    }
    impl Effects for BeginEffects {
        fn checking_old_task(&mut self) -> Result<()> {
            Ok(())
        }
        fn launching_map_type(&mut self, _: u32) -> Result<()> {
            Ok(())
        }
        fn creating_load_game_task(&mut self) -> Result<()> {
            Ok(())
        }
        fn begin_task_diagnostic(&mut self, _: bool) -> Result<()> {
            if self.fail {
                Err(missing("reached native diagnostic service failure"))
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn begin_task_preserves_native_stores_around_required_diagnostic_and_peer_error() {
        let mut owner = LocalBootstrap::from_native_constructors();
        owner.host_node = Some(crate::local_host::Node::before_session_reset().unwrap());
        owner.peer_list = vec![PeerLink::Node, PeerLink::Null];
        assert!(owner
            .begin_task_74daf0(true, &mut BeginEffects { fail: true })
            .is_err());
        assert_eq!(
            owner.host_node.as_ref().unwrap().vworld_init_required_8c,
            Some(1)
        );
        assert_eq!(owner.host_node.as_ref().unwrap().peer_task_34, 0);
        assert!(owner
            .begin_task_74daf0(true, &mut BeginEffects { fail: false })
            .is_err());
        assert_eq!(owner.host_node.as_ref().unwrap().peer_task_34, 1);
        owner.peer_list.pop();
        owner
            .begin_task_74daf0(false, &mut BeginEffects { fail: false })
            .unwrap();
        assert_eq!(
            owner.host_node.as_ref().unwrap().vworld_init_required_8c,
            Some(0)
        );
        assert_eq!(owner.host_node.as_ref().unwrap().peer_task_34, 1);
    }
}
