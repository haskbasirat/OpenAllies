//! TGC_NetNode / TGC_NetConn datagram layer over `std::net::UdpSocket`.
//!
//! Wire format and sequencing are the proven aa_orders::net::transport port
//! (header writer 0x764DA0/0x764CA0, receiver 0x764BB0/0x764600/0x764550,
//! finalizer 0x74D0A0); the admin packet type is 4 bits (writer 0x763A60,
//! reader 0x764950 = 0x411FA0(13)).
//!
//! Native caps and their replacement here (README section 10):
//! * 0x783D8F 64-entry fd_set / select(0x40): one socket per node, no
//!   socket-count limit.
//! * 0x783DC7 at most 32 receive dispatches per WinSock::Update: every
//!   pending datagram is drained ([`UdpNode::receive_all`]).
//! * 0x783BE5 4,096-byte static receive buffer: 65,535 bytes (the UDP
//!   maximum datagram).
//! * 0x76526D / 0x765133 4,096-byte send build buffers: growable
//!   `WireWriter`.
//! * NetLanBandwidth 200000.0 (0x4230BA), NetMaxBandwidthToUnknown 4000.0
//!   (0x423102), connection_types.tgi rates, +16 bytes accounting per
//!   datagram (0x765324), refill clamp (0x764FD7) in 0x764FA0/0x763E30: not
//!   ported, there is no application bandwidth cap.
//! * NetConn table: one connection per remote address created on demand by
//!   0x764970 (before any validation, as natively), kept in a `Vec`.
//!
//! The NetNode session id (NetNode+0xB0 = trunc(clock * 10.0) at Init
//! 0x764190) and the clock (NetNode+0x58, binary32 seconds) are explicit
//! inputs.
//!
//! The socket itself sits behind [`Transport`]: UDP for a LAN and a direct
//! address ([`UdpTransport`]), and another path (EOS P2P) gives the same
//! datagrams to the same sequencing, session and lockstep above it.
use crate::{Error, Result};
use aa_orders::commands::WireWriter;
use aa_orders::net::transport::{finalize, Received, Sequencing, X87Precision};
use aa_orders::{Cursor, Position};
use std::io::ErrorKind;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

/// Largest UDP datagram; replaces the 4,096-byte buffer at 0x783BE5.
pub const MAX_DATAGRAM: usize = 65_535;
/// 0x764950: 0x411FA0 with 13 possibilities.
pub const PACKET_TYPES: usize = 13;

/// Carries datagrams between the players of a session. Best effort, as UDP is:
/// a datagram may be lost, repeated or reordered, and the layers above (the
/// transport header of [`UdpNode`], the reliable channels) cope with that.
///
/// A peer is a [`SocketAddr`], which must be IPv4 and must mean the same peer on
/// every node of the session: the session snapshot sends these addresses to the
/// other peers (`PeerAddress`, 32 + 16 bits on the wire). A backend without IP
/// addresses gives each peer a stable virtual one.
pub trait Transport: Send {
    /// This node's own address on the path.
    fn local_addr(&self) -> Result<SocketAddr>;
    /// Send one datagram. An error is a failed send of this datagram only; the
    /// session goes on if the path recovers.
    fn send_to(&mut self, bytes: &[u8], to: SocketAddr) -> Result<()>;
    /// One waiting datagram into `buffer`: its length and sender, `None` when no
    /// datagram waits. Never blocks.
    fn recv_from(&mut self, buffer: &mut [u8]) -> Result<Option<(usize, SocketAddr)>>;
}

/// [`Transport`] over a UDP socket: a LAN and a direct address.
pub struct UdpTransport {
    socket: UdpSocket,
}
impl UdpTransport {
    pub fn new(socket: UdpSocket) -> Result<Self> {
        socket.set_nonblocking(true)?;
        Ok(Self { socket })
    }
}
impl Transport for UdpTransport {
    fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }
    fn send_to(&mut self, bytes: &[u8], to: SocketAddr) -> Result<()> {
        self.socket.send_to(bytes, to)?;
        Ok(())
    }
    fn recv_from(&mut self, buffer: &mut [u8]) -> Result<Option<(usize, SocketAddr)>> {
        loop {
            match self.socket.recv_from(buffer) {
                Ok(x) => return Ok(Some(x)),
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(None),
                // Windows reports an ICMP port-unreachable of an earlier send as
                // a receive error; the datagram queue continues.
                Err(e) if e.kind() == ErrorKind::ConnectionReset => continue,
                Err(e) => return Err(e.into()),
            }
        }
    }
}

/// One remote address (TGC_NetConn, 0xA4 bytes natively).
#[derive(Debug, Clone)]
pub struct UdpConn {
    pub addr: SocketAddr,
    pub seq: Sequencing,
}

/// A datagram accepted by the sequencing layer.
#[derive(Debug, Clone)]
pub struct Incoming {
    pub conn: usize,
    pub sequence: i32,
    /// Ack results to report to the AdminConn (0x74B4F0) in this order,
    /// before the admin dispatch.
    pub acks: Vec<(i32, bool)>,
    /// True when `sequence` is the highest received (NetConn+0x34): the
    /// "most recent" test of the admin readers.
    pub most_recent: bool,
    pub packet_type: u32,
    /// The finalized datagram and the position of the admin body.
    pub data: Vec<u8>,
    pub body: Position,
}
impl Incoming {
    /// Cursor at the admin body, bounded by the datagram's bit length.
    pub fn body(&self) -> Result<Cursor<'_>> {
        Ok(Cursor::for_packet(&self.data, Some(self.body))?)
    }
}

/// What happened to a received datagram that was not accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dropped {
    Sequencing { conn: usize, outcome: Received },
    Malformed { conn: usize, error: String },
}

pub struct UdpNode {
    socket: Box<dyn Transport>,
    /// NetNode+0xB0.
    pub own_session: i32,
    pub conns: Vec<UdpConn>,
    buffer: Vec<u8>,
    /// x87 precision of the network thread (PC53 by the lead's policy).
    pub precision: X87Precision,
    pub dropped: Vec<Dropped>,
    /// GameSpy QR2 queries that reached the game socket (FE FD 00, then the
    /// request key): who asked and the key. LobbyJoinDirectExecute 0x648030 ->
    /// 0x787750 asks before it joins; the game's protocol never starts so.
    pub queries: Vec<(SocketAddr, [u8; 4])>,
    /// The last datagram this path refused to send. The session goes on: the
    /// liveness rule of 0x75BD70 ends it if the path stays dead.
    pub send_failure: Option<String>,
}

impl UdpNode {
    pub fn bind(addr: impl ToSocketAddrs, own_session: i32, precision: X87Precision) -> Result<Self> {
        Self::from_socket(UdpSocket::bind(addr)?, own_session, precision)
    }
    /// A node on an already bound socket (the host's port search 0x7641E0).
    pub fn from_socket(socket: UdpSocket, own_session: i32, precision: X87Precision) -> Result<Self> {
        Ok(Self::on(Box::new(UdpTransport::new(socket)?), own_session, precision))
    }
    /// A node on any datagram path.
    pub fn on(socket: Box<dyn Transport>, own_session: i32, precision: X87Precision) -> Self {
        Self { socket, own_session, conns: Vec::new(), buffer: vec![0; MAX_DATAGRAM], precision, dropped: Vec::new(), queries: Vec::new(), send_failure: None }
    }
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.socket.local_addr()
    }
    /// 0x764970: the connection for `addr`, created on demand.
    pub fn conn_for(&mut self, addr: SocketAddr) -> usize {
        if let Some(i) = self.conns.iter().position(|c| c.addr == addr) {
            return i;
        }
        self.conns.push(UdpConn { addr, seq: Sequencing::default() });
        self.conns.len() - 1
    }
    /// SendPackets 0x764FA0 for one datagram: header, 4-bit admin type
    /// (0x763A60), body, finalize, sendto, then the sent-time record.
    /// Returns the sequence. The sequence is consumed even if sendto fails
    /// (as natively); the failure is kept in [`Self::send_failure`], and the
    /// reliable channels above send again what it lost.
    pub fn send(
        &mut self,
        conn: usize,
        packet_type: u32,
        now: f32,
        body: &mut dyn FnMut(&mut WireWriter, i32) -> Result<()>,
    ) -> Result<i32> {
        let own = self.own_session;
        let c = self.conns.get_mut(conn).ok_or_else(|| Error::Protocol(format!("no connection {conn}")))?;
        let mut w = WireWriter::new(Position::PACKET_START)?;
        let sequence = c.seq.write_header(&mut w, own)?;
        w.write_uint(packet_type, 4)?;
        body(&mut w, sequence)?;
        let bytes = finalize(&w)?;
        let addr = c.addr;
        c.seq.sent(sequence, now);
        if let Err(e) = self.socket.send_to(&bytes, addr) {
            self.send_failure = Some(e.to_string());
        }
        Ok(sequence)
    }
    /// Start of a SendPackets pass: NetConn+8 counters cleared (0x764FA0).
    pub fn begin_pass(&mut self) {
        for c in &mut self.conns {
            c.seq.sent_this_pass = 0;
        }
    }
    /// WinSock::Update + NetNode receive for every pending datagram (no
    /// 32-per-update bound).
    pub fn receive_all(&mut self, now: f32) -> Result<Vec<Incoming>> {
        let mut out = Vec::new();
        loop {
            let (len, addr) = match self.socket.recv_from(&mut self.buffer) {
                Ok(Some(x)) => x,
                Ok(None) => break,
                // A path that stopped working is silence: the liveness rule ends it.
                Err(Error::Io(_)) => break,
                Err(e) => return Err(e),
            };
            let data = self.buffer[..len].to_vec();
            if len >= 7 && data[..3] == [0xFE, 0xFD, 0x00] {
                self.queries.push((addr, [data[3], data[4], data[5], data[6]]));
                continue;
            }
            let conn = self.conn_for(addr);
            match self.accept(conn, data, now) {
                Ok(Some(incoming)) => out.push(incoming),
                Ok(None) => {}
                Err(e) => self.dropped.push(Dropped::Malformed { conn, error: e.to_string() }),
            }
        }
        Ok(out)
    }
    /// One datagram outside the game's protocol (a QR2 reply).
    pub fn send_raw(&mut self, to: SocketAddr, bytes: &[u8]) -> Result<()> {
        self.socket.send_to(bytes, to)
    }
    fn accept(&mut self, conn: usize, data: Vec<u8>, now: f32) -> Result<Option<Incoming>> {
        let own = self.own_session;
        let precision = self.precision;
        let c = &mut self.conns[conn];
        let mut r = Cursor::for_packet(&data, None)?;
        match c.seq.receive(&mut r, own, now, precision)? {
            Received::Accepted { sequence, acks, .. } => {
                let packet_type = r.read_possibility(PACKET_TYPES)?;
                let body = r.position();
                let most_recent = sequence == c.seq.highest;
                Ok(Some(Incoming { conn, sequence, acks, most_recent, packet_type, data, body }))
            }
            outcome => {
                self.dropped.push(Dropped::Sequencing { conn, outcome });
                Ok(None)
            }
        }
    }
}
