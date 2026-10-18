//!7738B0 over the SAME successful callback, Node/Sim/World and queue owners.
//!The authentic task type is LoadGameTask59E6F0, not the distinct ResourceManagerTaskSave
//!base checked by the native TypeInfo loop. No successful callback is inferred
//!from a worker status or a manager's ready byte.
use super::actual::Ready;
use super::*;
use aa_numeric::Precision;
use aa_runtime::{catalog::Catalog, world_frame_inputs::WorldFrameInputs};
use aa_simulation::FloatBits;

pub(crate) struct Complete {
    pub(super) frame: WorldFrameInputs,
}
#[derive(Debug, PartialEq, Eq)]
pub enum SuccessDiagnostic {
    OnWorldSuccess,
    SetWorldSuccess(bool),
}
trait SuccessWorld {
    fn diagnostic(&mut self) -> Result<()>;
    fn save_task(&mut self) -> Result<bool>;
    fn admin_time(&mut self) -> Result<FloatBits>;
    fn reset_admin(&mut self, time: FloatBits) -> Result<()>;
    fn clear_paused(&mut self) -> Result<()>;
    fn notify(&mut self, kind: u32, revision: bool) -> Result<()>;
    fn set_success(&mut self) -> Result<()>;
}
fn sequence<W: SuccessWorld>(world: &mut W) -> Result<()> {
    world.diagnostic()?;
    if world.save_task()? {
        world.notify(4, false)?;
    } else {
        let time = world.admin_time()?;
        world.reset_admin(time)?;
        world.clear_paused()?;
        world.notify(1, true)?;
    }
    world.set_success()
}
struct Current<'a, 'source, T> {
    owner: &'a mut LocalBootstrap,
    frame: &'a WorldFrameInputs,
    catalog: &'a Catalog<'source>,
    notifications: &'a Notifications,
    time: &'a mut crate::local_host::NodeTime<T>,
    precision: Precision,
    diagnostics: &'a mut Vec<SuccessDiagnostic>,
}
impl<T: crate::local_host::MillisecondTimer> SuccessWorld for Current<'_, '_, T> {
    fn diagnostic(&mut self) -> Result<()> {
        self.diagnostics.push(SuccessDiagnostic::OnWorldSuccess);
        Ok(())
    }
    fn save_task(&mut self) -> Result<bool> {
        //The private Ready came only from the native59E6F0 LoadGameTask
        //constructor. Its registered TypeInfo chain excludes B3DAC0.
        Ok(false)
    }
    fn admin_time(&mut self) -> Result<FloatBits> {
        let node = self
            .owner
            .host_node
            .as_ref()
            .ok_or_else(|| missing("7738B0 actual Node absent"))?;
        if !node.has_clock {
            return Err(missing("7738B0 actual AdminClock158 absent"));
        }
        node.clock
            .bounded_time(false, self.time, self.precision)
            .map_err(|e| missing(e.to_string()))
    }
    fn reset_admin(&mut self, time: FloatBits) -> Result<()> {
        self.owner
            .sim
            .reset_admin_time_76dce0(time, self.precision, || {
                Ok::<_, String>(self.frame.staged().clock.begin)
            })
            .map_err(missing)
    }
    fn clear_paused(&mut self) -> Result<()> {
        self.owner.sim.clear_paused_on_world_success_773936();
        Ok(())
    }
    fn notify(&mut self, kind: u32, revision: bool) -> Result<()> {
        let mut notification = Notification::native_kind(kind);
        notification.text = self
            .frame
            .success_warning_text(self.catalog)
            .map_err(missing)?;
        if revision {
            notification.word_38 = self.frame.revision().value();
        }
        self.notifications.push(notification)
    }
    fn set_success(&mut self) -> Result<()> {
        self.owner.set_world_success_74fec0(true, self.diagnostics)
    }
}

impl LocalBootstrap {
    pub(crate) fn finish_world_success_callback<T: crate::local_host::MillisecondTimer>(
        &mut self,
        ready: Ready,
        catalog: &Catalog<'_>,
        notifications: &Notifications,
        time: &mut crate::local_host::NodeTime<T>,
        precision: Precision,
        diagnostics: &mut Vec<SuccessDiagnostic>,
    ) -> std::result::Result<Complete, (Ready, Error)> {
        let result = (|| -> Result<()> {
            ready.frame.require_sai_complete(catalog).map_err(missing)?;
            self.sim
                .require_created_world(ready.frame.revision())
                .map_err(missing)?;
            sequence(&mut Current {
                owner: self,
                frame: &ready.frame,
                catalog,
                notifications,
                time,
                precision,
                diagnostics,
            })
        })();
        if let Err(error) = result {
            return Err((ready, error));
        }
        let Ready { frame, ticket } = ready;
        match self.finish_finished_task_callbacks(ticket) {
            Ok(()) => Ok(Complete { frame }),
            Err((ticket, error)) => Err((Ready { frame, ticket }, error)),
        }
    }
    ///74FEC0's progressive status store, then real74E970 request emission on
    ///the same Node/Admin streams. No detached heartbeat-node state exists.
    fn set_world_success_74fec0(
        &mut self,
        success: bool,
        diagnostics: &mut Vec<SuccessDiagnostic>,
    ) -> Result<()> {
        let node = self
            .host_node
            .as_mut()
            .ok_or_else(|| missing("74FEC0 actual Node absent"))?;
        let status = if !success {
            4
        } else if node
            .vworld_init_required_8c
            .ok_or_else(|| missing("74FEC0 current Node8C is unbound"))?
            == 0
        {
            3
        } else {
            2
        };
        node.task_status_90 = Some(status);
        diagnostics.push(SuccessDiagnostic::SetWorldSuccess(success));
        self.emit_task_status_74e970()
    }
    pub(crate) fn emit_task_status_74e970(&mut self) -> Result<()> {
        let _lock = self
            .world_lock
            .lock()
            .map_err(|_| missing("74E970 actual application World lock poisoned"))?;
        if !self.node_active {
            return Ok(());
        }
        //76AC60 reads actual Node8, not the current ordering-player slot.
        let player = self
            .local_player
            .ok_or_else(|| missing("74E970 running Node has NULL actual local player8"))?;
        let current = self
            .session
            .players
            .iter()
            .find(|p| p.core.id == player)
            .ok_or_else(|| missing("74E970 actual local player8 allocation is absent"))?;
        let node = self
            .host_node
            .as_mut()
            .ok_or_else(|| missing("74E970 actual Node absent"))?;
        let status = node
            .task_status_90
            .ok_or_else(|| missing("74E970 actual task status90 is unbound"))?;
        node.orders.emit_request(
            true,
            crate::heartbeat::TASK_STATUS_REQUEST,
            &[aa_orders::payload::Field::Bounded {
                n: 5,
                value: status & 7,
            }],
            current.core.id,
            &TaskSchema,
        )?;
        Ok(())
    }
}
pub(crate) struct TaskSchema;
impl aa_orders::payload::SchemaContext for TaskSchema {
    fn registry_count(&self, _: aa_orders::payload::Registry) -> aa_orders::Result<usize> {
        Err(aa_orders::Error::SchemaContext(
            "304 has no definition lookup",
        ))
    }
    fn team_command_flags(&self, _: u32) -> aa_orders::Result<u32> {
        Err(aa_orders::Error::SchemaContext(
            "304 has no TeamCommand lookup",
        ))
    }
    fn definition_kind(&self, _: aa_orders::payload::Registry, _: u32) -> aa_orders::Result<u32> {
        Err(aa_orders::Error::SchemaContext(
            "304 has no definition kind",
        ))
    }
    fn zero_width_list_budget(&self) -> usize {
        usize::MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    struct Replay {
        input: Value,
        precision: Precision,
        events: Vec<Value>,
        notifications: Vec<Value>,
        sim: aa_runtime::live_sim::Sim,
        paused: u8,
        status: Option<u32>,
    }
    impl Replay {
        fn event(&mut self, kind: &str, fields: Value) -> Result<()> {
            if self.input["stop"].as_str() == Some(kind) {
                return Err(missing(kind));
            }
            let mut value = json!({"kind":kind});
            for (key, field) in fields.as_object().unwrap() {
                value[key] = field.clone();
            }
            self.events.push(value);
            Ok(())
        }
    }
    impl SuccessWorld for Replay {
        fn diagnostic(&mut self) -> Result<()> {
            Ok(())
        }
        fn save_task(&mut self) -> Result<bool> {
            Ok(self.input["save"].as_bool().unwrap())
        }
        fn admin_time(&mut self) -> Result<FloatBits> {
            let value = self.input["admin"].as_u64().unwrap() as u32;
            self.event("clock", json!({"value":value}))?;
            Ok(FloatBits(value))
        }
        fn reset_admin(&mut self, time: FloatBits) -> Result<()> {
            self.event("reset", json!({"admin":time.0}))?;
            self.sim
                .reset_admin_time_76dce0(time, self.precision, || {
                    Ok::<_, String>(FloatBits(self.input["begin"].as_u64().unwrap() as u32))
                })
                .map_err(missing)
        }
        fn clear_paused(&mut self) -> Result<()> {
            self.paused = 0;
            Ok(())
        }
        fn notify(&mut self, kind: u32, revision: bool) -> Result<()> {
            self.event("warnings", json!({}))?;
            let mut notification = Notification::native_kind(kind);
            if revision {
                notification.word_38 = self.input["revision"].as_u64().unwrap() as u32;
            }
            self.event("notify", json!({}))?;
            //Native fixture records7851C0's actual argument BEFORE its
            //application-kind3 guard; it does not claim that queue accepted it.
            self.notifications.push(json!(notification));
            Ok(())
        }
        fn set_success(&mut self) -> Result<()> {
            self.event("task_success", json!({"value":1}))?;
            let status = if self.input["required"].as_u64() == Some(0) {
                3
            } else {
                2
            };
            self.status = Some(status);
            self.event("request", json!({"status":status,"player":0}))
        }
    }
    #[test]
    fn native_world_success_numeric_notification_and_required_prefixes() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../research/development10/world-ready-7757f0/native-world-success.json"
        ))
        .unwrap();
        assert_eq!(fixture["passed"], true);
        assert_eq!(fixture["cases"].as_array().unwrap().len(), 180);
        for row in fixture["cases"].as_array().unwrap() {
            let precision = match row["precision"].as_str().unwrap() {
                "Single" => Precision::Single24,
                "Double" => Precision::Double53,
                "Extended" => Precision::Extended64,
                _ => panic!("PC"),
            };
            let input = row["input"].clone();
            let mut replay = Replay {
                paused: input["paused"].as_u64().unwrap() as u8,
                input,
                precision,
                events: vec![],
                notifications: vec![],
                sim: aa_runtime::live_sim::Sim::from_native_constructor_fields(),
                status: None,
            };
            assert_eq!(
                sequence(&mut replay).is_err(),
                !row["stop"].is_null(),
                "{row}"
            );
            assert_eq!(json!(replay.events), row["events"], "{row}");
            assert_eq!(json!(replay.notifications), row["notifications"], "{row}");
            assert_eq!(
                replay.paused,
                row["paused"].as_u64().unwrap() as u8,
                "{row}"
            );
            if let Some(status) = replay.status {
                assert_eq!(status, row["status"].as_u64().unwrap() as u32);
            } else {
                assert_eq!(row["status"], 0xdeadbeefu32);
            }
            let times = replay.sim.time_words();
            for (index, word) in [
                times.begin_game,
                times.begin_admin,
                times.end_game,
                times.end_admin,
            ]
            .into_iter()
            .enumerate()
            {
                if let Some(word) = word {
                    assert_eq!(
                        word.0,
                        row["times"][index].as_u64().unwrap() as u32,
                        "{row}"
                    );
                } else {
                    assert_eq!(
                        row["times"][index], 0xa5a5a5a5u32,
                        "native unset heap input is NOT a Rust time value"
                    );
                }
            }
        }
    }
}
