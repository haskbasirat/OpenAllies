//! Owned, stopped projection of the native application/LoadGame SAI lifecycle.
//!
//! This is deliberately a new application scope, not evidence about an already
//! running native game. 7A0AA0 publishes a present inactive SAI; 76F240 leaves
//! Sim+3C null. The first 5A4280 load therefore runs 76AB80's actual empty-world
//! branch before 4F7910 and map dispatch. Existing-world destruction, task
//! completion/StartMission and active SAI remain separate unported services.
use aa_rules::{
    sources::{ResolvedFile, V14Sources},
    tgi::{Document, Item, Limits},
};
use std::sync::{Condvar, Mutex};

type Result<T> = std::result::Result<T, String>;

/// LoadGameTask+20 selects the wrapper; it is not MapResource's load mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskKind {
    Normal,
    Editor,
    Alternate,
}
impl TaskKind {
    /// 5A3ECB pushes1;5A3E8E pushes2;5A3F3B pushes4.
    pub fn native_load_mode(self) -> u32 {
        match self {
            Self::Normal => 1,
            Self::Editor => 2,
            Self::Alternate => 4,
        }
    }
    pub fn wrapper_address(self) -> u32 {
        match self {
            Self::Normal => 0x5a3eb0,
            Self::Editor => 0x5a3e20,
            Self::Alternate => 0x5a3f20,
        }
    }
}

/// Facts retained for diagnostics; these do not grant a loading capability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Initialization {
    pub uvars_path: String,
    pub uvars_sha256: String,
    pub elapsed_millis: u32,
    pub private_seed: u32,
}

/// 857D60 creates one initially nonsignaled auto-reset event and registers its
/// waitable owner. The stopped scope owns the Rust synchronization allocation;
/// it exposes no unported waiter/worker execution.
#[derive(Debug)]
struct Signal {
    _ready: Mutex<bool>,
    _changed: Condvar,
}
#[derive(Debug)]
struct Sai {
    signal: Box<Signal>,
    initialization: Initialization,
    // The constructor, loading observer and startup coordinator share this
    // actual manager. No active-byte/list snapshot is copied back later.
    engine: aa_mind::sai::manager::Sai,
    players: crate::load_sai_players::Arena,
    world_revisions: std::cell::RefCell<crate::world_revision::Counter>,
    frame_control: crate::live_sim::FrameControl,
    network: crate::standalone_network::ApplicationNetwork,
    game_spy_gate: crate::game_spy_manager_gate::State,
    load_diagnostics: crate::load_diagnostics::State,
    presentation_timing: crate::presentation_timing::State,
    heartbeat_counter: crate::heartbeat_process::Counter,
}

/// No default present/absent assumption. A default scope is Unknown and cannot
/// issue an InactiveLoad. This owner is non-Clone and never adopts an existing
/// world or copies a caller's guessed observer flag.
#[derive(Debug, Default)]
pub struct ScopedManagedLoad {
    initialized: Option<Sai>,
}

impl ScopedManagedLoad {
    ///776580 reads the SAME source-created SAI profile manager before a World
    ///or AIPlayer slot exists. The loan cannot replace/clone the Arena, which
    ///moves unchanged into InactiveLoad and StartupOwner at task handoff.
    pub fn with_source_profiles<T>(
        &mut self,
        catalog: &crate::catalog::Catalog<'_>,
        bank: &crate::definitions::Bank,
        read: impl FnOnce(&mut dyn crate::load_sai_players::ProfileService) -> Result<T>,
    ) -> Result<T> {
        self.with_source_profiles_and_game_spy(catalog,bank,|profiles,_|read(profiles))
    }
    ///Both are siblings of this SAME application owner. A caller cannot
    ///substitute a detached copied Internet flag while mutably loaning profiles.
    pub fn with_source_profiles_and_game_spy<T>(
        &mut self,
        catalog:&crate::catalog::Catalog<'_>,
        bank:&crate::definitions::Bank,
        read:impl FnOnce(&mut dyn crate::load_sai_players::ProfileService,&crate::game_spy_manager_gate::State)->Result<T>,
    )->Result<T> {
        self.with_source_profiles_game_spy_and_sync_flags(catalog,bank,|profiles,manager,_|read(profiles,manager))
    }
    ///77DA70/77DB10 read these SAME SAI+5C/+5D bytes only after actual
    ///simulation-thread equality. The immutable loan is not a cached flag copy.
    pub fn with_source_profiles_game_spy_and_sync_flags<T>(
        &mut self,
        catalog:&crate::catalog::Catalog<'_>,
        bank:&crate::definitions::Bank,
        read:impl FnOnce(&mut dyn crate::load_sai_players::ProfileService,&crate::game_spy_manager_gate::State,&[u8;3])->Result<T>,
    )->Result<T> {
        let state=self.initialized.as_mut().ok_or("fresh application is not initialized")?;
        let mut profiles=crate::load_sai_players::ProfileContext::new(catalog,bank,&mut state.players);
        read(&mut profiles,&state.game_spy_gate,&state.engine.bytes_5c)
    }
    /// Resolve the actual root UVars*.tgi set (419C76). This fresh scope does
    /// not import an existing original-game user profile. A second declaration
    /// of GoalPriorityCSV is explicitly unsupported until its override path is
    /// bound; it must not be ignored while accepting the base-file null value.
    pub fn from_sources(sources: &mut V14Sources, elapsed_millis: u32) -> Result<Self> {
        let mut names: Vec<_> = sources
            .catalog()
            .into_iter()
            .map(|entry| entry.logical_path)
            .filter(|name| {
                let lower = name.to_ascii_lowercase();
                !lower.contains(['/', '\\'])
                    && lower.starts_with("uvars")
                    && lower.ends_with(".tgi")
            })
            .collect();
        names.sort_by_key(|name| name.to_ascii_lowercase());
        let mut base = None;
        for name in names {
            let file = sources.read(&name).map_err(|e| e.to_string())?;
            if name.eq_ignore_ascii_case("UVars.tgi") {
                base = Some(file);
                continue;
            }
            let doc =
                Document::parse_native(file.bytes, Limits::default()).map_err(|e| e.to_string())?;
            for item in &doc.items {
                let Item::Node(node) = item else { continue };
                if !doc
                    .header_name(&node.header)
                    .is_some_and(|n| n.eq_ignore_ascii_case("Vars"))
                {
                    continue;
                }
                if node.header.tokens.len() != 1 {
                    return Err(format!(
                        "UVars node inheritance in {name} needs its native source owner"
                    ));
                }
                if node.items.iter().any(|item| matches!(item, Item::Property(property)
                    if doc.property_name(property).is_some_and(|n| n.eq_ignore_ascii_case("GoalPriorityCSV")))) {
                    return Err(format!("GoalPriorityCSV override in {name} needs the native override owner"));
                }
            }
        }
        Self::from_uvars(
            &base.ok_or("resolved UVars.tgi is missing")?,
            elapsed_millis,
        )
    }

    /// Initialize the bounded fresh-application observer and empty Sim owner.
    /// The actual resolved UVars source is required:7A5BF0 queries type4
    /// GoalPriorityCSV. Only the empty-file path is implemented; configured
    /// debug output is an error, not a silently omitted constructor callback.
    /// `elapsed_millis` is the application's actual 59ABA0 clock input, not
    /// simulation RNG. No existing game/profile/application is adopted here.
    pub fn from_uvars(file: &ResolvedFile, elapsed_millis: u32) -> Result<Self> {
        if !file.logical_path.eq_ignore_ascii_case("UVars.tgi") {
            return Err("fresh SAI initialization requires resolved UVars.tgi".into());
        }
        if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
            return Err("SAI UVars source hash differs from its bytes".into());
        }
        let doc = Document::parse_native(file.bytes.clone(), Limits::default())
            .map_err(|e| e.to_string())?;
        let mut values = Vec::new();
        for item in &doc.items {
            let Item::Node(node) = item else { continue };
            if !doc
                .header_name(&node.header)
                .is_some_and(|n| n.eq_ignore_ascii_case("Vars"))
            {
                continue;
            }
            if node.header.tokens.len() != 1 {
                return Err("UVars node inheritance needs its native source owner".into());
            }
            for item in &node.items {
                let Item::Property(property) = item else {
                    continue;
                };
                if !doc
                    .property_name(property)
                    .is_some_and(|n| n.eq_ignore_ascii_case("GoalPriorityCSV"))
                {
                    continue;
                }
                if property.left.len() != 2
                    || !doc
                        .text(property.left[0].span)
                        .is_some_and(|t| t.eq_ignore_ascii_case("string"))
                {
                    return Err(
                        "GoalPriorityCSV requires its assigned native string declaration".into(),
                    );
                }
                values.push(
                    doc.property_value(property)
                        .ok_or("GoalPriorityCSV has no assignment")?,
                );
            }
        }
        let [value] = values.as_slice() else {
            return Err("GoalPriorityCSV source is missing or ambiguous".into());
        };
        if !value.eq_ignore_ascii_case("null") && *value != "\"\"" {
            return Err("7A5BF0 nonempty GoalPriorityCSV initialization is unbound".into());
        }
        // Full constructor oracle proves85D8A0's encoding and the zero branch.
        let encoded = (elapsed_millis.wrapping_shl(16) | elapsed_millis) ^ 0x8e4b12a5;
        let initialization = Initialization {
            uvars_path: file.logical_path.clone(),
            uvars_sha256: file.sha256.clone(),
            elapsed_millis,
            private_seed: if encoded == 0 { 0x8e4b12a5 } else { encoded },
        };
        Ok(Self {
            initialized: Some(Sai {
                signal: Box::new(Signal {
                    _ready: Mutex::new(false),
                    _changed: Condvar::new(),
                }),
                initialization,
                engine: aa_mind::sai::manager::Sai::new(elapsed_millis),
                players: crate::load_sai_players::Arena::constructed(),
                world_revisions: std::cell::RefCell::new(crate::world_revision::Counter::fresh_application()),
                frame_control: crate::live_sim::FrameControl::fresh_application(),
                network: crate::standalone_network::ApplicationNetwork::fresh_application(),
                game_spy_gate: crate::game_spy_manager_gate::State::from_fresh_constructor_field_projection(),
                load_diagnostics: crate::load_diagnostics::State::from_fresh_application(),
                presentation_timing: crate::presentation_timing::State::from_native_constructor(),
                heartbeat_counter: crate::heartbeat_process::Counter::from_fresh_process(),
            }),
        })
    }

    /// Explicitly select the fresh local standalone service before any Internet
    ///manager. The single choice, child owners and even a failed prefix remain
    ///with this application through load/startup; this is not a supplied flag.
    pub fn select_local_offline(&mut self,catalog:&crate::catalog::Catalog<'_>,
        sources:&mut V14Sources,overrides:crate::load_view_variables::NativeOverrides,
        clock:&mut impl crate::standalone_network::Clock)->Result<()> {
        let app=self.initialized.as_mut().ok_or("fresh application is not initialized")?;
        if matches!(app.network,crate::standalone_network::ApplicationNetwork::Fresh) {
            app.game_spy_gate.select_standalone_prefix();
        }
        app.network.select_local(catalog,sources,overrides,clock)
    }
    pub fn game_spy_manager_gate(&self)->Result<&crate::game_spy_manager_gate::State> {
        Ok(&self.initialized.as_ref().ok_or("fresh application is not initialized")?.game_spy_gate)
    }
    pub fn standalone(&self)->Result<&crate::standalone_network::Standalone> {
        self.initialized.as_ref().ok_or("fresh application is not initialized")?.network.standalone()
    }

    /// Enter5A4280's load prefix, stopping at the selected MapResource load.
    /// The scope exclusively owns its fresh null Sim+3C;76AB80 therefore does
    /// no GWorld/SAIMap teardown. It cannot be used to skip a previous world.
    /// 4F7910's previous-load diagnostics are constructor-empty in this scope.
    /// No task-success flag, world-ready callback, or activation is synthesized.
    pub fn begin(self, task: TaskKind) -> Result<InactiveLoad> {
        let mut sai = self
            .initialized
            .ok_or("SAI lifecycle is Unknown: initialize a fresh managed application scope")?;
        if sai.engine.ready
            || !sai.engine.players_4c.is_empty()
            || !sai.engine.players_54.is_empty()
            || !sai.engine.players.is_empty()
            || !sai.engine.commands.is_empty()
            || !sai.engine.general_powers.is_empty()
            || !sai.engine.list_88.is_empty()
            || !sai.engine.vector_90.is_empty()
            || sai.engine.metagame
            || sai.engine.goal_fiber
            || sai.engine.rai_fiber
            || sai.engine.metagame_fiber
            || sai.engine.main_fiber
        {
            return Err(
                "existing/active SAI requires the real76AB80 old-world teardown owner".into(),
            );
        }
        sai.load_diagnostics.clear_4f7910();
        Ok(InactiveLoad {
            sai,
            task,
            valid: true,
        })
    }
}

/// Non-Clone capability for the interval after managed teardown and before
/// task completion/activation. Move it Attempt->Staged; Catalog only borrows it.
/// Private construction and Rust borrowing prevent a stale copied witness.
#[derive(Debug)]
pub struct InactiveLoad {
    sai: Sai,
    task: TaskKind,
    valid: bool,
}
impl InactiveLoad {
    ///Reached79F590→79EEC0 guard on the SAME constructor-owned slot vector.
    ///The caller proves the actual Kingdom's major index separately from its
    ///GTID. A nonempty vector requires real removal/trimming, even if all slots
    ///are null; inactivity alone never supplies an empty-player answer.
    pub(crate) fn require_empty_kingdom_removal_players(&self,kingdom:aa_actors::ActorId,major_index:i32)->Result<()> {
        self.check()?;
        if major_index<0 {return Err("79F590 was reached with a negative actual Kingdom major index".into());}
        if !self.sai.engine.players.is_empty() {
            return Err(format!("79EEC0 real SAI player removal/trimming required for Kingdom GTID{} major index{}",kingdom.raw(),major_index));
        }
        if self.sai.engine.ready {return Err("79F590 ready SAI region notification7BAB20 required".into());}
        Ok(())
    }
    ///Actual766960 appends local SAI players before7A0D40. This direct sibling
    ///loan retains the inactive interval; only initialize retires that interval.
    pub(crate) fn pre_start_parts(&mut self)->Result<(&mut aa_mind::sai::manager::Sai,&mut crate::load_sai_players::Arena)> {
        self.check()?;
        Ok((&mut self.sai.engine,&mut self.sai.players))
    }
    pub(crate) fn pre_start_parts_with_network(&mut self)->Result<(
        &mut aa_mind::sai::manager::Sai,&mut crate::load_sai_players::Arena,
        &crate::standalone_network::Standalone)> {
        self.check()?;
        Ok((&mut self.sai.engine,&mut self.sai.players,self.sai.network.standalone()?))
    }
    pub fn game_spy_manager_gate(&self)->&crate::game_spy_manager_gate::State {&self.sai.game_spy_gate}
    pub fn standalone(&self)->Result<&crate::standalone_network::Standalone> {self.sai.network.standalone()}
    pub(crate) fn construct_world_revision(&self)->Result<crate::world_revision::ConstructedWorldRevision> {
        self.check()?;
        Ok(self.sai.world_revisions.try_borrow_mut()
            .map_err(|_|"application World revision counter is already borrowed")?.construct())
    }
    fn check(&self) -> Result<()> {
        if !self.valid || self.sai.engine.ready {
            return Err("managed inactive SAI loading capability is no longer valid".into());
        }
        Ok(())
    }
    pub fn native_load_mode(&self) -> u32 {
        self.task.native_load_mode()
    }
    pub fn task(&self) -> TaskKind {
        self.task
    }
    pub fn initialization(&self) -> &Initialization {
        &self.sai.initialization
    }
    /// 5705C5 tests singleton presence, not its active byte. It is present.
    pub fn observer_present(&self) -> Result<bool> {
        self.check()?;
        Ok(true)
    }
    /// Read the actual SAI+20 owned by this load interval. A missing or
    /// invalidated interval must not be interpreted as an inactive observer.
    pub fn observer_active(&self) -> Result<bool> {
        self.check()?;
        Ok(self.sai.engine.ready)
    }
    /// 7A0C20 with+20=0 returns before even reading the actor argument. The
    /// original42-case oracle proves this exact no-op; no actor is inspected.
    pub fn notify_observer(&self, _actor: u32) -> Result<()> {
        self.check()
    }
    /// 775DB0 only reaches activation after successful task completion. Full
    /// 7757F0/7A0D40 is unbound: attempting to cross that boundary invalidates
    /// this loading capability and reports an error instead of enabling AI.
    pub fn request_world_ready(&mut self) -> Result<()> {
        self.valid = false;
        Err("7757F0/7A0D40 active SAI world-ready transition is unbound".into())
    }
    /// Lease the sole constructor-owned manager to the real7A0D40 coordinator.
    /// Invalidate this inactive interval before the first fallible effect;
    /// partial mutations stay in the manager on error.
    pub fn with_startup_engine<T>(
        &mut self,
        start: impl FnOnce(&mut aa_mind::sai::manager::Sai) -> Result<T>,
    ) -> Result<T> {
        self.check()?;
        self.valid = false;
        start(&mut self.sai.engine)
    }
    /// Disjoint loans of the SAME constructor-owned engine and AI allocations.
    /// Inactivation retires before any fallible startup/setup side effect.
    pub fn with_startup_engine_and_players<T>(
        &mut self,
        start: impl FnOnce(&mut aa_mind::sai::manager::Sai,&mut crate::load_sai_players::Arena)->Result<T>,
    )->Result<T> {
        self.check()?;
        self.valid=false;
        start(&mut self.sai.engine,&mut self.sai.players)
    }
    /// Move the manager after leaving the loading interval. This transition
    /// certifies neither a successful startup nor task/application readiness.
    /// A failed coordinator retains its actual partially mutated owner too.
    pub fn into_startup_owner(self) -> std::result::Result<StartupOwner, Self> {
        if self.valid { return Err(self); }
        Ok(StartupOwner { sai: self.sai })
    }
    /// Useful for checking that transfers keep the actual synchronization
    /// owner; it is neither a native pointer nor an observer identity.
    #[doc(hidden)]
    pub fn synchronization_owner_address(&self) -> usize {
        self.sai.signal.as_ref() as *const Signal as usize
    }
}

/// The started application's SAI manager ([B36528]) and its player
/// allocations, MOVED into a temporary Staged for one synchronous live
/// creation loan and moved back afterwards (`Retained::with_staged`).
#[derive(Debug)]
pub struct LiveSai {
    pub engine: aa_mind::sai::manager::Sai,
    pub players: crate::load_sai_players::Arena,
}
impl LiveSai {
    /// Constructor-shaped placeholders left in the application slot while the
    /// real owners are lent; nothing reads them before they are swapped back.
    pub(crate) fn placeholders() -> Self {
        Self { engine: aa_mind::sai::manager::Sai::new(0), players: crate::load_sai_players::Arena::constructed() }
    }
    /// 79EA90: whether a nonnull [SAI+60] slot's player has +8 == owner.
    /// Without a nonnull slot this is NULL without reading the arena.
    pub(crate) fn has_player_for(&self, _owner: u32) -> std::result::Result<bool, String> {
        if self.engine.players.iter().flatten().next().is_none() {
            return Ok(false);
        }
        Err("79EA90 positive SAI player slot kingdom lookup is not bound".into())
    }
}
/// The same application SAI after its inactive load capability was retired.
/// No Clone/Default and no readiness constructor; later native services borrow
/// its real fields, including the state retained by a failed startup.
#[derive(Debug)]
pub struct StartupOwner { sai: Sai }
impl StartupOwner {
    pub(crate) fn success_warning_text(&self)->Result<Vec<u16>> {self.sai.load_diagnostics.success_text_4f7930()}
    pub fn game_spy_manager_gate(&self)->&crate::game_spy_manager_gate::State {&self.sai.game_spy_gate}
    pub fn standalone(&self)->Result<&crate::standalone_network::Standalone> {self.sai.network.standalone()}
    /// Same process counter/latch owner, never recreated for a mission/frame.
    pub fn frame_control(&self)->&crate::live_sim::FrameControl {&self.sai.frame_control}
    pub fn frame_control_mut(&mut self)->&mut crate::live_sim::FrameControl {&mut self.sai.frame_control}
    ///Direct disjoint loans of the retained application statistics/counter.
    ///This provides no World/task/application activation capability.
    pub fn process_effects(&mut self)->(&mut crate::presentation_timing::State,&mut crate::heartbeat_process::Counter){
        (&mut self.sai.presentation_timing,&mut self.sai.heartbeat_counter)
    }
    ///Disjoint SAME application fields used during a native frame. The
    ///admitted world/command callbacks cannot mutate these held logger flags.
    pub(crate) fn frame_parts(&mut self)->(&mut crate::live_sim::FrameControl,&crate::game_spy_manager_gate::State,&[u8;3]){
        (&mut self.sai.frame_control,&self.sai.game_spy_gate,&self.sai.engine.bytes_5c)
    }
    ///ONE split of the SAME application owners. The counter and timing stats
    ///move through loading with Sai; neither is reconstructed for a frame.
    pub(crate) fn frame_all_parts(&mut self)->(
        &mut crate::live_sim::FrameControl,&crate::game_spy_manager_gate::State,
        &mut crate::presentation_timing::State,&mut crate::heartbeat_process::Counter,
        &crate::standalone_network::ApplicationNetwork,
        &mut aa_mind::sai::manager::Sai,&mut crate::load_sai_players::Arena,
    ){
        (&mut self.sai.frame_control,&self.sai.game_spy_gate,
            &mut self.sai.presentation_timing,&mut self.sai.heartbeat_counter,&self.sai.network,
            &mut self.sai.engine,&mut self.sai.players)
    }
    pub fn engine(&self) -> &aa_mind::sai::manager::Sai { &self.sai.engine }
    pub fn engine_mut(&mut self) -> &mut aa_mind::sai::manager::Sai { &mut self.sai.engine }
    pub fn with_engine_and_players<T>(&mut self,
        f:impl FnOnce(&mut aa_mind::sai::manager::Sai,&mut crate::load_sai_players::Arena)->T)->T {
        f(&mut self.sai.engine,&mut self.sai.players)
    }
    pub fn players(&self)->&crate::load_sai_players::Arena {&self.sai.players}
    pub fn initialization(&self) -> &Initialization { &self.sai.initialization }
    #[doc(hidden)]
    pub fn synchronization_owner_address(&self) -> usize {
        self.sai.signal.as_ref() as *const Signal as usize
    }
}
