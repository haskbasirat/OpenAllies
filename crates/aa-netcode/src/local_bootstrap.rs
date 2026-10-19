//! Owning local-startup prefix of the original Session/Node.
//!
//! This is deliberately not `Session::default()`: the wire projection's zeroes
//! are not native constructor values. Only reached, proved fields exist here.
//! In particular Session+E8 and Sim+10..1C are uninitialized by their native
//! constructors. Reset76EC70 supplies E8; no game-clock values are invented.
//! Native Host75AEE0 continues beyond this prefix into server/clock/settings
//! and launch services. This owner does not advertise a playable Host yet.
//!
//! Evidence: research/skirmish-20261016-local-bootstrap. Native allocation
//! failures and arbitrary callback reentry are outside the successful domain.
use crate::lobby::{GameOptions, LaunchState, NodeView, PeerLink, PropertyTable, SessionPlayer};
use crate::session::PlayerId;
use crate::{Error, Result};
use aa_orders::net::sync::{DEFAULT_MAX_ENTRIES, Synchronizer};
use aa_runtime::load_world_globals::SessionPhase;
use aa_runtime::localization::Localization;
use aa_simulation::NativeRng;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};

fn missing(message: impl Into<String>) -> Error {
    Error::Protocol(message.into())
}

///Lend the actual current TextManager only for the reached string operation.
///The guard must end before a later47EA40 can replace this same map table.
///Explicit text is retained solely for the existing diagnostic adapters.
pub(crate) fn with_launch_text<T>(
    catalog: &aa_runtime::catalog::Catalog<'_>,
    explicit: Option<&Localization>,
    read: impl FnOnce(&Localization) -> Result<T>,
) -> Result<T> {
    if let Some(text)=explicit { return read(text); }
    let current=catalog.localization.try_borrow()
        .map_err(|_|missing("actual current TextManager is already mutably borrowed"))?;
    let text=current.as_ref().ok_or_else(||missing("actual current TextManager is unbound"))?;
    read(text)
}

/// The exact GUID member widths passed to the eleven `%X` conversions in
/// 76ED71..76EDBE. CoCreateGuid is an OS input, not a simulation RNG draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}
impl Guid {
    pub fn native_text(self) -> Vec<u16> {
        let mut s = format!("{:X}{:X}{:X}", self.data1, self.data2, self.data3);
        for b in self.data4 {
            use std::fmt::Write;
            write!(&mut s, "{b:X}").unwrap();
        }
        s.encode_utf16().collect()
    }
}

/// Called at the native boundaries, rather than sampling both values before
/// EndSession. The platform adapter must call the real OS; fixtures explicitly
/// label their injected values. There is no entropy/timer fallback.
pub trait ResetOs {
    fn get_tick_count(&mut self) -> Result<u32>;
    fn co_create_guid(&mut self) -> Result<Guid>;
    /// Reached only if the actual synchronizer is enabled and unsuspended.
    /// This is the native simulation-thread/application-flag logger gate.
    fn sync_log_gate(&mut self) -> Result<bool>;
}

/// 40F7D0 -> 7841E0. The `Default` wire value has different +C/+10 words.
pub fn constructor_launch_state() -> LaunchState {
    LaunchState {
        kind: 0,
        flag_04: false,
        text: Vec::new(),
        field_0c: u32::MAX,
        field_10: 1,
        flag_14: false,
        flag_15: false,
        flag_16: false,
        field_18: 0,
        field_1c: 0,
        flag_20: false,
        field_24: 0,
    }
}

/// An actual queued 3C-byte UI notification value (7844F0/7519C0/784760).
/// EndSession overwrites the constructor's kind9 with kind2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Notification {
    pub kind: u32,
    pub text: Vec<u16>,
    pub state: u32,
    pub launch_state: LaunchState,
    pub flag_34: bool,
    pub word_38: u32,
}
impl Notification {
    fn end_session() -> Self {
        Self::native_kind(2)
    }
    fn native_kind(kind: u32) -> Self {
        Self {
            kind,
            text: Vec::new(),
            state: 0,
            launch_state: constructor_launch_state(),
            flag_34: false,
            word_38: 0,
        }
    }
}

struct QueueState {
    entries: VecDeque<Notification>,
}
/// 7850E0/784EF0 owns a critical section, signal and growable queue. Push7851C0
/// drops in application-kind3, otherwise locks, copies to the tail and signals.
/// The owner of B396C0 supplies and updates that application-kind input; it is
/// not Session+E4. No fixed native ring capacity is imposed.
pub struct Notifications {
    application_kind: AtomicU32,
    state: Mutex<QueueState>,
    changed: Condvar,
}
impl Notifications {
    ///Bounded normal interactive App+30 field projection:4196EC stores1,
    ///then41A4EA stores2 before41ABCF thread startup and local Host. This
    ///does not certify all intervening Application Init services completed.
    pub fn from_native_interactive_post_init() -> Self {Self::new(2)}
    pub fn new(application_kind: u32) -> Self {
        Self {
            application_kind: AtomicU32::new(application_kind),
            state: Mutex::new(QueueState {
                entries: VecDeque::new(),
            }),
            changed: Condvar::new(),
        }
    }
    pub fn set_application_kind(&self, value: u32) -> Result<()> {
        self.application_kind.store(value, Ordering::SeqCst);
        Ok(())
    }
    pub fn push(&self, value: Notification) -> Result<()> {
        // Native7851C0 checks B396C0 before acquiring the queue lock.
        if self.application_kind.load(Ordering::SeqCst) == 3 {
            return Ok(());
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| missing("notification critical section poisoned"))?;
        state
            .entries
            .try_reserve(1)
            .map_err(|_| missing("notification allocation failed"))?;
        state.entries.push_back(value);
        self.changed.notify_one();
        Ok(())
    }
    pub fn pop(&self) -> Result<Option<Notification>> {
        Ok(self
            .state
            .lock()
            .map_err(|_| missing("notification critical section poisoned"))?
            .entries
            .pop_front())
    }
    pub fn len(&self) -> Result<usize> {
        Ok(self
            .state
            .lock()
            .map_err(|_| missing("notification critical section poisoned"))?
            .entries
            .len())
    }
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }
    ///Read-only QA observation of the SAME queued values. It neither pops a
    ///notification nor supplies launch/ready facts to the application.
    pub fn diagnostic_snapshot(&self) -> Result<serde_json::Value> {
        let state=self.state.lock()
            .map_err(|_|missing("notification critical section poisoned"))?;
        serde_json::to_value(&state.entries).map_err(|error|missing(error.to_string()))
    }
}

/// Native 41AD7E..41AD9A creates the source B36568 table with79C6E0.
/// This owns the real initial source table, rather than pretending the source
/// pointer is null. Internet rated-map population is a separate required port.
pub struct InitialProperties {
    table: PropertyTable,
}
impl InitialProperties {
    pub fn from_native_constructor() -> Self {
        Self {
            table: PropertyTable {
                flag: false,
                count: 0,
                entries: Vec::new(),
            },
        }
    }
}

/// This is the sole Player+50 allocation, including its existing selection
/// List. The runtime owner preserves native constructor/destructor ordering.
type VWorldState = aa_runtime::player_view_state::State;
pub use aa_runtime::player_view_state::CurrentGroup;

/// Borrowed authoritative fields of the same retained VWorldState. Native
/// selection kernels can mutate them inline; no cloned actor selection or
/// group-presence boolean is supplied by the viewer.
pub struct SelectionBorrow<'a> {
    player_id: u32,
    pub list: &'a mut aa_runtime::player_selection::List,
    pub kingdom: &'a mut Option<aa_runtime::load_tail_world::KingdomId>,
    pub mask_words_10_1c: &'a mut [u32; 4],
    pub look_at_28_2c: &'a mut [u32; 2],
    pub group_20: &'a mut CurrentGroup,
}
impl SelectionBorrow<'_> {
    pub fn player_id(&self) -> u32 {
        self.player_id
    }
    pub fn group_is_null(&self) -> Result<bool> {
        Ok(self.group_20.is_null())
    }
    pub fn clear_group(&mut self) {
        self.group_20.clear();
    }
}
impl aa_runtime::player_selection::View for SelectionBorrow<'_> {
    fn player_id(&self) -> u32 {
        self.player_id
    }
    fn list(&self) -> &aa_runtime::player_selection::List {
        self.list
    }
    fn list_mut(&mut self) -> &mut aa_runtime::player_selection::List {
        self.list
    }
    fn kingdom(&self) -> Option<aa_runtime::load_tail_world::KingdomId> {
        *self.kingdom
    }
    fn mask_words_mut(&mut self) -> &mut [u32; 4] {
        self.mask_words_10_1c
    }
    fn look_at_mut(&mut self) -> &mut [u32; 2] {
        self.look_at_28_2c
    }
    fn group_is_null(&self) -> std::result::Result<bool, String> {
        Ok(self.group_20.is_null())
    }
    fn clear_group(&mut self) {
        self.group_20.clear();
    }
}
/// Read-only references to actual Session fields, split from its player's
/// mutable VWorldState. This is neither SessionFacts nor a cached phase. The
/// nickname is the resolved ordering player's actual+8; the DevMode UVar and
/// native application/thread gate still belong to their separate owners.
pub struct SelectionSessionBorrow<'a> {
    state_e4: &'a u32,
    phase_f8: &'a u32,
    multiplayer_f4: &'a bool,
    ordering_player_name_8: &'a [u16],
    local_player_name_8: std::result::Result<Option<&'a [u16]>, &'static str>,
}
/// Split siblings of the same retained Node/Session, for a real dispatcher
/// callback to resolve the current canonical Admin+284 at callback time.
pub struct RuntimeBorrow<'a> {
    pub(crate) owner:&'a Arc<()>,
    pub admin: &'a mut aa_orders::net::admin::AdminNode,
    pub players: PlayersBorrow<'a>,
    pub sync_a: &'a RefCell<Synchronizer>,
    pub server_clock: &'a mut aa_runtime::server_clock::ServerClock,
    pub local_client: LocalClientBorrow<'a>,
    pub sync_s: &'a mut Synchronizer,
    pub sim: &'a mut aa_runtime::live_sim::Sim,
    pub frame: crate::local_host::FrameFields<'a>,
    pub order_gate:order_gate::Borrow,
}
/// Actual local peer2C/6C, host2C4 and recorder268 siblings. Remote connection
/// owners are not synthesized from a peer-list ordinal or a player id.
pub struct LocalClientBorrow<'a> {
    pub(crate) owner_identity: &'a Arc<()>,
    pub(crate) player_2c: &'a mut Option<u32>,
    pub(crate) version_6c: &'a mut u8,
    pub(crate) host_2c4: &'a PeerLink,
    pub(crate) mirror_268: &'a crate::local_host::MirrorAttachment,
    pub(crate) peer_task_34: &'a mut u32,
    pub(crate) vworld_init_required_8c: &'a mut Option<u8>,
    pub(crate) task_status_90: &'a mut Option<u32>,
    pub(crate) peers_2ac: &'a [PeerLink],
    pub(crate) task_history_238: &'a mut world_ready::tasks::TaskHistory,
    pub(crate) takeover_2e0: &'a u32,
    pub(crate) commands_118: &'a mut node_commands::State,
    pub(crate) heartbeat: frame_heartbeat::Fields<'a>,
    pub(crate) version_ack_report: &'a mut version_ack::Report,
}
impl LocalClientBorrow<'_> {
    pub fn is_host_self(&self) -> bool {
        *self.host_2c4 == PeerLink::Node
    }
    pub fn order_version(&self) -> u8 {
        *self.version_6c
    }
    pub(crate) fn acknowledge_version_751fc0(&mut self) { *self.version_6c=1; }
    pub fn mirror_enabled(&self) -> bool {
        self.mirror_268.is_attached()
    }
    pub fn player_2c(&self) -> Option<u32> {
        *self.player_2c
    }
    pub(crate) fn bind_player_74b350(&mut self, id: u32) {
        *self.player_2c = Some(id);
    }
}
pub struct PlayersBorrow<'a> {
    session: &'a mut SessionFields,
    local_player: &'a Option<u32>,
    identity: &'a Arc<()>,
}
impl<'owner> PlayersBorrow<'owner> {
    pub(crate) fn require_same_client_owner(&self,client:&LocalClientBorrow<'_>)->Result<()> {
        if !Arc::ptr_eq(self.identity,client.owner_identity) {return Err(missing("actual Session/client owners differ"));}
        Ok(())
    }
    pub fn film_roster<'a>(
        &'a mut self,
        sim: &'a aa_runtime::live_sim::Sim,
    ) -> FilmRoster<'a, 'owner> {
        FilmRoster { players: self, sim, world: None }
    }
    ///Positive-globalWorld765E00 branch from the SAME current managed frame.
    ///The weak witness is identity metadata, not a cloned World or ready bit.
    ///Validation stays lazy until the native Kingdom getter is reached.
    pub fn film_roster_in_created_world<'a>(
        &'a mut self,
        sim: &'a aa_runtime::live_sim::Sim,
        world: aa_runtime::world_revision::Witness,
    ) -> FilmRoster<'a, 'owner> {
        FilmRoster { players: self, sim, world: Some(world) }
    }
    /// Native Session+BC traversal order, backed by the retained non-removing
    /// fresh roster. Handles identify allocations, never a guessed GTID.
    pub fn player_count(&self) -> usize {
        self.session.players.len()
    }
    pub fn player_at(&self, index: usize) -> Result<PlayerHandle> {
        let player = self
            .session
            .players
            .get(index)
            .ok_or_else(|| missing("SessionPlayer traversal index is not owned"))?;
        Ok(PlayerHandle {
            owner: Arc::downgrade(self.identity),
            id: player.core.id,
            slot: index,
            allocation: Arc::downgrade(&player.allocation_lifetime),
        })
    }
    pub fn player_id(&self, player: &PlayerHandle) -> Result<u32> {
        Ok(self.player(player)?.core.id)
    }
    pub fn player_ai(&self, player: &PlayerHandle) -> Result<bool> {
        Ok(self.player(player)?.core.ai)
    }
    pub fn player_implicit(&self, player: &PlayerHandle) -> Result<bool> {
        Ok(self.player(player)?.core.implicit)
    }
    pub fn player_display_name_8(&self, player: &PlayerHandle) -> Result<&[u16]> {
        Ok(&self.player(player)?.core.name)
    }
    pub fn is_local_player(&self, player: &PlayerHandle) -> Result<bool> {
        self.player(player)?;
        let Some(id) = *self.local_player else {
            return Ok(false);
        };
        let mut matches = self
            .session
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.core.id == id);
        let (slot, allocation) = matches
            .next()
            .ok_or_else(|| missing("actual Node8 is not in the retained Session roster"))?;
        if matches.next().is_some() {
            return Err(missing(
                "Node8 pointer disambiguation requires actual retained player allocation",
            ));
        }
        Ok(slot == player.slot
            && player
                .allocation
                .ptr_eq(&Arc::downgrade(&allocation.allocation_lifetime)))
    }
    pub fn set_seed_7710e0(&mut self, seed: u32, effects: &mut impl launch::Effects) -> Result<()> {
        launch::set_seed(self.session, seed, effects)
    }
    pub fn seed_override_100(&self) -> Result<u32> {
        self.session
            .seed_override_100
            .ok_or_else(|| missing("actual Session100 seed writer has not run"))
    }
    pub(crate) fn film_mode_24(&self) -> bool {
        self.session.film_24.read()!=0
    }
    pub(crate) fn find_order_player(&self, id: u32, film: bool) -> Option<PlayerHandle> {
        self.session
            .players
            .iter()
            .enumerate()
            .find(|(_, p)| p.core.id == id && p.core.film == film)
            .map(|(slot, p)| PlayerHandle {
                owner: Arc::downgrade(self.identity),
                id,
                slot,
                allocation: Arc::downgrade(&p.allocation_lifetime),
            })
    }
    fn player(&self, player: &PlayerHandle) -> Result<&OwnedPlayer> {
        if !Weak::ptr_eq(&player.owner, &Arc::downgrade(self.identity)) {
            return Err(missing(
                "SessionPlayer handle belongs to another native owner",
            ));
        }
        self.session
            .players
            .get(player.slot)
            .filter(|p| {
                p.core.id == player.id
                    && Weak::ptr_eq(&player.allocation, &Arc::downgrade(&p.allocation_lifetime))
            })
            .ok_or_else(|| missing("SessionPlayer allocation is no longer owned"))
    }
    pub(crate) fn player_film(&self, player: &PlayerHandle) -> Result<bool> {
        Ok(self.player(player)?.core.film)
    }
    pub(crate) fn player_link(&self, player: &PlayerHandle) -> Result<PeerLink> {
        Ok(self.player(player)?.core.link)
    }
    pub fn ordering_selection_and_session_for_slot(
        &mut self,
        slot: &u32,
    ) -> Result<(SelectionBorrow<'_>, SelectionSessionBorrow<'_>)> {
        LocalBootstrap::selection_session_parts(self.session, *self.local_player, slot)
    }
}
impl aa_runtime::load_tail_world::SessionRead for PlayersBorrow<'_> {
    type Player = PlayerHandle;
    fn local_player(&self) -> std::result::Result<Option<PlayerHandle>, String> {
        self.local_player
            .map(|id| {
                self.session
                    .players
                    .iter()
                    .position(|p| p.core.id == id)
                    .map(|slot| PlayerHandle {
                        owner: Arc::downgrade(self.identity),
                        id,
                        slot,
                        allocation: Arc::downgrade(&self.session.players[slot].allocation_lifetime),
                    })
                    .ok_or_else(|| {
                        "actual Node8 local player is not owned by this Session".to_owned()
                    })
            })
            .transpose()
    }
    fn player_name_24(&self, player: &PlayerHandle) -> std::result::Result<&[u16], String> {
        self.player(player)
            .map(|p| p.core.kingdom.as_slice())
            .map_err(|e| e.to_string())
    }
    fn player_kingdom_28(
        &self,
        player: &PlayerHandle,
    ) -> std::result::Result<Option<aa_runtime::load_tail_world::KingdomId>, String> {
        let p = self.player(player).map_err(|e| e.to_string())?;
        if !p.core.binding_complete || p.view.is_none() {
            return Err(
                "required765E10 actual player28/VWorld replacement has not completed".into(),
            );
        }
        Ok(p.core.field_28)
    }
}
impl SessionPhase for PlayersBorrow<'_> {
    fn current_phase(&self) -> std::result::Result<u32, String> {
        Ok(self.session.phase)
    }
    fn application_state(&self) -> std::result::Result<u32, String> {
        Ok(self.session.state)
    }
}
impl SelectionSessionBorrow<'_> {
    pub fn multiplayer(&self) -> bool {
        *self.multiplayer_f4
    }
    pub fn ordering_player_name_8(&self) -> &[u16] {
        self.ordering_player_name_8
    }
    ///40EAA0 uses Node+8, not Admin+284. Null local-player and an unresolved
    /// nonnull id remain distinct, and an unresolved id fails only on this read.
    pub fn local_player_name_8(&self) -> std::result::Result<Option<&[u16]>, String> {
        self.local_player_name_8.map_err(str::to_owned)
    }
}
impl SessionPhase for SelectionSessionBorrow<'_> {
    fn current_phase(&self) -> std::result::Result<u32, String> {
        Ok(*self.phase_f8)
    }
    fn application_state(&self) -> std::result::Result<u32, String> {
        Ok(*self.state_e4)
    }
}
fn construct_view(
    kingdom: Option<aa_runtime::load_tail_world::KingdomId>,
    localization: &Localization,
) -> Result<VWorldState> {
    VWorldState::construct(kingdom, |key| localization.display(key).map_err(|e| e.to_string()))
        .map_err(missing)
}

/// The live player fields, with native +28 represented by a typed actual
/// Kingdom identity instead of the wire projection's raw u32 pointer word.
#[derive(Serialize)]
struct PlayerFields {
    link: PeerLink,
    name: Vec<u16>,
    ai: bool,
    implicit: bool,
    film: bool,
    field_10: u32,
    text_14: Vec<u16>,
    status: u32,
    flag_1c: bool,
    id: u32,
    kingdom: Vec<u16>,
    field_28: Option<aa_runtime::load_tail_world::KingdomId>,
    flag_2c: bool,
    flag_2d: bool,
    text_30: Vec<u16>,
    text_34: Vec<u16>,
    flag_38: bool,
    field_3c: u32,
    field_40: u32,
    /// Constructor-null does not certify the later BeginPlaying binding.
    #[serde(skip)]
    binding_complete: bool,
}
impl PlayerFields {
    fn from_constructor(id: u32, host: PeerLink) -> Self {
        Self::from_native_fields(id, PeerLink::Node, false, false, host)
    }
    fn from_native_fields(
        id: u32,
        link: PeerLink,
        ai: bool,
        implicit: bool,
        host: PeerLink,
    ) -> Self {
        let SessionPlayer {
            link,
            name,
            ai,
            implicit,
            film,
            field_10,
            text_14,
            status,
            flag_1c,
            id,
            kingdom,
            field_28,
            flag_2c,
            flag_2d,
            text_30,
            text_34,
            flag_38,
            field_3c,
            field_40,
        } = SessionPlayer::new(link, id, ai, implicit, false, &NodeView { host });
        debug_assert_eq!(field_28, 0);
        Self {
            link,
            name,
            ai,
            implicit,
            film,
            field_10,
            text_14,
            status,
            flag_1c,
            id,
            kingdom,
            field_28: None,
            flag_2c,
            flag_2d,
            text_30,
            text_34,
            flag_38,
            field_3c,
            field_40,
            binding_complete: false,
        }
    }
}
struct OwnedPlayer {
    allocation_lifetime: Arc<()>,
    core: PlayerFields,
    vector_44: Vec<u32>,
    view: Option<VWorldState>,
}
/// Player identity is native ++Session+E8, scoped to its actual owner. It is
/// never an ActorId/GTID and cannot be transferred to another bootstrap owner.
#[derive(Clone, Debug)]
pub struct PlayerHandle {
    owner: Weak<()>,
    id: u32,
    /// Stable portable allocation slot for this non-removing fresh roster;
    /// native ids can repeat for different film flags. Not a native pointer.
    slot: usize,
    allocation: Weak<()>,
}
impl PlayerHandle {
    /// Proves the exact native-owner and player allocation still live. The
    /// private weak allocation token survives Vec movement but not deletion.
    pub fn validate_lifetime(&self) -> std::result::Result<(), String> {
        if self.owner.upgrade().is_none() || self.allocation.upgrade().is_none() {
            return Err("SessionPlayer native owner/allocation is no longer live".into());
        }
        Ok(())
    }
    pub fn id(&self) -> u32 {
        self.id
    }
}
impl aa_runtime::load_sai_players::SessionPlayer for PlayerHandle {
    fn validate(&self) -> std::result::Result<(), String> {
        self.validate_lifetime()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Progress {
    Constructed,
    Resetting,
    Reset,
    CreatingPlayer,
    LocalPlayerCreated,
    CreatingLocalHost,
    LocalHostCreated,
}

/// Only fields written by76F890 or the actual Reset prefix are represented.
/// There is no complete LaunchSettings/Session conversion that fills omitted
/// fields with Default. Subsequent services must bind their real producers.
struct SessionFields {
    status:session_status::State,
    launch_mode: u32,
    launch_game_type: u32,
    launch_file_5c: Vec<u16>,
    launch_kind_60: u32,
    launch_text_84:Vec<u16>,
    launch_state: LaunchState,
    options: GameOptions,
    properties: PropertyTable,
    players: Vec<OwnedPlayer>,
    created: Option<u32>,
    changed: [bool; 3],
    counter_e0: u32,
    state: u32,
    phase: u32,
    film_24: order_gate::Byte,
    last_player_id: Option<u32>,
    rng: NativeRng,
    multiplayer: bool,
    launch_countdown: i32,
    /// No native ctor/Reset store proved.7710E0 or77A750 must write it first.
    seed_override_100: Option<u32>,
    /// The sole actual Session124 task pointer, constructor76F998 NULL.
    /// Positive task creation/status has a separate native producer.
    task_124: leave_step::TaskSlot,
    host_countdown: i32,
    setup_ai: sai_setup::Queue,
    field_18: u32,
    ///76F890 constructs the distinct Session150 CString;7757F0 resets it
    ///from actual application CRTB43B40 before reading the current state.
    film_name_150: Vec<u16>,
    field_154: Option<u32>,
    field_158: u32,
    field_15c: u32,
    text_160: Vec<u16>,
    field_164: u32,
    flags_168_16e: [Option<bool>; 7],
}

/// The real local Node/Session prefix owner. Host remains null while the first
/// player is constructed; the resulting human status is0, not2.
pub struct LocalBootstrap {
    identity: Arc<()>,
    session: SessionFields,
    /// Actual Session+1C, physically split only to loan its sibling roster.
    /// This remains the sole move-only film allocation; ctor76FAD1 is null.
    film_1c: Option<aa_runtime::film_resource::Resource>,
    /// Actual Node+8/+2C, not a detached SessionFacts snapshot.
    local_player: Option<u32>,
    ordering_player: Option<u32>,
    node_host: PeerLink,
    node_active: bool,
    peer_list: Vec<PeerLink>,
    peers_changed: bool,
    local_identity: PlayerId,
    /// Actual Sim+29 and world+3C projection. Uninitialized timeline words
    /// deliberately have no storage until their actual producer is bound.
    sim: aa_runtime::live_sim::Sim,
    world_lock: Mutex<()>,
    sync_a: RefCell<Synchronizer>,
    sync_s: RefCell<Synchronizer>,
    host_node: Option<crate::local_host::Node>,
    host_input: Option<launch::Retained>,
    /// Partially bound native LaunchState constructor owners. None means the
    /// source-backed Creator constructor has not run here, never native NULL.
    load_selection: Option<launch::LoadSelection>,
    launch_other_creators:Option<aa_runtime::resource_creator::launch_creators::Others>,
    assignment_data_14: assignment::StagingData,
    progress: Progress,
    load_jobs: VecDeque<load_task::Job>,
    ///Modern retained debug transport; not a second native Session field.
    version_ack_diagnostics: version_ack::Report,
}

pub struct SessionLoadBorrow<'a> {
    state_e4: &'a u32,
    phase_f8: &'a u32,
    players: &'a [OwnedPlayer],
    local_player: &'a Option<u32>,
    identity: &'a Arc<()>,
}
impl SessionPhase for SessionLoadBorrow<'_> {
    fn current_phase(&self) -> std::result::Result<u32, String> {
        Ok(*self.phase_f8)
    }
    fn application_state(&self) -> std::result::Result<u32, String> {
        Ok(*self.state_e4)
    }
    fn clear_kingdom_references_in_created_world(
        &self,
        removed: aa_runtime::load_tail_world::KingdomId,
        world: &aa_runtime::world_revision::ConstructedWorldRevision,
    ) -> std::result::Result<(), String> {
        kingdom_cleanup::clear_in_created_world(self.players, removed, world)
    }
}
impl SessionLoadBorrow<'_> {
    fn player(&self, handle:&PlayerHandle)->std::result::Result<&OwnedPlayer,String> {
        if !handle.owner.ptr_eq(&Arc::downgrade(self.identity)) {
            return Err("resource SessionRead player belongs to another owner".into());
        }
        self.players.get(handle.slot).filter(|p|p.core.id==handle.id && handle.allocation.ptr_eq(&Arc::downgrade(&p.allocation_lifetime)))
            .ok_or_else(||"resource SessionRead player allocation no longer owned".into())
    }
}
impl aa_runtime::load_tail_world::SessionRead for SessionLoadBorrow<'_> {
    type Player=PlayerHandle;
    fn local_player(&self)->std::result::Result<Option<PlayerHandle>,String> {
        self.local_player.map(|id| {
            self.players.iter().position(|p|p.core.id==id).map(|slot|PlayerHandle{
                owner:Arc::downgrade(self.identity),id,slot,allocation:Arc::downgrade(&self.players[slot].allocation_lifetime)
            }).ok_or_else(||"actual Node8 player is absent from resource SessionRead".into())
        }).transpose()
    }
    fn player_name_24(&self,p:&PlayerHandle)->std::result::Result<&[u16],String>{Ok(&self.player(p)?.core.kingdom)}
    fn player_kingdom_28(&self,p:&PlayerHandle)->std::result::Result<Option<aa_runtime::load_tail_world::KingdomId>,String>{
        let p=self.player(p)?;
        if !p.core.binding_complete || p.view.is_none(){return Err("required765E10 actual player28/VWorld replacement has not completed".into());}
        Ok(p.core.field_28)
    }
}
pub struct ResourceLoadParts<'a> {
    pub session: SessionLoadBorrow<'a>,
    pub sim: &'a mut aa_runtime::live_sim::Sim,
    pub sync_a: &'a RefCell<Synchronizer>,
    pub sync_s: &'a RefCell<Synchronizer>,
    pub selected_creator: Option<&'a aa_runtime::resource_creator::Creator>,
}
pub struct ResourceWorkerParts<'a> {
    pub session: SessionLoadBorrow<'a>,
    pub sim: &'a mut aa_runtime::live_sim::Sim,
    pub sync_a: &'a mut Synchronizer,
    pub sync_s: &'a mut Synchronizer,
    pub selected_creator: Option<&'a aa_runtime::resource_creator::Creator>,
}

impl LocalBootstrap {
    ///5C3077 locks the SAME retained NetNode+8 critical section already used
    ///by the load/success path. This scoped loan creates no replacement lock.
    pub fn with_world_lock_5c3000<R>(&self,call:impl FnOnce()->std::result::Result<R,String>)->std::result::Result<R,String>{
        let _guard=self.world_lock.lock().map_err(|_|"actual NetNode+8 world critical section poisoned")?;
        call()
    }
    ///The immutable Host loan and SAME NetNode mutex span the complete UI
    ///operation, including synchronous backend RPC. The pure76AC90 getter
    ///reads F8; its expiring transport cannot outlive this locked scope.
    pub fn with_view_phase_lease_5c3000<R>(&self,call:impl FnOnce(view_startup::SessionPhaseLease)->std::result::Result<R,String>)->std::result::Result<R,String>{
        self.with_world_lock_5c3000(||view_startup::with_locked_phase(self.session.phase,call))
    }
    ///The same concrete40EAA0 adapter can be used by Audio, source timing
    ///and orders. DevMode belongs to the caller's actual Ui.uvars allocation;
    ///it is queried lazily after the native Session/Node name branch.
    pub fn can_process_40eaa0(&self,read_dev_mode:impl FnMut()->std::result::Result<u8,String>)->std::result::Result<bool,String>{
        struct Current<'a,F>{host:&'a LocalBootstrap,dev:F}
        impl<F:FnMut()->std::result::Result<u8,String>> application_gate::Sources for Current<'_,F>{
            fn session_state_e4(&mut self)->std::result::Result<Option<u32>,String>{Ok(Some(self.host.session.state))}
            fn multiplayer_f4(&mut self)->std::result::Result<bool,String>{Ok(self.host.session.multiplayer)}
            fn local_player_first_name_unit(&mut self)->std::result::Result<Option<u16>,String>{
                let Some(player)=self.host.local_player().map_err(|e|e.to_string())? else{return Ok(None)};
                let player=self.host.player(&player).map_err(|e|e.to_string())?;
                Ok(Some(player.core.name.first().copied().unwrap_or(0)))
            }
            fn dev_mode_62(&mut self)->std::result::Result<u8,String>{(self.dev)()}
        }
        application_gate::can_process_40eaa0(&mut Current{host:self,dev:read_dev_mode})
    }
    ///Actual76C2B0, including lazy GWorld access only for the first enabled
    ///nonempty current Session68 entry. It never substitutes finalCreator88.
    pub fn ui_selected_kingdom_76c2b0(&self,world:&aa_runtime::live_world_frame::World<'_>)->Result<Option<aa_runtime::load_tail_world::KingdomId>>{
        let settings=self.current_game_settings_68()?;
        if settings.string_3c().is_empty(){return Ok(None)}
        for entry in settings.entries(){
            if entry.flag==0&&!entry.string_18.is_empty(){
                let parts=world.presentation_parts().map_err(missing)?;
                self.sim.require_created_world(parts.revision).map_err(missing)?;
                return begin_playing::find_kingdom_in_store(parts.actors,parts.kingdoms.all(),&entry.string_18)
            }
        }
        Ok(None)
    }
    /// Separate loans of Session1C, the SAME BC roster, actual Node268 and
    /// actual Sim world slot. No roster/resource/Node snapshot is produced.
    pub fn recording_parts(
        &mut self,
    ) -> Result<(
        &mut Option<aa_runtime::film_resource::Resource>,
        PlayersBorrow<'_>,
        &mut crate::local_host::Node,
        &aa_runtime::live_sim::Sim,
    )> {
        let node = self
            .host_node
            .as_mut()
            .ok_or_else(|| missing("recording requires actual retained Node"))?;
        Ok((
            &mut self.film_1c,
            PlayersBorrow {
                session: &mut self.session,
                local_player: &self.local_player,
                identity: &self.identity,
            },
            node,
            &self.sim,
        ))
    }
    pub fn film_resource_mut(&mut self) -> Option<&mut aa_runtime::film_resource::Resource> {
        self.film_1c.as_mut()
    }
    ///7757F0 publishes the allocation returned by its genuine5A3250 producer.
    /// This does not write a placeholder header or attach Admin268 early.
    pub fn publish_film_resource(
        &mut self,
        resource: aa_runtime::film_resource::Resource,
    ) -> Result<()> {
        if self.film_1c.is_some() {
            return Err(missing("7757F0 old Session1C film was not closed"));
        }
        self.film_1c = Some(resource);
        Ok(())
    }
    ///7757F0 old-film branch: detach74B310, close the SAME resource, then clear
    ///1C. The caller must persist the returned TGCK bytes using its real depot.
    /// A close error retains Session1C and the preceding detach.
    pub fn close_existing_film(&mut self) -> Result<Option<Vec<u8>>> {
        let Some(resource) = self.film_1c.as_mut() else {
            return Ok(None);
        };
        let node = self
            .host_node
            .as_mut()
            .ok_or_else(|| missing("7757F0 film detach requires actual Node"))?;
        node.detach_recorder_74b310();
        let bytes = resource.close().map_err(missing)?;
        self.film_1c = None;
        Ok(Some(bytes))
    }
    pub fn sim(&self) -> &aa_runtime::live_sim::Sim {
        &self.sim
    }
    pub fn sim_mut(&mut self) -> &mut aa_runtime::live_sim::Sim {
        &mut self.sim
    }
    /// Successful-allocation constructor field projection76F890/76F240,
    /// options77B7B0 and properties79C6E0. Not a full unported Node ctor.
    pub fn from_native_constructors() -> Self {
        Self {
            identity: Arc::new(()),
            session: SessionFields {
                status:session_status::State::constructed(),
                launch_mode: 3,
                launch_game_type: 0,
                launch_file_5c: Vec::new(), //76E540 Launch+8 native empty CString
                launch_kind_60: 0,          //76B47E Launch+C, called by76E540 constructor
                launch_text_84:Vec::new(), //76E6C5..E1 then76B420 reset.
                launch_state: constructor_launch_state(),
                options: GameOptions {
                    name: Vec::new(),
                    password: Vec::new(),
                    max_players: -1,
                    field_d8: u32::MAX,
                    allow_observers: true,
                },
                properties: PropertyTable {
                    flag: false,
                    count: 0,
                    entries: Vec::new(),
                },
                players: Vec::new(),
                created: None,
                changed: [false; 3],
                counter_e0: 0,
                state: 0,
                phase: 0,
                film_24: order_gate::Byte::from_native_constructor_zero(), //actual76FAE7
                last_player_id: None,
                rng: NativeRng::default(),
                multiplayer: false,
                launch_countdown: 0,
                seed_override_100: None,
                task_124: leave_step::TaskSlot::NullFromNativeConstructor,
                host_countdown: 0,
                setup_ai: sai_setup::Queue::from_native_constructor(),
                field_18: 0,
                film_name_150: Vec::new(),
                field_154: None,
                field_158: 0,
                field_15c: 0,
                text_160: Vec::new(),
                field_164: 0,
                // Native ctor never writes +169. Reset also leaves it untouched.
                flags_168_16e: [
                    Some(false),
                    None,
                    Some(false),
                    Some(false),
                    Some(false),
                    Some(false),
                    Some(false),
                ],
            },
            film_1c: None,
            local_player: None,
            ordering_player: None,
            node_host: PeerLink::Null,
            node_active: false,
            peer_list: Vec::new(),
            peers_changed: true,
            local_identity: PlayerId {
                name: Vec::new(),
                serial: 0,
                a: 0,
                b: 0,
                c: 0,
            },
            sim: aa_runtime::live_sim::Sim::from_native_constructor_fields(),
            world_lock: Mutex::new(()),
            sync_a: RefCell::new(Synchronizer::new(DEFAULT_MAX_ENTRIES)),
            sync_s: RefCell::new(Synchronizer::new(DEFAULT_MAX_ENTRIES)),
            host_node: None,
            host_input: None,
            load_selection: None,
            launch_other_creators:None,
            assignment_data_14: assignment::StagingData::from_native_constructor(),
            progress: Progress::Constructed,
            load_jobs:VecDeque::new(),
            version_ack_diagnostics:version_ack::Report::new_modern_transport(),
        }
    }

    /// Real fresh-owner76EC70 Reset, including76E0F0 EndSession and the
    /// no-world branch of76AB80. Existing live worlds/replays/players are not
    /// silently discarded: those constructors/destructors require further ports.
    /// A failed OS/queue service retains all effects already committed.
    pub fn reset_fresh(
        &mut self,
        os: &mut dyn ResetOs,
        source: &InitialProperties,
        notifications: &Notifications,
    ) -> Result<()> {
        if self.progress != Progress::Constructed {
            return Err(missing(
                "fresh Reset cannot destroy a started/incomplete native owner",
            ));
        }
        self.progress = Progress::Resetting;
        if self.film_1c.is_some() {
            return Err(Error::Unsupported("76E0F0 existing film stop/close"));
        }
        // Session+1C is null by ctor76FAD1;76E0F0 still locks the real timeline
        // World slot through76AB80, then posts the native kind2 value.
        {
            let _guard = self
                .world_lock
                .lock()
                .map_err(|_| missing("world critical section poisoned"))?;
            if !self.sim.world_is_null() {
                return Err(Error::Unsupported("76AB80 live GWorld destruction"));
            }
        }
        notifications.push(Notification::end_session())?;
        let tick = os.get_tick_count()?;
        self.session.rng.reseed(tick); // actual85D880, not a new seed algorithm
        self.session.last_player_id = Some(0);
        // 74D460 actual peer list is empty in this fresh owner.
        if !self.peer_list.is_empty() {
            return Err(missing("fresh Reset has nonempty Node peer owner"));
        }
        if !self.session.players.is_empty() {
            return Err(Error::Unsupported("76EC70 SessionPlayer destruction"));
        }
        self.session.changed = [true; 3];
        self.session.flags_168_16e[2] = Some(false);
        self.session.state = 0;
        self.session.phase = 0;
        self.session.launch_countdown = 0;
        self.session.host_countdown = 0;
        self.session.field_18 = 0;
        self.session.field_158 = 0;
        self.session.field_15c = 0;
        // 76DD90(true) never reads undefined time words or ResetAdminTime.
        // Honor lazy logger gating, then reuse the actual synchronizer kernel.
        let sync_a = self.sync_a.get_mut();
        self.sim.store_frozen_after_sync(1, || -> Result<()> {
            let gate = if sync_a.enabled && sync_a.suspend < 1 {
                os.sync_log_gate()?
            } else {
                false
            };
            sync_a.log_bool(gate, "_frozen", true);
            Ok(())
        })?;
        self.session.flags_168_16e[4] = Some(false);
        self.session.flags_168_16e[0] = Some(false);
        let guid = os.co_create_guid()?;
        self.session.text_160 = guid.native_text();
        self.session.field_164 = 0;
        self.session.setup_ai.clear_count();
        self.session.flags_168_16e[3] = Some(false);
        self.session.flags_168_16e[5] = Some(false);
        self.session.flags_168_16e[6] = Some(false);
        self.session.multiplayer = false;
        self.session.field_154 = Some(0);
        // 79C910 preserves destination+10 flag; it copies count/entries only.
        self.session.properties.entries.clear();
        self.session.properties.count = source.table.count;
        self.session
            .properties
            .entries
            .clone_from(&source.table.entries);
        self.progress = Progress::Reset;
        Ok(())
    }

    /// Whole fresh751770 reached owner sequence, preserving its actual
    /// before/after76EC70 boundary. This creates ONE OrderAdmin allocation;
    /// callers using the lower Session-only reset cannot later skip this step.
    pub fn reset_node_fresh(
        &mut self,
        os: &mut dyn ResetOs,
        source: &InitialProperties,
        notifications: &Notifications,
    ) -> Result<()> {
        if self.progress != Progress::Constructed || self.host_node.is_some() {
            return Err(missing(
                "fresh Node Reset requires its untouched owning constructor",
            ));
        }
        self.host_node = Some(crate::local_host::Node::before_session_reset()?);
        self.reset_fresh(os, source, notifications)?;
        self.host_node.as_mut().unwrap().after_session_reset()?;
        Ok(())
    }

    /// Same ordinary local Host suffix75B1B1..75B20E. The source-created
    /// SVar is read lazily after ResetOrderVersion, then the same retained QPC
    /// timer restarts the actual ServerClock. Staging/launch is not implied.
    pub fn finish_fresh_self_host<T: crate::local_host::MillisecondTimer>(
        &mut self,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        time: &mut crate::local_host::NodeTime<T>,
        precision: aa_numeric::Precision,
    ) -> Result<()> {
        self.finish_fresh_self_host_with(time, || crate::local_host::admin_rate(catalog), precision)
    }
    pub(crate) fn finish_fresh_self_host_with<T: crate::local_host::MillisecondTimer>(
        &mut self,
        time: &mut crate::local_host::NodeTime<T>,
        rate: impl FnOnce() -> Result<u32>,
        precision: aa_numeric::Precision,
    ) -> Result<()> {
        if self.progress != Progress::LocalPlayerCreated || !self.peer_list.is_empty() {
            return Err(missing(
                "fresh self Host requires its sole constructed player and empty peer owner",
            ));
        }
        let node = self
            .host_node
            .as_mut()
            .ok_or_else(|| missing("whole751770 Node Reset has not run"))?;
        node.require_reset()?;
        self.progress = Progress::CreatingLocalHost;
        self.peer_list
            .try_reserve(1)
            .map_err(|_| missing("Host peer-list allocation failed"))?;
        self.peer_list.push(PeerLink::Node);
        self.peers_changed = true;
        node.set_self_server(time)?;
        self.node_host = PeerLink::Null; //actual75B1DA forced-null before SetHost
        node.set_self_host(time, rate, precision)?;
        self.node_host = PeerLink::Node;
        node.after_host_bound(time)?;
        self.sync_a.get_mut().reset(0, 0);
        self.progress = Progress::LocalHostCreated;
        Ok(())
    }
    pub fn host_node(&self) -> Result<&crate::local_host::Node> {
        self.host_node
            .as_ref()
            .ok_or_else(|| missing("whole751770 Node owner is not installed"))
    }
    pub fn order_node(&self) -> Result<&aa_orders::net::admin::AdminNode> {
        Ok(&self.host_node()?.orders)
    }
    pub fn order_node_mut(&mut self) -> Result<&mut aa_orders::net::admin::AdminNode> {
        Ok(&mut self
            .host_node
            .as_mut()
            .ok_or_else(|| missing("whole751770 Node owner is not installed"))?
            .orders)
    }
    /// No whole-Bootstrap RefCell or cloned players/logs. The actual dispatcher
    /// retains its Admin streams while callbacks borrow the same live player
    /// store and canonical ordering-player slot lazily.
    pub fn runtime_borrow(&mut self) -> Result<RuntimeBorrow<'_>> {
        let node = self
            .host_node
            .as_mut()
            .ok_or_else(|| missing("whole751770 Node owner is not installed"))?;
        Ok(RuntimeBorrow {
            owner:&self.identity,
            order_gate:order_gate::Borrow::from_current(&self.session.film_24,&node.order_abort_2e5,&self.identity),
            admin: &mut node.orders,
            players: PlayersBorrow {
                session: &mut self.session,
                local_player: &self.local_player,
                identity: &self.identity,
            },
            sync_a: &self.sync_a,
            server_clock: &mut node.clock,
            local_client: LocalClientBorrow {
                owner_identity: &self.identity,
                player_2c: &mut self.ordering_player,
                version_6c: &mut node.order_version,
                host_2c4: &self.node_host,
                mirror_268: &node.mirror_268,
                peer_task_34: &mut node.peer_task_34,
                vworld_init_required_8c: &mut node.vworld_init_required_8c,
                task_status_90: &mut node.task_status_90,
                peers_2ac: &self.peer_list,
                task_history_238: &mut node.task_history_238,
                takeover_2e0:&node.takeover_2e0,
                commands_118:&mut node.commands_118,
                version_ack_report:&mut self.version_ack_diagnostics,
                heartbeat:frame_heartbeat::Fields{
                    requests:&mut node.backup_requests,decrees:&mut node.backup_decrees,next:&mut node.next_backup_decree,
                    flags:&mut node.flags,vote:&mut node.vote,votes:&mut node.votes,max_patience:&mut node.max_patience,
                    rtt:&mut node.rtt,received:&mut node.received_fraction,frame_ratio:&mut node.frame_ratio_60,
                },
            },
            sync_s: self.sync_s.get_mut(),
            sim: &mut self.sim,
            frame: crate::local_host::FrameFields {
                running_2c0: &mut self.node_active,
                has_clock_158: &node.has_clock,
                server_is_self_2c8: &node.server_is_self,
                admin_time_28c: &mut node.admin_time_28c,
                disconnecting_2cd: &mut node.disconnecting_2cd,
                disconnect_started_2d0: &mut node.disconnect_started_2d0,
                heartbeat_2dc: &mut node.heartbeat_2dc,
                backup_mode_2d4: &mut node.backup_mode,
                backup_since_2d8: &mut node.backup_since,
                playback_26c: &node.playback_26c,
            },
        })
    }

    /// 75B126..75B1B1 local Host prefix. `nickname` is UserData+2C; the local
    /// identity key is [B40C24]+1C copied by77B8F0. Neither is a Kingdom name.
    /// Host/server/clock setup comes later, in its native order.
    pub fn create_first_local_player(
        &mut self,
        nickname: &[u16],
        local_identity_key: &[u16],
        localization: &Localization,
    ) -> Result<PlayerHandle> {
        self.create_first_local_player_with(nickname, local_identity_key, || {
            construct_view(None, localization)
        })
    }

    ///The actual application path uses its same retained nonnull GameSpy
    ///manager projection at770280, after publishing the new player/C8 and
    ///before name/Node8/Node2C writes. A failure preserves that exact prefix.
    pub fn create_first_local_player_with_game_spy(
        &mut self,
        nickname: &[u16],
        local_identity_key: &[u16],
        localization: &Localization,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        bank: &aa_runtime::definitions::Bank,
        manager: &aa_runtime::game_spy_manager_gate::State,
    ) -> Result<PlayerHandle> {
        self.create_first_local_player_with_rated(
            nickname,
            local_identity_key,
            || construct_view(None, localization),
            |owner| owner.update_rated_770280(catalog, bank, manager),
        )
    }

    fn create_first_local_player_with(
        &mut self,
        nickname: &[u16],
        local_identity_key: &[u16],
        view_constructor: impl FnOnce() -> Result<VWorldState>,
    ) -> Result<PlayerHandle> {
        self.create_first_local_player_with_rated(
            nickname,
            local_identity_key,
            view_constructor,
            |owner| {
                //Historical constructor-only projection proves no rule read
                //for mode3. Live application callers use the real manager API.
                if owner.session.launch_mode != 3 {
                    return Err(Error::Unsupported(
                        "first local player rated update outside ctor mode3",
                    ));
                }
                Ok(())
            },
        )
    }

    fn create_first_local_player_with_rated(
        &mut self,
        nickname: &[u16],
        local_identity_key: &[u16],
        view_constructor: impl FnOnce() -> Result<VWorldState>,
        rated: impl FnOnce(&mut Self) -> Result<()>,
    ) -> Result<PlayerHandle> {
        if self.progress != Progress::Reset
            || !self.session.players.is_empty()
            || self.node_host != PeerLink::Null
        {
            return Err(missing(
                "first local player requires the actual fresh Reset owner and null Host",
            ));
        }
        self.progress = Progress::CreatingPlayer;
        self.node_active = true;
        let id = self
            .session
            .last_player_id
            .ok_or_else(|| missing("Session+E8 has no Reset producer"))?
            .wrapping_add(1);
        self.session.last_player_id = Some(id);
        let core = PlayerFields::from_constructor(id, self.node_host);
        let view = view_constructor()?;
        self.session.players.push(OwnedPlayer {
            allocation_lifetime: Arc::new(()),
            core,
            vector_44: Vec::new(),
            view: Some(view),
        });
        self.session.created = Some(id);
        self.session.changed[0] = true;
        rated(self)?;
        // 766560 on the sole player has no other duplicate, including the
        // empty-name case where76CE10 returns this player itself.
        let p = &mut self.session.players[0].core;
        p.name = nickname.to_vec();
        p.flag_2c = true;
        self.local_player = Some(id);
        self.ordering_player = Some(id);
        self.local_identity = PlayerId {
            name: local_identity_key.to_vec(),
            serial: 0,
            a: 0,
            b: 0,
            c: 0,
        };
        self.progress = Progress::LocalPlayerCreated;
        Ok(PlayerHandle {
            owner: Arc::downgrade(&self.identity),
            id,
            slot: 0,
            allocation: Arc::downgrade(&self.session.players[0].allocation_lifetime),
        })
    }

    fn player(&self, handle: &PlayerHandle) -> Result<&OwnedPlayer> {
        if !Weak::ptr_eq(&handle.owner, &Arc::downgrade(&self.identity)) {
            return Err(missing(
                "SessionPlayer handle belongs to another native owner",
            ));
        }
        self.session
            .players
            .get(handle.slot)
            .filter(|p| {
                p.core.id == handle.id
                    && Weak::ptr_eq(&handle.allocation, &Arc::downgrade(&p.allocation_lifetime))
            })
            .ok_or_else(|| missing("SessionPlayer is no longer owned by this Session"))
    }
    fn player_mut(&mut self, handle: &PlayerHandle) -> Result<&mut OwnedPlayer> {
        if !Weak::ptr_eq(&handle.owner, &Arc::downgrade(&self.identity)) {
            return Err(missing(
                "SessionPlayer handle belongs to another native owner",
            ));
        }
        self.session
            .players
            .get_mut(handle.slot)
            .filter(|p| {
                p.core.id == handle.id
                    && Weak::ptr_eq(&handle.allocation, &Arc::downgrade(&p.allocation_lifetime))
            })
            .ok_or_else(|| missing("SessionPlayer is no longer owned by this Session"))
    }
    pub fn local_player(&self) -> Result<Option<PlayerHandle>> {
        self.local_player
            .map(|id| {
                let h = PlayerHandle {
                    owner: Arc::downgrade(&self.identity),
                    id,
                    allocation: Arc::downgrade(
                        &self
                            .session
                            .players
                            .iter()
                            .find(|p| p.core.id == id)
                            .ok_or_else(|| {
                                missing("actual Node8 player allocation is unavailable")
                            })?
                            .allocation_lifetime,
                    ),
                    slot: self
                        .session
                        .players
                        .iter()
                        .position(|p| p.core.id == id)
                        .ok_or_else(|| missing("actual Node8 local player is no longer owned"))?,
                };
                self.player(&h)?;
                Ok(h)
            })
            .transpose()
    }
    /// Native player+24. This reads the actual constructor/staging-owned
    /// CString; it never substitutes the profile nickname or a map label.
    pub fn kingdom_name(&self, player: &PlayerHandle) -> Result<&[u16]> {
        Ok(&self.player(player)?.core.kingdom)
    }
    pub fn player_name(&self, player: &PlayerHandle) -> Result<&[u16]> {
        Ok(&self.player(player)?.core.name)
    }
    pub fn player_status(&self, player: &PlayerHandle) -> Result<u32> {
        Ok(self.player(player)?.core.status)
    }
    /// Same non-Clone VWorldState+4/+8 node owner used by the native selection
    /// adapter and command traversal. The caller cannot fill a cloned Vec.
    pub fn selection(&self, player: &PlayerHandle) -> Result<&aa_runtime::player_selection::List> {
        Ok(&self
            .player(player)?
            .view
            .as_ref()
            .ok_or_else(|| missing("player VWorldState constructor incomplete"))?
            .selected)
    }
    pub fn selection_mut(
        &mut self,
        player: &PlayerHandle,
    ) -> Result<&mut aa_runtime::player_selection::List> {
        Ok(&mut self
            .player_mut(player)?
            .view
            .as_mut()
            .ok_or_else(|| missing("player VWorldState constructor incomplete"))?
            .selected)
    }
    pub fn selection_borrow(&mut self, player: &PlayerHandle) -> Result<SelectionBorrow<'_>> {
        let v = self
            .player_mut(player)?
            .view
            .as_mut()
            .ok_or_else(|| missing("player VWorldState constructor incomplete"))?;
        Ok(SelectionBorrow {
            player_id: player.id,
            list: &mut v.selected,
            kingdom: &mut v.kingdom,
            mask_words_10_1c: &mut v.mask_words_10_1c,
            look_at_28_2c: &mut v.look_at_28_2c,
            group_20: &mut v.current_group,
        })
    }

    /// Resolve the actual Admin+284 id against this Session, as76B8F0 does.
    /// Node+8/+2C is deliberately not a fallback for an absent ordering id.
    pub fn ordering_selection_borrow(
        &mut self,
        admin: &aa_orders::net::admin::AdminNode,
    ) -> Result<SelectionBorrow<'_>> {
        self.ordering_selection_and_session_borrow(admin)
            .map(|(view, _)| view)
    }

    /// Split disjoint real fields so a selection callback can lazily read the
    /// same Session while mutating its player's actual linked selection.
    pub fn ordering_selection_and_session_borrow(
        &mut self,
        admin: &aa_orders::net::admin::AdminNode,
    ) -> Result<(SelectionBorrow<'_>, SelectionSessionBorrow<'_>)> {
        self.ordering_selection_and_session_for_slot(admin.ordering_player_slot())
    }
    pub fn ordering_selection_for_slot(
        &mut self,
        ordering_player_284: &u32,
    ) -> Result<SelectionBorrow<'_>> {
        self.ordering_selection_and_session_for_slot(ordering_player_284)
            .map(|(selection, _)| selection)
    }
    /// Callback loan of the canonical Admin+284 slot while the actual Admin
    /// dispatcher holds its streams. Zero means null; no Node player fallback.
    pub fn ordering_selection_and_session_for_slot(
        &mut self,
        ordering_player_284: &u32,
    ) -> Result<(SelectionBorrow<'_>, SelectionSessionBorrow<'_>)> {
        Self::selection_session_parts(&mut self.session, self.local_player, ordering_player_284)
    }
    fn selection_session_parts<'a>(
        session: &'a mut SessionFields,
        local_player: Option<u32>,
        ordering_player_284: &u32,
    ) -> Result<(SelectionBorrow<'a>, SelectionSessionBorrow<'a>)> {
        let id = *ordering_player_284;
        if id == 0 {
            return Err(missing("actual Admin+284 ordering player is null"));
        }
        let ordering_index = session
            .players
            .iter()
            .position(|p| p.core.id == id)
            .ok_or_else(|| missing("actual Admin+284 player id is not owned by this Session"))?;
        let local_index =
            local_player.and_then(|id| session.players.iter().position(|p| p.core.id == id));
        let (p, other_local_name) = match local_index {
            Some(local) if local < ordering_index => {
                let (left, right) = session.players.split_at_mut(ordering_index);
                (&mut right[0], Some(left[local].core.name.as_slice()))
            }
            Some(local) if local > ordering_index => {
                let (left, right) = session.players.split_at_mut(local);
                (
                    &mut left[ordering_index],
                    Some(right[0].core.name.as_slice()),
                )
            }
            _ => (&mut session.players[ordering_index], None),
        };
        let local_player_name_8 = match (local_player, local_index) {
            (None, _) => Ok(None),
            (Some(_), None) => Err("actual Node+8 local player id is not owned by this Session"),
            (Some(_), Some(local)) if local == ordering_index => Ok(Some(p.core.name.as_slice())),
            _ => Ok(other_local_name),
        };
        let v = p
            .view
            .as_mut()
            .ok_or_else(|| missing("player VWorldState constructor incomplete"))?;
        let session_view = SelectionSessionBorrow {
            state_e4: &session.state,
            phase_f8: &session.phase,
            multiplayer_f4: &session.multiplayer,
            ordering_player_name_8: &p.core.name,
            local_player_name_8,
        };
        let selection = SelectionBorrow {
            player_id: id,
            list: &mut v.selected,
            kingdom: &mut v.kingdom,
            mask_words_10_1c: &mut v.mask_words_10_1c,
            look_at_28_2c: &mut v.look_at_28_2c,
            group_20: &mut v.current_group,
        };
        Ok((selection, session_view))
    }

    /// Native765E10 low-level writer. It is called by the real BeginPlaying
    /// producer773E30 after its ordered4F48E0 lookup, not by a FirstTick name
    /// inference. Validate a nonnull candidate against the actual ActorStore.
    /// The pointer store precedes old-view destruction/new construction, so a
    /// failed text service keeps the new+28 and an absent+50, without rollback.
    pub fn replace_kingdom_765e10(
        &mut self,
        player: &PlayerHandle,
        kingdom: Option<aa_runtime::load_tail_world::KingdomId>,
        actors: &aa_runtime::actor_world::ActorStore,
        localization: &Localization,
    ) -> Result<()> {
        if let Some(id) = kingdom {
            use aa_runtime::actor_world::{Source, SpecialSource};
            let key = actors
                .key(id)
                .ok_or_else(|| missing("765E10 Kingdom candidate is not live"))?;
            let value = actors.entity(key).map_err(|e| missing(e.to_string()))?;
            if !matches!(&value.source, Source::Special(s) if matches!(&s.source, SpecialSource::Kingdom(_)))
            {
                return Err(missing(
                    "765E10 candidate does not identify the actual Kingdom class",
                ));
            }
        }
        self.replace_kingdom_with(player, kingdom, || {
            construct_view(kingdom, localization)
        })
    }
    fn replace_kingdom_with(
        &mut self,
        player: &PlayerHandle,
        kingdom: Option<aa_runtime::load_tail_world::KingdomId>,
        constructor: impl FnOnce() -> Result<VWorldState>,
    ) -> Result<()> {
        let p = self.player_mut(player)?;
        p.core.field_28 = kingdom;
        p.core.binding_complete = false;
        //765E10 clears Player+50 only AFTER the same old allocation completed
        //4ED210. A reached error retains its exact partial destruction state.
        if let Some(view)=p.view.as_mut() { view.destroy().map_err(missing)?; }
        drop(p.view.take());
        p.view = Some(constructor()?);
        p.core.binding_complete = true;
        Ok(())
    }
    pub fn player_view_snapshot(&self, player: &PlayerHandle) -> Result<serde_json::Value> {
        let p = self.player(player)?;
        let v = p
            .view
            .as_ref()
            .ok_or_else(|| missing("player VWorldState constructor incomplete"))?;
        let g = v.wildcard().ok_or_else(||missing("VWorldState wildcard group was retired"))?;
        Ok(serde_json::json!({"player_vector_count":p.vector_44.len(),
            "view_present":true,"view_kingdom":v.kingdom,"view_word_0c":v.word_0c(),
            "mask_words_10_1c":v.mask_words_10_1c,"group_is_null":v.current_group.is_null(),
            "look_at_28_2c":v.look_at_28_2c,"selection_empty":v.selected.is_empty(),
            "group":{"key":g.key(),"title":g.title(),"text":g.text(),
            "count":g.members().len(),"flag_18":g.flag_18(),"word_1c":g.word_1c(),"flag_20":g.flag_20()}}))
    }
    pub fn progress(&self) -> Progress {
        self.progress
    }
    pub fn launch_mode(&self) -> u32 {
        self.session.launch_mode
    }
    pub fn launch_game_type(&self) -> u32 {
        self.session.launch_game_type
    }
    pub fn options(&self) -> &GameOptions {
        &self.session.options
    }
    pub fn last_player_id(&self) -> Result<u32> {
        self.session
            .last_player_id
            .ok_or_else(|| missing("native ctor leaves Session+E8 uninitialized"))
    }
    pub fn rng_state(&self) -> u32 {
        self.session.rng.state
    }
    pub fn reset_guid_text(&self) -> &[u16] {
        &self.session.text_160
    }
    /// Borrow the same native B3655C owner. Launch/clocked services must not
    /// replace this with a fresh log or copy a stale synchronizer snapshot.
    pub fn sync_a_mut(&mut self) -> &mut Synchronizer {
        self.sync_a.get_mut()
    }
    /// The same A-log allocation can be borrowed briefly by Driver and world
    /// callbacks; its mutable borrow must end before reentrant world services.
    pub fn sync_a_cell(&self) -> &RefCell<Synchronizer> {
        &self.sync_a
    }
    pub fn sync_s_cell(&self) -> &RefCell<Synchronizer> {
        &self.sync_s
    }
    ///Actual resource-worker readers borrow the same Session fields while
    ///the World constructor writes the sibling Sim and logs to the same A/S.
    ///No whole-owner alias or copied SessionFacts is created.
    pub fn resource_load_parts(&mut self) -> ResourceLoadParts<'_> {
        ResourceLoadParts {
            session: SessionLoadBorrow {
                state_e4: &self.session.state,phase_f8:&self.session.phase,
                players:&self.session.players,local_player:&self.local_player,identity:&self.identity,
            },
            sim: &mut self.sim,
            sync_a: &self.sync_a,
            sync_s: &self.sync_s,
            selected_creator: self.load_selection.as_ref().map(|s| s.creator.as_ref()),
        }
    }
    ///Scoped worker loan: direct A/S borrows are Send; RefCell references are
    ///kept on the application thread. No mutable synchronizer copy is made.
    pub fn resource_worker_parts(&mut self)->ResourceWorkerParts<'_>{
        ResourceWorkerParts{
            session:SessionLoadBorrow{state_e4:&self.session.state,phase_f8:&self.session.phase,players:&self.session.players,local_player:&self.local_player,identity:&self.identity},
            sim:&mut self.sim,sync_a:self.sync_a.get_mut(),sync_s:self.sync_s.get_mut(),
            selected_creator:self.load_selection.as_ref().map(|s|s.creator.as_ref()),
        }
    }
    pub fn host(&self) -> PeerLink {
        self.node_host
    }
    ///Legacy diagnostic gate. A queued native task is not an active battle;
    ///the consuming worker/callback/activation receipts remain authoritative.
    pub fn require_host_launch(&self) -> Result<()> {
        if self.host_input.as_ref().is_some_and(|h|h.stage==launch::InputStage::MainLaunchComplete){
            return Err(Error::Unsupported("actual queued task still requires worker result/775DB0 callbacks/application activation"));
        }
        if self.progress == Progress::LocalHostCreated {
            return Err(Error::Unsupported(
                "75B20E onwards: selected mode/settings and77A950 staging/kingdom assignment/launch",
            ));
        }
        Err(Error::Unsupported(
            "75B1B1 onwards: SetServer/SetHost/selected settings/kingdom assignment/77A950 launch",
        ))
    }
}

impl SessionPhase for LocalBootstrap {
    fn current_phase(&self) -> std::result::Result<u32, String> {
        Ok(self.session.phase)
    }
    fn application_state(&self) -> std::result::Result<u32, String> {
        Ok(self.session.state)
    }
}

impl aa_runtime::load_tail_world::SessionRead for LocalBootstrap {
    type Player = PlayerHandle;
    fn local_player(&self) -> std::result::Result<Option<Self::Player>, String> {
        LocalBootstrap::local_player(self).map_err(|e| e.to_string())
    }
    fn player_name_24(&self, player: &Self::Player) -> std::result::Result<&[u16], String> {
        self.kingdom_name(player).map_err(|e| e.to_string())
    }
    fn player_kingdom_28(
        &self,
        player: &Self::Player,
    ) -> std::result::Result<Option<aa_runtime::load_tail_world::KingdomId>, String> {
        let p = self.player(player).map_err(|e| e.to_string())?;
        if !p.core.binding_complete || p.view.is_none() {
            return Err(
                "required765E10 live player+28 Kingdom/VWorldState binding has not completed"
                    .into(),
            );
        }
        Ok(p.core.field_28)
    }
}

/// A read-only diagnostic snapshot, never used to update the retained owner.
#[derive(Debug, Serialize)]
pub struct Snapshot<'a> {
    pub progress: Progress,
    pub launch_mode: u32,
    pub launch_game_type: u32,
    pub launch_state: &'a LaunchState,
    pub options: &'a GameOptions,
    pub changed: [bool; 3],
    pub counter_e0: u32,
    pub state: u32,
    pub phase: u32,
    pub last_player_id: Option<u32>,
    pub rng_state: u32,
    pub multiplayer: bool,
    pub field_154: Option<u32>,
    pub text_160: &'a [u16],
    pub sim_frozen: bool,
    pub players: Vec<serde_json::Value>,
    pub local_player: Option<u32>,
    pub ordering_player: Option<u32>,
    pub host: PeerLink,
    pub node_active: bool,
    pub peer_list: &'a [PeerLink],
    pub peers_changed: bool,
    pub local_identity: &'a PlayerId,
    pub properties: &'a PropertyTable,
    pub sync_a_enabled: bool,
    pub sync_a_hash: i32,
    pub sync_a_end: i32,
    pub sync_s_enabled: bool,
}
impl LocalBootstrap {
    pub fn snapshot(&self) -> Snapshot<'_> {
        let sync_a = self.sync_a.borrow();
        Snapshot {
            progress: self.progress,
            launch_mode: self.session.launch_mode,
            launch_game_type: self.session.launch_game_type,
            launch_state: &self.session.launch_state,
            options: &self.session.options,
            changed: self.session.changed,
            counter_e0: self.session.counter_e0,
            state: self.session.state,
            phase: self.session.phase,
            last_player_id: self.session.last_player_id,
            rng_state: self.session.rng.state,
            multiplayer: self.session.multiplayer,
            field_154: self.session.field_154,
            text_160: &self.session.text_160,
            sim_frozen: self.sim.frozen(),
            players: self
                .session
                .players
                .iter()
                .map(|p| serde_json::to_value(&p.core).expect("player serialization is infallible"))
                .collect(),
            local_player: self.local_player,
            ordering_player: self.ordering_player,
            host: self.node_host,
            node_active: self.node_active,
            peer_list: &self.peer_list,
            peers_changed: self.peers_changed,
            local_identity: &self.local_identity,
            properties: &self.session.properties,
            sync_a_enabled: sync_a.enabled,
            sync_a_hash: sync_a.hash,
            sync_a_end: sync_a.end(),
            sync_s_enabled: self.sync_s.borrow().enabled,
        }
    }
}

#[cfg(test)]
#[path = "local_bootstrap_tests.rs"]
mod tests;

#[path = "local_session_kingdom_cleanup.rs"]
mod kingdom_cleanup;

#[path = "local_assignment.rs"]
pub mod assignment;
#[path = "local_begin_play_live.rs"]
pub mod begin_play_live;
#[path = "local_begin_play.rs"]
pub mod begin_playing;
#[path = "local_view_startup.rs"]
pub mod view_startup;
#[path = "local_application_gate.rs"]
pub mod application_gate;
#[path = "local_glyph_roster.rs"]
pub mod glyph_roster;
#[path = "local_implicit_ai.rs"]
pub mod implicit_ai;
#[path = "local_finalize.rs"]
pub mod finalize;
#[path = "local_launch.rs"]
pub mod launch;
#[path = "local_launch_live.rs"]
pub mod launch_live;
#[path = "local_leave_step.rs"]
pub mod leave_step;
#[path = "local_load_task.rs"]
pub mod load_task;
#[path = "local_rated.rs"]
pub mod rated;
#[path = "local_sai_setup.rs"]
pub mod sai_setup;
#[path = "local_world_ready.rs"]
pub mod world_ready;
#[path="local_all_tasks_complete.rs"]
pub mod all_tasks_complete;

///765E00 must read the SAME actual global World slot before player+28. This
///wrapper loans its Sim sibling and, for765E00, a sealed current World
///identity. SimNULL alone cannot substitute for the original global pointer.
///Session/roster values are never copied.
pub struct FilmRoster<'a, 'owner> {
    players: &'a mut PlayersBorrow<'owner>,
    sim: &'a aa_runtime::live_sim::Sim,
    world: Option<aa_runtime::world_revision::Witness>,
}
impl aa_runtime::film_session::Roster for FilmRoster<'_, '_> {
    type Player = PlayerHandle;
    fn count(&mut self) -> std::result::Result<usize, String> {
        Ok(self.players.player_count())
    }
    fn player_at(&mut self, index: usize) -> std::result::Result<Self::Player, String> {
        self.players.player_at(index).map_err(|e| e.to_string())
    }
    fn id(&mut self, player: &Self::Player) -> std::result::Result<u32, String> {
        self.players.player_id(player).map_err(|e| e.to_string())
    }
    fn name_24(&mut self, player: &Self::Player) -> std::result::Result<&[u16], String> {
        aa_runtime::load_tail_world::SessionRead::player_name_24(self.players, player)
    }
    fn kingdom_28(
        &mut self,
        player: &Self::Player,
    ) -> std::result::Result<Option<aa_runtime::load_tail_world::KingdomId>, String> {
        let world=self.world.as_ref().ok_or("765E00 requires the actual current globalWorld owner; SimNULL does not prove its absence")?;
        self.sim.require_created_world_witness(world)?;
        //Native765E00 reads raw28. The separate SessionRead complete-view
        //getter deliberately retains its stronger VWorld completion guard.
        Ok(self.players.player(player).map_err(|e|e.to_string())?.core.field_28)
    }
    fn ai_implicit(&mut self, player: &Self::Player) -> std::result::Result<(u8, u8), String> {
        Ok((
            u8::from(self.players.player_ai(player).map_err(|e| e.to_string())?),
            u8::from(
                self.players
                    .player_implicit(player)
                    .map_err(|e| e.to_string())?,
            ),
        ))
    }
}
impl aa_runtime::film_header_entries::Players for FilmRoster<'_, '_> {
    type Player = PlayerHandle;
    fn first(&mut self) -> std::result::Result<Option<PlayerHandle>, String> {
        if self.players.player_count() == 0 {
            Ok(None)
        } else {
            self.players
                .player_at(0)
                .map(Some)
                .map_err(|e| e.to_string())
        }
    }
    fn next(&mut self, player: &PlayerHandle) -> std::result::Result<Option<PlayerHandle>, String> {
        self.players.player(player).map_err(|e| e.to_string())?;
        let next = player
            .slot
            .checked_add(1)
            .ok_or("film roster traversal overflow")?;
        if next == self.players.player_count() {
            Ok(None)
        } else {
            self.players
                .player_at(next)
                .map(Some)
                .map_err(|e| e.to_string())
        }
    }
    fn ai_byte(&mut self, player: &PlayerHandle) -> std::result::Result<u8, String> {
        self.players
            .player_ai(player)
            .map(u8::from)
            .map_err(|e| e.to_string())
    }
    fn kingdom_765e00(
        &mut self,
        player: &PlayerHandle,
    ) -> std::result::Result<Option<aa_runtime::load_tail_world::KingdomId>, String> {
        aa_runtime::film_session::Roster::kingdom_28(self, player)
    }
    fn name_24(&mut self, player: &PlayerHandle) -> std::result::Result<&[u16], String> {
        aa_runtime::film_session::Roster::name_24(self, player)
    }
    fn display_name_08(&mut self, player: &PlayerHandle) -> std::result::Result<&[u16], String> {
        self.players
            .player_display_name_8(player)
            .map_err(|e| e.to_string())
    }
    fn is_local(&mut self, player: &PlayerHandle) -> std::result::Result<bool, String> {
        self.players
            .is_local_player(player)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod notification_kind_producer_tests {
    use super::*;
    #[test]
    fn interactive_kind_matches_reached_native_init_writer_slices(){
        let native:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-simulation-execution/native-application-kind.json")).unwrap();
        assert_eq!(native["rows"].as_array().unwrap().len(),24);
        for row in native["rows"].as_array().unwrap(){
            let queue=Notifications::from_native_interactive_post_init();
            assert_eq!(row["after_initial_store"],1);
            assert_eq!(queue.application_kind.load(Ordering::SeqCst),row["after_post_parse_store"].as_u64().unwrap()as u32);
            assert!(queue.is_empty().unwrap());
        }
    }
}

#[cfg(test)]
mod launch_text_borrow_tests {
    use super::*;
    #[test]
    fn current_text_loan_uses_same_owner_and_ends_before_next_mutation_even_on_error() {
        //Isolated portable lifetime test: empty source/text values here are
        //not constructors or a successful native Host/gameplay fixture.
        let actors=aa_rules::actors::ActorCatalog::from_files(Vec::new(),aa_rules::tgi::Limits::default());
        let catalog=aa_runtime::catalog::Catalog::new(&actors);
        *catalog.localization.borrow_mut()=Some(Localization::default());
        let address=catalog.localization.borrow().as_ref().unwrap() as *const Localization;
        with_launch_text(&catalog,None,|actual| {
            assert_eq!(actual as *const Localization,address);
            assert!(catalog.localization.try_borrow_mut().is_err());
            Ok(())
        }).unwrap();
        assert!(catalog.localization.try_borrow_mut().is_ok());
        let error=with_launch_text(&catalog,None,|actual| {
            assert_eq!(actual as *const Localization,address);
            Err::<(),_>(missing("injected text-consumer failure"))
        }).unwrap_err();
        assert!(error.to_string().contains("injected text-consumer failure"));
        assert!(catalog.localization.try_borrow_mut().is_ok());
        assert_eq!(catalog.localization.borrow().as_ref().unwrap() as *const Localization,address);
        *catalog.localization.borrow_mut()=None;
        assert!(with_launch_text(&catalog,None,|_|Ok(())).unwrap_err().to_string().contains("TextManager is unbound"));
    }
}

#[path="local_order_gate.rs"]
pub mod order_gate;
#[path="local_frame_host.rs"]
pub mod frame_host;
#[path="local_all_tasks_services.rs"]
pub mod all_tasks_services;
#[path="local_node_commands.rs"]
pub(crate) mod node_commands;
#[path="local_session_frame_update.rs"]
pub mod session_frame_update;
#[path="local_frame_task_effects.rs"]
pub mod frame_task_effects;
#[path="local_frame_heartbeat.rs"]
pub mod frame_heartbeat;
#[path="local_world_view.rs"]
pub mod world_view;
#[path="local_session_status.rs"]
pub mod session_status;
#[path="local_selected_map_confirm.rs"]
pub mod selected_map_confirm;
#[path="local_live_frame.rs"]
pub mod live_frame;
#[path="local_frame_duration.rs"]
pub(crate) mod frame_duration;
#[path="local_frame_end_condition.rs"]
pub mod frame_end_condition;
#[path="local_frame_end_world.rs"]
pub mod frame_end_world;
#[path="local_version_ack.rs"]
pub mod version_ack;
#[path="local_client_description.rs"]
pub(crate) mod client_description;
#[path="local_game_data.rs"]
pub(crate) mod game_data;
#[path="local_gameplay_order.rs"]
mod gameplay_order;
#[path="local_hud_reads.rs"]
pub mod hud_reads;
#[path="local_input_reads.rs"]
pub mod input_reads;
