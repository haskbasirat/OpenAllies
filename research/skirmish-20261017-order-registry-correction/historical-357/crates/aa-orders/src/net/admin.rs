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
//!   After each peer, including an empty peer, admin+0x208 = admin+0x284
//!   and admin+0x284 = null.
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
use crate::flow::{self, Log, OrderHost, SyncEntry, TryOrderHost};
use crate::payload::{self, Field, Resolver, SchemaContext};
use crate::{
    ClientState, Error, Event, GateSource, History, Packet, Position, Registry, Result, consume_next,
    dispatch_observed, dispatch_observed_fallible, native,
};

mod ordering;
pub use ordering::OrderingHost;
use ordering::{Host, ScopedHost};

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

/// Portable identity of an actual retained client owner. Remote tokens must
/// come from the transport's stable owner arena, never list position or GTID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientIdentity {
    OwnNode,
    Remote(u64),
}
/// Exact native Admin280 pointer state, with a separate unbound legacy-fixture
/// domain. A portable fixture cannot authorize production gameplay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentClient {
    Null,
    Bound(ClientIdentity),
    UnboundPortable,
}
pub struct IdentifiedPeer<'a> {
    pub peer: &'a mut Peer,
    pub client: ClientIdentity,
}

///A synchronous loan of SAME Admin254/210 while a decree callback runs.
///Native callbacks may emit requests before the current decree returns; no
///second stream or deferred-after-callback queue is introduced here.
pub struct RequestEmission<'a> {
    stream: &'a mut WireWriter,
    last_player: &'a mut Option<u32>,
}
impl RequestEmission<'_> {
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
        if *self.last_player != Some(player) {
            self.stream
                .write_uint(SET_PLAYER_REQUEST as u32, flow::DESCRIPTOR_BITS)?;
            self.stream.write_uint(player, 32)?;
            *self.last_player = Some(player);
        }
        self.stream
            .write_uint(request as u32, flow::DESCRIPTOR_BITS)?;
        payload::encode(request, fields, self.stream, context)?;
        Ok(true)
    }
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
    current_client_280: CurrentClient,
    /// +0x210 player id of the last SetPlayer request written.
    pub last_request_player: Option<u32>,
    /// +0x208 previous ordering player. The sole +0x284 owner is
    /// `client.field_284`; zero represents the native null pointer.
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
            // Derived Node constructor75CE80 writes+280=null.
            current_client_280: CurrentClient::Null,
            last_request_player: None,
            previous_ordering_player: None,
            film_ordering_player: None,
        })
    }

    /// Native Admin+0x284. This reads the same slot written by 0x74FD10
    /// and 0x74B3A0; Node/Peer+0x2C is a separate owner.
    pub fn ordering_player(&self) -> Option<u32> {
        nonnull(self.client.field_284)
    }

    /// Scoped native slot for consumers already holding the other Admin fields.
    pub fn ordering_player_slot(&self) -> &u32 {
        &self.client.field_284
    }

    /// Disjoint loans of the same native pointer slots for direct75D710
    /// calls; neither slot is copied or supplied by a detached context.
    pub fn ordering_context(&mut self) -> (&mut u32, &CurrentClient) {
        (&mut self.client.field_284, &self.current_client_280)
    }

    pub fn current_client(&self) -> &CurrentClient {
        &self.current_client_280
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
        RequestEmission {
            stream: &mut self.request_stream,
            last_player: &mut self.last_request_player,
        }
        .emit_request(active, request, fields, player, context)
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
        let index=self.seal_decrees_packet()?;
        host.sync(SyncEntry::Int {
            log: Log::A,
            label: "m_decree_history.GetMax()-1",
            value: (self.decrees.end() - 1) as u32,
        })?;
        Ok(index)
    }
    ///Same752EA0 stores/log, lending the actual Admin284/280 to the live host.
    pub fn seal_decrees_with_ordering(&mut self,host:&mut dyn OrderingHost)->Result<i32>{
        let index=self.seal_decrees_packet()?;
        host.sync(&mut self.client.field_284,&self.current_client_280,SyncEntry::Int{
            log:Log::A,label:"m_decree_history.GetMax()-1",value:self.decrees.end().wrapping_sub(1)as u32})?;
        Ok(index)
    }
    fn seal_decrees_packet(&mut self)->Result<i32>{
        let index=Self::seal(&mut self.decree_stream,&mut self.decrees)?;
        let floor=self.decrees.end()-DECREE_HISTORY_RETENTION;
        if self.decrees.begin()<floor{self.decrees.retire_before(floor)?}Ok(index)
    }

    /// Existing portable host entry point. Gameplay that reads/writes Admin284
    /// must use `host_execute_with_ordering` or `host_execute_owned_requests`.
    pub fn host_execute(
        &mut self,
        peers: &mut [Peer],
        host: &mut dyn OrderHost,
        resolver: &dyn Resolver,
        context: &dyn SchemaContext,
        gate: &mut dyn GateSource,
    ) -> Result<()> {
        self.host_execute_inner(peers, Host::Portable(host), resolver, context, gate)
    }

    /// 0x752E60 with a live shared Admin284 borrow in every host callback.
    pub fn host_execute_with_ordering(
        &mut self,
        peers: &mut [IdentifiedPeer<'_>],
        host: &mut dyn OrderingHost,
        resolver: &dyn Resolver,
        context: &dyn SchemaContext,
        gate: &mut dyn GateSource,
    ) -> Result<()> {
        let mut host = Host::Live(host);
        for identified in peers {
            self.current_client_280 = CurrentClient::Bound(identified.client);
            let peer = &mut *identified.peer;
            execute_peer(
                &self.registry,
                &mut self.decree_stream,
                &mut self.mirror_stream,
                &mut self.request_stream,
                &mut self.last_request_player,
                &mut self.client.field_284,
                &self.current_client_280,
                &mut self.previous_ordering_player,
                &mut peer.requests,
                peer.ordering_player,
                &mut host,
                resolver,
                context,
                gate,
            )?;
        }
        self.current_client_280 = CurrentClient::Null;
        Ok(())
    }

    fn host_execute_inner(
        &mut self,
        peers: &mut [Peer],
        mut host: Host<'_>,
        resolver: &dyn Resolver,
        context: &dyn SchemaContext,
        gate: &mut dyn GateSource,
    ) -> Result<()> {
        for peer in peers {
            self.current_client_280 = CurrentClient::UnboundPortable;
            execute_peer(
                &self.registry,
                &mut self.decree_stream,
                &mut self.mirror_stream,
                &mut self.request_stream,
                &mut self.last_request_player,
                &mut self.client.field_284,
                &self.current_client_280,
                &mut self.previous_ordering_player,
                &mut peer.requests,
                peer.ordering_player,
                &mut host,
                resolver,
                context,
                gate,
            )?;
        }
        self.current_client_280 = CurrentClient::Null;
        Ok(())
    }

    /// Native 0x75AEE0 puts the host node first in +0x2AC. Borrow its actual
    /// +0x0C history here, then the remote peer histories; no history snapshot
    /// or copy-back is created. `own_player` is that same host Node+0x2C read.
    #[allow(clippy::too_many_arguments)]
    pub fn host_execute_owned_requests(
        &mut self,
        own_player: Option<u32>,
        remote_peers: &mut [IdentifiedPeer<'_>],
        host: &mut dyn OrderingHost,
        resolver: &dyn Resolver,
        context: &dyn SchemaContext,
        gate: &mut dyn GateSource,
    ) -> Result<()> {
        let mut host = Host::Live(host);
        self.current_client_280 = CurrentClient::Bound(ClientIdentity::OwnNode);
        execute_peer(
            &self.registry,
            &mut self.decree_stream,
            &mut self.mirror_stream,
            &mut self.request_stream,
            &mut self.last_request_player,
            &mut self.client.field_284,
            &self.current_client_280,
            &mut self.previous_ordering_player,
            &mut self.requests,
            own_player,
            &mut host,
            resolver,
            context,
            gate,
        )?;
        for identified in remote_peers {
            self.current_client_280 = CurrentClient::Bound(identified.client);
            let peer = &mut *identified.peer;
            execute_peer(
                &self.registry,
                &mut self.decree_stream,
                &mut self.mirror_stream,
                &mut self.request_stream,
                &mut self.last_request_player,
                &mut self.client.field_284,
                &self.current_client_280,
                &mut self.previous_ordering_player,
                &mut peer.requests,
                peer.ordering_player,
                &mut host,
                resolver,
                context,
                gate,
            )?;
        }
        self.current_client_280 = CurrentClient::Null;
        Ok(())
    }

    /// Existing portable 0x74FD10 entry point.
    pub fn client_execute(
        &mut self,
        index: i32,
        host: &mut dyn OrderHost,
        resolver: &dyn Resolver,
        context: &dyn SchemaContext,
        gate: &mut dyn GateSource,
    ) -> Result<()> {
        self.client_execute_inner(index, Host::Portable(host), resolver, context, gate)
    }

    /// 0x74FD10: callbacks read/write the canonical +0x284 slot installed
    /// from +0x250 before dispatch and cleared only after a successful packet.
    pub fn client_execute_with_ordering(
        &mut self,
        index: i32,
        host: &mut dyn OrderingHost,
        resolver: &dyn Resolver,
        context: &dyn SchemaContext,
        gate: &mut dyn GateSource,
    ) -> Result<()> {
        self.client_execute_inner(index, Host::Live(host), resolver, context, gate)
    }

    fn client_execute_inner(
        &mut self,
        index: i32,
        mut host: Host<'_>,
        resolver: &dyn Resolver,
        context: &dyn SchemaContext,
        gate: &mut dyn GateSource,
    ) -> Result<()> {
        self.client.next_index = index;
        let registry = &self.registry;
        let mirror = &mut self.mirror_stream;
        let request_stream = &mut self.request_stream;
        let last_request_player = &mut self.last_request_player;
        let result = consume_next(
            &mut self.decrees,
            &mut self.client,
            |packet, start, state, _history| {
                let scoped = ScopedHost {
                    host: &mut host,
                    slot: &mut state.field_284,
                    client: &self.current_client_280,
                    request_stream,
                    last_request_player,
                };
                let host_cell = std::cell::RefCell::new(scoped);
                dispatch_observed_fallible(
                    packet,
                    start,
                    registry,
                    true,
                    gate,
                    |descriptor, cursor, _gate| {
                        let fields = payload::decode(descriptor, cursor, context)
                            .map_err(|e| e.to_string())?;
                        flow::try_process_decree(
                            descriptor,
                            &fields,
                            &mut *host_cell.borrow_mut(),
                            resolver,
                            context,
                            mirror,
                        )
                        .map_err(|e| e.to_string())
                    },
                    |event| {
                        if let Event::DiagnosticBits { bits } = event {
                            host_cell.borrow_mut().sync(SyncEntry::Int {
                                log: Log::A,
                                label: "i.GetBitSize()",
                                value: *bits as u32,
                            })?;
                        }
                        Ok(())
                    },
                )
            },
        );
        let receipt = result?;
        ScopedHost {
            host: &mut host,
            slot: &mut self.client.field_284,
            client: &self.current_client_280,
            request_stream: &mut self.request_stream,
            last_request_player: &mut self.last_request_player,
        }
        .sync(SyncEntry::Int {
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

    /// Existing portable film entry point.
    pub fn play_film_record(
        &mut self,
        record: &[u8],
        host: &mut dyn OrderHost,
        resolver: &dyn Resolver,
        context: &dyn SchemaContext,
        gate: &mut dyn GateSource,
    ) -> Result<()> {
        self.play_film_inner(record, Host::Portable(host), resolver, context, gate)
    }

    /// 0x751A40 with the real film/current player stores on the same +0x284.
    pub fn play_film_record_with_ordering(
        &mut self,
        record: &[u8],
        host: &mut dyn OrderingHost,
        resolver: &dyn Resolver,
        context: &dyn SchemaContext,
        gate: &mut dyn GateSource,
    ) -> Result<()> {
        self.play_film_inner(record, Host::Live(host), resolver, context, gate)
    }

    fn play_film_inner(
        &mut self,
        record: &[u8],
        mut host: Host<'_>,
        resolver: &dyn Resolver,
        context: &dyn SchemaContext,
        gate: &mut dyn GateSource,
    ) -> Result<()> {
        let length = u32::from_le_bytes(
            record
                .get(..4)
                .ok_or(Error::InvalidPacket)?
                .try_into()
                .unwrap(),
        ) as usize;
        let body = record.get(4..4 + length).ok_or(Error::InvalidPacket)?;
        let packet = Packet::new(body.to_vec())?;
        self.client.field_284 = self.film_ordering_player.unwrap_or(0);
        gate.store_session_byte(1)?;
        let mut scoped = ScopedHost {
            host: &mut host,
            slot: &mut self.client.field_284,
            client: &self.current_client_280,
            request_stream: &mut self.request_stream,
            last_request_player: &mut self.last_request_player,
        };
        let result = dispatch_observed(
            Some(&packet),
            None,
            &self.registry,
            false,
            gate,
            |descriptor, cursor, _gate| {
                let fields =
                    payload::decode(descriptor, cursor, context).map_err(|e| e.to_string())?;
                flow::try_process_decree(
                    descriptor,
                    &fields,
                    &mut scoped,
                    resolver,
                    context,
                    &mut self.mirror_stream,
                )
                .map_err(|e| e.to_string())
            },
            |_| {},
        );
        gate.store_session_byte(0)?;
        self.film_ordering_player = nonnull(std::mem::take(&mut self.client.field_284));
        result.map(drop)
    }
}

/// Body bits of a packet: `len*8 - 3 - pad` (0x751B60 skips empty packets).
pub fn body_bits(packet: &Packet) -> i64 {
    let b = packet.bytes();
    (b.len() as i64) * 8 - 3 - i64::from(b[0] & 7)
}

fn nonnull(raw: u32) -> Option<u32> {
    (raw != 0).then_some(raw)
}

/// 0x7509F0; the process callback borrows the real Admin284 slot.
fn set_player_decree(
    decree: &mut WireWriter,
    mirror: &mut WireWriter,
    player: u32,
    host: &mut dyn TryOrderHost,
    context: &dyn SchemaContext,
) -> Result<()> {
    host.sync(SyncEntry::Int {
        log: Log::A,
        label: "o.GetBitSize()",
        value: bits(decree)?,
    })?;
    let fields = [Field::Uint {
        width: 32,
        value: player,
    }];
    decree.write_uint(SET_PLAYER_DECREE as u32, flow::DESCRIPTOR_BITS)?;
    payload::encode(SET_PLAYER_DECREE, &fields, decree, context)?;
    host.sync(SyncEntry::Text {
        log: Log::A,
        label: "m_decree_type.m_name",
        value: "TGC_SetPlayerOrderDecree",
    })?;
    host.sync(SyncEntry::Int {
        log: Log::A,
        label: "m_decree_type.m_id",
        value: SET_PLAYER_DECREE as u32,
    })?;
    let mut order = fields.to_vec();
    host.process(SET_PLAYER_DECREE, &mut order)?;
    if host.mirror_enabled()? {
        mirror.write_uint(SET_PLAYER_DECREE as u32, flow::DESCRIPTOR_BITS)?;
        payload::encode(SET_PLAYER_DECREE, &order, mirror, context)?;
    }
    host.sync(SyncEntry::Int {
        log: Log::A,
        label: "o.GetBitSize()",
        value: bits(decree)?,
    })
}

#[allow(clippy::too_many_arguments)]
fn execute_peer(
    registry: &Registry,
    decree: &mut WireWriter,
    mirror: &mut WireWriter,
    request_stream: &mut WireWriter,
    last_request_player: &mut Option<u32>,
    slot: &mut u32,
    client: &CurrentClient,
    previous: &mut Option<u32>,
    requests: &mut History,
    player: Option<u32>,
    host: &mut Host<'_>,
    resolver: &dyn Resolver,
    context: &dyn SchemaContext,
    gate: &mut dyn GateSource,
) -> Result<()> {
    let mut announced = false;
    while requests.begin() < requests.end() {
        let index = requests.begin();
        let packet = requests.get(index)?.cloned();
        if let Some(packet) = packet.as_ref().filter(|p| body_bits(p) > 0) {
            let mut scoped = ScopedHost {
                host,
                slot,
                client,
                request_stream,
                last_request_player,
            };
            if !announced {
                set_player_decree(
                    decree,
                    mirror,
                    player.ok_or(Error::MissingCurrentPlayer)?,
                    &mut scoped,
                    context,
                )?;
                announced = true;
            }
            dispatch_observed(
                Some(packet),
                None,
                registry,
                false,
                gate,
                |descriptor, cursor, _gate| {
                    let fields =
                        payload::decode(descriptor, cursor, context).map_err(|e| e.to_string())?;
                    flow::try_process_request(
                        descriptor,
                        &fields,
                        &mut scoped,
                        resolver,
                        context,
                        decree,
                        mirror,
                    )
                    .map(drop)
                    .map_err(|e| e.to_string())
                },
                |_| {},
            )?;
        }
        requests.retire_before(index + 1)?;
    }
    // 0x751B60 copies/clears at the end of *each peer*, including empty ones.
    *previous = nonnull(std::mem::take(slot));
    Ok(())
}
