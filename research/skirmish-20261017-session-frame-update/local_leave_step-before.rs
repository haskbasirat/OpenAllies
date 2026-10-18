//! Concrete fresh770500 and constructor-null775DB0 over the retained owners.
//! The terminal74DA10 clears the SAME Node's vote-to-kick list. It is not a
//!task/message queue, and positive world/film/roster unload remains required.
use super::*;

/// Native76F998/76FAB4 stores the sole Session124 pointer NULL. There is no
/// fabricated completed-task status; a later actual task producer must extend
///this owner with its concrete move-only LoadGameTask allocation.
pub(super) enum TaskSlot {
    NullFromNativeConstructor,
    Load(Arc<super::load_task::Task>),
}
impl LocalBootstrap {
    ///775DB0 checks124 first and returns immediately when NULL, without
    ///reading task status, block argument, synchronization or UI services.
    pub fn check_finished_task_775db0(&mut self, block: bool) -> Result<()> {
        match &self.session.task_124 {
            TaskSlot::NullFromNativeConstructor => Ok(()),
            TaskSlot::Load(_) => {
                let status=self.task_status_14()?.ok_or_else(||missing("775DB0 positive task disappeared"))?;
                if !matches!(status,2|3)&&!block{return Ok(());}
                Err(Error::Unsupported("775DB0 positive task requires actual worker blocking/completion and7757F0/7738B0 callbacks"))
            },
        }
    }
    pub fn task_is_null_124(&self) -> bool {
        matches!(self.session.task_124, TaskSlot::NullFromNativeConstructor)
    }
    ///770500 fresh/default-state path plus its reached notification branch.
    ///Other in-progress state bodies cannot be skipped through this method.
    pub fn launch_step_770500(&mut self, notifications: &Notifications) -> Result<()> {
        match self.session.state {
            1 => {
                return Err(Error::Unsupported(
                    "770500 state1 requires actual40B450 UI leave service",
                ));
            }
            2 | 5 => {
                return Err(Error::Unsupported(
                    "770500 state2/5 requires actual world/roster/film leave services",
                ));
            }
            4 => {
                return Err(Error::Unsupported(
                    "770500 state4 requires actual76CF30 countdown leave service",
                ));
            }
            _ => {}
        }
        //The original short-circuit reads58 only when54==1. Both fields are
        //the retained Session values, not copied Host arguments or a task result.
        if self.session.launch_mode == 1 && self.session.launch_game_type == 4 {
            notifications.push(Notification::native_kind(3))?;
        }
        //74DA10 always dereferences the actual global Node before its list.
        if self.host_node.is_none() {
            return Err(missing("770500 terminal74DA10 requires actual Node owner"));
        }
        //No callback mutates2AC during native operator-delete. Borrow its
        //retained order while mutating only the disjoint real vote list.
        for peer in &self.peer_list {
            match peer {
                PeerLink::Node => self.host_node.as_mut().unwrap().clear_votes_74da10(),
                PeerLink::Null => return Err(missing("74DA10 peer pointer is NULL")),
                PeerLink::Conn(_) => {
                    return Err(Error::Unsupported(
                        "74DA10 actual remote peer64/68 owner is not bound",
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "local_leave_step_tests.rs"]
mod tests;
