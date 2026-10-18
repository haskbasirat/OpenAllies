//! Native request/decree control flow (template `TGC_RequestDecreeOrder<T>`).
//!
//! Host side, `T::ProcessRequest` (e.g. 0x61FB40), in this order:
//! 1. reject when there is no ordering client (0x766C30: admin+0x284 player,
//!    its +4 client), log line 0x114;
//! 2. versioned orders: reject when the client has not acknowledged the order
//!    version (0x766C40: client+0x6C), line 0x118;
//! 3. film-checked orders: reject while the session plays a film
//!    (0x766C70: session+0xE4 == 4), line 0x11C;
//! 4. order-specific validity (inline cheat permission, 0x5092F0 ...);
//! 5. sync-log A "o.GetBitSize()" = decree-stream bits, write the 9-bit decree
//!    id (0x766D00) and the payload re-serialized from the resolved order into
//!    admin+0x1F8, sync-log A decree name and id, then S name/id when the
//!    instantiation has them and the decree is not TGC_SetPlayerOrderDecree,
//!    execute the decree locally, query the mirror (0x766C50, admin+0x268) and
//!    re-serialize into admin+0x270 when set, and finally sync-log A
//!    "o.GetBitSize()" again unless the decree is TGC_PlayerJoinOrderDecree.
//!
//! Client side the decree handler decodes, logs, executes and mirrors.
//!
//! Which steps an instantiation contains is a property of the compiled
//! template; [`shape`] is the table recorded from every original handler by
//! research/phase8-orders/native_order_flow.py (order-flow.json).
use crate::commands::WireWriter;
use crate::payload::{self, Field, Resolver, SchemaContext};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

include!("flow_table.rs");

pub const SET_PLAYER_DECREE: usize = 298;
pub const PLAYER_JOIN_DECREE: usize = 317;
/// Width of an order descriptor id for the native 357-entry table (0x40C7C0(357)).
pub const DESCRIPTOR_BITS: u8 = 9;

/// Validity check compiled into a request instantiation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Validity {
    None,
    /// Inline `[[0xB39AB0]+0x10C] != 0 && [[0xB39AB0]+0x124] == 0` (KKC_Cheat*).
    CheatsEnabled,
    /// Member function returning bool (e.g. 0x5092F0 KKC_TeamCommand::IsValid).
    Function(u32),
}

/// Static shape of one native handler instantiation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shape {
    /// Descriptor ordinal of the handler.
    pub descriptor: u16,
    /// Request handlers: decree descriptor written; decree handlers: itself.
    pub decree: u16,
    pub request: bool,
    pub version_check: bool,
    pub film_check: bool,
    pub validity: Validity,
    /// Synchronizer S receives the decree name/id (when not SetPlayer).
    pub s_log: bool,
    /// The handler queries 0x766C50 and mirrors the decree.
    pub mirror: bool,
    /// Request handlers: trailing A "o.GetBitSize()" (skipped for PlayerJoin).
    pub trailing_bits_log: bool,
    /// Native process function (the order's effect, external to this crate).
    pub process: u32,
}

pub fn shape(descriptor: usize) -> Result<&'static Shape> {
    SHAPES
        .get(descriptor)
        .ok_or(Error::UnsupportedNativeDescriptor(descriptor))
}

/// Native decree name (the string hashed by the synchronizer).
pub fn decree_name(descriptor: usize) -> Result<&'static str> {
    crate::native::DESCRIPTORS
        .get(descriptor)
        .map(|d| d.name)
        .ok_or(Error::UnsupportedNativeDescriptor(descriptor))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Log {
    /// Synchronizer A (0xB3655C): validated by TGC_SetSyncValue.
    A,
    /// Synchronizer S (0xB36560): simulation log, film validation.
    S,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncEntry {
    Int { log: Log, label: &'static str, value: u32 },
    Text { log: Log, label: &'static str, value: &'static str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestOutcome {
    /// Log line 0x114.
    NoOrderingClient,
    /// Log line 0x118.
    VersionNotAcknowledged,
    /// Log line 0x11C.
    WouldInterfereWithFilm,
    /// The instantiation's validity check failed (no decree written).
    Invalid,
    Accepted,
}

/// Explicit host state and effects. There is no default: every leaf is the
/// real admin/session state or a ported gameplay/lobby implementation.
pub trait OrderHost {
    /// 0x766C30: admin+0x284 ordering player and its +4 client exist.
    fn has_ordering_client(&mut self) -> bool;
    /// 0x766C40: ordering client +0x6C.
    fn version_acknowledged(&mut self) -> bool;
    /// 0x766C70: session+0xE4 == 4.
    fn film_session(&mut self) -> bool;
    /// Instantiation validity; the decoded payload is supplied.
    fn valid(&mut self, validity: Validity, request: usize, fields: &[Field]) -> Result<bool>;
    fn sync(&mut self, entry: SyncEntry) -> Result<()>;
    /// The order's process function (native `shape.process`). It may change
    /// the resolved order (e.g. a permission callback replacing a command);
    /// the mirror then serializes the changed fields, like the native object.
    fn process(&mut self, decree: usize, fields: &mut Vec<Field>) -> Result<()>;
    /// 0x766C50: admin+0x268 film recorder attached.
    fn mirror_enabled(&mut self) -> bool;
}

/// Required native owner reads may fail before returning a boolean. The
/// portable OrderHost adapter and live Admin adapter share the same kernels.
pub trait TryOrderHost {
    fn has_ordering_client(&mut self)->Result<bool>;
    fn version_acknowledged(&mut self)->Result<bool>;
    fn film_session(&mut self)->Result<bool>;
    fn valid(&mut self,validity:Validity,request:usize,fields:&[Field])->Result<bool>;
    fn sync(&mut self,entry:SyncEntry)->Result<()>;
    fn process(&mut self,decree:usize,fields:&mut Vec<Field>)->Result<()>;
    fn mirror_enabled(&mut self)->Result<bool>;
}
struct PortableHost<'a>(&'a mut dyn OrderHost);
impl TryOrderHost for PortableHost<'_> {
    fn has_ordering_client(&mut self)->Result<bool>{Ok(self.0.has_ordering_client())}
    fn version_acknowledged(&mut self)->Result<bool>{Ok(self.0.version_acknowledged())}
    fn film_session(&mut self)->Result<bool>{Ok(self.0.film_session())}
    fn valid(&mut self,v:Validity,r:usize,f:&[Field])->Result<bool>{self.0.valid(v,r,f)}
    fn sync(&mut self,e:SyncEntry)->Result<()>{self.0.sync(e)}
    fn process(&mut self,d:usize,f:&mut Vec<Field>)->Result<()>{self.0.process(d,f)}
    fn mirror_enabled(&mut self)->Result<bool>{Ok(self.0.mirror_enabled())}
}

fn bits(w: &WireWriter) -> Result<u32> {
    let p = w.position().absolute()?;
    u32::try_from(p - 3).map_err(|_| Error::InvalidPosition)
}

fn write_order(
    w: &mut WireWriter,
    decree: usize,
    fields: &[Field],
    context: &dyn SchemaContext,
) -> Result<()> {
    w.write_uint(decree as u32, DESCRIPTOR_BITS)?;
    payload::encode(decree, fields, w, context)
}

fn log_decree(host: &mut dyn TryOrderHost, s: &Shape) -> Result<()> {
    let decree = s.decree as usize;
    let name = decree_name(decree)?;
    host.sync(SyncEntry::Text { log: Log::A, label: "m_decree_type.m_name", value: name })?;
    host.sync(SyncEntry::Int { log: Log::A, label: "m_decree_type.m_id", value: decree as u32 })?;
    if s.s_log && decree != SET_PLAYER_DECREE {
        host.sync(SyncEntry::Text { log: Log::S, label: "m_decree_type.m_name", value: name })?;
        host.sync(SyncEntry::Int { log: Log::S, label: "m_decree_type.m_id", value: decree as u32 })?;
    }
    Ok(())
}

/// Host-side `ProcessRequest` of an already decoded request payload.
/// `fields` are the request's decoded fields; the decree is serialized from
/// their resolved form ([`payload::normalize_for_decree`]). Errors are
/// nontransactional (earlier writes/effects remain).
#[allow(clippy::too_many_arguments)]
pub fn process_request(
    request: usize,
    fields: &[Field],
    host: &mut dyn OrderHost,
    resolver: &dyn Resolver,
    context: &dyn SchemaContext,
    decree_stream: &mut WireWriter,
    mirror_stream: &mut WireWriter,
) -> Result<RequestOutcome> {
    try_process_request(request,fields,&mut PortableHost(host),resolver,context,decree_stream,mirror_stream)
}

/// Fallible owner form of the same native request coordinator.
#[allow(clippy::too_many_arguments)]
pub fn try_process_request(
    request: usize,
    fields: &[Field],
    host: &mut dyn TryOrderHost,
    resolver: &dyn Resolver,
    context: &dyn SchemaContext,
    decree_stream: &mut WireWriter,
    mirror_stream: &mut WireWriter,
) -> Result<RequestOutcome> {
    let s = shape(request)?;
    if !s.request {
        return Err(Error::UnsupportedNativeDescriptor(request));
    }
    if !host.has_ordering_client()? {
        return Ok(RequestOutcome::NoOrderingClient);
    }
    if s.version_check && !host.version_acknowledged()? {
        return Ok(RequestOutcome::VersionNotAcknowledged);
    }
    if s.film_check && host.film_session()? {
        return Ok(RequestOutcome::WouldInterfereWithFilm);
    }
    if s.validity != Validity::None && !host.valid(s.validity, request, fields)? {
        return Ok(RequestOutcome::Invalid);
    }
    let decree = s.decree as usize;
    // The decree is serialized from the resolved request object before the
    // order executes; the process sees the order itself (unresolved references
    // stay unresolved for the host, as the native null pointers do).
    let resolved = payload::reserialize(request, fields, resolver, context)?;
    host.sync(SyncEntry::Int { log: Log::A, label: "o.GetBitSize()", value: bits(decree_stream)? })?;
    write_order(decree_stream, decree, &resolved, context)?;
    log_decree(host, s)?;
    let mut order = fields.to_vec();
    host.process(decree, &mut order)?;
    if s.mirror && host.mirror_enabled()? {
        let mirrored = payload::reserialize(request, &order, resolver, context)?;
        write_order(mirror_stream, decree, &mirrored, context)?;
    }
    if s.trailing_bits_log && decree != PLAYER_JOIN_DECREE {
        host.sync(SyncEntry::Int { log: Log::A, label: "o.GetBitSize()", value: bits(decree_stream)? })?;
    }
    Ok(RequestOutcome::Accepted)
}

/// Decree handler after decoding (any node): logs, executes, mirrors.
pub fn process_decree(
    decree: usize,
    fields: &[Field],
    host: &mut dyn OrderHost,
    resolver: &dyn Resolver,
    context: &dyn SchemaContext,
    mirror_stream: &mut WireWriter,
) -> Result<()> {
    try_process_decree(decree,fields,&mut PortableHost(host),resolver,context,mirror_stream)
}

/// Fallible owner form of the same native decree coordinator.
pub fn try_process_decree(
    decree: usize,
    fields: &[Field],
    host: &mut dyn TryOrderHost,
    resolver: &dyn Resolver,
    context: &dyn SchemaContext,
    mirror_stream: &mut WireWriter,
) -> Result<()> {
    let s = shape(decree)?;
    if s.request {
        return Err(Error::UnsupportedNativeDescriptor(decree));
    }
    log_decree(host, s)?;
    let mut order = fields.to_vec();
    host.process(decree, &mut order)?;
    if s.mirror && host.mirror_enabled()? {
        // The mirror re-serializes the (possibly changed) resolved order.
        let mirrored = payload::reserialize(decree, &order, resolver, context)?;
        write_order(mirror_stream, decree, &mirrored, context)?;
    }
    Ok(())
}
