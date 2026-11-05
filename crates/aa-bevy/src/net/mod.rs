//! Multiplayer in the client: MainMenu -> `MultiplayerMenu` (`0x653780`) ->
//! `LobbyMenu` in LAN mode (`0x64AB20`, `0x648270`) -> `CreateGameDialog`
//! (`0x6EBC50`) or `JoinDirectDialog` (`0x6EC6D0`) -> the staging screen of a
//! networked session -> a battle run by `aa_netcode::lan::LanNode`.
//!
//! The session is the ported admin layer; this module is its front end, as
//! `ui/custom.rs` is the front end of an offline session. `Custom` stays the
//! staging screen's view of the session (`Custom::net_*`): on every node it is
//! rebuilt from `aa_netcode::lobby::Session`, so what a person sees and what a
//! launch builds are the replicated session's.
//!
//! The datagrams of a session go through `aa_netcode::udp::Transport`: UDP here
//! for a LAN and a direct address, and [`Lan::host_with`] / [`Lan::join_with`]
//! take any other path. Everything above it is the same.
pub mod data;
#[cfg(feature = "eos")]
pub mod eos;
#[cfg(not(feature = "eos"))]
#[path = "eos_off.rs"]
pub mod eos;
pub mod game;

use crate::session::Battle;
use crate::ui::custom::{Custom, NetRow, NetSource};
use aa_netcode::lan::registries::{self, NativeRegistries};
use aa_netcode::lan::{self, browse, descriptor as d, HostOptions, LanConfig, LanData, LanEvent, LanNode, NoWorld};
use aa_netcode::lobby::{self, LaunchSettings, Session};
use aa_netcode::session::PlayerId;
use aa_netcode::udp::Transport;
use aa_orders::payload::Field;
use aa_play::player_orders::native::{Outbox, Replay, Tables};
use aa_ui::staging::{Occupant, StagingState};
use aa_ui::ui::Ui;
use aa_ui::widget::{Widget, WidgetClass as C};
use bevy::log::{info, warn};
use data::{text, units, Data};
use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// The node of the session, shared between the front end and `step_battle`.
pub type SharedNode = Arc<Mutex<Option<LanNode>>>;

const LOBBY: &str = "/UI/Menus/lobby.tgi:LobbyMenu";
const MULTIPLAYER: &str = "/UI/Menus/Multiplayer.tgi:MultiplayerMenu";
const CREATE: &str = "/UI/Menus/create_game_dialog.tgi:CreateGameDialog";
const JOIN: &str = "/UI/Menus/join_direct_dialog.tgi:JoinDirectDialog";
const CREATE_ROOT: &str = "Root/CreateGameDialog";
const JOIN_ROOT: &str = "Root/JoinDirectDialog";

/// `0x653780` MultiplayerMenu (the menu's slot, Init with its three buttons).
pub fn multiplayer_menu(ui: &mut Ui) {
    ui.clear();
    aa_ui::screens::menu_backdrop(ui);
    let mut m = Widget::new(C::TopLevel, MULTIPLAYER, true);
    for (n, c) in [("MultiplayerMenuButtonLAN", "RequestLobbyLAN"), ("MultiplayerMenuButtonInternet", "RequestLobbyInternet"), ("MultiplayerMenuButtonMain", "RequestMainMenu")] {
        m.add_child(Widget::push_button(n, true, c));
    }
    aa_ui::screens::show(ui, m);
    ui.layout();
}

/// `0x64AB20` LobbyMenu in LAN mode: the children of Init `0x648270` this mode
/// shows (`0x647F00`, no-chat `0x647DC0(3)`: the browser takes the `full_` bounds
/// of the file, ChatPanel and the room list stay hidden).
pub fn lobby_menu(ui: &mut Ui) {
    ui.clear();
    aa_ui::screens::menu_backdrop(ui);
    let mut m = Widget::new(C::Interface, LOBBY, false);
    let mut logo = Widget::new(C::Decal, "GameSpyLogoDecal", false);
    logo.set_enabled(false);
    m.add_child(logo);
    let mut browser = Widget::new(C::Panel, "ServerBrowserPanel", false);
    browser.add_child(Widget::composite(C::Listbox, "ServerListBox", false));
    for header in ["PasswordButton", "ServerNameButton", "PlayersButton", "PingButton", "MapButton"] {
        browser.add_child(Widget::toggle_button(header, false, ""));
    }
    m.add_child(browser);
    m.add_child(Widget::push_button("RefreshButton", false, "LobbyRefreshServerList"));
    let mut server_info = Widget::push_button("ServerInfoButton", false, "");
    server_info.set_enabled(false);
    m.add_child(server_info);
    let mut lan = Widget::new(C::Panel, "LANButtonsPanel", false);
    for (n, c) in [("CreateButton", "LobbyHost"), ("JoinButton", "LobbyJoin"), ("JoinDirectButton", "LobbyJoinDirect"), ("OptionsButton", "OpenOptionsMenu"), ("ExitButton", "LobbyConfirmExit")] {
        lan.add_child(Widget::push_button(n, false, c));
    }
    m.add_child(lan);
    aa_ui::screens::show(ui, m);
    // nothing to join yet: LobbyJoin 0x64A280 needs a selected game
    ui.with_widget("LobbyMenu/LANButtonsPanel/JoinButton", |w| w.set_enabled(false));
    // 0x647DC0(3): the bounds [full_L .. full_B], whose size is 719 x 662
    if let Some((w, h)) = ui.store.node(LOBBY).ok().and_then(|n| Some((full_size(&n, "full_R")?, full_size(&n, "full_B")?))) {
        ui.with_widget("LobbyMenu/ServerBrowserPanel", |p| aa_ui::geometry::set_size(p, w, h));
    }
    ui.layout();
}

/// `string full_R = +719` of the lobby's file, as a size.
fn full_size(node: &aa_ui::node::UiNode, key: &str) -> Option<f32> {
    node.string_or_empty(key).trim().trim_start_matches('+').parse().ok()
}

/// What the module asks of the front end.
pub enum Action {
    /// Keep the page (the module changed widgets).
    None,
    /// Show the staging screen of the joined or hosted session.
    Staging,
    /// Back to the lobby (a failed join, a left session).
    Lobby,
    /// A launch's world is to be built (`LanEvent::LoadWorld`).
    LoadWorld,
    /// Every peer finished its task: the battle runs.
    BeginPlay,
    /// The session ended while a battle ran: its score screen.
    SessionLost,
}

#[derive(Default)]
struct Warm {
    data: Option<Arc<Data>>,
}

/// Makes the path of a join again: a game with a password asks, and the next
/// attempt is a new connection.
pub type TransportMaker = Box<dyn FnMut() -> aa_netcode::Result<Box<dyn Transport>> + Send + Sync>;

/// The address the person typed, being resolved (`0x6EC460`, `0x7877E0`).
struct Resolving {
    label: String,
    /// `None` until the lookup answered.
    answer: Arc<Mutex<Option<Option<SocketAddr>>>>,
}

/// The wait for a session (`0x64A350`: the box with its timeout).
struct Joining {
    /// The game as the box names it.
    label: String,
    addr: SocketAddr,
    /// UDP unless the path is another (`Lan::join_with`).
    maker: Option<TransportMaker>,
    started: std::time::Instant,
    /// UVar JoinTimeout.
    seconds: i32,
    shown: i32,
    /// The password of this attempt.
    password: String,
}

/// The front end's side of a LAN session.
pub struct Lan {
    pub node: SharedNode,
    events: Arc<Mutex<Vec<LanEvent>>>,
    root: PathBuf,
    data: Arc<Mutex<Warm>>,
    warming: bool,
    /// The lobby's dialog on show.
    dialog: Option<&'static str>,
    /// The staging screen shows this session (rebuild on change).
    dirty: bool,
    /// The source the host's `Custom` was last synchronized to.
    source: Option<NetSource>,
    /// A battle is linked to the node.
    pub in_battle: bool,
    pub local_name: String,
    pub own_ready: bool,
    /// The file the digest of every world tick goes to (`AA_LAN_DIGEST`).
    digest_path: Option<PathBuf>,
    resolving: Option<Resolving>,
    joining: Option<Joining>,
    /// The LAN list, while the lobby is open.
    browser: Option<browse::Browser>,
    /// A password box waits for a session that refused (`Joining` stays).
    asking_password: bool,
    /// The Internet button's side (`eos.rs`).
    online: eos::Online,
}

/// `Lan` needs `Send` for the front end's resource: everything it holds is.
struct ArcData(Arc<Data>);
impl LanData for ArcData {
    fn registries(&self) -> &NativeRegistries {
        self.0.registries()
    }
    fn random_map_config(&self, key: &[u16]) -> Option<Vec<u16>> {
        self.0.random_map_config(key)
    }
    fn scenario(&self, key: &[u16]) -> Option<Vec<u16>> {
        self.0.scenario(key)
    }
    fn format(&self, key: &str, args: &[Vec<u16>]) -> Vec<u16> {
        self.0.format(key, args)
    }
    fn countdown_time(&self) -> i32 {
        self.0.countdown_time()
    }
    fn nation_factions(&self, nation: &[u16]) -> aa_netcode::Result<Vec<Vec<u16>>> {
        self.0.nation_factions(nation)
    }
    fn speed_settings(&self) -> aa_runtime::simulation::SpeedSettings {
        self.0.speed_settings()
    }
    fn kingdom_open(&self, session: &Session, record: usize) -> Option<bool> {
        self.0.kingdom_open(session, record)
    }
    fn rmc_header(&self, rmc: &[u16]) -> Option<lobby::MapHeader> {
        self.0.rmc_header(rmc)
    }
}

/// [0xB36530]+8 of the stock v14 data, as the original sent it in a Connect
/// (research/phase8-net/README.txt section 13). The function that computes it
/// is not ported: other data has another value, and an original peer refuses it.
const STOCK_DATA_CHECKSUM: u32 = 170_871_826;

/// 0x5AF810: `"%s %x"` of each depot in load order, the word at +26 of an archive
/// header, 0 for the loose folder (as the original sent it: `Data.rwd e1e2d0c9
/// data/ 0 Music.rwd 2e37de23`).
fn depot_checksums(game_root: &std::path::Path) -> String {
    let word = |name: &str| -> u32 {
        use std::io::Read;
        let mut header = [0u8; 30];
        match std::fs::File::open(game_root.join(name)).and_then(|mut f| f.read_exact(&mut header)) {
            Ok(()) => u32::from_le_bytes([header[26], header[27], header[28], header[29]]),
            Err(_) => 0,
        }
    };
    format!("Data.rwd {:x} data/ 0 Music.rwd {:x}", word("Data.rwd"), word("Music.rwd"))
}

fn player_serial() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1)
}

impl Lan {
    pub fn new(root: PathBuf) -> Self {
        Self {
            node: Default::default(),
            events: Default::default(),
            root,
            data: Default::default(),
            warming: false,
            dialog: None,
            dirty: false,
            source: None,
            in_battle: false,
            local_name: String::new(),
            own_ready: false,
            digest_path: std::env::var_os("AA_LAN_DIGEST").map(PathBuf::from),
            resolving: None,
            joining: None,
            browser: None,
            asking_password: false,
            online: Default::default(),
        }
    }

    /// A session exists.
    pub fn active(&self) -> bool {
        self.node.lock().is_ok_and(|n| n.is_some())
    }

    fn game_root(&self) -> PathBuf {
        self.root.parent().map_or_else(|| self.root.clone(), PathBuf::from)
    }

    /// The registries and maps of a session, in the background from the moment
    /// the multiplayer menu opens (`RequestMultiplayer`).
    pub fn warm(&mut self, ui: &Ui, custom: &mut Custom) {
        if self.warming {
            return;
        }
        self.warming = true;
        let root = self.game_root();
        let slot = self.data.clone();
        let store = ui.store.root.clone();
        let strings = ui.strings.clone();
        let countdown = ui.avars.get("LaunchCountdownTime").and_then(|v| aa_ui::node::unquote(v).trim().parse().ok()).unwrap_or(5);
        let game = data::GameParts::of(custom.game_data(ui));
        std::thread::spawn(move || {
            let cache = crate::session::own_profile().map(|p| p.join("lan-registries.json"));
            let reg = match NativeRegistries::load_v14(&root, cache.as_deref()) {
                Ok(r) => r,
                Err(e) => {
                    warn!("lan: the registries did not load: {e}");
                    return;
                }
            };
            let maps: Vec<_> = crate::ui::gamedata::list_maps(&store).iter().filter_map(|p| crate::ui::gamedata::read_map(&store, p)).collect();
            let rules = crate::preload::game_data(&root).ok();
            let rule = |name: &str, default: f32| rules.as_ref().and_then(|d| d.rule_ci(name)).unwrap_or(default);
            let data = Data::new(reg, game, &maps, strings, countdown, rule);
            if let Ok(mut s) = slot.lock() {
                s.data = Some(data);
            }
        });
    }

    fn wait_data(&self) -> Option<Arc<Data>> {
        for _ in 0..600 {
            if let Ok(s) = self.data.lock()
                && let Some(d) = &s.data
            {
                return Some(d.clone());
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        None
    }

    fn config(&self, password: Vec<u16>) -> LanConfig {
        let version = aa_parity::recording::GameVersion::load(&self.game_root()).ok();
        let (v, b) = version.map_or((String::new(), String::new()), |g| (g.version, g.application));
        LanConfig {
            name: units(&self.local_name),
            password,
            version: units(&v),
            build: units(&b),
            data_checksum: STOCK_DATA_CHECKSUM,
            depot_checksums: units(&depot_checksums(&self.game_root())),
            mods: Vec::new(),
            player_id: PlayerId { name: units(&self.local_name), serial: player_serial(), a: 0, b: 0, c: 0 },
        }
    }

    /// A lobby command; `None` when it is not one of the lobby's.
    pub fn lobby_command(&mut self, ui: &mut Ui, custom: &mut Custom, cmd: &str) -> Option<Action> {
        // the Internet button and its lobby (`eos.rs`)
        if let Some(action) = self.online_command(ui, cmd) {
            return Some(action);
        }
        match cmd {
            "RequestLobbyLAN" => {
                self.dialog = None;
                return Some(Action::Lobby);
            }
            // 0x64A6E0: ui_lobby_confirm_exit with LobbyExit
            "LobbyConfirmExit" => {
                let text = ui.strings.resolve("#ui_lobby_confirm_exit");
                let (ok, cancel) = (ui.strings.resolve("#button_ok"), ui.strings.resolve("#button_cancel"));
                ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(ok, "LobbyExit".into()), (cancel, String::new())] });
                return Some(Action::None);
            }
            // 0x64A6E0 -> LobbyExit: the lobby closes, and its list
            "LobbyExit" => {
                self.browser = None;
                return None;
            }
            // 0x6F40C0: the list starts again
            "LobbyRefreshServerList" => {
                ui.listbox_clear(SERVER_LIST);
                ui.with_widget("LobbyMenu/LANButtonsPanel/JoinButton", |w| w.set_enabled(false));
                if let Some(b) = &mut self.browser {
                    b.refresh();
                }
                return Some(Action::None);
            }
            // 0x64A280: the selected game; 0x64A280 asks for one without a selection
            "LobbyJoin" => return Some(self.join_selected(ui)),
            // 0x75D460 RequestCancelJoin, the box's button and its timeout
            "LobbyCancelJoin" => {
                self.cancel_join();
                return Some(Action::None);
            }
            // 0x414C84: the password box's answer
            c if c.starts_with("ServerPassword ") => {
                let password = c["ServerPassword ".len()..].trim().trim_matches('"').to_string();
                self.asking_password = false;
                return Some(self.retry_with_password(ui, password));
            }
            // 0x6471B0: CreateGameDialog
            "LobbyHost" => {
                self.open_create(ui);
                return Some(Action::None);
            }
            "LobbyJoinDirect" => {
                self.open_join(ui);
                return Some(Action::None);
            }
            "CreateGameCancel" => {
                self.close_dialog(ui);
                return Some(Action::None);
            }
            // 0x6EC640 (the box of 0x6EC460 too): the dialog closes, and the
            // answer of the name's lookup finds nobody waiting
            "JoinDirectCancel" => {
                self.resolving = None;
                self.close_dialog(ui);
                return Some(Action::None);
            }
            "CreateGameCreate" => return Some(self.create(ui, custom)),
            "JoinDirectJoin" => return Some(self.join(ui)),
            _ => {}
        }
        None
    }

    fn close_dialog(&mut self, ui: &mut Ui) {
        if let Some(name) = self.dialog.take() {
            ui.disconnect(name);
            ui.layout();
        }
    }

    /// `0x6EAFD0` CreateGameDialog: name, password, maximum players, observers.
    fn open_create(&mut self, ui: &mut Ui) {
        self.close_dialog(ui);
        let mut d = Widget::new(C::TopLevel, CREATE, true);
        for label in ["GameNameLabel", "PasswordLabel", "MaxPlayersLabel", "AllowObserversLabel"] {
            d.add_child(Widget::new(C::Label, label, false));
        }
        d.add_child(Widget::composite(C::Editbox, "GameNameEditbox", false));
        d.add_child(Widget::composite(C::Editbox, "PasswordEditbox", false));
        d.add_child(Widget::composite(C::DropDown, "MaxPlayersDropDown", false));
        d.add_child(Widget::toggle_button("AllowObserversToggle", false, ""));
        d.add_child(Widget::push_button("CreateButton", false, "CreateGameCreate"));
        d.add_child(Widget::push_button("CancelButton", false, "CreateGameCancel"));
        aa_ui::screens::show(ui, d);
        self.dialog = Some("CreateGameDialog");
        // 0x6EB530: ui_lobby_default_game_name with the profile's nickname
        let name = ui.strings.format("ui_lobby_default_game_name|s", &[&self.local_name]);
        ui.editbox_set_text(&format!("{CREATE_ROOT}/GameNameEditbox"), &name);
        let list = format!("{CREATE_ROOT}/MaxPlayersDropDown");
        let lo = ui.avars.float("MultiplayerMinNumPlayers").unwrap_or(2.0) as i32;
        let hi = ui.avars.float("MultiplayerMaxNumPlayers").unwrap_or(12.0) as i32;
        for n in lo..=hi {
            ui.listbox_add(&format!("{list}/Listbox"), &n.to_string(), &n.to_string());
        }
        ui.dropdown_select(&list, (hi - lo).max(0) as usize);
        ui.layout();
    }

    /// `0x6EC060` JoinDirectDialog: address and port (HostPortDefault).
    fn open_join(&mut self, ui: &mut Ui) {
        self.close_dialog(ui);
        let mut d = Widget::new(C::TopLevel, JOIN, true);
        d.add_child(Widget::new(C::Label, "IPLabel", false));
        d.add_child(Widget::composite(C::Editbox, "IPEditbox", false));
        d.add_child(Widget::new(C::Label, "PortLabel", false));
        d.add_child(Widget::composite(C::Editbox, "PortEditbox", false));
        d.add_child(Widget::push_button("JoinButton", false, "JoinDirectJoin"));
        d.add_child(Widget::push_button("CancelButton", false, "JoinDirectCancel"));
        aa_ui::screens::show(ui, d);
        self.dialog = Some("JoinDirectDialog");
        let port = ui.avars.float("HostPortDefault").map_or(lan::HOST_PORT_DEFAULT, |p| p as u16);
        ui.editbox_set_text(&format!("{JOIN_ROOT}/PortEditbox"), &port.to_string());
        ui.layout();
    }

    /// `0x6EBAB0` -> `0x6EB960` -> `0x651720` (TGC_AdminNode::Host 0x75AEE0): the
    /// session staging the multiplayer default (RestoreLaunchSettings 0x77A000,
    /// fresh profile): RMC `StagingMultiPlayerDefaultRMCIds`, its size and team
    /// format.
    fn create(&mut self, ui: &mut Ui, custom: &mut Custom) -> Action {
        let Some(data) = self.wait_data() else {
            warn!("lan: no session data");
            return Action::None;
        };
        let name = ui.editbox_text(&format!("{CREATE_ROOT}/GameNameEditbox")).unwrap_or_default();
        let password = ui.editbox_text(&format!("{CREATE_ROOT}/PasswordEditbox")).unwrap_or_default();
        let list = format!("{CREATE_ROOT}/MaxPlayersDropDown");
        let max = ui.listbox_selected(&format!("{list}/Listbox")).and_then(|s| ui.listbox_entries(&format!("{list}/Listbox")).get(s).and_then(|e| e.param.parse().ok())).unwrap_or(ui.avars.float("MultiplayerMaxNumPlayers").unwrap_or(12.0) as i32);
        let observers = ui.widget(&format!("{CREATE_ROOT}/AllowObserversToggle")).is_some_and(|t| t.flags & aa_ui::state::widget_flag::SELECTED != 0);
        self.close_dialog(ui);
        let rmc = ui.avars.get("StagingMultiPlayerDefaultRMCIds").map(aa_ui::node::unquote).unwrap_or_default();
        let w = ui.avars.float("StagingMultiPlayerDefaultRMCWidth").unwrap_or(384.0);
        let h = ui.avars.float("StagingMultiPlayerDefaultRMCHeight").unwrap_or(384.0);
        let format = ui.avars.get("StagingMultiPlayerDefaultRMCTeamConfig").map(aa_ui::node::unquote).unwrap_or_default();
        let mut launch = LaunchSettings { mode: 0, game_type: 2, field_0c: 1, random_map: Some(units(&rmc)), ..Default::default() };
        launch.map_random.header = data.rmc_header(&units(&rmc)).unwrap_or_default();
        launch.map_random.size_x = w as u32;
        launch.map_random.size_y = h as u32;
        launch.state.field_10 = 1;
        launch.state.field_18 = 4;
        launch.state.field_1c = 1;
        let port = ui.avars.float("HostPortDefault").map_or(lan::HOST_PORT_DEFAULT, |p| p as u16);
        let tries = ui.avars.float("NetworkNumPortsToTry").map_or(lan::PORTS_TO_TRY, |p| p as u16);
        // 0.0.0.0 natively; AA_LAN_BIND keeps a test on this machine
        let bind = std::env::var("AA_LAN_BIND").ok().and_then(|b| b.parse().ok()).unwrap_or(Ipv4Addr::UNSPECIFIED);
        let options = HostOptions {
            game_name: units(&name),
            password: units(&password),
            max_players: max,
            allow_observers: observers,
            port,
            ports_to_try: tries,
            bind,
        };
        let config = self.config(Vec::new());
        match LanNode::host(&config, &options, launch, Box::new(ArcData(data.clone()))) {
            Ok(node) => self.hosted(node, &format),
            Err(e) => {
                // 0x75B00B: no port of the range bound
                let text = ui.strings.format("networking_unable_to_bind|d", &[&port.to_string()]);
                warn!("lan: host failed: {e}");
                box_ok(ui, text);
                Action::None
            }
        }
    }

    fn hosted(&mut self, mut node: LanNode, format: &str) -> Action {
        info!("lan: hosting on {:?}", node.local_addr());
        if !format.is_empty() {
            let _ = node.request(d::SET_TEAMS_REQUEST, &[Field::Text(units(format))]);
        }
        if let Ok(mut n) = self.node.lock() {
            *n = Some(node);
        }
        self.browser = None;
        self.dirty = true;
        self.source = None;
        Action::Staging
    }

    /// The name and password of a host on another path come from that path's
    /// lobby; everything else is [`Self::create`]'s. The staging opens on the
    /// action this returns (`Front::lan_action`, `Action::Staging`).
    pub fn host_with(&mut self, ui: &mut Ui, transport: Box<dyn Transport>, game_name: &str, password: &str, max_players: i32, allow_observers: bool) -> Result<Action, String> {
        let data = self.wait_data().ok_or("no session data")?;
        let rmc = ui.avars.get("StagingMultiPlayerDefaultRMCIds").map(aa_ui::node::unquote).unwrap_or_default();
        let w = ui.avars.float("StagingMultiPlayerDefaultRMCWidth").unwrap_or(384.0);
        let h = ui.avars.float("StagingMultiPlayerDefaultRMCHeight").unwrap_or(384.0);
        let format = ui.avars.get("StagingMultiPlayerDefaultRMCTeamConfig").map(aa_ui::node::unquote).unwrap_or_default();
        let mut launch = LaunchSettings { mode: 0, game_type: 2, field_0c: 1, random_map: Some(units(&rmc)), ..Default::default() };
        launch.map_random.header = data.rmc_header(&units(&rmc)).unwrap_or_default();
        launch.map_random.size_x = w as u32;
        launch.map_random.size_y = h as u32;
        launch.state.field_10 = 1;
        launch.state.field_18 = 4;
        launch.state.field_1c = 1;
        let options = HostOptions { game_name: units(game_name), password: units(password), max_players, allow_observers, port: 0, ports_to_try: 0, bind: Ipv4Addr::UNSPECIFIED };
        let config = self.config(Vec::new());
        let node = LanNode::host_on(transport, &config, &options, launch, Box::new(ArcData(data))).map_err(|e| e.to_string())?;
        Ok(self.hosted(node, &format))
    }

    /// `0x6EC460`: the box "Resolving address for %s" while the address is looked
    /// up (`0x7877E0` on GameSpy's thread); `0x6EC8B0` has the answer in [`Self::pump`].
    fn join(&mut self, ui: &mut Ui) -> Action {
        let address = ui.editbox_text(&format!("{JOIN_ROOT}/IPEditbox")).unwrap_or_default().trim().to_string();
        let port = ui.editbox_text(&format!("{JOIN_ROOT}/PortEditbox")).and_then(|p| p.trim().parse::<u16>().ok()).unwrap_or(lan::HOST_PORT_DEFAULT);
        self.close_dialog(ui);
        let text = ui.strings.format("ui_lobby_join_direct_resolve|s", &[&address]);
        let cancel = ui.strings.resolve("#button_cancel");
        ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(cancel, "JoinDirectCancel".into())] });
        let answer: Arc<Mutex<Option<Option<SocketAddr>>>> = Default::default();
        let tell = answer.clone();
        let name = address.clone();
        std::thread::spawn(move || {
            // The network layer is IPv4 (the original's 0x781F30).
            let found = (name.as_str(), port).to_socket_addrs().ok().and_then(|mut a| a.find(SocketAddr::is_ipv4));
            if let Ok(mut t) = tell.lock() {
                *t = Some(found);
            }
        });
        self.resolving = Some(Resolving { label: address, answer });
        Action::None
    }

    /// `0x64A280`: the selected game of the list (`0x6523C0`). The game's name
    /// goes in the box.
    fn join_selected(&mut self, ui: &mut Ui) -> Action {
        let list = self.browser.as_ref().map(|b| b.games.clone()).unwrap_or_default();
        let entries = ui.listbox_entries(SERVER_LIST);
        let Some(game) = ui.listbox_selected(SERVER_LIST).and_then(|s| entries.get(s)).and_then(|e| list.iter().find(|g| g.address.to_string() == e.param)).cloned() else {
            let text = ui.strings.resolve("#ui_lobby_select_server");
            box_ok(ui, text);
            return Action::None;
        };
        self.start_join(ui, game.address, game.info.name, None, String::new())
    }

    /// `0x651A10` / `0x6523C0`: TGC_AdminNode::RequestJoin 0x75D390 and the box
    /// `0x64A350`.
    fn start_join(&mut self, ui: &mut Ui, addr: SocketAddr, label: String, mut maker: Option<TransportMaker>, password: String) -> Action {
        let Some(data) = self.wait_data() else {
            warn!("lan: no session data");
            return Action::None;
        };
        let config = self.config(units(&password));
        let node = match maker.as_mut() {
            Some(make) => make().and_then(|t| LanNode::join_on(t, &config, addr, Box::new(ArcData(data)))),
            None => LanNode::join(&config, addr, Box::new(ArcData(data))),
        };
        match node {
            Ok(node) => {
                info!("lan: joining {addr}");
                if let Ok(mut n) = self.node.lock() {
                    *n = Some(node);
                }
                self.browser = None;
                self.dirty = true;
                self.source = None;
                let seconds = ui.store.node("UVars.tgi:Vars").ok().and_then(|n| n.int("JoinTimeout").ok()).unwrap_or(60);
                let joining = Joining { label, addr, maker, started: std::time::Instant::now(), seconds, shown: seconds, password };
                let text = joining_text(ui, &joining.label, seconds);
                let cancel = ui.strings.resolve("#button_cancel");
                ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(cancel, "LobbyCancelJoin".into())] });
                self.joining = Some(joining);
            }
            Err(e) => {
                warn!("lan: join failed: {e}");
                let text = ui.strings.resolve("#ui_lobby_connect_failed");
                box_ok(ui, text);
            }
        }
        Action::None
    }

    /// A join on another path, with `label` in the box; the address is the host's
    /// on that path. `maker` makes the path, again for a second attempt when the
    /// game asked for a password.
    pub fn join_with(&mut self, ui: &mut Ui, maker: TransportMaker, addr: SocketAddr, label: String) -> Action {
        self.start_join(ui, addr, label, Some(maker), String::new())
    }

    /// 0x75D460: the node leaves, the box is gone by then.
    fn cancel_join(&mut self) {
        self.joining = None;
        self.asking_password = false;
        self.with_node(LanNode::leave);
        // nothing was joined: nothing is left to tell the host for long
        if let Ok(mut n) = self.node.lock()
            && n.as_ref().is_some_and(|node| !node.joined())
        {
            *n = None;
        }
    }

    fn retry_with_password(&mut self, ui: &mut Ui, password: String) -> Action {
        let Some(j) = self.joining.take() else { return Action::None };
        self.start_join(ui, j.addr, j.label, j.maker, password)
    }

    /// The lobby came up (Activate `0x6495F0`): the list asks (`0x6F40C0(1)`),
    /// and the first time, a nickname (`+0x200 = !profile+0x28`, `0x649475`).
    pub fn lobby_shown(&mut self, ui: &mut Ui, first_time: bool) {
        // AA_LAN_BROWSE=127.0.0.1 keeps a test on this machine
        let broadcast = match std::env::var("AA_LAN_BROWSE").ok().and_then(|b| b.parse::<Ipv4Addr>().ok()) {
            Some(ip) if ip.is_loopback() => None,
            Some(ip) => Some(ip),
            None => Some(Ipv4Addr::BROADCAST),
        };
        match browse::Browser::new(broadcast) {
            Ok(b) => self.browser = Some(b),
            Err(e) => warn!("lan: the game list: {e}"),
        }
        if first_time {
            let text = ui.strings.resolve("#ui_lobby_prompt_lan_nickname");
            let (ok, cancel) = (ui.strings.resolve("#button_ok"), ui.strings.resolve("#button_cancel"));
            let name = self.local_name.clone();
            let cmd = "LobbySetLANNickname \"%s\"".to_string();
            ui.show_prompt_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(ok, cmd), (cancel, String::new())] }, &name);
        }
    }

    /// One frame of the lobby: the list's answers, the address being looked up,
    /// the box of a join.
    fn lobby_frame(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        // Update 0x649380: Join only with a selected game (0x6F3930)
        if self.browser.is_some() {
            let selected = ui.listbox_selected(SERVER_LIST).is_some();
            let mut changed = false;
            ui.with_widget("LobbyMenu/LANButtonsPanel/JoinButton", |w| {
                if (w.flags & 4 != 0) != selected {
                    w.set_enabled(selected);
                    changed = true;
                }
            });
            if changed {
                ui.layout();
            }
        }
        if let Some(b) = &mut self.browser
            && b.pump()
        {
            let keep = ui.listbox_selected(SERVER_LIST).and_then(|s| ui.listbox_entries(SERVER_LIST).get(s).map(|e| e.param.clone()));
            ui.listbox_clear(SERVER_LIST);
            for (i, g) in b.games.iter().enumerate() {
                let row = format!("{}    {}/{}    {}    {}", g.info.name, g.info.players, g.info.max_players, g.ping_ms, g.info.map);
                ui.listbox_add(SERVER_LIST, &row, &g.address.to_string());
                if keep.as_deref() == Some(&*g.address.to_string()) {
                    ui.listbox_select(SERVER_LIST, i, false);
                }
            }
            ui.layout();
        }
        if let Some(r) = &self.resolving
            && let Some(found) = r.answer.lock().ok().and_then(|mut a| a.take())
        {
            let label = r.label.clone();
            self.resolving = None;
            ui.close_message_box();
            match found {
                // 0x6EC8B0: the address as the box says it, inet_ntoa of the answer
                Some(addr) => actions.push(self.start_join(ui, addr, addr.ip().to_string(), None, String::new())),
                None => {
                    let text = ui.strings.format("ui_lobby_join_direct_failed|s", &[&label]);
                    box_ok(ui, text);
                }
            }
        }
        if !self.asking_password
            && let Some(j) = &mut self.joining
        {
            let left = j.seconds - j.started.elapsed().as_secs() as i32;
            if left <= 0 {
                self.cancel_join();
                let text = ui.strings.resolve("#ui_lobby_connect_failed");
                box_ok(ui, text);
            } else if left != j.shown {
                j.shown = left;
                let text = joining_text(ui, &j.label, left);
                ui.set_message_box_text(&text);
            }
        }
    }

    /// The staging commands of a networked session; `None` passes it on to
    /// `Custom` (dialogs, the exit box).
    pub fn staging_command(&mut self, ui: &mut Ui, custom: &mut Custom, cmd: &str, widget: &str) -> Option<Action> {
        // the map dialogs' choice: the host's orders (0x75EEB0, 0x75F760)
        if let Some(rest) = cmd.strip_prefix("StagingReady ") {
            // 0x65CFD0: TGC_SetPlayerReadyStatusOrderRequest
            let on = rest.trim() != "0";
            self.with_node(|node| {
                if let Some(me) = node.local_player() {
                    let _ = node.request(d::SET_READY_REQUEST, &[Field::Player(me), Field::Uint { width: 32, value: u32::from(on) }]);
                }
            });
            self.own_ready = on;
            return Some(Action::None);
        }
        if cmd == "StagingReady %d" || cmd == "StagingReady" {
            let on = !self.own_ready;
            self.with_node(|node| {
                if let Some(me) = node.local_player() {
                    let _ = node.request(d::SET_READY_REQUEST, &[Field::Player(me), Field::Uint { width: 32, value: u32::from(on) }]);
                }
            });
            self.own_ready = on;
            return Some(Action::None);
        }
        match cmd {
            "StagingAbortLaunch" => {
                self.with_node(|node| {
                    let _ = node.request(d::STOP_COUNTDOWN_REQUEST, &[]);
                });
                return Some(Action::None);
            }
            // 0x69D6C0 MoveHere: this player into the slot's kingdom
            "StagingMoveHere" => {
                if let Some(ids) = custom.net_slot_ids(ui, widget) {
                    self.with_node(|node| {
                        if let Some(me) = node.local_player() {
                            let _ = node.request(d::SET_PLAYER_KINGDOM_REQUEST, &[Field::Player(me), Field::Text(units(&ids))]);
                        }
                    });
                }
                return Some(Action::None);
            }
            "StagingMoveToObserver" => {
                self.with_node(|node| {
                    if let Some(me) = node.local_player() {
                        let _ = node.request(d::SET_PLAYER_KINGDOM_REQUEST, &[Field::Player(me), Field::Text(Vec::new())]);
                    }
                });
                return Some(Action::None);
            }
            // the chat line (`ChatMessage`, public 0x755E00)
            "ChatMessage" => {
                let path = "Root/StagingMenu/StagingChatPanel/MessageEntryPanel/MessageEditbox";
                let line = ui.editbox_text(path).unwrap_or_default();
                if !line.is_empty() {
                    ui.editbox_set_text(path, "");
                    self.with_node(|node| {
                        let _ = node.request(d::PUBLIC_CHAT_REQUEST, &[Field::Uint { width: 32, value: 0 }, Field::Uint { width: 32, value: 0 }, Field::Text(units(&line))]);
                    });
                }
                return Some(Action::None);
            }
            _ => {}
        }
        if let Some(arg) = cmd.strip_prefix("StagingSelectSAI ") {
            // 0x69CDC0: an AI in the slot (TGC_CreateExplicitAIOrder), `_open`
            // takes its kingdom, and 0x7744C0 destroys an AI without one
            let arg = arg.trim().trim_matches('"').to_string();
            if let Some(ids) = custom.net_slot_ids(ui, widget) {
                self.with_node(|node| {
                    let session = node.session();
                    let holder = session.players.iter().find(|p| text(&p.kingdom).eq_ignore_ascii_case(&ids)).map(|p| (p.id, p.ai));
                    if arg == "_open" {
                        if let Some((id, true)) = holder {
                            let _ = node.request(d::SET_PLAYER_KINGDOM_REQUEST, &[Field::Player(id), Field::Text(Vec::new())]);
                        }
                    } else if holder.is_none() {
                        let profile = if arg == "_random" { Vec::new() } else { units(&arg) };
                        let name = units("Computer");
                        let _ = node.request(d::CREATE_EXPLICIT_AI_REQUEST, &[Field::Text(units(&ids)), Field::Text(profile), Field::Text(name)]);
                    }
                });
            }
            return Some(Action::None);
        }
        if let Some(format) = cmd.strip_prefix("TeamFormatDropdown ") {
            let format = format.trim().trim_matches('"').to_string();
            self.with_node(|node| {
                let _ = node.request(d::SET_TEAMS_REQUEST, &[Field::Text(units(&format))]);
            });
            return Some(Action::None);
        }
        None
    }

    /// Nation, faction, handicap of a slot (0x69E080 / 0x69C1E0 / 0x69C2F0).
    pub fn staging_select(&mut self, ui: &mut Ui, custom: &mut Custom, widget: &str, param: &str) {
        let which = widget.rsplit('/').find(|s| matches!(*s, "Nation" | "Faction" | "Handicap")).unwrap_or("").to_string();
        let Some(ids) = custom.net_slot_ids(ui, widget) else { return };
        let def = (!param.starts_with('_')).then(|| param.to_string());
        let data = self.data.lock().ok().and_then(|s| s.data.clone());
        let Some(data) = data else { return };
        let (descriptor, registry) = match which.as_str() {
            "Nation" => (d::SET_KINGDOM_NATION_REQUEST, registries::NATIONS),
            "Faction" => (d::SET_KINGDOM_FACTION_REQUEST, registries::FACTIONS),
            "Handicap" => (d::SET_AI_HANDICAP_REQUEST, registries::HANDICAPS),
            _ => return,
        };
        let Some(count) = data.registries.count(registry) else { return };
        let index = def.as_deref().and_then(|v| data.registries.slots.get(&registry.0).and_then(|s| s.iter().position(|x| x.eq_ignore_ascii_case(v)))).map_or(count as u32, |i| i as u32);
        self.with_node(|node| {
            let _ = node.request(descriptor, &[Field::Text(units(&ids)), Field::Def { registry, index }]);
        });
    }

    fn with_node(&mut self, f: impl FnOnce(&mut LanNode)) {
        if let Ok(mut n) = self.node.lock()
            && let Some(node) = n.as_mut()
        {
            f(node);
        }
    }

    /// After a command went to `Custom`: its dialogs may have chosen a map.
    /// The host's order for it (the choice shows when its decree comes back).
    pub fn after_custom_command(&mut self, custom: &mut Custom) {
        let Some(wanted) = custom.net_source() else { return };
        if self.source.as_ref() == Some(&wanted) {
            return;
        }
        let data = self.data.lock().ok().and_then(|s| s.data.clone());
        let Some(data) = data else { return };
        let rules = custom.film_rule_rows();
        let conditions = conditions_of(&data, &rules);
        let host = self.node.lock().is_ok_and(|n| n.as_ref().is_some_and(|n| n.role() == lan::Role::Host));
        if !host {
            return;
        }
        self.source = Some(wanted.clone());
        match wanted {
            NetSource::Map { title, size } => {
                let Some((records, _)) = data.maps.get(&title.to_ascii_lowercase()) else { return };
                let game = lobby::GameSettings { header: data.header(records), conditions, ..Default::default() };
                let file = units(&title);
                let format = custom.net_format();
                self.with_node(|node| {
                    match lan::load_map_fields(&file, &game, 0, node.data()) {
                        Ok(fields) => {
                            let _ = node.request(d::SET_SESSION_LOAD_MAP_REQUEST, &fields);
                            let _ = node.request(d::SET_MAP_SIZE_REQUEST, &[Field::Uint { width: 32, value: size.0 as u32 }, Field::Uint { width: 32, value: size.1 as u32 }]);
                            if !format.is_empty() {
                                let _ = node.request(d::SET_TEAMS_REQUEST, &[Field::Text(units(&format))]);
                            }
                        }
                        Err(e) => warn!("lan: the map's order: {e}"),
                    };
                });
            }
            NetSource::Random { rmc, size, terrain } => {
                let format = custom.net_format();
                let settings = lobby::RmcSettings { list_4: terrain.iter().map(|(n, v)| lobby::NamedWord { name: units(n), value: v.to_bits() }).collect(), ..Default::default() };
                self.with_node(|node| {
                    let _ = node.request(d::SET_RMC_REQUEST, &[Field::Named { kind: aa_orders::payload::NameKind::RandomMapConfig, units: units(&rmc) }]);
                    let _ = node.request(d::SET_MAP_SIZE_REQUEST, &[Field::Uint { width: 32, value: size.0 as u32 }, Field::Uint { width: 32, value: size.1 as u32 }]);
                    let mut fields = vec![Field::Uint { width: 32, value: settings.field_0 }, Field::Uint { width: 32, value: settings.list_4.len() as u32 }];
                    for row in &settings.list_4 {
                        fields.push(Field::Text(row.name.clone()));
                        fields.push(Field::Uint { width: 32, value: row.value });
                    }
                    fields.push(Field::Uint { width: 32, value: 0 });
                    fields.push(Field::Uint { width: 32, value: settings.field_1c });
                    let _ = node.request(d::SET_RMC_SETTINGS_REQUEST, &fields);
                    if !format.is_empty() {
                        let _ = node.request(d::SET_TEAMS_REQUEST, &[Field::Text(units(&format))]);
                    }
                });
            }
        }
    }

    /// The staging's view of the session.
    pub fn sync_staging(&mut self, ui: &mut Ui, custom: &mut Custom, show: bool) {
        let data = self.data.lock().ok().and_then(|s| s.data.clone());
        let Some(data) = data else { return };
        let guard = self.node.lock();
        let Ok(guard) = guard else { return };
        let Some(node) = guard.as_ref() else { return };
        let s = node.session();
        // the source
        let format = text(&s.info_text);
        let source = source_of(s);
        if self.source.as_ref() != source.as_ref() || custom.net_format() != format {
            if let Some(src) = &source {
                custom.net_set_source(ui, src, &format, &rule_rows(&data, &s.launch.active_map().conditions));
            }
            self.source = source;
        }
        // the rows
        let local = node.local_player();
        let we_host = node.role() == lan::Role::Host;
        let mut rows = Vec::new();
        for k in &s.launch.active_map().header.kingdoms {
            let ids = text(&k.name);
            let def = |r: aa_orders::payload::Registry, i: Option<u32>| i.and_then(|i| data.registries.slots.get(&r.0)?.get(i as usize).cloned());
            let nation = def(registries::NATIONS, k.nation);
            let faction = def(registries::FACTIONS, k.faction);
            let handicap = def(registries::HANDICAPS, k.ai_handicap);
            let occupant = s.players.iter().find(|p| p.kingdom == k.name).map(|p| occupant_of(p, local, we_host, s));
            let ai = s.players.iter().find(|p| p.kingdom == k.name).filter(|p| p.ai).map(|p| (!p.text_14.is_empty()).then(|| text(&p.text_14)));
            rows.push(NetRow { ids, occupant, ai, nation, faction, handicap });
        }
        custom.net_rows(rows);
        let mut state = custom.net_state(ui);
        state.networked = true;
        state.local_is_host = we_host;
        state.countdown = s.host_countdown > 0 || s.launch_countdown > 0;
        state.game_name = text(&s.options.name);
        state.password = text(&s.options.password);
        state.observers_allowed = s.options.allow_observers;
        let humans = s.players.iter().filter(|p| !p.ai && !p.film).count() as i32;
        state.players = (humans, s.options.max_players);
        let me = local.and_then(|id| s.players.iter().find(|p| p.id == id));
        state.local_has_kingdom = me.is_some_and(|p| !p.kingdom.is_empty());
        state.local_ready = me.is_some_and(|p| p.status == 1 || p.status == 2);
        state.observers = s.players.iter().filter(|p| !p.ai && !p.film && p.kingdom.is_empty()).map(|p| occupant_of(p, local, we_host, s)).collect();
        state.can_join_team = vec![true; state.teams.len()];
        // 0x6673A0: PlayersListbox, the people of the session
        let people: Vec<(String, u32)> = s.players.iter().filter(|p| !p.ai && !p.film).map(|p| (text(&p.name), p.id)).collect();
        drop(guard);
        if show {
            custom.net_show(ui, state);
        } else {
            custom.net_set_state(ui, state);
        }
        ui.listbox_clear(PLAYERS_LIST);
        for (name, id) in people {
            ui.listbox_add(PLAYERS_LIST, &name, &id.to_string());
        }
        ui.layout();
        self.dirty = false;
    }

    /// One frame of the session while no battle runs it, and what its events
    /// ask of the front end.
    pub fn pump(&mut self, ui: &mut Ui, staging_page: bool, custom: &mut Custom) -> Vec<Action> {
        let mut actions = Vec::new();
        self.lobby_frame(ui, &mut actions);
        self.online_frame(ui, &mut actions);
        if !self.in_battle {
            let events = self.node.lock().ok().and_then(|mut n| {
                let node = n.as_mut()?;
                match node.frame(&mut NoWorld) {
                    Ok(e) => Some(e),
                    Err(e) => {
                        warn!("lan: the session's frame failed: {e}");
                        Some(vec![LanEvent::Disconnected { reason: None }])
                    }
                }
            });
            if let (Some(events), Ok(mut q)) = (events, self.events.lock()) {
                q.extend(events);
            }
        }
        let events = self.events.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default();
        for e in events {
            match e {
                LanEvent::Joined { player } => {
                    info!("lan: joined as player {player}");
                    if self.node.lock().is_ok_and(|n| n.as_ref().is_none_or(LanNode::leaving)) {
                        continue;
                    }
                    self.joining = None;
                    ui.close_message_box();
                    self.dirty = true;
                    actions.push(Action::Staging);
                }
                LanEvent::PlayersChanged | LanEvent::StateChanged { .. } => self.dirty = true,
                LanEvent::LoadWorld => actions.push(Action::LoadWorld),
                LanEvent::BeginPlay => {
                    info!("lan: every peer loaded, the battle runs");
                    actions.push(Action::BeginPlay);
                }
                LanEvent::TasksFailed => {
                    warn!("lan: a peer's task failed");
                    actions.push(self.end_session(ui, "#ui_lobby_connect_failed"));
                }
                LanEvent::Message { key, args, number, .. } => {
                    let a: Vec<String> = args.iter().map(|x| text(x)).collect();
                    let mut b: Vec<&str> = a.iter().map(String::as_str).collect();
                    let n = number.map(|n| n.to_string());
                    if let Some(n) = &n {
                        b.push(n);
                    }
                    let line = ui.strings.format(&key, &b);
                    send_message(ui, &line);
                    self.dirty = true;
                }
                LanEvent::Chat { player, fields } => {
                    let line = fields.iter().find_map(|f| if let Field::Text(t) = f { Some(text(t)) } else { None }).unwrap_or_default();
                    let name = player
                        .and_then(|id| self.node.lock().ok().and_then(|n| n.as_ref().and_then(|n| n.session().players.iter().find(|p| p.id == id).map(|p| text(&p.name)))))
                        .unwrap_or_default();
                    let line = ui.strings.format("public_chat|ss", &[&name, &line]);
                    send_message(ui, &line);
                }
                LanEvent::SyncFailure { player } => {
                    let name = self.node.lock().ok().and_then(|n| n.as_ref().and_then(|n| n.session().players.iter().find(|p| p.id == player).map(|p| text(&p.name)))).unwrap_or_default();
                    warn!("lan: {name} is out of sync with the host (0x75BBC0)");
                    let status = ui.strings.resolve("#status_icon_out_of_sync");
                    let line = ui.strings.format("public_chat|ss", &[&name, &status]);
                    send_message(ui, &line);
                }
                // 0x75BD70: the silence is counted down (the connection dialog's
                // text; the dialog itself is not built)
                LanEvent::PeerTimedOut { address } => {
                    warn!("lan: no datagram from {address:?}");
                    let line = ui.strings.format("csd_disconnecting_in_seconds|d", &[&(lan::LOST_PATIENCE as i32).to_string()]);
                    send_message(ui, &line);
                }
                LanEvent::ConnectRefused { reason, info } => warn!("lan: refused {} (reason {reason})", text(&info.name)),
                LanEvent::BadDatagram { address, error } => warn!("lan: a datagram from {address:?} refused: {error}"),
                LanEvent::Paused { paused, .. } => info!("lan: paused {paused}"),
                LanEvent::Disconnected { reason } => {
                    info!("lan: disconnected, reason {reason:?}");
                    // 0x414C2A: a game with a password asks for it, and again when it was wrong
                    if reason == Some(1) && self.joining.is_some() {
                        let tried = self.joining.as_ref().is_some_and(|j| !j.password.is_empty());
                        let key = if tried { "#networking_disconnect_prompt_incorrect_password" } else { "#networking_disconnect_prompt_password" };
                        let text = ui.strings.resolve(key);
                        let (ok, cancel) = (ui.strings.resolve("#button_ok"), ui.strings.resolve("#button_cancel"));
                        if let Ok(mut n) = self.node.lock() {
                            *n = None;
                        }
                        self.asking_password = true;
                        ui.show_prompt_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(ok, "ServerPassword \"%s\"".into()), (cancel, "LobbyCancelJoin".into())] }, "");
                        continue;
                    }
                    self.joining = None;
                    // (type 8 without a backup server to take over: the host quit)
                    let line = reason.and_then(disconnect_key).unwrap_or_else(|| "#networking_disconnect_quit".into());
                    actions.push(self.end_session(ui, &line));
                }
                LanEvent::Left => {
                    if let Ok(mut n) = self.node.lock() {
                        *n = None;
                    }
                    info!("lan: left the session");
                }
            }
        }
        if self.dirty && staging_page {
            self.sync_staging(ui, custom, false);
        }
        actions
    }

    fn end_session(&mut self, ui: &mut Ui, key: &str) -> Action {
        if let Ok(mut n) = self.node.lock() {
            *n = None;
        }
        let in_battle = std::mem::take(&mut self.in_battle);
        let line = ui.strings.resolve(key);
        let ok = ui.strings.resolve("#button_ok");
        ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text: line, buttons: vec![(ok, String::new())] });
        if in_battle { Action::SessionLost } else { Action::Lobby }
    }

    /// Leaving the staging or the game: the connections are told, the node
    /// closes when the delay ran out (`LanEvent::Left`).
    pub fn leave(&mut self) {
        self.in_battle = false;
        self.own_ready = false;
        self.source = None;
        self.with_node(LanNode::leave);
    }

    /// The host's StagingLaunch (0x65DE80 -> 0x651950).
    pub fn start_launch(&mut self) {
        self.with_node(|node| {
            if let Err(e) = node.start_launch() {
                warn!("lan: launch: {e}");
            }
        });
    }

    /// The launch's world did not load: the other nodes are told (304 with
    /// status 4, 0x74FEC0), and every peer's session ends (0x773AE0).
    pub fn load_failed(&mut self) {
        self.with_node(|node| {
            if let Err(e) = node.report_world_loaded(false) {
                warn!("lan: task status: {e}");
            }
        });
    }

    /// The local kingdom and the map seed of a launch: what the front end needs
    /// to build the battle on every node alike.
    pub fn launch_seed(&self) -> Option<u32> {
        let n = self.node.lock().ok()?;
        let node = n.as_ref()?;
        Some(node.session().launch.active_map().field_4c)
    }

    /// The local player's kingdom in the setup's order (the records with a
    /// player, in record order).
    pub fn kingdoms(&self, custom: &Custom) -> (BTreeMap<u32, aa_play::KingdomId>, Option<aa_play::KingdomId>) {
        let mut out = BTreeMap::new();
        let Ok(n) = self.node.lock() else { return (out, None) };
        let Some(node) = n.as_ref() else { return (out, None) };
        let s = node.session();
        let order = custom.net_player_records();
        for (kingdom, ids) in order.iter().enumerate() {
            if let Some(p) = s.players.iter().find(|p| text(&p.kingdom).eq_ignore_ascii_case(ids)) {
                out.insert(p.id, kingdom as aa_play::KingdomId);
            }
        }
        let local = node.local_player().and_then(|me| out.get(&me).copied());
        (out, local)
    }

    /// The battle the loader built joins the session: what its decrees need,
    /// and the task report (304).
    pub fn attach(&mut self, battle: &mut Battle, kingdoms: BTreeMap<u32, aa_play::KingdomId>) {
        let reg = aa_parity::registries::Registries::cached(&self.game_root(), None);
        let tables: Tables = match reg {
            Ok(r) => aa_parity::recording::wire_tables(&r),
            Err(e) => {
                warn!("lan: the registries did not load: {e:#}");
                return;
            }
        };
        battle.world.trace.record(true);
        battle.paused = false;
        let digest_log = self.digest_path.as_ref().and_then(|p| std::fs::File::create(p).ok());
        battle.lan = Some(game::Link {
            node: self.node.clone(),
            kingdoms,
            tables,
            outbox: Outbox::default(),
            replay: Replay::default(),
            pending: Vec::new(),
            refused: 0,
            events: self.events.clone(),
            failed: false,
            digest_log,
            paused_asked: false,
            speed_asked: battle.speed,
        });
        self.in_battle = true;
        self.with_node(|node| {
            if let Err(e) = node.report_world_loaded(true) {
                warn!("lan: task status: {e}");
            }
        });
    }
}

const SERVER_LIST: &str = "Root/LobbyMenu/ServerBrowserPanel/ServerListBox";
const PLAYERS_LIST: &str = "Root/StagingMenu/StagingChatPanel/PlayersListBoxPanel/PlayersListbox";

fn box_ok(ui: &mut Ui, text: String) {
    let ok = ui.strings.resolve("#button_ok");
    ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(ok, String::new())] });
}

/// `0x64A350`: ui_lobby_joining_game_format, a new line, ui_lobby_join_timeout.
fn joining_text(ui: &Ui, label: &str, seconds: i32) -> String {
    let joining = ui.strings.format("ui_lobby_joining_game_format|s", &[label]);
    let timeout = ui.strings.format("ui_lobby_join_timeout|d", &[&seconds.to_string()]);
    format!("{joining}\n{timeout}")
}

/// The text of a ClientDisconnectDecree's reason (`packets::REASON_KEYS`, 0x75A780).
fn disconnect_key(reason: u32) -> Option<String> {
    let key = aa_netcode::packets::REASON_KEYS.get(reason as usize)?;
    Some(if key.starts_with("networking_") { format!("#{key}") } else { format!("#networking_disconnect_{key}") })
}

fn send_message(ui: &mut Ui, line: &str) {
    let uv = ui.uvars.clone();
    ui.messages.send(&uv, "GENERIC_EVENT", line, None);
}

fn occupant_of(p: &lobby::SessionPlayer, local: Option<u32>, we_host: bool, s: &Session) -> Occupant {
    let is_local = Some(p.id) == local;
    let on_host = matches!(p.link, lobby::PeerLink::Node) && we_host || (!we_host && s.players.iter().next().is_some_and(|h| h.id == p.id));
    if p.ai {
        Occupant::Ai { name: text(&p.name), client: String::new(), sai: (!p.text_14.is_empty()).then(|| text(&p.text_14)), local: we_host }
    } else {
        Occupant::Human { name: text(&p.name), local: is_local, on_host, ready: p.status == 1 || p.status == 2, ping_ms: 0, download: None, swap_from: false, swap_to: false }
    }
}

/// The session's stage as `Custom` names it.
fn source_of(s: &Session) -> Option<NetSource> {
    let map = s.launch.active_map();
    match s.launch.mode {
        0 => {
            let rmc = text(s.launch.random_map.as_deref()?);
            let terrain = s.launch.rmc_settings.list_4.iter().map(|r| (text(&r.name), f32::from_bits(r.value))).collect();
            Some(NetSource::Random { rmc, size: (map.size_x as f32, map.size_y as f32), terrain })
        }
        2 => Some(NetSource::Map { title: text(&s.launch.file), size: (map.size_x as i32, map.size_y as i32) }),
        _ => None,
    }
}

/// `Custom`'s rule rows (IDS, native kind, raw) as the creator's conditions:
/// the rules whose value differs from the default travel, in the registry's
/// order (0x511BB0).
fn conditions_of(data: &Data, rows: &[(String, u32, u32)]) -> lobby::WorldConditions {
    let mut rules = Vec::new();
    for (ids, _kind, raw) in rows {
        let Some(slots) = data.registries.slots.get(&registries::WORLD_RULES.0) else { break };
        if let Some(i) = slots.iter().position(|s| s.eq_ignore_ascii_case(ids)) {
            rules.push(lobby::WorldRule { rule: Some(i as u32), enabled: true, value: *raw });
        }
    }
    lobby::WorldConditions { rules, ..Default::default() }
}

fn rule_rows(data: &Data, conditions: &lobby::WorldConditions) -> Vec<(String, u32, u32)> {
    let Some(slots) = data.registries.slots.get(&registries::WORLD_RULES.0) else { return Vec::new() };
    conditions
        .rules
        .iter()
        .filter_map(|r| {
            let i = r.rule? as usize;
            Some((slots.get(i)?.clone(), *data.registries.world_rule_kinds.get(i)?, r.value))
        })
        .collect()
}
