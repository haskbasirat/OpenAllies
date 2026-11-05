//! The original's Internet button, on Epic Online Services (the `eos` feature).
//!
//! The original's Internet lobby is GameSpy's, which no longer exists. The same
//! screen stands here on [`aa_eos`]: `LobbyMenu` with its Internet panel and game
//! list (`0x64AB20`, `0x648270`), `CreateGameDialog`, and the session the LAN uses
//! on another datagram path. A game on the list is an EOS lobby; what is played
//! is the same session ([`aa_netcode::lan::LanNode`]) as on a LAN.
//!
//! GameSpy's chat rooms, buddy lists, quick match and statistics have nothing
//! behind them here, so those widgets of the file are not made. Everything that
//! can go wrong ends in a message box: a person is never left at a dead screen.
use super::{box_ok, source_of, text, Action, Lan, NetSource, TransportMaker, CREATE_ROOT, LOBBY, SERVER_LIST};
use aa_eos::{Config, Event, Failure, Link, LobbyInfo, LobbySpec, Service, UserId};
use aa_netcode::udp::Transport;
use aa_ui::ui::Ui;
use aa_ui::widget::{Widget, WidgetClass as C};
use bevy::log::{info, warn};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// A session's datagrams over EOS P2P.
struct EosTransport(Link);

impl Transport for EosTransport {
    fn local_addr(&self) -> aa_netcode::Result<SocketAddr> {
        Ok(self.0.local_addr())
    }
    fn send_to(&mut self, bytes: &[u8], to: SocketAddr) -> aa_netcode::Result<()> {
        // An address nobody has is dropped, as UDP drops what reaches nobody.
        self.0.send_to(bytes, to);
        Ok(())
    }
    fn recv_from(&mut self, buffer: &mut [u8]) -> aa_netcode::Result<Option<(usize, SocketAddr)>> {
        Ok(self.0.recv_from(buffer))
    }
}

/// What a box on show is waiting for.
#[derive(Default)]
enum Wait {
    #[default]
    None,
    SigningIn {
        since: Instant,
    },
    Creating {
        since: Instant,
        name: String,
        password: String,
        max_players: i32,
        observers: bool,
    },
    Joining {
        since: Instant,
        lobby: LobbyInfo,
    },
}

/// No answer in this long is an answer: the SDK's own requests give up sooner.
const PATIENCE: Duration = Duration::from_secs(60);
/// The list stays fresh while it is on show.
const LIST_EVERY: Duration = Duration::from_secs(20);
/// A lobby's attributes change no more often than this: Epic limits the rate.
const UPDATE_EVERY: Duration = Duration::from_secs(3);

/// The front end's side of online play.
#[derive(Default)]
pub struct Online {
    service: Option<Service>,
    me: Option<UserId>,
    /// The lobby page is the Internet lobby.
    internet: bool,
    wait: Wait,
    lobbies: Vec<LobbyInfo>,
    /// In a lobby: this player owns it, or its owner hosts the session.
    in_lobby: bool,
    /// A leave was asked and Epic has not answered: the frame keeps running.
    leaving: bool,
    owner: bool,
    host: Option<UserId>,
    next_list: Option<Instant>,
    /// The last search was a person's: its failure is theirs to see.
    asked_list: bool,
    shown: Option<LobbySpec>,
    shown_at: Option<Instant>,
}

fn cant_connect(ui: &mut Ui, why: &str) {
    let head = ui.strings.resolve("#ui_lobby_cant_connect");
    box_ok(ui, format!("{head}\n{why}"));
}

/// The box of a wait, with the button that gives up on it.
fn waiting_box(ui: &mut Ui, text: String) {
    let cancel = ui.strings.resolve("#button_cancel");
    ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(cancel, "OnlineCancel".into())] });
}

/// What a person can do about a failure, when there is something.
fn explained(f: &Failure) -> String {
    let hint = match f.result.as_str() {
        "EOS_NoConnection" | "EOS_TimedOut" | "EOS_ServiceFailure" => "\nCheck the internet connection and try again.",
        "EOS_InvalidCredentials" | "EOS_InvalidAuth" | "EOS_AccessDenied" | "EOS_Forbidden" => "\nThe game's online settings were refused by Epic.",
        "EOS_Lobby_LobbyAlreadyExists" => "\nLeave the other game first.",
        "EOS_Lobby_TooManyPlayers" => "\nThe game is full.",
        _ => "",
    };
    format!("{f}{hint}")
}

/// The map as the list names it.
fn map_label(source: Option<NetSource>) -> String {
    match source {
        Some(NetSource::Map { title, .. }) => title,
        Some(NetSource::Random { rmc, .. }) => rmc,
        None => String::new(),
    }
}

/// `0x64AB20` LobbyMenu in Internet mode: as [`super::lobby_menu`], with the
/// Internet panel. The statistics button stays dead: GameSpy kept the statistics.
fn lobby_menu(ui: &mut Ui) {
    ui.clear();
    aa_ui::screens::menu_backdrop(ui);
    let mut m = Widget::new(C::Interface, LOBBY, false);
    // GameSpyLogoDecal is not made: this lobby is not GameSpy's, and a disabled
    // decal still draws
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
    let mut panel = Widget::new(C::Panel, "InternetButtonsPanel", false);
    for (n, c) in [("CreateButton", "LobbyHost"), ("JoinButton", "LobbyJoin"), ("OptionsButton", "OpenOptionsMenu"), ("ExitButton", "LobbyConfirmExit")] {
        panel.add_child(Widget::push_button(n, false, c));
    }
    let mut stats = Widget::push_button("StatsButton", false, "");
    stats.set_enabled(false);
    panel.add_child(stats);
    m.add_child(panel);
    aa_ui::screens::show(ui, m);
    // as the LAN lobby: 0x647DC0(3), no chat, so the list takes the full bounds
    if let Some((w, h)) = ui.store.node(LOBBY).ok().and_then(|n| Some((super::full_size(&n, "full_R")?, super::full_size(&n, "full_B")?))) {
        ui.with_widget("LobbyMenu/ServerBrowserPanel", |p| aa_ui::geometry::set_size(p, w, h));
    }
    ui.layout();
}

impl Lan {
    /// The Internet lobby comes up when the page is. `false`: the LAN's page.
    pub fn internet_lobby(&mut self, ui: &mut Ui, first_time: bool) -> bool {
        if !self.online.internet {
            return false;
        }
        lobby_menu(ui);
        if first_time {
            self.ask_nickname(ui);
        }
        self.online_list(ui, true);
        true
    }

    /// Something waits for an answer while no session runs the frame.
    pub fn online_busy(&self) -> bool {
        !matches!(self.online.wait, Wait::None) || self.online.in_lobby || self.online.leaving
    }

    /// A command of the Internet button or lobby; `None` when it is not one.
    pub(super) fn online_command(&mut self, ui: &mut Ui, cmd: &str) -> Option<Action> {
        match cmd {
            "RequestLobbyInternet" => return Some(self.online_enter(ui)),
            "RequestLobbyLAN" => self.online.internet = false,
            "OnlineCancel" => {
                self.online_cancel();
                return Some(Action::None);
            }
            _ => {}
        }
        if !self.online.internet {
            return None;
        }
        match cmd {
            "LobbyRefreshServerList" => {
                self.online_list(ui, true);
                Some(Action::None)
            }
            "LobbyJoin" => Some(self.online_join(ui)),
            "CreateGameCreate" => Some(self.online_create(ui)),
            "LobbyExit" => {
                self.online.internet = false;
                self.online.next_list = None;
                None
            }
            _ => None,
        }
    }

    /// Internet: the SDK's library and the sign-in, then the lobby.
    fn online_enter(&mut self, ui: &mut Ui) -> Action {
        if self.online.me.is_some() {
            self.online.internet = true;
            return Action::Lobby;
        }
        if self.online.service.as_ref().is_some_and(Service::finished) {
            cant_connect(ui, "Online play could not start in this run of the game. Restart the game to try again.");
            return Action::None;
        }
        if self.online.service.is_none() {
            let cache = crate::session::own_profile().map(|p| p.join("eos"));
            let started = Config::load("OpenAllies", env!("CARGO_PKG_VERSION"), cache).map_err(|e| e.to_string()).and_then(|c| Service::start(c).map_err(|e| e.to_string()));
            match started {
                Ok(s) => self.online.service = Some(s),
                Err(why) => {
                    warn!("eos: {why}");
                    cant_connect(ui, &why);
                    return Action::None;
                }
            }
        }
        if let Some(s) = &self.online.service {
            s.sign_in(&self.local_name);
        }
        self.online.wait = Wait::SigningIn { since: Instant::now() };
        waiting_box(ui, "Connecting to Epic Online Services".to_string());
        Action::None
    }

    /// The box's Cancel. What the SDK is doing finishes; its answer finds nobody
    /// waiting ([`Self::online_frame`] undoes a lobby made or joined meanwhile).
    fn online_cancel(&mut self) {
        self.online.wait = Wait::None;
    }

    /// `0x6EBAB0`: the dialog's values go to a lobby first, and the session starts
    /// when Epic has it, so a refused lobby leaves nothing half made.
    fn online_create(&mut self, ui: &mut Ui) -> Action {
        let name = ui.editbox_text(&format!("{CREATE_ROOT}/GameNameEditbox")).unwrap_or_default();
        let password = ui.editbox_text(&format!("{CREATE_ROOT}/PasswordEditbox")).unwrap_or_default();
        let list = format!("{CREATE_ROOT}/MaxPlayersDropDown/Listbox");
        let fallback = ui.avars.float("MultiplayerMaxNumPlayers").unwrap_or(12.0) as i32;
        let max_players = ui.listbox_selected(&list).and_then(|s| ui.listbox_entries(&list).get(s).and_then(|e| e.param.parse().ok())).unwrap_or(fallback);
        let observers = ui.widget(&format!("{CREATE_ROOT}/AllowObserversToggle")).is_some_and(|t| t.flags & aa_ui::state::widget_flag::SELECTED != 0);
        self.close_dialog(ui);
        let Some(service) = self.online.service.as_ref().filter(|_| self.online.me.is_some()) else {
            cant_connect(ui, "The sign-in has ended. Leave the lobby and press Internet again.");
            return Action::None;
        };
        let rmc = ui.avars.get("StagingMultiPlayerDefaultRMCIds").map(aa_ui::node::unquote).unwrap_or_default();
        service.create_lobby(LobbySpec { name: name.clone(), map: rmc, players: 1, max_players: max_players as u32, password: !password.is_empty(), observers, version: env!("CARGO_PKG_VERSION").to_string() });
        self.online.wait = Wait::Creating { since: Instant::now(), name: name.clone(), password, max_players, observers };
        waiting_box(ui, name);
        Action::None
    }

    /// `0x64A280`: the selected game; it asks for one without a selection.
    fn online_join(&mut self, ui: &mut Ui) -> Action {
        let entries = ui.listbox_entries(SERVER_LIST);
        let selected = ui.listbox_selected(SERVER_LIST).and_then(|s| entries.get(s)).and_then(|e| self.online.lobbies.iter().find(|l| l.id == e.param)).cloned();
        let Some(lobby) = selected else {
            let text = ui.strings.resolve("#ui_lobby_select_server");
            box_ok(ui, text);
            return Action::None;
        };
        let Some(service) = self.online.service.as_ref().filter(|_| self.online.me.is_some()) else {
            cant_connect(ui, "The sign-in has ended. Leave the lobby and press Internet again.");
            return Action::None;
        };
        service.join_lobby(&lobby);
        let text = ui.strings.format("ui_lobby_joining_game_format|s", &[&lobby.name]);
        self.online.wait = Wait::Joining { since: Instant::now(), lobby };
        waiting_box(ui, text);
        Action::None
    }

    /// Asks for the list, now or when it is time.
    fn online_list(&mut self, _ui: &mut Ui, now: bool) {
        let due = now || self.online.next_list.is_some_and(|t| Instant::now() >= t);
        if !due {
            return;
        }
        if let Some(s) = self.online.service.as_ref().filter(|_| self.online.me.is_some()) {
            s.find_lobbies();
            self.online.next_list = Some(Instant::now() + LIST_EVERY);
            self.online.asked_list = now;
        }
    }

    /// The list's rows, as the LAN's; the selection stays with its game.
    fn online_rows(&mut self, ui: &mut Ui, lobbies: Vec<LobbyInfo>) {
        let keep = ui.listbox_selected(SERVER_LIST).and_then(|s| ui.listbox_entries(SERVER_LIST).get(s).map(|e| e.param.clone()));
        ui.listbox_clear(SERVER_LIST);
        for l in &lobbies {
            // EOS gives no ping before a connection
            let row = format!("{}    {}/{}    -    {}", l.name, l.players, l.max_players, l.map);
            let at = ui.listbox_add(SERVER_LIST, &row, &l.id);
            if keep.as_deref() == Some(&*l.id) {
                ui.listbox_select(SERVER_LIST, at, false);
            }
        }
        self.online.lobbies = lobbies;
        ui.layout();
    }

    /// One frame: what the SDK's thread reported, a wait that took too long, a
    /// lobby whose session ended, and the list.
    pub(super) fn online_frame(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let Some(service) = &self.online.service else { return };
        let events = service.poll();
        for e in events {
            self.online_event(ui, e, actions);
        }
        let since = match &self.online.wait {
            Wait::None => None,
            Wait::SigningIn { since } | Wait::Creating { since, .. } | Wait::Joining { since, .. } => Some(*since),
        };
        if since.is_some_and(|s| s.elapsed() > PATIENCE) {
            self.online.wait = Wait::None;
            ui.close_message_box();
            cant_connect(ui, "Epic Online Services did not answer.");
        }
        // The session ended: so does its lobby. The node's goodbye has gone by
        // then (`LanEvent::Left`), and a join waiting for a password has no node.
        if self.online.in_lobby && !self.active() && self.joining.is_none() && !self.asking_password {
            self.online.in_lobby = false;
            self.online.leaving = true;
            self.online.owner = false;
            self.online.host = None;
            self.online.shown = None;
            if let Some(s) = &self.online.service {
                s.leave_lobby();
            }
        }
        self.online_show_game();
        if self.online.internet && !self.active() && !self.online_busy() {
            self.online_list(ui, false);
        }
    }

    /// The host keeps its lobby's attributes true: the map and the players.
    fn online_show_game(&mut self) {
        if !self.online.owner {
            return;
        }
        if self.online.shown_at.is_some_and(|t| t.elapsed() < UPDATE_EVERY) {
            return;
        }
        let spec = {
            let Ok(guard) = self.node.lock() else { return };
            let Some(node) = guard.as_ref() else { return };
            let s = node.session();
            LobbySpec {
                name: text(&s.options.name),
                map: map_label(source_of(s)),
                players: s.players.iter().filter(|p| !p.ai && !p.film).count() as u32,
                max_players: s.options.max_players as u32,
                password: !s.options.password.is_empty(),
                observers: s.options.allow_observers,
                version: env!("CARGO_PKG_VERSION").to_string(),
            }
        };
        if self.online.shown.as_ref() != Some(&spec) {
            if let Some(s) = &self.online.service {
                s.update_lobby(spec.clone());
            }
            self.online.shown = Some(spec);
            self.online.shown_at = Some(Instant::now());
        }
    }

    fn online_event(&mut self, ui: &mut Ui, event: Event, actions: &mut Vec<Action>) {
        match event {
            Event::SdkLog(line) => warn!("eos: {line}"),
            Event::StartFailed(why) => {
                warn!("eos: {why}");
                self.online.wait = Wait::None;
                ui.close_message_box();
                cant_connect(ui, &why);
            }
            Event::SignedIn { user } => {
                info!("eos: signed in");
                self.online.me = Some(user);
                if matches!(self.online.wait, Wait::SigningIn { .. }) {
                    self.online.wait = Wait::None;
                    self.online.internet = true;
                    ui.close_message_box();
                    actions.push(Action::Lobby);
                }
            }
            Event::SignInFailed(f) => {
                warn!("eos: {f}");
                if matches!(self.online.wait, Wait::SigningIn { .. }) {
                    self.online.wait = Wait::None;
                    ui.close_message_box();
                    cant_connect(ui, &explained(&f));
                }
            }
            Event::SignedOut { why } => {
                warn!("eos: signed out: {why}");
                self.online.me = None;
                let line = format!("Signed out of Epic Online Services: {why}");
                if self.active() {
                    super::send_message(ui, &line);
                } else if self.online.internet {
                    box_ok(ui, format!("{line}\nLeave the lobby and press Internet again."));
                }
            }
            Event::LobbyCreated { id } => {
                info!("eos: lobby {id} created");
                self.online.in_lobby = true;
                self.online.owner = true;
                self.online.shown = None;
                self.online.shown_at = Some(Instant::now());
                let wait = std::mem::take(&mut self.online.wait);
                let (Wait::Creating { name, password, max_players, observers, .. }, Some(service), Some(me)) = (wait, &self.online.service, &self.online.me) else {
                    // the box's Cancel came first; the frame closes the lobby
                    return;
                };
                ui.close_message_box();
                let transport = Box::new(EosTransport(service.link(me)));
                match self.host_with(ui, transport, &name, &password, max_players, observers) {
                    Ok(a) => actions.push(a),
                    Err(why) => {
                        warn!("eos: hosting: {why}");
                        cant_connect(ui, &why);
                    }
                }
            }
            Event::LobbyCreateFailed(f) => {
                warn!("eos: {f}");
                if matches!(self.online.wait, Wait::Creating { .. }) {
                    self.online.wait = Wait::None;
                    ui.close_message_box();
                    cant_connect(ui, &explained(&f));
                }
            }
            Event::LobbyUpdateFailed(f) => warn!("eos: {f}"),
            Event::LobbiesFound(lobbies) => {
                if self.online.internet {
                    self.online_rows(ui, lobbies);
                }
            }
            Event::LobbySearchFailed(f) => {
                warn!("eos: {f}");
                if std::mem::take(&mut self.online.asked_list) {
                    cant_connect(ui, &explained(&f));
                }
            }
            Event::LobbyJoined { id, owner } => {
                info!("eos: joined lobby {id}");
                self.online.in_lobby = true;
                self.online.owner = false;
                self.online.host = Some(owner.clone());
                let wait = std::mem::take(&mut self.online.wait);
                let (Wait::Joining { lobby, .. }, Some(service), Some(me)) = (wait, &self.online.service, &self.online.me) else {
                    // the box's Cancel came first; the frame leaves the lobby
                    return;
                };
                ui.close_message_box();
                let link = service.link(me);
                let host = link.add_peer(&owner);
                let maker: TransportMaker = Box::new(move || Ok(Box::new(EosTransport(link.clone())) as Box<dyn Transport>));
                actions.push(self.join_with(ui, maker, host, lobby.name));
            }
            Event::LobbyJoinFailed(f) => {
                warn!("eos: {f}");
                if matches!(self.online.wait, Wait::Joining { .. }) {
                    self.online.wait = Wait::None;
                    ui.close_message_box();
                    let head = ui.strings.resolve("#ui_lobby_connect_failed");
                    box_ok(ui, format!("{head}\n{}", explained(&f)));
                }
            }
            Event::LobbyLeft => {
                self.online.leaving = false;
                info!("eos: left the lobby");
            }
            Event::PeerClosed { user, why } => {
                warn!("eos: the connection to {} ended: {why}", user.0);
                if self.online.host.as_ref() != Some(&user) {
                    return;
                }
                // The host's own goodbye comes through the session. Anything else
                // would only show as silence until the session's patience runs out.
                if why == aa_eos::CLOSED_BY_PEER {
                    return;
                }
                if self.joining.is_some() {
                    self.cancel_join();
                    ui.close_message_box();
                    let head = ui.strings.resolve("#ui_lobby_connect_failed");
                    box_ok(ui, format!("{head}\n{why}"));
                } else if self.active() {
                    let head = ui.strings.resolve("#ui_lobby_disconnect");
                    let a = self.end_session(ui, &format!("{head}\nThe connection to the host was lost: {why}"));
                    actions.push(a);
                }
            }
        }
    }

    /// The nickname box is the LAN lobby's: the session is what shows the name.
    fn ask_nickname(&mut self, ui: &mut Ui) {
        let text = ui.strings.resolve("#ui_lobby_prompt_lan_nickname");
        let (ok, cancel) = (ui.strings.resolve("#button_ok"), ui.strings.resolve("#button_cancel"));
        let name = self.local_name.clone();
        ui.show_prompt_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(ok, "LobbySetLANNickname \"%s\"".to_string()), (cancel, String::new())] }, &name);
    }
}
