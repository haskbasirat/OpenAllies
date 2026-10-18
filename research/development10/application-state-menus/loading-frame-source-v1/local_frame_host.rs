//! Native frame Host over direct sibling loans of the retained local Node.
//! No NetworkCore/Timeline/Driver is reconstructed. Positive transport, AVI,
//! UI smoothing and session-end leaves remain explicit required services.
use super::*;
use crate::{
    local_host::{FrameFields, MillisecondTimer, NodeTime},
    local_order_host,
};
use aa_numeric::Precision;
use aa_orders::{
    flow::Log,
    net::{
        admin::{AdminNode, OrderingHost},
        seams::GameData,
    },
    payload::Field,
};
use aa_runtime::{
    frame_variables,
    server_clock::ServerClock,
    simulation::{
        self,
        borrowed::{self, GateFlag, GateWord, NodeFlag, NodeWord},
    },
};
use aa_simulation::FloatBits;

///Read the SAME process field lazily at each native getter. The client window
///owner supplies its read-only capability; no initial focused value is assumed.
pub trait Focus {
    fn flags_b396c4(&self) -> std::result::Result<u32, String>;
}
impl Focus for aa_runtime::application_window_state::Reader {
    fn flags_b396c4(&self) -> std::result::Result<u32, String> {
        aa_runtime::application_window_state::Reader::flags_b396c4(self)
    }
}
pub struct NodeLoan<'owner, T> {
    pub(crate) fields: FrameFields<'owner>,
    pub(crate) clock: &'owner mut ServerClock,
    pub(crate) time: &'owner mut NodeTime<T>,
}
///Temporary cells contain only disjoint loans of ONE real RuntimeBorrow.
///No native state is initialized, copied or reset at this frame boundary.
pub struct Cells<'owner, T: MillisecondTimer> {
    admin: RefCell<&'owner mut AdminNode>,
    players: RefCell<PlayersBorrow<'owner>>,
    local: RefCell<LocalClientBorrow<'owner>>,
    node: RefCell<NodeLoan<'owner, T>>,
    gate: RefCell<order_gate::Borrow>,
    sync_a: &'owner RefCell<Synchronizer>,
    sync_s: RefCell<&'owner mut Synchronizer>,
    sim: RefCell<&'owner mut aa_runtime::live_sim::Sim>,
}
impl<'owner, T: MillisecondTimer> Cells<'owner, T> {
    pub fn from_runtime(
        runtime: RuntimeBorrow<'owner>,
        time: &'owner mut NodeTime<T>,
    ) -> Result<Self> {
        let RuntimeBorrow {
            owner,
            admin,
            players,
            sync_a,
            server_clock,
            local_client,
            sync_s,
            sim,
            frame,
            order_gate,
        } = runtime;
        if !Arc::ptr_eq(owner, players.identity) {
            return Err(missing("runtime frame loan belongs to another Session"));
        }
        order_gate.require_current(&players, &local_client)?;
        Ok(Self {
            admin: RefCell::new(admin),
            players: RefCell::new(players),
            local: RefCell::new(local_client),
            node: RefCell::new(NodeLoan {
                fields: frame,
                clock: server_clock,
                time,
            }),
            gate: RefCell::new(order_gate),
            sync_a,
            sync_s: RefCell::new(sync_s),
            sim: RefCell::new(sim),
        })
    }
    pub fn players(&self) -> &RefCell<PlayersBorrow<'owner>> {
        &self.players
    }
    pub fn same_sim(&self) -> &RefCell<&'owner mut aa_runtime::live_sim::Sim> {
        &self.sim
    }
    pub fn same_node(&self) -> &RefCell<NodeLoan<'owner, T>> {
        &self.node
    }
    pub fn same_a(&self) -> &'owner RefCell<Synchronizer> {
        self.sync_a
    }
    pub fn abort_writer(&self) -> Result<order_gate::AdminWriter> {
        Ok(self
            .gate
            .try_borrow()
            .map_err(|e| missing(e.to_string()))?
            .admin_writer())
    }
}
///Only unresolved native services live here; all methods are required. They
///cannot grant mission/SAI/view completion or manufacture an active Session.
pub trait Hooks: local_order_host::Effects {
    fn net_pump(
        &mut self,
        players: &mut PlayersBorrow<'_>,
        local: &mut LocalClientBorrow<'_>,
    ) -> simulation::Result<()>;
    fn process_commands(
        &mut self,
        admin: &mut AdminNode,
        players: &mut PlayersBorrow<'_>,
        local: &mut LocalClientBorrow<'_>,
        gate: &mut order_gate::Borrow,
    ) -> simulation::Result<()>;
    fn avi_pump(&mut self) -> simulation::Result<i32>;
    fn timing_smoothing(&mut self) -> simulation::Result<()>;
    fn end_condition_check(&mut self, players: &mut PlayersBorrow<'_>) -> simulation::Result<()>;
    fn mission_end_message(&mut self,_text:&[u16])->simulation::Result<()>{
        Err(simulation::Error::Unsupported("actual40BC50 kind15 UI dispatcher"))
    }
    fn mission_end_gamespy(&mut self,_player_kingdom_name:&[u16])->simulation::Result<()>{
        Err(simulation::Error::Unsupported("actual793B40 GameSpy message owner"))
    }
    fn heartbeat(
        &mut self,
        players: &mut PlayersBorrow<'_>,
        local: &mut LocalClientBorrow<'_>,
        admin: &mut AdminNode,
        sources:&dyn frame_heartbeat::Sources,
    ) -> simulation::Result<()>;
    fn backup_mode_entered(&mut self) -> simulation::Result<()>;
}
fn error(e: impl std::fmt::Display) -> simulation::Error {
    simulation::Error::Seam(e.to_string())
}
fn required<T>(s: &'static str) -> simulation::Result<T> {
    Err(simulation::Error::Unsupported(s))
}
struct HeartbeatSources<'a,T>{owner:&'a Arc<()>,server:&'a bool,time:&'a NodeTime<T>}
impl<T:MillisecondTimer> frame_heartbeat::Sources for HeartbeatSources<'_,T>{
    fn owner(&self)->&Arc<()>{self.owner}
    fn server_is_self(&self)->bool{*self.server}
    fn falling_behind(&self)->u8{self.time.falling_behind()}
}

pub struct Host<'a, 'owner, T: MillisecondTimer, D: GameData, H: Hooks, F: Focus + ?Sized> {
    admin: &'a RefCell<&'owner mut AdminNode>,
    players: &'a RefCell<PlayersBorrow<'owner>>,
    local: &'a RefCell<LocalClientBorrow<'owner>>,
    node: &'a RefCell<NodeLoan<'owner, T>>,
    gate: &'a RefCell<order_gate::Borrow>,
    sync_a: &'owner RefCell<Synchronizer>,
    sync_s: &'a RefCell<&'owner mut Synchronizer>,
    variables: &'a frame_variables::State,
    focus: &'a F,
    data: &'a D,
    hooks: &'a mut H,
    precision: Precision,
}
impl<'a, 'owner, T: MillisecondTimer, D: GameData, H: Hooks, F: Focus + ?Sized>
    Host<'a, 'owner, T, D, H, F>
{
    pub fn bind(
        cells: &'a Cells<'owner, T>,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        variables: &'a frame_variables::State,
        focus: &'a F,
        data: &'a D,
        hooks: &'a mut H,
        precision: Precision,
    ) -> Result<Self> {
        variables
            .validate(catalog)
            .map_err(|e| missing(e.to_string()))?;
        cells
            .gate
            .try_borrow()
            .map_err(|e| missing(e.to_string()))?
            .require_current(
                &*cells
                    .players
                    .try_borrow()
                    .map_err(|e| missing(e.to_string()))?,
                &*cells
                    .local
                    .try_borrow()
                    .map_err(|e| missing(e.to_string()))?,
            )?;
        Ok(Self {
            admin: &cells.admin,
            players: &cells.players,
            local: &cells.local,
            node: &cells.node,
            gate: &cells.gate,
            sync_a: cells.sync_a,
            sync_s: &cells.sync_s,
            variables,
            focus,
            data,
            hooks,
            precision,
        })
    }
}
impl<T: MillisecondTimer, D: GameData, H: Hooks, F: Focus + ?Sized> Host<'_, '_, T, D, H, F> {
    fn with_order<R>(
        &mut self,
        call: impl FnOnce(
            &mut AdminNode,
            &mut dyn OrderingHost,
            &mut order_gate::Borrow,
        ) -> aa_orders::Result<R>,
    ) -> simulation::Result<R> {
        let mut gate = self.gate.try_borrow_mut().map_err(error)?;
        {
            let players=self.players.try_borrow().map_err(error)?;
            let local=self.local.try_borrow().map_err(error)?;
            gate.require_current(&players,&local).map_err(error)?;
        }
        let mut admin = self.admin.try_borrow_mut().map_err(error)?;
        let mut world = local_order_host::CellWorld {
            players: self.players,
            local_client: self.local,
            sync_a: self.sync_a,
            sync_s: self.sync_s,
            effects: self.hooks,
        };
        call(&mut admin, &mut world, &mut gate).map_err(error)
    }
    fn emit_sync(&mut self, request: usize, fields: [Field; 2]) -> simulation::Result<()> {
        let active = *self.node.try_borrow().map_err(error)?.fields.running_2c0;
        let player = if active {
            let players = self.players.try_borrow().map_err(error)?;
            let id = players
                .local_player
                .ok_or_else(|| error("active emitter has no actual Node8"))?;
            players
                .session
                .players
                .iter()
                .find(|p| p.core.id == id)
                .ok_or_else(|| error("actual Node8 player allocation is absent"))?
                .core
                .id
        } else {
            0
        };
        self.admin
            .try_borrow_mut()
            .map_err(error)?
            .emit_request(active, request, &fields, player, self.data)
            .map_err(error)?;
        Ok(())
    }
}
impl<T: MillisecondTimer, D: GameData, H: Hooks, F: Focus + ?Sized> borrowed::Host
    for Host<'_, '_, T, D, H, F>
{
    fn begin_world_duration(&mut self,p:Precision)->simulation::Result<Option<u32>>{
        let ms=aa_runtime::server_clock::TimeSource::milliseconds(self.node.try_borrow_mut().map_err(error)?.time).map_err(error)?;
        super::frame_duration::begin(ms,p).map(Some).map_err(error)
    }
    fn finish_world_duration(&mut self,start:u32,p:Precision)->simulation::Result<()>{
        //Release the timer loan before borrowing raw history/Node60. No
        //World callback runs while either current Node field loan is held.
        let ms=aa_runtime::server_clock::TimeSource::milliseconds(self.node.try_borrow_mut().map_err(error)?.time).map_err(error)?;
        super::frame_duration::finish(&mut *self.local.try_borrow_mut().map_err(error)?,start,ms,p).map_err(error)
    }
    fn role(&self) -> simulation::Result<simulation::Role> {
        Ok(if self.local.try_borrow().map_err(error)?.is_host_self() {
            simulation::Role::Host
        } else {
            simulation::Role::Client
        })
    }
    fn flag(&self, f: NodeFlag) -> simulation::Result<bool> {
        let n = self.node.try_borrow().map_err(error)?;
        Ok(match f {
            NodeFlag::Running => *n.fields.running_2c0,
            NodeFlag::HasAdminClock => *n.fields.has_clock_158,
            NodeFlag::Disconnecting => *n.fields.disconnecting_2cd,
            NodeFlag::BackupMode => *n.fields.backup_mode_2d4,
            NodeFlag::ReplayRecorder => self.local.try_borrow().map_err(error)?.mirror_enabled(),
            NodeFlag::ReplaySource => n.fields.playback_26c.is_attached(),
            NodeFlag::IsServer => *n.fields.server_is_self_2c8,
        })
    }
    fn word(&self, f: NodeWord) -> simulation::Result<FloatBits> {
        let n = self.node.try_borrow().map_err(error)?;
        Ok(FloatBits(match f {
            NodeWord::AdminTime => *n.fields.admin_time_28c,
            NodeWord::DisconnectStarted => *n.fields.disconnect_started_2d0,
            NodeWord::BackupSince => *n.fields.backup_since_2d8,
            NodeWord::HeartbeatAt => *n.fields.heartbeat_2dc,
        }))
    }
    fn store_flag(&mut self, f: NodeFlag, v: bool) -> simulation::Result<()> {
        let mut n = self.node.try_borrow_mut().map_err(error)?;
        match f {
            NodeFlag::Running => *n.fields.running_2c0 = v,
            NodeFlag::Disconnecting => *n.fields.disconnecting_2cd = v,
            NodeFlag::BackupMode => *n.fields.backup_mode_2d4 = v,
            _ => return required("frame has no writer for this actual Node flag"),
        };
        Ok(())
    }
    fn store_word(&mut self, f: NodeWord, v: FloatBits) -> simulation::Result<()> {
        let mut n = self.node.try_borrow_mut().map_err(error)?;
        *match f {
            NodeWord::AdminTime => &mut *n.fields.admin_time_28c,
            NodeWord::DisconnectStarted => &mut *n.fields.disconnect_started_2d0,
            NodeWord::BackupSince => &mut *n.fields.backup_since_2d8,
            NodeWord::HeartbeatAt => &mut *n.fields.heartbeat_2dc,
        } = v.0;
        Ok(())
    }
    fn processed(&self) -> simulation::Result<i32> {
        Ok(self.admin.try_borrow().map_err(error)?.client.next_index)
    }
    fn received(&self) -> simulation::Result<i32> {
        Ok(self.admin.try_borrow().map_err(error)?.decrees.end())
    }
    fn gate_flag(&self, f: GateFlag) -> simulation::Result<bool> {
        Ok(match f {
            GateFlag::SessionF4 => {
                self.players
                    .try_borrow()
                    .map_err(error)?
                    .session
                    .multiplayer
            }
            GateFlag::SinglePlayerAutoPause => self.variables.auto_pause_byte() != 0,
            GateFlag::RenderDumpAvi => self.variables.render_dump_avi_byte() != 0,
        })
    }
    fn gate_word(&self, f: GateWord) -> simulation::Result<u32> {
        Ok(match f {
            GateWord::SessionMode => self.players.try_borrow().map_err(error)?.session.state,
            GateWord::FlagsB396C4 => self.focus.flags_b396c4().map_err(error)?,
            GateWord::AviFramesPerTick => self.variables.render_avi_frames_per_tick() as u32,
        })
    }
    fn async_ticks_per_second(&self) -> simulation::Result<FloatBits> {
        Ok(FloatBits(self.variables.async_ticks_per_second_bits()))
    }
    fn replay_client_tick(&mut self) -> simulation::Result<()> {
        let index = self.admin.try_borrow().map_err(error)?.client.next_index;
        let data = self.data;
        self.with_order(|admin, world, gate| {
            admin.client_execute_with_ordering(index, world, data, data, gate)
        })
    }
}
impl<T: MillisecondTimer, D: GameData, H: Hooks, F: Focus + ?Sized> simulation::NetworkSeams
    for Host<'_, '_, T, D, H, F>
{
    fn net_pump(&mut self) -> simulation::Result<()> {
        super::session_frame_update::run(
            &mut *self.players.try_borrow_mut().map_err(error)?,
            &mut *self.local.try_borrow_mut().map_err(error)?,
        )
        .map_err(error)
    }
    fn process_commands(&mut self) -> simulation::Result<()> {
        if self
            .local
            .try_borrow()
            .map_err(error)?
            .commands_118
            .constructor_empty_prefix()
            .map_err(error)?
        {
            return Ok(());
        }
        self.hooks.process_commands(
            &mut **self.admin.try_borrow_mut().map_err(error)?,
            &mut *self.players.try_borrow_mut().map_err(error)?,
            &mut *self.local.try_borrow_mut().map_err(error)?,
            &mut *self.gate.try_borrow_mut().map_err(error)?,
        )
    }
    fn real_admin_clock(&mut self) -> simulation::Result<FloatBits> {
        let mut n = self.node.try_borrow_mut().map_err(error)?;
        if !*n.fields.has_clock_158 || !*n.fields.server_is_self_2c8 {
            return required("actual nonlocal Admin158 clock owner");
        }
        let NodeLoan { clock, time, .. } = &mut *n;
        clock
            .bounded_time(true, *time, self.precision)
            .map_err(error)
    }
    fn avi_pump(&mut self) -> simulation::Result<i32> {
        self.hooks.avi_pump()
    }
    fn sync_log(&mut self, entry: simulation::SyncLog) -> simulation::Result<()> {
        let mut a;
        let mut s;
        let target = if matches!(
            entry,
            simulation::SyncLog::GameTimeBeginTick {
                second_log: true,
                ..
            }
        ) {
            s = self.sync_s.try_borrow_mut().map_err(error)?;
            &mut **s
        } else {
            a = self.sync_a.try_borrow_mut().map_err(error)?;
            &mut *a
        };
        if !target.enabled || target.suspend >= 1 {
            return Ok(());
        }
        let log = if matches!(
            entry,
            simulation::SyncLog::GameTimeBeginTick {
                second_log: true,
                ..
            }
        ) {
            Log::S
        } else {
            Log::A
        };
        let open = self.hooks.sync_log_gate(log).map_err(error)?;
        log_tick(target, open, entry);
        Ok(())
    }
    fn replay_record(&mut self) -> simulation::Result<()> {
        let bytes = self
            .admin
            .try_borrow_mut()
            .map_err(error)?
            .record_film()
            .map_err(error)?;
        self.local
            .try_borrow()
            .map_err(error)?
            .mirror_268
            .recorder()
            .ok_or_else(|| error("actual recorder268 is absent"))?
            .write(&bytes)
            .map_err(error)
    }
    fn replay_playback(&mut self) -> simulation::Result<()> {
        required("actual retained positive replay source26C")
    }
    fn timing_smoothing(&mut self) -> simulation::Result<()> {
        self.hooks.timing_smoothing()
    }
    fn end_condition_check(&mut self) -> simulation::Result<()> {
        self.hooks.end_condition_check(&mut *self.players.try_borrow_mut().map_err(error)?)
    }
    fn client_sync_flush(&mut self) -> simulation::Result<()> {
        let fields = {
            let s = self.sync_a.try_borrow().map_err(error)?;
            sync_fields(&s)
        };
        self.emit_sync(302, fields)
    }
    fn host_sync_flush(&mut self) -> simulation::Result<()> {
        let fields = {
            let s = self.sync_s.try_borrow().map_err(error)?;
            sync_fields(&s)
        };
        self.emit_sync(299, fields)
    }
    fn admin_history(&mut self) -> simulation::Result<()> {
        self.admin
            .try_borrow_mut()
            .map_err(error)?
            .seal_requests()
            .map(|_| ())
            .map_err(error)
    }
    fn host_rate(&mut self, seconds: FloatBits) -> simulation::Result<()> {
        let mut n = self.node.try_borrow_mut().map_err(error)?;
        let NodeLoan { clock, time, .. } = &mut *n;
        clock
            .host_rate(seconds.0, *time, self.precision)
            .map_err(error)
    }
    fn host_execute_requests(&mut self) -> simulation::Result<()> {
        let own = {
            let local = self.local.try_borrow().map_err(error)?;
            if local.peers_2ac != [crate::lobby::PeerLink::Node] {
                return required("actual ordered remote request-history owners");
            }
            *local.player_2c
        };
        let data = self.data;
        self.with_order(|admin, world, gate| {
            admin.host_execute_owned_requests(own, &mut [], world, data, data, gate)
        })
    }
    fn host_record_history(&mut self) -> simulation::Result<()> {
        self.with_order(|admin, world, _| admin.seal_decrees_with_ordering(world).map(|_| ()))
    }
    fn client_replay_history(&mut self, _: i32) -> simulation::Result<()> {
        required("prefix-only client replay cannot replace whole74FD10")
    }
    fn heartbeat(&mut self) -> simulation::Result<()> {
        let mut players=self.players.try_borrow_mut().map_err(error)?;
        let node=self.node.try_borrow().map_err(error)?;
        let sources=HeartbeatSources{owner:players.identity,server:node.fields.server_is_self_2c8,time:&*node.time};
        self.hooks.heartbeat(
            &mut *players,
            &mut *self.local.try_borrow_mut().map_err(error)?,
            &mut **self.admin.try_borrow_mut().map_err(error)?,
            &sources,
        )
    }
    fn backup_clients(&mut self) -> simulation::Result<Vec<simulation::BackupClient>> {
        if self
            .local
            .try_borrow()
            .map_err(error)?
            .peers_2ac
            .iter()
            .any(|p| *p != crate::lobby::PeerLink::Node)
        {
            return required("actual remote backup-client fields");
        }
        Ok(Vec::new())
    }
    fn backup_mode_entered(&mut self) -> simulation::Result<()> {
        self.hooks.backup_mode_entered()
    }
}
fn sync_fields(s: &Synchronizer) -> [Field; 2] {
    [
        Field::Uint {
            width: 32,
            value: s.end().wrapping_sub(1) as u32,
        },
        Field::Uint {
            width: 32,
            value: s.hash as u32,
        },
    ]
}
fn log_tick(target: &mut Synchronizer, open: bool, entry: simulation::SyncLog) {
    use simulation::SyncLog::*;
    match entry {
        NextTickEventAdminTime(v) => {
            target.log_float_bits(open, "GetNextTickEventAdminTime()", v.0)
        }
        GameTimeBeginTick { value, .. } => {
            target.log_float_bits(open, "g_gworld->GetGameTimeBeginTick()", value.0)
        }
        IsFrozen(v) => target.log_bool(open, "IsFrozen()", v),
        AdminTickEndGameTime(v) => target.log_float_bits(open, "m_admin_tick_end_game_time", v.0),
        AdminTickEndAdminTime(v) => target.log_float_bits(open, "m_admin_tick_end_admin_time", v.0),
        GameTimePerTick(v) => target.log_float_bits(open, "m_game_time_per_tick", v.0),
        Frozen(v) => target.log_bool(open, "_frozen", v),
    }
}

#[cfg(test)]
mod ownership_tests {
    use super::*;
    struct NoTimer;
    impl MillisecondTimer for NoTimer {
        fn milliseconds(&mut self) -> std::result::Result<u32, String> {
            panic!("a field loan must not sample the platform clock")
        }
    }
    fn owner() -> LocalBootstrap {
        let mut value = LocalBootstrap::from_native_constructors();
        value.host_node = Some(crate::local_host::Node::before_session_reset().unwrap());
        value
    }
    #[test]
    fn frame_cells_lend_the_same_node_admin_sim_and_logs_without_reset() {
        let mut value = owner();
        // Explicit ownership probes, not successful Host/mission construction.
        value.node_active = true;
        value.host_node.as_mut().unwrap().admin_time_28c = 0x3f400000;
        value.host_node.as_mut().unwrap().orders.client.next_index = 23;
        let sim_address = &value.sim as *const _;
        let admin_address = &value.host_node.as_ref().unwrap().orders as *const _;
        let a_address = &value.sync_a as *const _;
        let s_address = value.sync_s.get_mut() as *mut _;
        let mut time = NodeTime::from_native_constructor(NoTimer);
        let time_address = &time as *const _;
        {
            let cells = Cells::from_runtime(value.runtime_borrow().unwrap(), &mut time).unwrap();
            assert_eq!(&**cells.same_sim().borrow() as *const _, sim_address);
            assert_eq!(&**cells.admin.borrow() as *const _, admin_address);
            assert_eq!(cells.same_a() as *const _, a_address);
            assert_eq!(&mut **cells.sync_s.borrow_mut() as *mut _, s_address);
            assert_eq!(&*cells.same_node().borrow().time as *const _, time_address);
            assert_eq!(cells.admin.borrow().client.next_index, 23);
            assert_eq!(*cells.same_node().borrow().fields.admin_time_28c, 0x3f400000);
            *cells.same_node().borrow_mut().fields.admin_time_28c = 0x3fc00000;
            *cells.same_node().borrow_mut().fields.running_2c0 = false;
            cells.admin.borrow_mut().client.next_index = 24;
            cells.abort_writer().unwrap().store(128).unwrap();
            assert!(cells.local.try_borrow_mut().is_ok());
            // Sibling cells allow actual nested effects without a whole-Host
            // RefCell, while rejecting aliases of the same native allocation.
            let _admin = cells.admin.borrow_mut();
            assert!(cells.players.try_borrow_mut().is_ok());
            assert!(cells.admin.try_borrow_mut().is_err());
        }
        assert!(!value.node_active);
        assert_eq!(value.host_node().unwrap().admin_time_28c, 0x3fc00000);
        assert_eq!(value.order_node().unwrap().client.next_index, 24);
        assert_eq!(value.host_node().unwrap().order_abort_2e5.read(), 128);
        assert_eq!(time.cached_seconds_bits(), 0x5368d4a5);
    }
    #[test]
    fn a_foreign_session_gate_cannot_enter_the_sealed_frame_loan() {
        let mut first = owner();
        let mut other = owner();
        let foreign_gate = other.runtime_borrow().unwrap().order_gate;
        let mut runtime = first.runtime_borrow().unwrap();
        runtime.order_gate = foreign_gate;
        let mut time = NodeTime::from_native_constructor(NoTimer);
        let error = match Cells::from_runtime(runtime, &mut time) {
            Ok(_) => panic!("foreign Session gate was admitted"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("another current Host"));
        assert_eq!(first.order_node().unwrap().client.next_index, 0);
        assert_eq!(other.order_node().unwrap().client.next_index, 0);
    }
    #[test]
    fn native_order_callbacks_release_actual_players_for_reference_resolution(){
        use aa_orders::{payload::{self,Resolver,SchemaContext,Registry,ThingCheck,NameKind},net::admin::{OrderingHost,CurrentClient}};
        struct Required;
        impl local_order_host::Effects for Required{
            fn sync_log_gate(&mut self,_:aa_orders::flow::Log)->aa_orders::Result<bool>{unreachable!()}
            fn diagnostic(&mut self,_:local_order_host::Diagnostic)->aa_orders::Result<()>{unreachable!()}
            fn stop_film(&mut self,_:&mut PlayersBorrow<'_>)->aa_orders::Result<()>{unreachable!()}
            fn set_admin_abort(&mut self)->aa_orders::Result<()>{unreachable!()}
            fn valid(&mut self,_:&mut PlayersBorrow<'_>,_:&mut u32,_:&CurrentClient,_:aa_orders::flow::Validity,_:usize,_:&[Field])->aa_orders::Result<bool>{unreachable!()}
            fn process(&mut self,_:&mut PlayersBorrow<'_>,_:&mut LocalClientBorrow<'_>,_:&mut u32,_:&CurrentClient,_:usize,_:&mut Vec<Field>)->aa_orders::Result<()>{unreachable!()}
        }
        struct Data<'a,'owner>(&'a RefCell<PlayersBorrow<'owner>>);
        impl Resolver for Data<'_,'_>{
            fn thing(&self,_:ThingCheck,_:u16)->Option<u16>{unreachable!()}
            fn player(&self,_:u32)->bool{unreachable!()}
            fn definition(&self,_:Registry,_:u32)->bool{unreachable!()}
            fn named(&self,_:NameKind,_:&[u16])->Option<Vec<u16>>{unreachable!()}
            fn try_player(&self,id:u32)->aa_orders::Result<bool>{
                let players=self.0.try_borrow().map_err(|_|aa_orders::Error::SchemaContext("actual Session still held across callbacks"))?;
                Ok(players.find_order_player(id,false).is_some())
            }
        }
        impl SchemaContext for Data<'_,'_>{
            fn registry_count(&self,_:Registry)->aa_orders::Result<usize>{unreachable!()}
            fn team_command_flags(&self,_:u32)->aa_orders::Result<u32>{unreachable!()}
            fn definition_kind(&self,_:Registry,_:u32)->aa_orders::Result<u32>{unreachable!()}
            fn zero_width_list_budget(&self)->usize{0}
        }
        //Real constructor owners with explicitly installed player/284 fields;
        //this verifies callback lifetime/error prefix, not a completed Host.
        let mut value=owner();
        value.session.players.push(OwnedPlayer{allocation_lifetime:Arc::new(()),core:PlayerFields::from_constructor(7,crate::lobby::PeerLink::Node),vector_44:vec![],view:None});
        let parts=value.runtime_borrow().unwrap();
        let players=RefCell::new(parts.players);let local=RefCell::new(parts.local_client);let s=RefCell::new(parts.sync_s);
        let data=Data(&players);let mut required=Required;
        let mut world=local_order_host::CellWorld{players:&players,local_client:&local,sync_a:parts.sync_a,sync_s:&s,effects:&mut required};
        let mut slot=7;
        assert!(world.has_ordering_client(&mut slot,&CurrentClient::Null).unwrap());
        assert_eq!(payload::normalize_for_decree(&[Field::Player(7),Field::Player(99)],&data,&data).unwrap(),[Field::Player(7),Field::Player(0)]);
        assert!(world.acknowledge_order_version(&mut slot,&CurrentClient::Null).is_err());
        assert_eq!(local.borrow().order_version(),1);
        assert!(players.try_borrow_mut().is_ok());assert!(s.try_borrow_mut().is_ok());
        assert_eq!(payload::normalize_for_decree(&[Field::Player(7)],&data,&data).unwrap(),[Field::Player(7)]);
    }
}
