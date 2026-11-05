//! The thread that owns the SDK: every SDK call in the program is made here, and
//! the SDK's callbacks run here, inside `EOS_Platform_Tick`.
//!
//! The SDK copies its option structures during the call, so the strings and
//! structures built here need only live until the call returns. Callbacks do no
//! more than copy what they were given into [`DONE`]; the loop acts on it after
//! the tick. A callback must not panic (it would cross the C boundary), so the
//! ones here have no failing path.
use crate::api::{self, Api};
use crate::config::{self, Config};
use crate::frame::{Framer, Reassembler, MAX_PACKET};
use crate::service::{Command, Event, Failure, LobbyInfo, LobbySpec, Shared, UserId};
use eos_sys as sys;
use std::collections::HashMap;
use std::ffi::{c_void, CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Lobbies of this game are found by this.
const BUCKET: &str = "openallies";
/// The P2P socket's name: letters and digits only (the SDK's rule).
const SOCKET: &str = "OpenAllies";
const CHANNEL: u8 = 0;
/// Longest a display name may be: `EOS_CONNECT_USERLOGININFO_DISPLAYNAME_MAX_LENGTH`.
const NAME_MAX: usize = sys::EOS_CONNECT_USERLOGININFO_DISPLAYNAME_MAX_LENGTH as usize;
/// `EOS_PRODUCTUSERID_MAX_LENGTH` and its terminator.
const USER_BUFFER: usize = sys::EOS_PRODUCTUSERID_MAX_LENGTH as usize + 1;
/// How long the loop rests when a session is on, and when nothing is.
const REST_BUSY: Duration = Duration::from_millis(4);
const REST_IDLE: Duration = Duration::from_millis(33);
/// How long leaving may take when the game ends.
const SHUTDOWN_PATIENCE: Duration = Duration::from_millis(1500);

/// What the callbacks hand over. Pointer values travel as `usize` so that this
/// stays plain data; only the worker makes them pointers again.
enum Done {
    DeviceId { result: i32 },
    DeviceIdDeleted,
    Login { result: i32, user: Option<String>, continuance: usize },
    CreateUser { result: i32, user: Option<String> },
    AuthExpiring,
    LoginStatus { logged_in: bool },
    LobbyCreated { result: i32, id: String },
    LobbyUpdated { result: i32 },
    LobbyFind { result: i32 },
    LobbyJoined { result: i32, id: String },
    LobbyLeft { result: i32 },
    ConnectionRequest { user: Option<String> },
    ConnectionClosed { user: Option<String>, reason: i32 },
}

/// The callbacks' mailbox. One platform per process, so one mailbox.
static DONE: Mutex<Vec<Done>> = Mutex::new(Vec::new());
/// The SDK's log lines; its log callback may run on any thread.
static LOGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn done(d: Done) {
    if let Ok(mut q) = DONE.lock() {
        q.push(d);
    }
}

/// # Safety
/// `p` is null or a valid NUL-terminated string.
unsafe fn text(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: as this function's contract says.
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

fn c(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

/// The result of a callback that the SDK will call again: it retries by itself.
fn retrying(result: i32) -> bool {
    result == sys::EOS_EResult_EOS_OperationWillRetry
}

/// Text of an ID, usable in a callback: goes through the table the worker
/// published before anything could call back.
static USER_TO_STRING: Mutex<Option<unsafe extern "C" fn(sys::EOS_ProductUserId, *mut c_char, *mut i32) -> i32>> = Mutex::new(None);

fn user_text(user: sys::EOS_ProductUserId) -> Option<String> {
    if user.is_null() {
        return None;
    }
    let to_string = (*USER_TO_STRING.lock().ok()?)?;
    let mut buffer = [0 as c_char; USER_BUFFER];
    let mut len = USER_BUFFER as i32;
    // SAFETY: the buffer and its length are this size; the handle came from the SDK.
    let r = unsafe { to_string(user, buffer.as_mut_ptr(), &mut len) };
    if r != sys::EOS_EResult_EOS_Success {
        return None;
    }
    // SAFETY: the SDK wrote a terminated string on success.
    Some(unsafe { text(buffer.as_ptr()) })
}

unsafe extern "C" fn on_device_id(info: *const sys::EOS_Connect_CreateDeviceIdCallbackInfo) {
    // SAFETY: the SDK passes a valid structure for the duration of the call.
    let result = unsafe { (*info).ResultCode };
    if !retrying(result) {
        done(Done::DeviceId { result });
    }
}
unsafe extern "C" fn on_device_id_deleted(info: *const sys::EOS_Connect_DeleteDeviceIdCallbackInfo) {
    // SAFETY: as in `on_device_id`.
    let result = unsafe { (*info).ResultCode };
    if !retrying(result) {
        done(Done::DeviceIdDeleted);
    }
}
unsafe extern "C" fn on_login(info: *const sys::EOS_Connect_LoginCallbackInfo) {
    // SAFETY: as in `on_device_id`.
    let info = unsafe { &*info };
    if !retrying(info.ResultCode) {
        done(Done::Login { result: info.ResultCode, user: user_text(info.LocalUserId), continuance: info.ContinuanceToken as usize });
    }
}
unsafe extern "C" fn on_create_user(info: *const sys::EOS_Connect_CreateUserCallbackInfo) {
    // SAFETY: as in `on_device_id`.
    let info = unsafe { &*info };
    if !retrying(info.ResultCode) {
        done(Done::CreateUser { result: info.ResultCode, user: user_text(info.LocalUserId) });
    }
}
unsafe extern "C" fn on_auth_expiring(_: *const sys::EOS_Connect_AuthExpirationCallbackInfo) {
    done(Done::AuthExpiring);
}
unsafe extern "C" fn on_login_status(info: *const sys::EOS_Connect_LoginStatusChangedCallbackInfo) {
    // SAFETY: as in `on_device_id`.
    let logged_in = unsafe { (*info).CurrentStatus } == sys::EOS_ELoginStatus_EOS_LS_LoggedIn;
    done(Done::LoginStatus { logged_in });
}
unsafe extern "C" fn on_lobby_created(info: *const sys::EOS_Lobby_CreateLobbyCallbackInfo) {
    // SAFETY: as in `on_device_id`; a null ID reads as empty.
    let (result, id) = unsafe { ((*info).ResultCode, text((*info).LobbyId)) };
    if !retrying(result) {
        done(Done::LobbyCreated { result, id });
    }
}
unsafe extern "C" fn on_lobby_updated(info: *const sys::EOS_Lobby_UpdateLobbyCallbackInfo) {
    // SAFETY: as in `on_device_id`.
    let result = unsafe { (*info).ResultCode };
    if !retrying(result) {
        done(Done::LobbyUpdated { result });
    }
}
unsafe extern "C" fn on_lobby_find(info: *const sys::EOS_LobbySearch_FindCallbackInfo) {
    // SAFETY: as in `on_device_id`.
    let result = unsafe { (*info).ResultCode };
    if !retrying(result) {
        done(Done::LobbyFind { result });
    }
}
unsafe extern "C" fn on_lobby_joined(info: *const sys::EOS_Lobby_JoinLobbyCallbackInfo) {
    // SAFETY: as in `on_lobby_created`.
    let (result, id) = unsafe { ((*info).ResultCode, text((*info).LobbyId)) };
    if !retrying(result) {
        done(Done::LobbyJoined { result, id });
    }
}
unsafe extern "C" fn on_lobby_left(info: *const sys::EOS_Lobby_LeaveLobbyCallbackInfo) {
    // SAFETY: as in `on_device_id`.
    let result = unsafe { (*info).ResultCode };
    if !retrying(result) {
        done(Done::LobbyLeft { result });
    }
}
unsafe extern "C" fn on_lobby_destroyed(info: *const sys::EOS_Lobby_DestroyLobbyCallbackInfo) {
    // SAFETY: as in `on_device_id`.
    let result = unsafe { (*info).ResultCode };
    if !retrying(result) {
        done(Done::LobbyLeft { result });
    }
}
unsafe extern "C" fn on_connection_request(info: *const sys::EOS_P2P_OnIncomingConnectionRequestInfo) {
    // SAFETY: as in `on_device_id`.
    let user = user_text(unsafe { (*info).RemoteUserId });
    done(Done::ConnectionRequest { user });
}
unsafe extern "C" fn on_connection_closed(info: *const sys::EOS_P2P_OnRemoteConnectionClosedInfo) {
    // SAFETY: as in `on_device_id`.
    let (user, reason) = unsafe { (user_text((*info).RemoteUserId), (*info).Reason) };
    done(Done::ConnectionClosed { user, reason });
}
unsafe extern "C" fn on_log(message: *const api::LogMessage) {
    if message.is_null() {
        return;
    }
    // SAFETY: the SDK passes a valid structure for the duration of the call.
    let line = unsafe { format!("{}: {}", text((*message).category), text((*message).message)) };
    if let Ok(mut q) = LOGS.lock() {
        q.push(line);
    }
}

/// What a connection's end means, in words.
fn closed_reason(reason: i32) -> &'static str {
    match reason {
        sys::EOS_EConnectionClosedReason_EOS_CCR_ClosedByLocalUser => "closed here",
        sys::EOS_EConnectionClosedReason_EOS_CCR_ClosedByPeer => crate::service::CLOSED_BY_PEER,
        sys::EOS_EConnectionClosedReason_EOS_CCR_TimedOut => "timed out",
        sys::EOS_EConnectionClosedReason_EOS_CCR_TooManyConnections => "too many connections",
        sys::EOS_EConnectionClosedReason_EOS_CCR_InvalidMessage | sys::EOS_EConnectionClosedReason_EOS_CCR_InvalidData => "bad data on the connection",
        sys::EOS_EConnectionClosedReason_EOS_CCR_ConnectionFailed => "the connection failed",
        sys::EOS_EConnectionClosedReason_EOS_CCR_ConnectionClosed => "the connection closed",
        sys::EOS_EConnectionClosedReason_EOS_CCR_NegotiationFailed => "the two machines could not connect",
        sys::EOS_EConnectionClosedReason_EOS_CCR_UnexpectedError => "an unexpected error",
        sys::EOS_EConnectionClosedReason_EOS_CCR_ConnectionIgnored => "the other player ignored the connection",
        _ => "unknown",
    }
}

fn attribute_text<'a>(key: &'a CStr, value: &'a CStr) -> sys::EOS_Lobby_AttributeData {
    let mut data: sys::EOS_Lobby_AttributeData = sys::EOS_Lobby_AttributeData::default();
    data.ApiVersion = sys::EOS_LOBBY_ATTRIBUTEDATA_API_LATEST as i32;
    data.Key = key.as_ptr();
    data.Value.AsUtf8 = value.as_ptr();
    data.ValueType = sys::EOS_EAttributeType_EOS_AT_STRING;
    data
}
fn attribute_int(key: &CStr, value: i64) -> sys::EOS_Lobby_AttributeData {
    let mut data: sys::EOS_Lobby_AttributeData = sys::EOS_Lobby_AttributeData::default();
    data.ApiVersion = sys::EOS_LOBBY_ATTRIBUTEDATA_API_LATEST as i32;
    data.Key = key.as_ptr();
    data.Value.AsInt64 = value;
    data.ValueType = sys::EOS_EAttributeType_EOS_AT_INT64;
    data
}

const KEY_NAME: &CStr = c"NAME";
const KEY_MAP: &CStr = c"MAP";
const KEY_PLAYERS: &CStr = c"PLAYERS";
const KEY_MAX: &CStr = c"MAXPLAYERS";
const KEY_PASSWORD: &CStr = c"PASSWORD";
const KEY_VERSION: &CStr = c"VERSION";

struct Worker {
    api: Api,
    shared: Arc<Shared>,
    platform: sys::EOS_HPlatform,
    connect: sys::EOS_HConnect,
    lobby: sys::EOS_HLobby,
    p2p: sys::EOS_HP2P,
    socket: sys::EOS_P2P_SocketId,
    local: sys::EOS_ProductUserId,
    display_name: CString,
    signed_in: bool,
    /// A renewal of the sign-in is under way: its result is no news if it works.
    renewing: bool,
    /// The lobby this player is in, and whether they own it.
    lobby_id: Option<CString>,
    owner: bool,
    pending_spec: Option<LobbySpec>,
    created_reported: bool,
    join_owner: Option<UserId>,
    search: sys::EOS_HLobbySearch,
    found: HashMap<String, sys::EOS_HLobbyDetails>,
    remote: HashMap<String, sys::EOS_ProductUserId>,
    framer: Framer,
    reassembler: Reassembler,
    receive_buffer: Vec<u8>,
    notify_request: sys::EOS_NotificationId,
    notify_closed: sys::EOS_NotificationId,
    notify_auth: sys::EOS_NotificationId,
    notify_status: sys::EOS_NotificationId,
    send_errors: u64,
}

/// Runs until the game ends. Whatever happens, `finished` is set at the end.
pub(crate) fn run(api: Api, config: Config, rx: Receiver<Command>, shared: Arc<Shared>) {
    struct Finish(Arc<Shared>);
    impl Drop for Finish {
        fn drop(&mut self) {
            self.0.finished.store(true, Ordering::SeqCst);
        }
    }
    let _finish = Finish(shared.clone());
    match Worker::create(api, config, shared.clone()) {
        Ok(mut w) => w.run(&rx),
        Err(why) => {
            shared.event(Event::StartFailed(why));
        }
    }
}

impl Worker {
    fn create(api: Api, config: Config, shared: Arc<Shared>) -> Result<Self, String> {
        let name = c(&config.product_name);
        let version = c(&config.product_version);
        let mut init = sys::EOS_InitializeOptions::default();
        init.ApiVersion = sys::EOS_INITIALIZE_API_LATEST as i32;
        init.ProductName = name.as_ptr();
        init.ProductVersion = version.as_ptr();
        // SAFETY: the options are valid for the call; the SDK copies them.
        let r = unsafe { (api.EOS_Initialize)(&init) };
        if r != sys::EOS_EResult_EOS_Success && r != sys::EOS_EResult_EOS_AlreadyConfigured {
            return Err(format!("the Epic SDK did not start: {}", result_name(&api, r)));
        }
        if let (Some(set_callback), Some(set_level)) = (api.log_set_callback, api.log_set_level) {
            // Warnings by default. AA_EOS_LOG=info is for a developer.
            let level = if std::env::var_os("AA_EOS_LOG").is_some() { api::LOG_INFO } else { api::LOG_WARNING };
            // SAFETY: the callback is a plain function that lives for the program.
            unsafe {
                let _ = set_callback(Some(on_log));
                let _ = set_level(api::LOG_ALL_CATEGORIES, level);
            }
        }
        if let Ok(mut f) = USER_TO_STRING.lock() {
            *f = Some(api.EOS_ProductUserId_ToString);
        }
        let product = c(&config.product_id);
        let sandbox = c(&config.sandbox_id);
        let deployment = c(&config.deployment_id);
        let client = c(&config.client_id);
        let secret = c(config.client_secret.expose());
        let cache = config.cache_dir.as_ref().map(|p| {
            let _ = std::fs::create_dir_all(p);
            c(&p.to_string_lossy())
        });
        let mut options = sys::EOS_Platform_Options::default();
        options.ApiVersion = sys::EOS_PLATFORM_OPTIONS_API_LATEST as i32;
        options.ProductId = product.as_ptr();
        options.SandboxId = sandbox.as_ptr();
        options.DeploymentId = deployment.as_ptr();
        options.ClientCredentials.ClientId = client.as_ptr();
        options.ClientCredentials.ClientSecret = secret.as_ptr();
        // No overlay: nothing here is for it, and it would hook the renderer.
        options.Flags = u64::from(sys::EOS_PF_DISABLE_OVERLAY);
        options.CacheDirectory = cache.as_ref().map_or(ptr::null(), |c| c.as_ptr());
        // SAFETY: the options and their strings outlive the call; the SDK copies them.
        let platform = unsafe { (api.EOS_Platform_Create)(&options) };
        config::wipe(secret.into_bytes());
        if platform.is_null() {
            return Err("the Epic platform did not start: check the product, sandbox, deployment and client IDs".to_string());
        }
        // SAFETY: a valid platform handle.
        let (connect, lobby, p2p) = unsafe { ((api.EOS_Platform_GetConnectInterface)(platform), (api.EOS_Platform_GetLobbyInterface)(platform), (api.EOS_Platform_GetP2PInterface)(platform)) };
        let mut socket = sys::EOS_P2P_SocketId { ApiVersion: sys::EOS_P2P_SOCKETID_API_LATEST as i32, SocketName: [0; 33] };
        for (to, from) in socket.SocketName.iter_mut().zip(SOCKET.bytes()) {
            *to = from as c_char;
        }
        let mut w = Self {
            api,
            shared,
            platform,
            connect,
            lobby,
            p2p,
            socket,
            local: ptr::null_mut(),
            display_name: CString::default(),
            signed_in: false,
            renewing: false,
            lobby_id: None,
            owner: false,
            pending_spec: None,
            created_reported: false,
            join_owner: None,
            search: ptr::null_mut(),
            found: HashMap::new(),
            remote: HashMap::new(),
            framer: Framer::default(),
            reassembler: Reassembler::default(),
            receive_buffer: vec![0; MAX_PACKET],
            notify_request: 0,
            notify_closed: 0,
            notify_auth: 0,
            notify_status: 0,
            send_errors: 0,
        };
        // Relays are what let two machines behind strict NATs still play.
        let relay = sys::EOS_P2P_SetRelayControlOptions { ApiVersion: sys::EOS_P2P_SETRELAYCONTROL_API_LATEST as i32, RelayControl: sys::EOS_ERelayControl_EOS_RC_AllowRelays };
        // SAFETY: valid handle and options.
        let _ = unsafe { (w.api.EOS_P2P_SetRelayControl)(w.p2p, &relay) };
        w.watch_sign_in();
        Ok(w)
    }

    fn watch_sign_in(&mut self) {
        let auth = sys::EOS_Connect_AddNotifyAuthExpirationOptions { ApiVersion: sys::EOS_CONNECT_ADDNOTIFYAUTHEXPIRATION_API_LATEST as i32 };
        let status = sys::EOS_Connect_AddNotifyLoginStatusChangedOptions { ApiVersion: sys::EOS_CONNECT_ADDNOTIFYLOGINSTATUSCHANGED_API_LATEST as i32 };
        // SAFETY: valid handle and options; the callbacks live for the program.
        unsafe {
            self.notify_auth = (self.api.EOS_Connect_AddNotifyAuthExpiration)(self.connect, &auth, ptr::null_mut(), Some(on_auth_expiring));
            self.notify_status = (self.api.EOS_Connect_AddNotifyLoginStatusChanged)(self.connect, &status, ptr::null_mut(), Some(on_login_status));
        }
    }

    /// The P2P notifications need the signed-in user.
    fn watch_p2p(&mut self) {
        if self.notify_request != 0 {
            return;
        }
        let request = sys::EOS_P2P_AddNotifyPeerConnectionRequestOptions { ApiVersion: sys::EOS_P2P_ADDNOTIFYPEERCONNECTIONREQUEST_API_LATEST as i32, LocalUserId: self.local, SocketId: &self.socket };
        let closed = sys::EOS_P2P_AddNotifyPeerConnectionClosedOptions { ApiVersion: sys::EOS_P2P_ADDNOTIFYPEERCONNECTIONCLOSED_API_LATEST as i32, LocalUserId: self.local, SocketId: &self.socket };
        // SAFETY: valid handle and options; the callbacks live for the program.
        unsafe {
            self.notify_request = (self.api.EOS_P2P_AddNotifyPeerConnectionRequest)(self.p2p, &request, ptr::null_mut(), Some(on_connection_request));
            self.notify_closed = (self.api.EOS_P2P_AddNotifyPeerConnectionClosed)(self.p2p, &closed, ptr::null_mut(), Some(on_connection_closed));
        }
    }

    fn failure(&self, doing: &'static str, code: i32) -> Failure {
        Failure { doing, code, result: result_name(&self.api, code) }
    }

    fn run(&mut self, rx: &Receiver<Command>) {
        loop {
            let rest = if self.lobby_id.is_some() || !self.remote.is_empty() { REST_BUSY } else { REST_IDLE };
            let mut shutdown = false;
            match rx.recv_timeout(rest) {
                Ok(c) => {
                    shutdown |= self.command(c);
                    while let Ok(c) = rx.try_recv() {
                        shutdown |= self.command(c);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                // the game dropped its handle
                Err(RecvTimeoutError::Disconnected) => shutdown = true,
            }
            if shutdown {
                self.shutdown();
                return;
            }
            self.turn();
        }
    }

    /// One tick and what it brought.
    fn turn(&mut self) {
        // SAFETY: the platform handle lives until `shutdown`.
        unsafe { (self.api.EOS_Platform_Tick)(self.platform) };
        let finished = DONE.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default();
        for d in finished {
            self.finished(d);
        }
        self.receive();
        let lines = LOGS.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default();
        for line in lines {
            self.shared.event(Event::SdkLog(line));
        }
    }

    /// `true` when the game is ending.
    fn command(&mut self, c: Command) -> bool {
        match c {
            Command::SignIn { display_name } => self.sign_in(&display_name),
            Command::CreateLobby(spec) => self.create_lobby(spec),
            Command::UpdateLobby(spec) => self.update_lobby(&spec),
            Command::FindLobbies => self.find_lobbies(),
            Command::JoinLobby { id, owner } => self.join_lobby(&id, owner),
            Command::LeaveLobby => self.leave_lobby(),
            Command::Send { to, data } => self.send(&to, &data),
            Command::Shutdown => return true,
        }
        false
    }

    fn finished(&mut self, d: Done) {
        match d {
            Done::DeviceId { result } => {
                // One already there is what a second run finds.
                if result == sys::EOS_EResult_EOS_Success || result == sys::EOS_EResult_EOS_DuplicateNotAllowed {
                    self.login();
                } else {
                    self.shared.event(Event::SignInFailed(self.failure("Creating this device's ID", result)));
                }
            }
            // whatever came of it, the new one is what counts
            Done::DeviceIdDeleted => self.create_device_id(),
            Done::Login { result, user, continuance } => {
                if result == sys::EOS_EResult_EOS_Success {
                    self.signed_in_as(user);
                } else if result == sys::EOS_EResult_EOS_InvalidUser && continuance != 0 {
                    // the first time this device is seen: make its user
                    let options = sys::EOS_Connect_CreateUserOptions { ApiVersion: sys::EOS_CONNECT_CREATEUSER_API_LATEST as i32, ContinuanceToken: continuance as sys::EOS_ContinuanceToken };
                    // SAFETY: valid handle and options; the token came from this login.
                    unsafe { (self.api.EOS_Connect_CreateUser)(self.connect, &options, ptr::null_mut(), Some(on_create_user)) };
                } else {
                    self.sign_in_failed("Signing in", result);
                }
            }
            Done::CreateUser { result, user } => {
                if result == sys::EOS_EResult_EOS_Success {
                    self.signed_in_as(user);
                } else {
                    self.sign_in_failed("Creating this player's account", result);
                }
            }
            // The SDK warns ahead of time; a new login extends it.
            Done::AuthExpiring => {
                self.renewing = true;
                self.login();
            }
            Done::LoginStatus { logged_in } => {
                if !logged_in && self.signed_in {
                    self.signed_in = false;
                    self.shared.event(Event::SignedOut { why: "Epic ended the sign-in".to_string() });
                }
            }
            Done::LobbyCreated { result, id } => self.lobby_created(result, &id),
            Done::LobbyUpdated { result } => {
                let report_created = !self.created_reported;
                if report_created {
                    self.created_reported = true;
                    if result == sys::EOS_EResult_EOS_Success {
                        let id = self.lobby_id.as_ref().map(|i| i.to_string_lossy().into_owned()).unwrap_or_default();
                        self.shared.event(Event::LobbyCreated { id });
                    } else {
                        self.shared.event(Event::LobbyCreateFailed(self.failure("Showing the game on the list", result)));
                    }
                } else if result != sys::EOS_EResult_EOS_Success {
                    self.shared.event(Event::LobbyUpdateFailed(self.failure("Updating the game on the list", result)));
                }
            }
            Done::LobbyFind { result } => self.lobby_found(result),
            Done::LobbyJoined { result, id } => {
                let owner = self.join_owner.take();
                match (result == sys::EOS_EResult_EOS_Success, owner) {
                    (true, Some(owner)) => {
                        self.lobby_id = Some(c(&id));
                        self.owner = false;
                        self.shared.peer(&owner);
                        self.shared.event(Event::LobbyJoined { id, owner });
                    }
                    _ => self.shared.event(Event::LobbyJoinFailed(self.failure("Joining the game", result))),
                }
            }
            Done::LobbyLeft { result } => {
                if result != sys::EOS_EResult_EOS_Success {
                    self.shared.event(Event::SdkLog(format!("leaving the lobby: {}", result_name(&self.api, result))));
                }
                self.shared.event(Event::LobbyLeft);
            }
            Done::ConnectionRequest { user } => {
                // A host takes whoever comes: the session's Connect packet is
                // where a game refuses a player, as on a LAN. Nobody else is
                // listening, so a stray request goes unanswered.
                if let Some(user) = user
                    && self.lobby_id.is_some()
                    && self.owner
                {
                    self.accept(&user);
                }
            }
            Done::ConnectionClosed { user, reason } => {
                if let Some(user) = user {
                    self.reassembler.forget(&user);
                    self.remote.remove(&user);
                    // leaving closes them on purpose, and says so itself
                    if reason != sys::EOS_EConnectionClosedReason_EOS_CCR_ClosedByLocalUser {
                        self.shared.event(Event::PeerClosed { user: UserId(user), why: closed_reason(reason).to_string() });
                    }
                }
            }
        }
    }

    fn sign_in(&mut self, display_name: &str) {
        let mut name = display_name.trim();
        if name.is_empty() {
            name = "Player";
        }
        let mut end = name.len().min(NAME_MAX);
        while !name.is_char_boundary(end) {
            end -= 1;
        }
        self.display_name = c(&name[..end]);
        // For testing two players on one machine. Epic keeps a device ID per
        // Windows user and product (Credential Manager, `eos-sdk:<product>:1`),
        // not in any folder a test can move, so the second client asks for a new
        // one; the first stays signed in with what it has. This replaces the
        // product's anonymous player for this Windows user.
        if std::env::var_os("AA_EOS_NEW_DEVICE_ID").is_some() {
            let options = sys::EOS_Connect_DeleteDeviceIdOptions { ApiVersion: sys::EOS_CONNECT_DELETEDEVICEID_API_LATEST as i32 };
            // SAFETY: valid handle and options.
            unsafe { (self.api.EOS_Connect_DeleteDeviceId)(self.connect, &options, ptr::null_mut(), Some(on_device_id_deleted)) };
            return;
        }
        self.create_device_id();
    }

    fn create_device_id(&mut self) {
        let model = c"PC Windows";
        let options = sys::EOS_Connect_CreateDeviceIdOptions { ApiVersion: sys::EOS_CONNECT_CREATEDEVICEID_API_LATEST as i32, DeviceModel: model.as_ptr() };
        // SAFETY: valid handle and options.
        unsafe { (self.api.EOS_Connect_CreateDeviceId)(self.connect, &options, ptr::null_mut(), Some(on_device_id)) };
    }

    fn login(&mut self) {
        let credentials = sys::EOS_Connect_Credentials { ApiVersion: sys::EOS_CONNECT_CREDENTIALS_API_LATEST as i32, Token: ptr::null(), Type: sys::EOS_EExternalCredentialType_EOS_ECT_DEVICEID_ACCESS_TOKEN };
        let info = sys::EOS_Connect_UserLoginInfo { ApiVersion: sys::EOS_CONNECT_USERLOGININFO_API_LATEST as i32, DisplayName: self.display_name.as_ptr(), NsaIdToken: ptr::null() };
        let options = sys::EOS_Connect_LoginOptions { ApiVersion: sys::EOS_CONNECT_LOGIN_API_LATEST as i32, Credentials: &credentials, UserLoginInfo: &info };
        // SAFETY: valid handle; the options and what they point at outlive the call.
        unsafe { (self.api.EOS_Connect_Login)(self.connect, &options, ptr::null_mut(), Some(on_login)) };
    }

    fn signed_in_as(&mut self, user: Option<String>) {
        let Some(user) = user else {
            self.sign_in_failed("Signing in", sys::EOS_EResult_EOS_InvalidUser);
            return;
        };
        let was_renewing = std::mem::take(&mut self.renewing);
        let id = c(&user);
        // SAFETY: a terminated string.
        self.local = unsafe { (self.api.EOS_ProductUserId_FromString)(id.as_ptr()) };
        self.signed_in = true;
        self.watch_p2p();
        if !was_renewing {
            self.shared.event(Event::SignedIn { user: UserId(user) });
        }
    }

    fn sign_in_failed(&mut self, doing: &'static str, result: i32) {
        if std::mem::take(&mut self.renewing) {
            self.signed_in = false;
            self.shared.event(Event::SignedOut { why: self.failure("Renewing the sign-in", result).to_string() });
        } else {
            self.shared.event(Event::SignInFailed(self.failure(doing, result)));
        }
    }

    fn create_lobby(&mut self, spec: LobbySpec) {
        if self.lobby_id.is_some() {
            // only one game at a time on this path
            self.shared.event(Event::LobbyCreateFailed(self.failure("Creating the game", sys::EOS_EResult_EOS_Lobby_LobbyAlreadyExists)));
            return;
        }
        let bucket = c(BUCKET);
        let mut options = sys::EOS_Lobby_CreateLobbyOptions::default();
        options.ApiVersion = sys::EOS_LOBBY_CREATELOBBY_API_LATEST as i32;
        options.LocalUserId = self.local;
        // A session's observers come on top of its players, and the session is
        // what counts them, so the lobby must not refuse them first.
        options.MaxLobbyMembers = if spec.observers { sys::EOS_LOBBY_MAX_LOBBY_MEMBERS } else { spec.max_players.clamp(1, sys::EOS_LOBBY_MAX_LOBBY_MEMBERS) };
        options.PermissionLevel = sys::EOS_ELobbyPermissionLevel_EOS_LPL_PUBLICADVERTISED;
        options.BucketId = bucket.as_ptr();
        // The session ends with its host (a port of the original's backup server
        // is not built), and a lobby without its host has nobody to reach.
        options.bDisableHostMigration = sys::EOS_TRUE as i32;
        self.pending_spec = Some(spec);
        self.created_reported = false;
        // SAFETY: valid handle and options.
        unsafe { (self.api.EOS_Lobby_CreateLobby)(self.lobby, &options, ptr::null_mut(), Some(on_lobby_created)) };
    }

    fn lobby_created(&mut self, result: i32, id: &str) {
        if result != sys::EOS_EResult_EOS_Success {
            self.pending_spec = None;
            self.created_reported = true;
            self.shared.event(Event::LobbyCreateFailed(self.failure("Creating the game", result)));
            return;
        }
        self.lobby_id = Some(c(id));
        self.owner = true;
        if let Some(spec) = self.pending_spec.take() {
            self.update_lobby(&spec);
        }
    }

    fn update_lobby(&mut self, spec: &LobbySpec) {
        let Some(id) = self.lobby_id.as_ref().filter(|_| self.owner) else { return };
        let options = sys::EOS_Lobby_UpdateLobbyModificationOptions { ApiVersion: sys::EOS_LOBBY_UPDATELOBBYMODIFICATION_API_LATEST as i32, LocalUserId: self.local, LobbyId: id.as_ptr() };
        let mut change: sys::EOS_HLobbyModification = ptr::null_mut();
        // SAFETY: valid handle, options and out pointer.
        let r = unsafe { (self.api.EOS_Lobby_UpdateLobbyModification)(self.lobby, &options, &mut change) };
        if r != sys::EOS_EResult_EOS_Success {
            self.update_failed(r);
            return;
        }
        let (name, map, version) = (c(&spec.name), c(&spec.map), c(&spec.version));
        let attributes = [
            attribute_text(KEY_NAME, &name),
            attribute_text(KEY_MAP, &map),
            attribute_text(KEY_VERSION, &version),
            attribute_int(KEY_PLAYERS, i64::from(spec.players)),
            attribute_int(KEY_MAX, i64::from(spec.max_players)),
            attribute_int(KEY_PASSWORD, i64::from(spec.password)),
        ];
        let mut failed = None;
        for a in &attributes {
            let options = sys::EOS_LobbyModification_AddAttributeOptions { ApiVersion: sys::EOS_LOBBYMODIFICATION_ADDATTRIBUTE_API_LATEST as i32, Attribute: a, Visibility: sys::EOS_ELobbyAttributeVisibility_EOS_LAT_PUBLIC };
            // SAFETY: valid handle; the attribute's strings outlive the call.
            let r = unsafe { (self.api.EOS_LobbyModification_AddAttribute)(change, &options) };
            if r != sys::EOS_EResult_EOS_Success {
                failed = Some(r);
            }
        }
        if let Some(r) = failed {
            // SAFETY: the handle came from the SDK and is released once.
            unsafe { (self.api.EOS_LobbyModification_Release)(change) };
            self.update_failed(r);
            return;
        }
        let options = sys::EOS_Lobby_UpdateLobbyOptions { ApiVersion: sys::EOS_LOBBY_UPDATELOBBY_API_LATEST as i32, LobbyModificationHandle: change };
        // SAFETY: valid handles; the SDK keeps its own copy of the change.
        unsafe {
            (self.api.EOS_Lobby_UpdateLobby)(self.lobby, &options, ptr::null_mut(), Some(on_lobby_updated));
            (self.api.EOS_LobbyModification_Release)(change);
        }
    }

    fn update_failed(&mut self, r: i32) {
        if self.created_reported {
            self.shared.event(Event::LobbyUpdateFailed(self.failure("Updating the game on the list", r)));
        } else {
            self.created_reported = true;
            self.shared.event(Event::LobbyCreateFailed(self.failure("Showing the game on the list", r)));
        }
    }

    fn find_lobbies(&mut self) {
        self.release_search();
        let options = sys::EOS_Lobby_CreateLobbySearchOptions { ApiVersion: sys::EOS_LOBBY_CREATELOBBYSEARCH_API_LATEST as i32, MaxResults: sys::EOS_LOBBY_MAX_SEARCH_RESULTS };
        let mut search: sys::EOS_HLobbySearch = ptr::null_mut();
        // SAFETY: valid handle, options and out pointer.
        let r = unsafe { (self.api.EOS_Lobby_CreateLobbySearch)(self.lobby, &options, &mut search) };
        if r != sys::EOS_EResult_EOS_Success {
            self.shared.event(Event::LobbySearchFailed(self.failure("Searching for games", r)));
            return;
        }
        self.search = search;
        let bucket = c(BUCKET);
        let key = sys::EOS_LOBBY_SEARCH_BUCKET_ID;
        let key = CStr::from_bytes_with_nul(key).unwrap_or(c"bucket");
        let parameter = attribute_text(key, &bucket);
        let options = sys::EOS_LobbySearch_SetParameterOptions { ApiVersion: sys::EOS_LOBBYSEARCH_SETPARAMETER_API_LATEST as i32, Parameter: &parameter, ComparisonOp: sys::EOS_EComparisonOp_EOS_CO_EQUAL };
        // SAFETY: valid handle; the parameter's strings outlive the call.
        let r = unsafe { (self.api.EOS_LobbySearch_SetParameter)(self.search, &options) };
        if r != sys::EOS_EResult_EOS_Success {
            self.release_search();
            self.shared.event(Event::LobbySearchFailed(self.failure("Searching for games", r)));
            return;
        }
        let options = sys::EOS_LobbySearch_FindOptions { ApiVersion: sys::EOS_LOBBYSEARCH_FIND_API_LATEST as i32, LocalUserId: self.local };
        // SAFETY: valid handle and options.
        unsafe { (self.api.EOS_LobbySearch_Find)(self.search, &options, ptr::null_mut(), Some(on_lobby_find)) };
    }

    fn release_search(&mut self) {
        if !self.search.is_null() {
            // SAFETY: the handle came from the SDK and is released once.
            unsafe { (self.api.EOS_LobbySearch_Release)(self.search) };
            self.search = ptr::null_mut();
        }
        for (_, details) in self.found.drain() {
            // SAFETY: as above.
            unsafe { (self.api.EOS_LobbyDetails_Release)(details) };
        }
    }

    fn lobby_found(&mut self, result: i32) {
        // "nothing matched" is a result, not a failure
        if result == sys::EOS_EResult_EOS_NotFound {
            self.release_search();
            self.shared.event(Event::LobbiesFound(Vec::new()));
            return;
        }
        if result != sys::EOS_EResult_EOS_Success || self.search.is_null() {
            self.release_search();
            self.shared.event(Event::LobbySearchFailed(self.failure("Searching for games", result)));
            return;
        }
        let count_options = sys::EOS_LobbySearch_GetSearchResultCountOptions { ApiVersion: sys::EOS_LOBBYSEARCH_GETSEARCHRESULTCOUNT_API_LATEST as i32 };
        // SAFETY: valid handle and options.
        let count = unsafe { (self.api.EOS_LobbySearch_GetSearchResultCount)(self.search, &count_options) };
        let mut lobbies = Vec::new();
        let me = user_text(self.local);
        for index in 0..count {
            let options = sys::EOS_LobbySearch_CopySearchResultByIndexOptions { ApiVersion: sys::EOS_LOBBYSEARCH_COPYSEARCHRESULTBYINDEX_API_LATEST as i32, LobbyIndex: index };
            let mut details: sys::EOS_HLobbyDetails = ptr::null_mut();
            // SAFETY: valid handle, options and out pointer.
            let r = unsafe { (self.api.EOS_LobbySearch_CopySearchResultByIndex)(self.search, &options, &mut details) };
            if r != sys::EOS_EResult_EOS_Success || details.is_null() {
                continue;
            }
            match self.read_details(details) {
                Some(l) if Some(&l.owner.0) != me.as_ref() => {
                    self.found.insert(l.id.clone(), details);
                    lobbies.push(l);
                }
                // SAFETY: the handle came from the SDK and is released once.
                _ => unsafe { (self.api.EOS_LobbyDetails_Release)(details) },
            }
        }
        // SAFETY: the copies above stay valid without the search.
        unsafe { (self.api.EOS_LobbySearch_Release)(self.search) };
        self.search = ptr::null_mut();
        self.shared.event(Event::LobbiesFound(lobbies));
    }

    fn read_details(&self, details: sys::EOS_HLobbyDetails) -> Option<LobbyInfo> {
        let options = sys::EOS_LobbyDetails_CopyInfoOptions { ApiVersion: sys::EOS_LOBBYDETAILS_COPYINFO_API_LATEST as i32 };
        let mut info: *mut sys::EOS_LobbyDetails_Info = ptr::null_mut();
        // SAFETY: valid handle, options and out pointer.
        let r = unsafe { (self.api.EOS_LobbyDetails_CopyInfo)(details, &options, &mut info) };
        if r != sys::EOS_EResult_EOS_Success || info.is_null() {
            return None;
        }
        // SAFETY: the SDK filled the structure; it is released once, after the copies.
        let (id, owner, max, available) = unsafe {
            let copy = ((*info).LobbyId, (*info).LobbyOwnerUserId, (*info).MaxMembers, (*info).AvailableSlots);
            let out = (text(copy.0), user_text(copy.1), copy.2, copy.3);
            (self.api.EOS_LobbyDetails_Info_Release)(info);
            out
        };
        let owner = UserId(owner?);
        let members = max.saturating_sub(available);
        Some(LobbyInfo {
            id,
            owner,
            name: self.attribute(details, KEY_NAME).map(|a| a.0).unwrap_or_default(),
            map: self.attribute(details, KEY_MAP).map(|a| a.0).unwrap_or_default(),
            players: self.attribute(details, KEY_PLAYERS).map_or(members, |a| a.1 as u32),
            max_players: self.attribute(details, KEY_MAX).map_or(max, |a| a.1 as u32),
            password: self.attribute(details, KEY_PASSWORD).is_some_and(|a| a.1 != 0),
            version: self.attribute(details, KEY_VERSION).map(|a| a.0).unwrap_or_default(),
        })
    }

    /// A public attribute as text and as a number; whichever it is not is empty.
    fn attribute(&self, details: sys::EOS_HLobbyDetails, key: &CStr) -> Option<(String, i64)> {
        let options = sys::EOS_LobbyDetails_CopyAttributeByKeyOptions { ApiVersion: sys::EOS_LOBBYDETAILS_COPYATTRIBUTEBYKEY_API_LATEST as i32, AttrKey: key.as_ptr() };
        let mut attribute: *mut sys::EOS_Lobby_Attribute = ptr::null_mut();
        // SAFETY: valid handle, options and out pointer.
        let r = unsafe { (self.api.EOS_LobbyDetails_CopyAttributeByKey)(details, &options, &mut attribute) };
        if r != sys::EOS_EResult_EOS_Success || attribute.is_null() {
            return None;
        }
        // SAFETY: the SDK filled the structure, and `ValueType` says which union
        // member holds the value. It is released once, after the copy.
        unsafe {
            let data = (*attribute).Data;
            let out = if data.is_null() {
                None
            } else if (*data).ValueType == sys::EOS_EAttributeType_EOS_AT_STRING {
                Some((text((*data).Value.AsUtf8), 0))
            } else if (*data).ValueType == sys::EOS_EAttributeType_EOS_AT_INT64 {
                Some((String::new(), (*data).Value.AsInt64))
            } else {
                None
            };
            (self.api.EOS_Lobby_Attribute_Release)(attribute);
            out
        }
    }

    fn join_lobby(&mut self, id: &str, owner: UserId) {
        let Some(&details) = self.found.get(id) else {
            self.shared.event(Event::LobbyJoinFailed(self.failure("Joining the game", sys::EOS_EResult_EOS_NotFound)));
            return;
        };
        self.join_owner = Some(owner);
        let mut options = sys::EOS_Lobby_JoinLobbyOptions::default();
        options.ApiVersion = sys::EOS_LOBBY_JOINLOBBY_API_LATEST as i32;
        options.LobbyDetailsHandle = details;
        options.LocalUserId = self.local;
        // SAFETY: valid handle and options; the details stay until the next search.
        unsafe { (self.api.EOS_Lobby_JoinLobby)(self.lobby, &options, ptr::null_mut(), Some(on_lobby_joined)) };
    }

    fn leave_lobby(&mut self) {
        self.close_connections();
        let Some(id) = self.lobby_id.take() else {
            self.shared.event(Event::LobbyLeft);
            return;
        };
        if std::mem::take(&mut self.owner) {
            let options = sys::EOS_Lobby_DestroyLobbyOptions { ApiVersion: sys::EOS_LOBBY_DESTROYLOBBY_API_LATEST as i32, LocalUserId: self.local, LobbyId: id.as_ptr() };
            // SAFETY: valid handle and options.
            unsafe { (self.api.EOS_Lobby_DestroyLobby)(self.lobby, &options, ptr::null_mut(), Some(on_lobby_destroyed)) };
        } else {
            let options = sys::EOS_Lobby_LeaveLobbyOptions { ApiVersion: sys::EOS_LOBBY_LEAVELOBBY_API_LATEST as i32, LocalUserId: self.local, LobbyId: id.as_ptr() };
            // SAFETY: valid handle and options.
            unsafe { (self.api.EOS_Lobby_LeaveLobby)(self.lobby, &options, ptr::null_mut(), Some(on_lobby_left)) };
        }
    }

    fn close_connections(&mut self) {
        self.reassembler.clear();
        self.remote.clear();
        if let Ok(mut q) = self.shared.incoming.lock() {
            q.clear();
        }
        if self.local.is_null() {
            return;
        }
        let options = sys::EOS_P2P_CloseConnectionsOptions { ApiVersion: sys::EOS_P2P_CLOSECONNECTIONS_API_LATEST as i32, LocalUserId: self.local, SocketId: &self.socket };
        // SAFETY: valid handle and options.
        let _ = unsafe { (self.api.EOS_P2P_CloseConnections)(self.p2p, &options) };
    }

    fn remote_handle(&mut self, user: &str) -> sys::EOS_ProductUserId {
        if let Some(&h) = self.remote.get(user) {
            return h;
        }
        let id = c(user);
        // SAFETY: a terminated string.
        let h = unsafe { (self.api.EOS_ProductUserId_FromString)(id.as_ptr()) };
        // SAFETY: the SDK treats any value as a question here.
        if !h.is_null() && unsafe { (self.api.EOS_ProductUserId_IsValid)(h) } != 0 {
            self.remote.insert(user.to_string(), h);
        }
        h
    }

    fn accept(&mut self, user: &str) {
        let remote = self.remote_handle(user);
        let options = sys::EOS_P2P_AcceptConnectionOptions { ApiVersion: sys::EOS_P2P_ACCEPTCONNECTION_API_LATEST as i32, LocalUserId: self.local, RemoteUserId: remote, SocketId: &self.socket };
        // SAFETY: valid handle and options.
        let r = unsafe { (self.api.EOS_P2P_AcceptConnection)(self.p2p, &options) };
        if r != sys::EOS_EResult_EOS_Success {
            self.shared.event(Event::SdkLog(format!("accepting a player's connection: {}", result_name(&self.api, r))));
        }
    }

    /// One datagram, as many packets as it takes.
    fn send(&mut self, to: &UserId, data: &[u8]) {
        if !self.signed_in {
            return;
        }
        let remote = self.remote_handle(&to.0);
        let mut packets = Vec::new();
        if !self.framer.frame(data, |p| packets.push(p.to_vec())) {
            self.shared.event(Event::SdkLog(format!("a datagram of {} bytes is too long to send", data.len())));
            return;
        }
        for packet in packets {
            let options = sys::EOS_P2P_SendPacketOptions {
                ApiVersion: sys::EOS_P2P_SENDPACKET_API_LATEST as i32,
                LocalUserId: self.local,
                RemoteUserId: remote,
                SocketId: &self.socket,
                Channel: CHANNEL,
                DataLengthBytes: packet.len() as u32,
                Data: packet.as_ptr() as *const c_void,
                // The session's first packet opens the connection: it waits for that.
                bAllowDelayedDelivery: sys::EOS_TRUE as i32,
                // The session's own sequencing and channels sit above this.
                Reliability: sys::EOS_EPacketReliability_EOS_PR_UnreliableUnordered,
                bDisableAutoAcceptConnection: sys::EOS_FALSE as i32,
            };
            // SAFETY: valid handle and options; the SDK copies the packet.
            let r = unsafe { (self.api.EOS_P2P_SendPacket)(self.p2p, &options) };
            if r != sys::EOS_EResult_EOS_Success {
                self.send_errors += 1;
                // often the same error for every packet: the first says it
                if self.send_errors == 1 {
                    self.shared.event(Event::SdkLog(format!("sending to a player: {}", result_name(&self.api, r))));
                }
            }
        }
    }

    /// Everything that waits: reassembled, under the sender's address.
    fn receive(&mut self) {
        if !self.signed_in {
            return;
        }
        loop {
            let size_options = sys::EOS_P2P_GetNextReceivedPacketSizeOptions { ApiVersion: sys::EOS_P2P_GETNEXTRECEIVEDPACKETSIZE_API_LATEST as i32, LocalUserId: self.local, RequestedChannel: ptr::null() };
            let mut size = 0u32;
            // SAFETY: valid handle, options and out pointer.
            let r = unsafe { (self.api.EOS_P2P_GetNextReceivedPacketSize)(self.p2p, &size_options, &mut size) };
            if r != sys::EOS_EResult_EOS_Success {
                return;
            }
            if self.receive_buffer.len() < size as usize {
                self.receive_buffer.resize(size as usize, 0);
            }
            let options = sys::EOS_P2P_ReceivePacketOptions { ApiVersion: sys::EOS_P2P_RECEIVEPACKET_API_LATEST as i32, LocalUserId: self.local, MaxDataSizeBytes: self.receive_buffer.len() as u32, RequestedChannel: ptr::null() };
            let mut peer: sys::EOS_ProductUserId = ptr::null_mut();
            let mut socket = sys::EOS_P2P_SocketId::default();
            let mut channel = 0u8;
            let mut written = 0u32;
            // SAFETY: valid handle, options and out pointers; the buffer is as
            // long as `MaxDataSizeBytes` says.
            let r = unsafe { (self.api.EOS_P2P_ReceivePacket)(self.p2p, &options, &mut peer, &mut socket, &mut channel, self.receive_buffer.as_mut_ptr() as *mut c_void, &mut written) };
            if r != sys::EOS_EResult_EOS_Success {
                return;
            }
            let Some(user) = user_text(peer) else { continue };
            let n = (written as usize).min(self.receive_buffer.len());
            let Some(datagram) = self.reassembler.accept(&user, &self.receive_buffer[..n]) else { continue };
            let user = UserId(user);
            let from = self.shared.peer(&user);
            if let Ok(mut q) = self.shared.incoming.lock() {
                q.push_back((from, datagram));
            }
        }
    }

    /// Leaves what can be left, in the little time a closing game has.
    fn shutdown(&mut self) {
        self.leave_lobby();
        let start = Instant::now();
        while start.elapsed() < SHUTDOWN_PATIENCE {
            self.turn();
            let left = self.shared.events.lock().is_ok_and(|q| q.iter().any(|e| *e == Event::LobbyLeft));
            if left {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        self.release_search();
        // SAFETY: valid handles; nothing is used after this, and no callback runs
        // without a tick.
        unsafe {
            if self.notify_request != 0 {
                (self.api.EOS_P2P_RemoveNotifyPeerConnectionRequest)(self.p2p, self.notify_request);
            }
            if self.notify_closed != 0 {
                (self.api.EOS_P2P_RemoveNotifyPeerConnectionClosed)(self.p2p, self.notify_closed);
            }
            (self.api.EOS_Connect_RemoveNotifyAuthExpiration)(self.connect, self.notify_auth);
            (self.api.EOS_Connect_RemoveNotifyLoginStatusChanged)(self.connect, self.notify_status);
            (self.api.EOS_Platform_Release)(self.platform);
            let _ = (self.api.EOS_Shutdown)();
        }
    }
}

/// `EOS_EResult_ToString`.
fn result_name(api: &Api, code: i32) -> String {
    // SAFETY: the SDK returns a static string for any code.
    unsafe { text((api.EOS_EResult_ToString)(code)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_socket_name_is_one_the_sdk_takes() {
        assert!(SOCKET.len() <= sys::EOS_P2P_SOCKETID_SOCKETNAME_SIZE as usize - 1);
        assert!(SOCKET.bytes().all(|b| b.is_ascii_alphanumeric()));
    }

    #[test]
    fn every_closing_reason_has_words() {
        // 0 is the SDK's own EOS_CCR_Unknown
        for r in 1..=11 {
            assert_ne!(closed_reason(r), "unknown", "{r}");
        }
        assert_eq!(closed_reason(0), "unknown");
        assert_eq!(closed_reason(99), "unknown");
    }
}
