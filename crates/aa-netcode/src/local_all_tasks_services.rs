//! Concrete775BF0 children over the SAME reentrant OrderLoans and Node clock.
//! The public order dispatcher still requires real task/view completion; this
//! adapter never manufactures it or opens a second mutable World/frame loan.
use super::frame_host::{Focus, NodeLoan};
use super::*;
use crate::local_host::MillisecondTimer;
use aa_numeric::Precision;
use aa_runtime::{
    catalog::Catalog,
    frame_variables,
    live_clocked::SyncBinding,
    live_sim::Sim,
    live_world_frame::OrderLoans,
    simulation::{
        self,
        borrowed::{self, GateFlag, GateWord, NodeFlag, NodeWord},
    },
};
use aa_simulation::{FloatBits, NativeClock};

pub struct Current<'a, 'world, 'owner, 'source, T: MillisecondTimer, F: Focus + ?Sized> {
    pub orders: &'a mut OrderLoans<'world>,
    pub catalog: &'a Catalog<'source>,
    pub node: &'a RefCell<NodeLoan<'owner, T>>,
    pub variables: &'a frame_variables::State,
    pub focus: &'a F,
    pub sync_gate: &'a mut dyn FnMut() -> std::result::Result<bool, String>,
    pub precision: Precision,
}
impl<T: MillisecondTimer, F: Focus + ?Sized> all_tasks_complete::Services
    for Current<'_, '_, '_, '_, T, F>
{
    fn unfreeze_76dd90(
        &mut self,
        same_sim: &mut Sim,
        same_a: &RefCell<Synchronizer>,
        players: &PlayersBorrow<'_>,
    ) -> Result<()> {
        same_sim
            .require_created_world(self.orders.revision())
            .map_err(|e| missing(e.to_string()))?;
        let world = Clock(self.orders.world_clock());
        let mut host = ClockHost {
            node: self.node,
            players,
            variables: self.variables,
            focus: self.focus,
            sync_a: same_a,
            sync_gate: &mut *self.sync_gate,
            precision: self.precision,
        };
        borrowed::SimMath {
            sim: same_sim,
            precision: self.precision,
        }
        .set_frozen(false, &mut host, &world)
        .map_err(|e| missing(e.to_string()))
    }
    fn human_opposition(&mut self, players: &PlayersBorrow<'_>) -> Result<bool> {
        all_tasks_complete::human_opposition(players, self.orders.actors)
    }
    fn running_node_2c0(&mut self) -> Result<bool> {
        Ok(*self
            .node
            .try_borrow()
            .map_err(|e| missing(e.to_string()))?
            .fields
            .running_2c0)
    }
    fn initial_world_sync_4f8ae0(
        &mut self,
        label: &str,
        same_a: &RefCell<Synchronizer>,
        _same_s: &mut Synchronizer,
    ) -> Result<()> {
        self.orders
            .synchronize_initial(
                self.catalog,
                label,
                &mut SyncBinding::Shared {
                    owner: same_a,
                    gate: &mut *self.sync_gate,
                },
            )
            .map_err(|e| missing(e.to_string()))
    }
    fn current_svar_sync_76d820(&mut self,inventory:&aa_runtime::inactive_svar_dump::Inventory,same_a:&RefCell<Synchronizer>)->Result<()>{
        self.orders.synchronize_svars(self.catalog,inventory,&mut SyncBinding::Shared{owner:same_a,gate:&mut *self.sync_gate}).map_err(missing)
    }
}
struct Clock<'a>(&'a NativeClock);
impl simulation::WorldSeams for Clock<'_> {
    fn world_clock(&self) -> Option<NativeClock> {
        Some(*self.0)
    }
    fn world_update(&mut self) -> simulation::Result<NativeClock> {
        Err(simulation::Error::Unsupported(
            "76DD90 does not update World",
        ))
    }
}
///Only76DD90/76DC90's actually reached Host reads are supported. This is not
///a second driver; the native arithmetic is shared with borrowed::Kernel.
struct ClockHost<'a, 'owner, 'players, T: MillisecondTimer, F: Focus + ?Sized> {
    node: &'a RefCell<NodeLoan<'owner, T>>,
    players: &'a PlayersBorrow<'players>,
    variables: &'a frame_variables::State,
    focus: &'a F,
    sync_a: &'a RefCell<Synchronizer>,
    sync_gate: &'a mut dyn FnMut() -> std::result::Result<bool, String>,
    precision: Precision,
}
fn error(e: impl std::fmt::Display) -> simulation::Error {
    simulation::Error::Seam(e.to_string())
}
fn unexpected<T>() -> simulation::Result<T> {
    Err(simulation::Error::Unsupported(
        "unreached76DD90 Host service",
    ))
}
impl<T: MillisecondTimer, F: Focus + ?Sized> borrowed::Host for ClockHost<'_, '_, '_, T, F> {
    fn begin_world_duration(&mut self,_:Precision)->simulation::Result<Option<u32>>{unexpected()}
    fn finish_world_duration(&mut self,_:u32,_:Precision)->simulation::Result<()>{unexpected()}
    fn role(&self) -> simulation::Result<simulation::Role> {
        unexpected()
    }
    fn flag(&self, _: NodeFlag) -> simulation::Result<bool> {
        unexpected()
    }
    fn word(&self, field: NodeWord) -> simulation::Result<FloatBits> {
        if field != NodeWord::AdminTime {
            return unexpected();
        }
        Ok(FloatBits(
            *self.node.try_borrow().map_err(error)?.fields.admin_time_28c,
        ))
    }
    fn store_flag(&mut self, _: NodeFlag, _: bool) -> simulation::Result<()> {
        unexpected()
    }
    fn store_word(&mut self, _: NodeWord, _: FloatBits) -> simulation::Result<()> {
        unexpected()
    }
    fn processed(&self) -> simulation::Result<i32> {
        unexpected()
    }
    fn received(&self) -> simulation::Result<i32> {
        unexpected()
    }
    fn gate_flag(&self, field: GateFlag) -> simulation::Result<bool> {
        Ok(match field {
            GateFlag::SessionF4 => self.players.session.multiplayer,
            GateFlag::SinglePlayerAutoPause => self.variables.auto_pause_byte() != 0,
            GateFlag::RenderDumpAvi => self.variables.render_dump_avi_byte() != 0,
        })
    }
    fn gate_word(&self, field: GateWord) -> simulation::Result<u32> {
        Ok(match field {
            GateWord::FlagsB396C4 => self.focus.flags_b396c4().map_err(error)?,
            GateWord::SessionMode => self.players.session.state,
            GateWord::AviFramesPerTick => self.variables.render_avi_frames_per_tick() as u32,
        })
    }
    fn async_ticks_per_second(&self) -> simulation::Result<FloatBits> {
        unexpected()
    }
    fn replay_client_tick(&mut self) -> simulation::Result<()> {
        unexpected()
    }
}
macro_rules! unused {($($name:ident($($arg:ident:$type:ty),*)->$out:ty;)+)=>{$(fn $name(&mut self,$($arg:$type),*)->simulation::Result<$out>{$(let _=$arg;)*unexpected()})+}}
impl<T: MillisecondTimer, F: Focus + ?Sized> simulation::NetworkSeams
    for ClockHost<'_, '_, '_, T, F>
{
    fn real_admin_clock(&mut self) -> simulation::Result<FloatBits> {
        let mut n = self.node.try_borrow_mut().map_err(error)?;
        if !*n.fields.has_clock_158 || !*n.fields.server_is_self_2c8 {
            return Err(simulation::Error::Unsupported(
                "actual nonlocal76DD90 Admin158 clock owner",
            ));
        }
        let NodeLoan { clock, time, .. } = &mut *n;
        clock
            .bounded_time(true, *time, self.precision)
            .map_err(error)
    }
    fn sync_log(&mut self, entry: simulation::SyncLog) -> simulation::Result<()> {
        let simulation::SyncLog::Frozen(value) = entry else {
            return unexpected();
        };
        let mut a = self.sync_a.try_borrow_mut().map_err(error)?;
        if !a.enabled || a.suspend >= 1 {
            return Ok(());
        }
        let open = (self.sync_gate)().map_err(error)?;
        a.log_bool(open, "_frozen", value);
        Ok(())
    }
    unused! {net_pump()->();process_commands()->();avi_pump()->i32;replay_record()->();replay_playback()->();
    timing_smoothing()->();end_condition_check()->();client_sync_flush()->();host_sync_flush()->();admin_history()->();
    host_rate(seconds:FloatBits)->();host_execute_requests()->();host_record_history()->();client_replay_history(index:i32)->();
    heartbeat()->();backup_clients()->Vec<simulation::BackupClient>;backup_mode_entered()->();}
}
