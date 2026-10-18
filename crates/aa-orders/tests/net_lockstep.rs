//! Composition check of the admin layer (built only from oracle-proven
//! parts): requests emitted on a host are sealed, executed by the host
//! (0x751B60 path) into decree packet #0, which a client replays (0x74FD10
//! path). Both must execute the same decrees with the same fields in the same
//! order, and their synchronizer-A hashes must agree (the host logs
//! "o.GetBitSize()" where the client logs "i.GetBitSize()" with the same
//! values; labels are not hashed). Not a native oracle by itself.
use aa_orders::flow::{Log, OrderHost, SyncEntry, Validity};
use aa_orders::net::admin::{AdminNode, Peer};
use aa_orders::net::sync::Synchronizer;
use aa_orders::payload::{Field, NameKind, Registry, Resolver, SchemaContext, ThingCheck};
use aa_orders::{Gate, History, Packet};

struct Ctx;
impl SchemaContext for Ctx {
    fn registry_count(&self, _r: Registry) -> aa_orders::Result<usize> {
        Ok(7)
    }
    fn team_command_flags(&self, _i: u32) -> aa_orders::Result<u32> {
        Ok(0)
    }
    fn definition_kind(&self, _r: Registry, _i: u32) -> aa_orders::Result<u32> {
        Ok(0)
    }
    fn zero_width_list_budget(&self) -> usize {
        0
    }
}
impl Resolver for Ctx {
    fn thing(&self, _c: ThingCheck, slot: u16) -> Option<u16> {
        Some(slot)
    }
    fn player(&self, id: u32) -> bool {
        id == 1
    }
    fn definition(&self, _r: Registry, i: u32) -> bool {
        i < 7
    }
    fn named(&self, _k: NameKind, key: &[u16]) -> Option<Vec<u16>> {
        Some(key.to_vec())
    }
}

struct Node {
    a: Synchronizer,
    executed: Vec<(usize, Vec<Field>)>,
}
impl OrderHost for Node {
    fn has_ordering_client(&mut self) -> bool {
        true
    }
    fn version_acknowledged(&mut self) -> bool {
        true
    }
    fn film_session(&mut self) -> bool {
        false
    }
    fn valid(&mut self, _v: Validity, _r: usize, _f: &[Field]) -> aa_orders::Result<bool> {
        Ok(true)
    }
    fn sync(&mut self, e: SyncEntry) -> aa_orders::Result<()> {
        match e {
            SyncEntry::Int { log: Log::A, label, value } => self.a.log_int(true, label, value),
            SyncEntry::Text { log: Log::A, label, value } => {
                let units: Vec<u16> = value.encode_utf16().collect();
                self.a.log_text(true, label, &units)
            }
            _ => {}
        }
        Ok(())
    }
    fn process(&mut self, d: usize, f: &mut Vec<Field>) -> aa_orders::Result<()> {
        self.executed.push((d, f.clone()));
        Ok(())
    }
    fn mirror_enabled(&mut self) -> bool {
        false
    }
}

#[test]
fn host_decrees_replay_identically_on_a_client() {
    let mut a = Synchronizer::new(0x10000);
    a.reset(0, 0);
    let mut host = Node { a: a.clone(), executed: Vec::new() };
    let mut client = Node { a, executed: Vec::new() };
    let mut h = AdminNode::new(0).unwrap();
    let move_fields = vec![
        Field::Text("usa_infantry".encode_utf16().collect()),
        Field::Uint { width: 32, value: 0x4300_0000 },
        Field::Uint { width: 32, value: 0xc298_0000 },
        Field::Uint { width: 32, value: 0 },
    ];
    let speed = vec![Field::Uint { width: 32, value: 2 }];
    assert!(h.emit_request(true, 47, &move_fields, 1, &Ctx).unwrap());
    assert!(h.emit_request(true, 54, &speed, 1, &Ctx).unwrap());
    let sealed = h.seal_requests().unwrap();
    // The host node is the first peer of its own list (0x75AEE0).
    let mut own = History::new(sealed, 4).unwrap();
    own.put(sealed, h.requests.get(sealed).unwrap().cloned()).unwrap();
    let mut peers = vec![Peer { requests: own, ordering_player: Some(1) }];
    let mut gate = Gate::default();
    h.host_execute(&mut peers, &mut host, &Ctx, &Ctx, &mut gate).unwrap();
    let decree_index = h.seal_decrees(&mut host).unwrap();
    assert_eq!(decree_index, 0);
    let packet: Packet = h.decrees.get(0).unwrap().cloned().unwrap();

    let mut c = AdminNode::new(0).unwrap();
    c.decrees.put(0, Some(packet)).unwrap();
    c.client_execute(0, &mut client, &Ctx, &Ctx, &mut gate).unwrap();

    let ids: Vec<usize> = host.executed.iter().map(|e| e.0).collect();
    // SetPlayer for the peer (0x7509F0), SetPlayer from the embedded request
    // 320, then the move and game-speed decrees.
    assert_eq!(ids, [298, 298, 66, 73]);
    assert_eq!(host.executed, client.executed);
    assert_eq!(host.a.hash, client.a.hash);
    assert_eq!(host.a.end(), client.a.end());
}
