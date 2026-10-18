//! TGC_AdminNode order streams and histories (admin layer above the
//! transport). Native addresses refer to the original AA.exe.
//!
//! * Request stream admin+0x254 (client emitters such as 0x61E960): a
//!   TGC_SetPlayerOrderRequest (320, 32-bit player id, writer 0x75DC40) is
//!   written first whenever the issuing player differs from admin+0x210
//!   (0x74B360), then the 9-bit request id and payload.
//! * 0x752DA0 seals the request stream into one packet appended to this
//!   node's request history (admin+0xC; keeps sequences >= end-200).
//! * Host 0x752E60/0x751B60: for every peer of admin+0x2AC (the host node is
//!   first), every queued request packet with body bits is preceded, once per
//!   peer and tick, by a TGC_SetPlayerOrderDecree for the peer's ordering
//!   player (0x7509F0) and dispatched by 0x74D710 without diagnostics; each
//!   request runs `flow::process_request`. Packets are retired one by one.
//!   Afterwards admin+0x208 = admin+0x284 and admin+0x284 = null.
//! * 0x752EA0 seals the decree stream admin+0x1F8 as decree packet
//!   #end (history admin+0x214 keeps end-1000) and logs A
//!   "m_decree_history.GetMax()-1" = new end - 1.
//! * Client 0x74FD10 replays decree packet `index` with diagnostics (A
//!   "i.GetBitSize()" around every decree except after PlayerJoin), resets
//!   the resume fields and logs A "m_client_first_unprocessed_decree".
//! * Film: 0x74D640 records the finalized mirror stream as
//!   `[u32 LE byte length][bytes]` and resets it; 0x751A40 plays one record
//!   back with session+0x24 = 1 and the film ordering player admin+0x288.
//!
//! Sync-log entries are delivered through [`OrderHost::sync`] in native
//! order; synchronizer state lives with the host ([`super::sync`]).
use super::transport::finalize;
use crate::commands::WireWriter;
use crate::flow::{self, Log, OrderHost, SyncEntry};
use crate::payload::{self, Field, Resolver, SchemaContext};
use crate::{
    ClientState, Error, Event, Gate, History, Packet, Position, Registry, Result, consume_next,
    dispatch_observed, native,
};

pub const SET_PLAYER_REQUEST: usize = 320;
pub const SET_PLAYER_DECREE: usize = 298;
/// 0x752E0E: request history keeps sequences >= end-200.
pub const REQUEST_HISTORY_RETENTION: i32 = 200;
/// 0x752F14: host decree history keeps sequences >= end-1000.
pub const DECREE_HISTORY_RETENTION: i32 = 1000;

/// Queued request packets of one peer (AdminConn/peer base +0xC) and its
/// ordering player (+0x2C), identified by SessionPlayer id.
pub struct Peer {
    pub requests: History,
    pub ordering_player: Option<u32>,
}

pub struct AdminNode {
    pub registry: Registry,
    pub request_stream: WireWriter,
    pub decree_stream: WireWriter,
    pub mirror_stream: WireWriter,
    /// This node's sealed request packets (+0xC).
    pub requests: History,
    /// Decree packets (+0x214).
    pub decrees: History,
    pub client: ClientState,
    /// +0x210 player id of the last SetPlayer request written.
    pub last_request_player: Option<u32>,
    /// +0x284 ordering player id while dispatching, +0x208 previous one,
    /// +0x288 film ordering player.
    pub ordering_player: Option<u32>,
    pub previous_ordering_player: Option<u32>,
    pub film_ordering_player: Option<u32>,
}

fn stream() -> Result<WireWriter> {
    WireWriter::new(Position::PACKET_START)
}
fn bits(w: &WireWriter) -> Result<u32> {
    Ok((w.position().absolute()? - 3) as u32)
}

impl AdminNode {
    /// Histories start at `first_index` with the native initial capacities
    /// (0x400 requests 0x752A60, 0x100 decrees 0x74B790); both grow.
    pub fn new(first_index: i32) -> Result<Self> {
        Ok(Self {
            registry: native::registry()?,
            request_stream: stream()?,
            decree_stream: stream()?,
            mirror_stream: stream()?,
            requests: History::new(first_index, 0x400)?,
            decrees: History::new(first_index, 0x100)?,
            client: ClientState::new(first_index),
            last_request_player: None,
            ordering_player: None,
            previous_ordering_player: None,
            film_ordering_player: None,
        })
    }

    /// Request emitter template (0x61E960 family). `active` is 0x766CD0
    /// (admin+0x2C0); an inactive node drops the request like the original.
    pub fn emit_request(
        &mut self,
        active: bool,
        request: usize,
        fields: &[Field],
        player: u32,
        context: &dyn SchemaContext,
    ) -> Result<bool> {
        if !active {
            return Ok(false);
        }
        if self.last_request_player != Some(player) {
            self.request_stream.write_uint(SET_PLAYER_REQUEST as u32, flow::DESCRIPTOR_BITS)?;
            self.request_stream.write_uint(player, 32)?;
            self.last_request_player = Some(player);
        }
        self.request_stream.write_uint(request as u32, flow::DESCRIPTOR_BITS)?;
        payload::encode(request, fields, &mut self.request_stream, context)?;
        Ok(true)
    }

    fn seal(stream: &mut WireWriter, history: &mut History) -> Result<i32> {
        let packet = Packet::new(finalize(stream)?)?;
        *stream = WireWriter::new(Position::PACKET_START)?;
        let index = history.end();
        history.set_end(index.checked_add(1).ok_or(Error::IndexExhausted)?)?;
        history.put(index, Some(packet))?;
        Ok(index)
    }

    /// 0x752DA0.
    pub fn seal_requests(&mut self) -> Result<i32> {
        let index = Self::seal(&mut self.request_stream, &mut self.requests)?;
        let floor = self.requests.end() - REQUEST_HISTORY_RETENTION;
        if self.requests.begin() < floor {
            self.requests.retire_before(floor)?;
        }
        Ok(index)
    }

    /// 0x752EA0.
    pub fn seal_decrees(&mut self, host: &mut dyn OrderHost) -> Result<i32> {
        let index = Self::seal(&mut self.decree_stream, &mut self.decrees)?;
        let floor = self.decrees.end() - DECREE_HISTORY_RETENTION;
        if self.decrees.begin() < floor {
            self.decrees.retire_before(floor)?;
        }
        host.sync(SyncEntry::Int {
            log: Log::A,
            label: "m_decree_history.GetMax()-1",
            value: (self.decrees.end() - 1) as u32,
        })?;
        Ok(index)
    }

    /// 0x7509F0: host-generated TGC_SetPlayerOrderDecree for a peer's
    /// ordering player (A logs, no S log, process 0x75D710, mirror).
    fn set_player_decree(
        &mut self,
        player: u32,
        host: &mut dyn OrderHost,
        context: &dyn SchemaContext,
    ) -> Result<()> {
        host.sync(SyncEntry::Int { log: Log::A, label: "o.GetBitSize()", value: bits(&self.decree_stream)? })?;
        let fields = [Field::Uint { width: 32, value: player }];
        self.decree_stream.write_uint(SET_PLAYER_DECREE as u32, flow::DESCRIPTOR_BITS)?;
        payload::encode(SET_PLAYER_DECREE, &fields, &mut self.decree_stream, context)?;
        host.sync(SyncEntry::Text { log: Log::A, label: "m_decree_type.m_name", value: "TGC_SetPlayerOrderDecree" })?;
        host.sync(SyncEntry::Int { log: Log::A, label: "m_decree_type.m_id", value: SET_PLAYER_DECREE as u32 })?;
        let mut order = fields.to_vec();
        host.process(SET_PLAYER_DECREE, &mut order)?;
        if host.mirror_enabled() {
            self.mirror_stream.write_uint(SET_PLAYER_DECREE as u32, flow::DESCRIPTOR_BITS)?;
            payload::encode(SET_PLAYER_DECREE, &order, &mut self.mirror_stream, context)?;
        }
        host.sync(SyncEntry::Int { log: Log::A, label: "o.GetBitSize()", value: bits(&self.decree_stream)? })
    }

    /// 0x752E60 over `peers` (admin+0x2AC order) with 0x751B60 for each.
    /// `gate` is the 0x74D710 abort pair (session+0x24, admin+0x2E5).
    pub fn host_execute(
        &mut self,
        peers: &mut [Peer],
        host: &mut dyn OrderHost,
        resolver: &dyn Resolver,
        context: &dyn SchemaContext,
        gate: &mut Gate,
    ) -> Result<()> {
        for peer in peers.iter_mut() {
            let mut announced = false;
            while peer.requests.begin() < peer.requests.end() {
                let index = peer.requests.begin();
                let packet = peer.requests.get(index)?.cloned();
                if let Some(packet) = packet.as_ref().filter(|p| body_bits(p) > 0) {
                    if !announced {
                        let player = peer.ordering_player.ok_or(Error::MissingCurrentPlayer)?;
                        self.set_player_decree(player, host, context)?;
                        announced = true;
                    }
                    self.dispatch_requests(packet, host, resolver, context, gate)?;
                }
                peer.requests.retire_before(index + 1)?;
            }
        }
        self.previous_ordering_player = self.ordering_player.take();
        Ok(())
    }

    fn dispatch_requests(
        &mut self,
        packet: &Packet,
        host: &mut dyn OrderHost,
        resolver: &dyn Resolver,
        context: &dyn SchemaContext,
        gate: &mut Gate,
    ) -> Result<()> {
        let registry = std::mem::replace(&mut self.registry, Registry::new(vec![String::new()])?);
        let decree = &mut self.decree_stream;
        let mirror = &mut self.mirror_stream;
        let result = dispatch_observed(
            Some(packet),
            None,
            &registry,
            false,
            gate,
            |descriptor, cursor, _gate| {
                let fields = payload::decode(descriptor, cursor, context).map_err(|e| e.to_string())?;
                flow::process_request(descriptor, &fields, host, resolver, context, decree, mirror)
                    .map(drop)
                    .map_err(|e| e.to_string())
            },
            |_| {},
        );
        self.registry = registry;
        result.map(drop)
    }

    /// 0x74FD10 for decree packet `index`; the caller (tick driver) performs
    /// the final +0x20C increment of its own counter.
    pub fn client_execute(
        &mut self,
        index: i32,
        host: &mut dyn OrderHost,
        resolver: &dyn Resolver,
        context: &dyn SchemaContext,
        gate: &mut Gate,
    ) -> Result<()> {
        self.client.next_index = index;
        let registry = std::mem::replace(&mut self.registry, Registry::new(vec![String::new()])?);
        let mirror = &mut self.mirror_stream;
        let mut sync_error = None;
        let result = consume_next(&mut self.decrees, &mut self.client, |packet, start, _state, _history| {
            let host_cell = std::cell::RefCell::new(&mut *host);
            dispatch_observed(
                packet,
                start,
                &registry,
                true,
                gate,
                |descriptor, cursor, _gate| {
                    let fields = payload::decode(descriptor, cursor, context).map_err(|e| e.to_string())?;
                    let mut h = host_cell.borrow_mut();
                    flow::process_decree(descriptor, &fields, &mut **h, resolver, context, mirror)
                        .map_err(|e| e.to_string())
                },
                |event| {
                    if let Event::DiagnosticBits { bits } = event {
                        let r = host_cell.borrow_mut().sync(SyncEntry::Int {
                            log: Log::A,
                            label: "i.GetBitSize()",
                            value: *bits as u32,
                        });
                        if let Err(e) = r {
                            sync_error.get_or_insert(e);
                        }
                    }
                },
            )
        });
        self.registry = registry;
        let receipt = result?;
        if let Some(e) = sync_error {
            return Err(e);
        }
        host.sync(SyncEntry::Int {
            log: Log::A,
            label: "m_client_first_unprocessed_decree",
            value: receipt.completed_index as u32,
        })
    }

    /// 0x74D640: one film record of the mirror stream, which is then reset.
    pub fn record_film(&mut self) -> Result<Vec<u8>> {
        let bytes = finalize(&self.mirror_stream)?;
        self.mirror_stream = stream()?;
        let mut record = Vec::with_capacity(4 + bytes.len());
        record.extend((bytes.len() as u32).to_le_bytes());
        record.extend(bytes);
        Ok(record)
    }

    /// 0x751A40: dispatch one recorded film packet (session+0x24 set by the
    /// caller through `gate.session` for the duration, as natively).
    pub fn play_film_record(
        &mut self,
        record: &[u8],
        host: &mut dyn OrderHost,
        resolver: &dyn Resolver,
        context: &dyn SchemaContext,
        gate: &mut Gate,
    ) -> Result<()> {
        let length = u32::from_le_bytes(record.get(..4).ok_or(Error::InvalidPacket)?.try_into().unwrap()) as usize;
        let body = record.get(4..4 + length).ok_or(Error::InvalidPacket)?;
        let packet = Packet::new(body.to_vec())?;
        self.ordering_player = self.film_ordering_player;
        gate.session = true;
        let registry = std::mem::replace(&mut self.registry, Registry::new(vec![String::new()])?);
        let mirror = &mut self.mirror_stream;
        let result = dispatch_observed(
            Some(&packet),
            None,
            &registry,
            false,
            gate,
            |descriptor, cursor, _gate| {
                let fields = payload::decode(descriptor, cursor, context).map_err(|e| e.to_string())?;
                flow::process_decree(descriptor, &fields, host, resolver, context, mirror)
                    .map_err(|e| e.to_string())
            },
            |_| {},
        );
        self.registry = registry;
        gate.session = false;
        self.film_ordering_player = self.ordering_player.take();
        result.map(drop)
    }
}

/// Body bits of a packet: `len*8 - 3 - pad` (0x751B60 skips empty packets).
pub fn body_bits(packet: &Packet) -> i64 {
    let b = packet.bytes();
    (b.len() as i64) * 8 - 3 - i64::from(b[0] & 7)
}
