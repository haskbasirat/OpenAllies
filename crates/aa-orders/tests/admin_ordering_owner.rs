//! Ownership/integration checks, not a second native oracle. Native284 boundary
//! stores are independently pinned in native-admin-ordering-owner.json.
use aa_orders::flow::{Log, SyncEntry, Validity};
use aa_orders::net::admin::{
    AdminNode, ClientIdentity, CurrentClient, IdentifiedPeer, OrderingHost, Peer,
};
use aa_orders::payload::{Field, NameKind, Registry, Resolver, SchemaContext, ThingCheck};
use aa_orders::{Gate, History};
struct Ctx;
impl SchemaContext for Ctx {
    fn registry_count(&self, _: Registry) -> aa_orders::Result<usize> {
        Ok(7)
    }
    fn team_command_flags(&self, _: u32) -> aa_orders::Result<u32> {
        Ok(0)
    }
    fn definition_kind(&self, _: Registry, _: u32) -> aa_orders::Result<u32> {
        Ok(0)
    }
    fn zero_width_list_budget(&self) -> usize {
        0
    }
}
impl Resolver for Ctx {
    fn thing(&self, _: ThingCheck, s: u16) -> Option<u16> {
        Some(s)
    }
    fn player(&self, id: u32) -> bool {
        id != 0
    }
    fn definition(&self, _: Registry, i: u32) -> bool {
        i < 7
    }
    fn named(&self, _: NameKind, s: &[u16]) -> Option<Vec<u16>> {
        Some(s.to_vec())
    }
}
#[derive(Default)]
struct Host {
    address: Option<usize>,
    client_address: Option<usize>,
    current_clients: Vec<CurrentClient>,
    seen: Vec<(usize, u32)>,
    fail: bool,
    fail_client: bool,
    fail_diagnostic_after: Option<usize>,
}
impl Host {
    fn observe(&mut self, slot: &u32) {
        let p = slot as *const u32 as usize;
        if let Some(previous) = self.address {
            assert_eq!(previous, p, "callback must borrow same Admin284 allocation");
        } else {
            self.address = Some(p);
        }
    }
}
impl OrderingHost for Host {
    fn has_ordering_client(&mut self, s: &mut u32, c: &CurrentClient) -> aa_orders::Result<bool> {
        self.observe(s);
        let address = c as *const CurrentClient as usize;
        if let Some(previous) = self.client_address {
            assert_eq!(previous, address);
        } else {
            self.client_address = Some(address);
        }
        if self.fail_client {
            return Err(aa_orders::Error::MissingCurrentPlayer);
        }
        Ok(*s != 0)
    }
    fn version_acknowledged(&mut self, s: &mut u32, c: &CurrentClient) -> aa_orders::Result<bool> {
        self.observe(s);
        let address = c as *const CurrentClient as usize;
        if let Some(previous) = self.client_address {
            assert_eq!(previous, address);
        } else {
            self.client_address = Some(address);
        }
        Ok(true)
    }
    fn film_session(&mut self, s: &mut u32, c: &CurrentClient) -> aa_orders::Result<bool> {
        self.observe(s);
        let address = c as *const CurrentClient as usize;
        if let Some(previous) = self.client_address {
            assert_eq!(previous, address);
        } else {
            self.client_address = Some(address);
        }
        Ok(false)
    }
    fn valid(
        &mut self,
        s: &mut u32,
        c: &CurrentClient,
        _: Validity,
        _: usize,
        _: &[Field],
    ) -> aa_orders::Result<bool> {
        self.observe(s);
        let address = c as *const CurrentClient as usize;
        if let Some(previous) = self.client_address {
            assert_eq!(previous, address);
        } else {
            self.client_address = Some(address);
        }
        Ok(true)
    }
    fn sync(&mut self, s: &mut u32, c: &CurrentClient, e: SyncEntry) -> aa_orders::Result<()> {
        self.observe(s);
        let address = c as *const CurrentClient as usize;
        if let Some(previous) = self.client_address {
            assert_eq!(previous, address);
        } else {
            self.client_address = Some(address);
        }
        if matches!(
            e,
            SyncEntry::Int {
                label: "i.GetBitSize()",
                ..
            }
        ) && self
            .fail_diagnostic_after
            .is_some_and(|n| self.seen.len() >= n)
        {
            return Err(aa_orders::Error::MissingCurrentPlayer);
        }
        Ok(())
    }
    fn process(
        &mut self,
        s: &mut u32,
        c: &CurrentClient,
        d: usize,
        f: &mut Vec<Field>,
    ) -> aa_orders::Result<()> {
        self.observe(s);
        let address = c as *const CurrentClient as usize;
        if let Some(previous) = self.client_address {
            assert_eq!(previous, address);
        } else {
            self.client_address = Some(address);
        }
        if d == 298 {
            let Field::Uint { width: 32, value } = f[0] else {
                panic!("actual SetPlayer field")
            };
            *s = value;
        }
        self.seen.push((d, *s));
        self.current_clients.push(*c);
        if self.fail && d == 73 {
            *s = 99;
            return Err(aa_orders::Error::MissingCurrentPlayer);
        }
        Ok(())
    }
    fn mirror_enabled(&mut self, s: &mut u32, c: &CurrentClient) -> aa_orders::Result<bool> {
        self.observe(s);
        let address = c as *const CurrentClient as usize;
        if let Some(previous) = self.client_address {
            assert_eq!(previous, address);
        } else {
            self.client_address = Some(address);
        }
        Ok(false)
    }
}
fn speed(a: &mut AdminNode, player: u32) {
    a.emit_request(
        true,
        54,
        &[Field::Uint {
            width: 32,
            value: 2,
        }],
        player,
        &Ctx,
    )
    .unwrap();
    a.seal_requests().unwrap();
}
#[test]
fn own_history_first_and_same_slot_during_live_host_and_client_callbacks() {
    let mut a = AdminNode::new(0).unwrap();
    speed(&mut a, 1);
    let mut remote = AdminNode::new(0).unwrap();
    speed(&mut remote, 2);
    let mut peers = [Peer {
        requests: remote.requests,
        ordering_player: Some(2),
    }];
    let own_begin = a.requests.begin();
    let end = a.requests.end();
    let p = a.ordering_player_slot() as *const u32 as usize;
    let cp = a.current_client() as *const CurrentClient as usize;
    let mut h = Host::default();
    let mut gate = Gate::default();
    let mut identified = peers
        .iter_mut()
        .enumerate()
        .map(|(i, peer)| IdentifiedPeer {
            peer,
            client: ClientIdentity::Remote(i as u64 + 1),
        })
        .collect::<Vec<_>>();
    a.host_execute_owned_requests(Some(1), &mut identified, &mut h, &Ctx, &Ctx, &mut gate)
        .unwrap();
    assert!(own_begin < end);
    assert_eq!(a.requests.begin(), end);
    assert_eq!(peers[0].requests.begin(), peers[0].requests.end());
    assert_eq!(h.address, Some(p));
    assert_eq!(h.client_address, Some(cp));
    assert_eq!(
        h.current_clients,
        [
            CurrentClient::Bound(ClientIdentity::OwnNode),
            CurrentClient::Bound(ClientIdentity::OwnNode),
            CurrentClient::Bound(ClientIdentity::OwnNode),
            CurrentClient::Bound(ClientIdentity::Remote(1)),
            CurrentClient::Bound(ClientIdentity::Remote(1)),
            CurrentClient::Bound(ClientIdentity::Remote(1))
        ]
    );
    assert_eq!(a.current_client(), &CurrentClient::Null);
    assert_eq!(
        h.seen,
        [(298, 1), (298, 1), (73, 1), (298, 2), (298, 2), (73, 2)]
    );
    assert_eq!(a.previous_ordering_player, Some(2));
    assert_eq!(a.ordering_player(), None);
    // Seal only needs sync; the portable fixture verifies the old seal method.
    struct SyncOnly;
    impl aa_orders::flow::OrderHost for SyncOnly {
        fn has_ordering_client(&mut self) -> bool {
            panic!()
        }
        fn version_acknowledged(&mut self) -> bool {
            panic!()
        }
        fn film_session(&mut self) -> bool {
            panic!()
        }
        fn valid(&mut self, _: Validity, _: usize, _: &[Field]) -> aa_orders::Result<bool> {
            panic!()
        }
        fn sync(&mut self, e: SyncEntry) -> aa_orders::Result<()> {
            assert!(matches!(e, SyncEntry::Int { log: Log::A, .. }));
            Ok(())
        }
        fn process(&mut self, _: usize, _: &mut Vec<Field>) -> aa_orders::Result<()> {
            panic!()
        }
        fn mirror_enabled(&mut self) -> bool {
            panic!()
        }
    }
    a.seal_decrees(&mut SyncOnly).unwrap();
    let mut c = AdminNode::new(0).unwrap();
    c.decrees
        .put(0, a.decrees.get(0).unwrap().cloned())
        .unwrap();
    c.client.field_250 = 17;
    let mut ch = Host::default();
    let cp = c.ordering_player_slot() as *const u32 as usize;
    c.client_execute_with_ordering(0, &mut ch, &Ctx, &Ctx, &mut gate)
        .unwrap();
    assert_eq!(ch.address, Some(cp));
    assert_eq!(ch.seen, h.seen);
    assert_eq!(c.client.field_284, 0);
    assert_eq!(c.client.field_250, 0);
}
#[test]
fn each_empty_peer_clears284_and_failure_keeps_actual_prefix() {
    let mut a = AdminNode::new(0).unwrap();
    a.client.field_284 = 17;
    let mut peers = [
        Peer {
            requests: History::new(0, 1).unwrap(),
            ordering_player: None,
        },
        Peer {
            requests: History::new(0, 1).unwrap(),
            ordering_player: None,
        },
    ];
    a.host_execute_with_ordering(
        &mut peers
            .iter_mut()
            .enumerate()
            .map(|(i, peer)| IdentifiedPeer {
                peer,
                client: ClientIdentity::Remote(i as u64 + 1),
            })
            .collect::<Vec<_>>(),
        &mut Host::default(),
        &Ctx,
        &Ctx,
        &mut Gate::default(),
    )
    .unwrap();
    // Native751B60 runs for each peer: first previous17, then previousNULL.
    assert_eq!(a.previous_ordering_player, None);
    assert_eq!(a.client.field_284, 0);
    speed(&mut a, 1);
    let begin = a.requests.begin();
    let mut h = Host {
        fail: true,
        ..Host::default()
    };
    assert!(
        a.host_execute_owned_requests(Some(1), &mut [], &mut h, &Ctx, &Ctx, &mut Gate::default())
            .is_err()
    );
    assert_eq!(a.client.field_284, 99);
    assert_eq!(
        a.current_client(),
        &CurrentClient::Bound(ClientIdentity::OwnNode)
    );
    assert_eq!(a.requests.begin(), begin);
    assert_eq!(a.previous_ordering_player, None);
}

#[test]
fn original_admin284_boundary_rows() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-player-orders/native-admin-ordering-owner.json");
    let fixture: serde_json::Value = serde_json::from_slice(&std::fs::read(root).unwrap()).unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 114);
    for row in rows {
        let i = &row["input"];
        let o = &row["output"];
        let u = |v: &serde_json::Value| v.as_u64().unwrap() as u32;
        let mut a = AdminNode::new(0).unwrap();
        a.client.field_284 = u(&i["current"]);
        match row["kind"].as_str().unwrap() {
            "setter" => {
                // 74B3A0 is exactly one raw pointer store. Demonstrate that
                // the public getter reads this canonical allocation directly.
                a.client.field_284 = u(&i["value"]);
                assert_eq!(a.client.field_284, u(&o["current"]));
                assert_eq!(
                    a.ordering_player(),
                    (u(&o["current"]) != 0).then_some(u(&o["current"]))
                );
            }
            "empty" => {
                a.previous_ordering_player = Some(u(&i["previous"]));
                let mut peers = [Peer {
                    requests: History::new(0, 1).unwrap(),
                    ordering_player: None,
                }];
                a.host_execute_with_ordering(
                    &mut peers
                        .iter_mut()
                        .enumerate()
                        .map(|(i, peer)| IdentifiedPeer {
                            peer,
                            client: ClientIdentity::Remote(i as u64 + 1),
                        })
                        .collect::<Vec<_>>(),
                    &mut Host::default(),
                    &Ctx,
                    &Ctx,
                    &mut Gate::default(),
                )
                .unwrap();
                assert_eq!(a.client.field_284, u(&o["current"]));
                assert_eq!(a.previous_ordering_player.unwrap_or(0), u(&o["previous"]));
            }
            "client" => {
                a.client.field_250 = u(&i["resume"]);
                a.decrees.put(0, None).unwrap();
                let mut entry = None;
                let r = aa_orders::consume_next(&mut a.decrees, &mut a.client, |_, _, state, _| {
                    entry = Some(state.field_284);
                    state.field_284 = u(&i["selected"]);
                    if i["stop"].as_bool().unwrap() {
                        Err(aa_orders::Error::MissingCurrentPlayer)
                    } else {
                        Ok(aa_orders::Dispatch {
                            events: vec![],
                            position: aa_orders::Position::PACKET_START,
                            executed: 0,
                            stopped_before_execute: None,
                        })
                    }
                });
                assert_eq!(r.is_err(), i["stop"].as_bool().unwrap());
                assert_eq!(entry, Some(u(&o["calls"][0]["entry284"])));
                assert_eq!(a.client.field_284, u(&o["current"]));
                assert_eq!(a.client.field_250, u(&o["resume_player"]));
                assert_eq!(a.client.next_index as u32, u(&o["next"]));
                assert_eq!(
                    [a.client.resume_byte as u32, a.client.resume_bit as u32],
                    [u(&o["cursor"][0]), u(&o["cursor"][1])]
                );
            }
            other => panic!("unexpected {other}"),
        }
    }
}

#[test]
fn original_peer_order_boundaries_and_film_use_canonical_slot() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-player-orders/native-admin-peer-order.json");
    let fixture: serde_json::Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 24);
    for row in rows {
        let mut a = AdminNode::new(0).unwrap();
        a.client.field_284 = 17;
        a.previous_ordering_player = Some(99);
        let count = row["peer_count"].as_u64().unwrap() as usize;
        let mut peers = (1..count)
            .map(|_| Peer {
                requests: History::new(0, 1).unwrap(),
                ordering_player: None,
            })
            .collect::<Vec<_>>();
        if count == 0 {
            a.host_execute_with_ordering(
                &mut [],
                &mut Host::default(),
                &Ctx,
                &Ctx,
                &mut Gate::default(),
            )
            .unwrap();
        } else {
            a.host_execute_owned_requests(
                None,
                &mut peers
                    .iter_mut()
                    .enumerate()
                    .map(|(i, peer)| IdentifiedPeer {
                        peer,
                        client: ClientIdentity::Remote(i as u64 + 1),
                    })
                    .collect::<Vec<_>>(),
                &mut Host::default(),
                &Ctx,
                &Ctx,
                &mut Gate::default(),
            )
            .unwrap();
        }
        assert_eq!(
            a.client.field_284,
            row["output"]["current"].as_u64().unwrap() as u32
        );
        assert_eq!(
            a.previous_ordering_player.unwrap_or(0),
            row["output"]["previous"].as_u64().unwrap() as u32
        );
    }
    // Integration only: bound film callback sees same allocation and its
    // changed player is transferred to +288 before canonical +284 is cleared.
    let mut w = aa_orders::commands::WireWriter::new(aa_orders::Position::PACKET_START).unwrap();
    for (id, value) in [(298, 2), (73, 3)] {
        w.write_uint(id, 9).unwrap();
        aa_orders::payload::encode(
            id as usize,
            &[Field::Uint { width: 32, value }],
            &mut w,
            &Ctx,
        )
        .unwrap();
    }
    let body = aa_orders::net::transport::finalize(&w).unwrap();
    let mut record = (body.len() as u32).to_le_bytes().to_vec();
    record.extend(body);
    let mut a = AdminNode::new(0).unwrap();
    a.film_ordering_player = Some(17);
    let p = a.ordering_player_slot() as *const u32 as usize;
    let mut h = Host::default();
    let mut gate = Gate::default();
    a.play_film_record_with_ordering(&record, &mut h, &Ctx, &Ctx, &mut gate)
        .unwrap();
    assert_eq!(h.address, Some(p));
    assert_eq!(h.seen, [(298, 2), (73, 2)]);
    assert_eq!(a.film_ordering_player, Some(2));
    assert_eq!(a.client.field_284, 0);
    assert!(!gate.session);
}

#[test]
fn required_diagnostic_failure_stops_before_later_effects_and_cleanup() {
    for (after, expected_slot) in [(0, 17), (1, 2)] {
        let mut w =
            aa_orders::commands::WireWriter::new(aa_orders::Position::PACKET_START).unwrap();
        for (id, value) in [(298, 2), (73, 3), (73, 4)] {
            w.write_uint(id, 9).unwrap();
            aa_orders::payload::encode(
                id as usize,
                &[Field::Uint { width: 32, value }],
                &mut w,
                &Ctx,
            )
            .unwrap();
        }
        let body = aa_orders::net::transport::finalize(&w).unwrap();
        let mut a = AdminNode::new(0).unwrap();
        a.client.field_250 = 17;
        a.decrees
            .put(0, Some(aa_orders::Packet::new(body).unwrap()))
            .unwrap();
        let mut h = Host {
            fail_diagnostic_after: Some(after),
            ..Host::default()
        };
        assert!(
            a.client_execute_with_ordering(0, &mut h, &Ctx, &Ctx, &mut Gate::default())
                .is_err()
        );
        assert_eq!(h.seen.len(), after);
        assert_eq!(a.client.field_284, expected_slot);
        assert_eq!(a.client.field_250, 17);
        assert_eq!(a.client.next_index, 0);
    }
}

#[test]
fn missing_live_owner_read_is_an_error_not_a_false_rejection() {
    let mut a = AdminNode::new(0).unwrap();
    speed(&mut a, 1);
    let begin = a.requests.begin();
    let mut h = Host {
        fail_client: true,
        ..Host::default()
    };
    assert!(
        a.host_execute_owned_requests(Some(1), &mut [], &mut h, &Ctx, &Ctx, &mut Gate::default())
            .is_err()
    );
    assert_eq!(h.seen, [(298, 1)]);
    assert_eq!(a.requests.begin(), begin);
    assert_eq!(a.client.field_284, 1);
    assert_eq!(a.previous_ordering_player, None);
    assert_eq!(
        a.current_client(),
        &CurrentClient::Bound(ClientIdentity::OwnNode)
    );
}
