//! The Epic SDK's functions, found in its library at run time.
//!
//! The game must start, and say something a person can read, on a machine
//! without the library, so nothing links against it: linking would stop the
//! program before `main` with the operating system's own error. The types and
//! constants come from `eos-sys`; each function here is checked at compile time
//! against the signature `eos-sys` declares (the `const _` in the macro, which
//! makes no reference into the library).
#![allow(non_snake_case)]
use eos_sys as sys;
use libloading::Library;
use std::fmt;
use std::os::raw::{c_char, c_void};
use std::path::{Path, PathBuf};

/// The library's file name on this system.
pub const LIBRARY_FILE: &str = if cfg!(windows) {
    "EOSSDK-Win64-Shipping.dll"
} else if cfg!(target_os = "macos") {
    "libEOSSDK-Mac-Shipping.dylib"
} else {
    "libEOSSDK-Linux-Shipping.so"
};

/// Why the library could not be used.
#[derive(Debug)]
pub enum LoadError {
    /// No library at any of these places.
    Missing { searched: Vec<PathBuf> },
    /// A file is there, and the system would not load it.
    Unloadable { path: PathBuf, why: String },
    /// The library lacks a function: another SDK version than the bindings.
    MissingFunction { path: PathBuf, name: &'static str },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Missing { searched } => {
                write!(f, "{LIBRARY_FILE} was not found. Online play needs it beside the game. Looked in:")?;
                for p in searched {
                    write!(f, " {}", p.display())?;
                }
                Ok(())
            }
            LoadError::Unloadable { path, why } => write!(f, "{} would not load: {why}", path.display()),
            LoadError::MissingFunction { path, name } => write!(f, "{} has no function {name}: it is another version of the SDK", path.display()),
        }
    }
}
impl std::error::Error for LoadError {}

/// Where the library may be: beside the program, then `EOS_SDK_DIR/Bin` for a
/// developer's build. Full paths only, so no other folder's file of that name
/// gets loaded.
fn places() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        out.push(dir);
    }
    if let Some(sdk) = std::env::var_os("EOS_SDK_DIR") {
        out.push(PathBuf::from(sdk).join("Bin"));
    }
    out
}

macro_rules! api {
    ($($name:ident ( $($arg:ty),* ) $(-> $ret:ty)? ;)*) => {
        /// The functions of a loaded library; the library lives as long as this.
        pub(crate) struct Api {
            $(pub $name: unsafe extern "C" fn($($arg),*) $(-> $ret)?,)*
            pub log_set_callback: Option<unsafe extern "C" fn(Option<LogCallback>) -> sys::EOS_EResult>,
            pub log_set_level: Option<unsafe extern "C" fn(i32, i32) -> sys::EOS_EResult>,
            _library: Library,
        }
        $(const _: unsafe extern "C" fn($($arg),*) $(-> $ret)? = sys::$name;)*
        impl Api {
            fn resolve(library: Library, path: PathBuf) -> Result<Self, LoadError> {
                // SAFETY: each symbol is given the signature `eos-sys` declares
                // for it, checked by the constants above.
                unsafe {
                    $(
                        let $name: unsafe extern "C" fn($($arg),*) $(-> $ret)? = *library
                            .get(concat!(stringify!($name), "\0").as_bytes())
                            .map_err(|_| LoadError::MissingFunction { path: path.clone(), name: stringify!($name) })?;
                    )*
                    let log_set_callback = library.get::<unsafe extern "C" fn(Option<LogCallback>) -> sys::EOS_EResult>(b"EOS_Logging_SetCallback\0").ok().map(|s| *s);
                    let log_set_level = library.get::<unsafe extern "C" fn(i32, i32) -> sys::EOS_EResult>(b"EOS_Logging_SetLogLevel\0").ok().map(|s| *s);
                    Ok(Self { $($name,)* log_set_callback, log_set_level, _library: library })
                }
            }
        }
    };
}

/// `EOS_LogMessage` of eos_logging.h, which `eos-sys` does not bind.
#[repr(C)]
pub struct LogMessage {
    pub category: *const c_char,
    pub message: *const c_char,
    pub level: i32,
}
pub type LogCallback = unsafe extern "C" fn(*const LogMessage);
/// `EOS_LC_ALL_CATEGORIES`.
pub const LOG_ALL_CATEGORIES: i32 = 0x7fff_ffff;
/// `EOS_LOG_Warning`.
pub const LOG_WARNING: i32 = 300;
/// `EOS_LOG_Info`.
pub const LOG_INFO: i32 = 400;

api! {
    EOS_Initialize(*const sys::EOS_InitializeOptions) -> sys::EOS_EResult;
    EOS_Shutdown() -> sys::EOS_EResult;
    EOS_Platform_Create(*const sys::EOS_Platform_Options) -> sys::EOS_HPlatform;
    EOS_Platform_Release(sys::EOS_HPlatform);
    EOS_Platform_Tick(sys::EOS_HPlatform);
    EOS_Platform_GetConnectInterface(sys::EOS_HPlatform) -> sys::EOS_HConnect;
    EOS_Platform_GetLobbyInterface(sys::EOS_HPlatform) -> sys::EOS_HLobby;
    EOS_Platform_GetP2PInterface(sys::EOS_HPlatform) -> sys::EOS_HP2P;
    EOS_EResult_ToString(sys::EOS_EResult) -> *const c_char;
    EOS_ProductUserId_IsValid(sys::EOS_ProductUserId) -> sys::EOS_Bool;
    EOS_ProductUserId_ToString(sys::EOS_ProductUserId, *mut c_char, *mut i32) -> sys::EOS_EResult;
    EOS_ProductUserId_FromString(*const c_char) -> sys::EOS_ProductUserId;

    EOS_Connect_CreateDeviceId(sys::EOS_HConnect, *const sys::EOS_Connect_CreateDeviceIdOptions, *mut c_void, sys::EOS_Connect_OnCreateDeviceIdCallback);
    EOS_Connect_DeleteDeviceId(sys::EOS_HConnect, *const sys::EOS_Connect_DeleteDeviceIdOptions, *mut c_void, sys::EOS_Connect_OnDeleteDeviceIdCallback);
    EOS_Connect_Login(sys::EOS_HConnect, *const sys::EOS_Connect_LoginOptions, *mut c_void, sys::EOS_Connect_OnLoginCallback);
    EOS_Connect_CreateUser(sys::EOS_HConnect, *const sys::EOS_Connect_CreateUserOptions, *mut c_void, sys::EOS_Connect_OnCreateUserCallback);
    EOS_Connect_AddNotifyAuthExpiration(sys::EOS_HConnect, *const sys::EOS_Connect_AddNotifyAuthExpirationOptions, *mut c_void, sys::EOS_Connect_OnAuthExpirationCallback) -> sys::EOS_NotificationId;
    EOS_Connect_RemoveNotifyAuthExpiration(sys::EOS_HConnect, sys::EOS_NotificationId);
    EOS_Connect_AddNotifyLoginStatusChanged(sys::EOS_HConnect, *const sys::EOS_Connect_AddNotifyLoginStatusChangedOptions, *mut c_void, sys::EOS_Connect_OnLoginStatusChangedCallback) -> sys::EOS_NotificationId;
    EOS_Connect_RemoveNotifyLoginStatusChanged(sys::EOS_HConnect, sys::EOS_NotificationId);

    EOS_Lobby_CreateLobby(sys::EOS_HLobby, *const sys::EOS_Lobby_CreateLobbyOptions, *mut c_void, sys::EOS_Lobby_OnCreateLobbyCallback);
    EOS_Lobby_DestroyLobby(sys::EOS_HLobby, *const sys::EOS_Lobby_DestroyLobbyOptions, *mut c_void, sys::EOS_Lobby_OnDestroyLobbyCallback);
    EOS_Lobby_JoinLobby(sys::EOS_HLobby, *const sys::EOS_Lobby_JoinLobbyOptions, *mut c_void, sys::EOS_Lobby_OnJoinLobbyCallback);
    EOS_Lobby_LeaveLobby(sys::EOS_HLobby, *const sys::EOS_Lobby_LeaveLobbyOptions, *mut c_void, sys::EOS_Lobby_OnLeaveLobbyCallback);
    EOS_Lobby_UpdateLobbyModification(sys::EOS_HLobby, *const sys::EOS_Lobby_UpdateLobbyModificationOptions, *mut sys::EOS_HLobbyModification) -> sys::EOS_EResult;
    EOS_Lobby_UpdateLobby(sys::EOS_HLobby, *const sys::EOS_Lobby_UpdateLobbyOptions, *mut c_void, sys::EOS_Lobby_OnUpdateLobbyCallback);
    EOS_Lobby_CreateLobbySearch(sys::EOS_HLobby, *const sys::EOS_Lobby_CreateLobbySearchOptions, *mut sys::EOS_HLobbySearch) -> sys::EOS_EResult;
    EOS_LobbyModification_AddAttribute(sys::EOS_HLobbyModification, *const sys::EOS_LobbyModification_AddAttributeOptions) -> sys::EOS_EResult;
    EOS_LobbyModification_Release(sys::EOS_HLobbyModification);
    EOS_LobbyDetails_CopyInfo(sys::EOS_HLobbyDetails, *const sys::EOS_LobbyDetails_CopyInfoOptions, *mut *mut sys::EOS_LobbyDetails_Info) -> sys::EOS_EResult;
    EOS_LobbyDetails_Info_Release(*mut sys::EOS_LobbyDetails_Info);
    EOS_LobbyDetails_CopyAttributeByKey(sys::EOS_HLobbyDetails, *const sys::EOS_LobbyDetails_CopyAttributeByKeyOptions, *mut *mut sys::EOS_Lobby_Attribute) -> sys::EOS_EResult;
    EOS_Lobby_Attribute_Release(*mut sys::EOS_Lobby_Attribute);
    EOS_LobbyDetails_Release(sys::EOS_HLobbyDetails);
    EOS_LobbySearch_SetParameter(sys::EOS_HLobbySearch, *const sys::EOS_LobbySearch_SetParameterOptions) -> sys::EOS_EResult;
    EOS_LobbySearch_Find(sys::EOS_HLobbySearch, *const sys::EOS_LobbySearch_FindOptions, *mut c_void, sys::EOS_LobbySearch_OnFindCallback);
    EOS_LobbySearch_GetSearchResultCount(sys::EOS_HLobbySearch, *const sys::EOS_LobbySearch_GetSearchResultCountOptions) -> u32;
    EOS_LobbySearch_CopySearchResultByIndex(sys::EOS_HLobbySearch, *const sys::EOS_LobbySearch_CopySearchResultByIndexOptions, *mut sys::EOS_HLobbyDetails) -> sys::EOS_EResult;
    EOS_LobbySearch_Release(sys::EOS_HLobbySearch);

    EOS_P2P_SendPacket(sys::EOS_HP2P, *const sys::EOS_P2P_SendPacketOptions) -> sys::EOS_EResult;
    EOS_P2P_GetNextReceivedPacketSize(sys::EOS_HP2P, *const sys::EOS_P2P_GetNextReceivedPacketSizeOptions, *mut u32) -> sys::EOS_EResult;
    EOS_P2P_ReceivePacket(sys::EOS_HP2P, *const sys::EOS_P2P_ReceivePacketOptions, *mut sys::EOS_ProductUserId, *mut sys::EOS_P2P_SocketId, *mut u8, *mut c_void, *mut u32) -> sys::EOS_EResult;
    EOS_P2P_AddNotifyPeerConnectionRequest(sys::EOS_HP2P, *const sys::EOS_P2P_AddNotifyPeerConnectionRequestOptions, *mut c_void, sys::EOS_P2P_OnIncomingConnectionRequestCallback) -> sys::EOS_NotificationId;
    EOS_P2P_RemoveNotifyPeerConnectionRequest(sys::EOS_HP2P, sys::EOS_NotificationId);
    EOS_P2P_AddNotifyPeerConnectionClosed(sys::EOS_HP2P, *const sys::EOS_P2P_AddNotifyPeerConnectionClosedOptions, *mut c_void, sys::EOS_P2P_OnRemoteConnectionClosedCallback) -> sys::EOS_NotificationId;
    EOS_P2P_RemoveNotifyPeerConnectionClosed(sys::EOS_HP2P, sys::EOS_NotificationId);
    EOS_P2P_AcceptConnection(sys::EOS_HP2P, *const sys::EOS_P2P_AcceptConnectionOptions) -> sys::EOS_EResult;
    EOS_P2P_CloseConnections(sys::EOS_HP2P, *const sys::EOS_P2P_CloseConnectionsOptions) -> sys::EOS_EResult;
    EOS_P2P_SetRelayControl(sys::EOS_HP2P, *const sys::EOS_P2P_SetRelayControlOptions) -> sys::EOS_EResult;
}

/// Finds and opens the library.
pub(crate) fn load() -> Result<Api, LoadError> {
    let places = places();
    let mut searched = Vec::new();
    for dir in places {
        let path = dir.join(LIBRARY_FILE);
        if !path.is_file() {
            searched.push(dir);
            continue;
        }
        // SAFETY: loading runs the library's start-up code; it is Epic's SDK at
        // a full path beside the program, or where the developer pointed.
        let library = unsafe { Library::new(&path) }.map_err(|e| LoadError::Unloadable { path: path.clone(), why: e.to_string() })?;
        return Api::resolve(library, path);
    }
    Err(LoadError::Missing { searched })
}
