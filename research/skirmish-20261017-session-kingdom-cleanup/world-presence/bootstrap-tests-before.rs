use super::*;
use serde_json::{Value, json};

struct Os {
    tick: u32,
    guid: Guid,
    calls: Vec<&'static str>,
    fail: Option<&'static str>,
}
impl ResetOs for Os {
    fn get_tick_count(&mut self) -> Result<u32> {
        self.calls.push("tick");
        if self.fail == Some("tick") {
            return Err(missing("injected OS tick failure"));
        }
        Ok(self.tick)
    }
    fn co_create_guid(&mut self) -> Result<Guid> {
        self.calls.push("guid");
        if self.fail == Some("guid") {
            return Err(missing("injected CoCreateGuid failure"));
        }
        Ok(self.guid)
    }
    fn sync_log_gate(&mut self) -> Result<bool> {
        panic!("disabled fresh synchronizer must not read a fabricated logger gate")
    }
}
fn os(tick: u32, bytes: &[u8]) -> Os {
    Os {
        tick,
        guid: Guid {
            data1: u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
            data2: u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
            data3: u16::from_le_bytes(bytes[6..8].try_into().unwrap()),
            data4: bytes[8..16].try_into().unwrap(),
        },
        calls: Vec::new(),
        fail: None,
    }
}

#[test]
fn resource_worker_parts_loan_actual_session_sim_and_same_a_s_logs() {
    let mut owner = LocalBootstrap::from_native_constructors();
    let sim = &owner.sim as *const _;
    let a = &owner.sync_a as *const _;
    let s = &owner.sync_s as *const _;
    {
        let parts = owner.resource_load_parts();
        assert_eq!(parts.sim as *const _, sim);
        assert_eq!(parts.sync_a as *const _, a);
        assert_eq!(parts.sync_s as *const _, s);
        assert_eq!(parts.session.current_phase().unwrap(), 0);
        assert_eq!(parts.session.application_state().unwrap(), 0);
        parts.sync_a.borrow_mut().hash = 123;
        parts.sync_s.borrow_mut().hash = 456;
    }
    assert_eq!(owner.sync_a.borrow().hash, 123);
    assert_eq!(owner.sync_s.borrow().hash, 456);
    //Explicit actual-field mutation probe, not a public producer/default.
    owner.session.phase = 7;
    owner.session.state = 2;
    let parts = owner.resource_load_parts();
    assert_eq!(parts.session.current_phase().unwrap(), 7);
    assert_eq!(parts.session.application_state().unwrap(), 2);
    assert_eq!(parts.sync_s.borrow().hash, 456);
}

#[test]
fn actual_worker_loan_is_send_and_shares_session_and_synchronizers() {
    let mut owner=LocalBootstrap::from_native_constructors();
    owner.session.state=2;owner.session.phase=12;
    std::thread::scope(|scope|{
        let parts=owner.resource_worker_parts();
        scope.spawn(move||{
            assert_eq!(parts.session.application_state().unwrap(),2);
            assert_eq!(parts.session.current_phase().unwrap(),12);
            assert!(aa_runtime::load_tail_world::SessionRead::local_player(&parts.session).unwrap().is_none());
            parts.sync_a.hash=0x1234;parts.sync_s.hash=0x5678;
            assert!(parts.sim.world_is_null());
        }).join().unwrap();
    });
    assert_eq!(owner.sync_a.borrow().hash,0x1234);
    assert_eq!(owner.sync_s.borrow().hash,0x5678);
}

#[test]
fn first_player_rated_failure_retains_publication_before_name_and_node_writes() {
    let mut owner = LocalBootstrap::from_native_constructors();
    owner
        .reset_node_fresh(
            &mut os(7, &[0; 16]),
            &InitialProperties::from_native_constructor(),
            &Notifications::new(0),
        )
        .unwrap();
    let name: Vec<u16> = "unwritten nickname".encode_utf16().collect();
    let result = owner.create_first_local_player_with_rated(
        &name,
        &[],
        || Ok(VWorldState::with_caption(vec![0x41])),
        |actual| {
            assert_eq!(actual.session.players.len(), 1);
            assert_eq!(actual.session.created, Some(1));
            assert!(actual.session.changed[0]);
            assert!(actual.session.players[0].core.name.is_empty());
            assert!(actual.local_player.is_none());
            assert!(actual.ordering_player.is_none());
            Err(missing("explicit770280 reached dependency failure"))
        },
    );
    assert!(result.is_err());
    assert_eq!(owner.session.players.len(), 1);
    assert_eq!(owner.progress, Progress::CreatingPlayer);
    assert!(owner.local_player.is_none());
}

#[test]
fn local_film_header_reads_same_roster_node_and_live_display_fields() {
    use aa_runtime::film_header_entries::Players;
    let mut owner = LocalBootstrap::from_native_constructors();
    owner
        .reset_node_fresh(
            &mut os(7, &[0; 16]),
            &InitialProperties::from_native_constructor(),
            &Notifications::new(0),
        )
        .unwrap();
    let name: Vec<u16> = "Actual player allocation".encode_utf16().collect();
    let player = owner
        .create_first_local_player_with(&name, &[], || {
            Ok(VWorldState::with_caption(
                "explicit native text fixture".encode_utf16().collect(),
            ))
        })
        .unwrap();
    owner
        .set_kingdom_name_7664d0(&player, &"KingdomChoice".encode_utf16().collect::<Vec<_>>())
        .unwrap();
    for display in [&name[..], &[0x41, 0, 0x42][..]] {
        owner.player_mut(&player).unwrap().core.name = display.to_vec(); //actual-field mutation probe, not a user-facing writer
        let (film, mut players, node, sim) = owner.recording_parts().unwrap();
        assert!(film.is_none());
        assert!(node.recorder_268().is_none());
        let mut roster = players.film_roster(sim);
        let first = Players::first(&mut roster).unwrap().unwrap();
        assert!(Players::is_local(&mut roster, &first).unwrap());
        assert!(Players::next(&mut roster, &first).unwrap().is_none());
        assert!(
            Players::kingdom_765e00(&mut roster, &first)
                .unwrap()
                .is_none()
        ); //actual constructor World is absent
        assert_eq!(
            Players::display_name_08(&mut roster, &first).unwrap(),
            display
        );
        assert_eq!(
            Players::name_24(&mut roster, &first).unwrap(),
            "KingdomChoice".encode_utf16().collect::<Vec<_>>()
        );
        assert_eq!(Players::ai_byte(&mut roster, &first).unwrap(), 0);
    }
}
fn assert_projection(got: &Value, expected: &Value) {
    for (key, value) in expected.as_object().unwrap() {
        if key == "players" {
            let mut normalized = value.clone();
            for player in normalized.as_array_mut().unwrap() {
                assert_eq!(
                    player["field_28"], 0,
                    "constructor native pointer is actually null"
                );
                player["field_28"] = Value::Null;
            }
            assert_eq!(got[key], normalized, "native projected field {key}");
        } else {
            assert_eq!(&got[key], value, "native projected field {key}");
        }
    }
}

#[test]
fn native_whole_fresh_reset_and_first_local_player_replay() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-local-bootstrap/native-bootstrap.json"
    ))
    .unwrap();
    assert_eq!(f["passed"], true);
    assert_eq!(f["cases"].as_array().unwrap().len(), 24);
    for row in f["cases"].as_array().unwrap() {
        let mut owner = LocalBootstrap::from_native_constructors();
        assert!(owner.last_player_id().is_err());
        assert_eq!(owner.launch_mode(), 3);
        assert_eq!(owner.launch_game_type(), 0);
        assert_eq!(owner.options().max_players, -1);
        assert_eq!(owner.options().field_d8, u32::MAX);
        assert!(owner.options().allow_observers);
        assert_eq!(
            serde_json::to_value(constructor_launch_state()).unwrap(),
            row["constructor_launch_state"]
        );
        let mut host = os(
            row["tick"].as_u64().unwrap() as u32,
            &serde_json::from_value::<Vec<u8>>(row["guid_bytes"].clone()).unwrap(),
        );
        let notifications = Notifications::new(row["application_kind"].as_u64().unwrap() as u32);
        owner
            .reset_fresh(
                &mut host,
                &InitialProperties::from_native_constructor(),
                &notifications,
            )
            .unwrap();
        assert_eq!(host.calls, ["tick", "guid"]);
        assert_projection(
            &serde_json::to_value(owner.snapshot()).unwrap(),
            &row["reset"],
        );
        assert_eq!(
            notifications.len().unwrap() as u64,
            row["queued_count"].as_u64().unwrap()
        );
        let expected_event = &row["notification"][0];
        if row["application_kind"] != 3 {
            assert_eq!(
                serde_json::to_value(notifications.pop().unwrap().unwrap()).unwrap(),
                *expected_event
            );
            assert_eq!(row["queued_values"], json!([expected_event]));
        } else {
            assert_eq!(row["queued_values"], json!([]));
        }
        assert!(notifications.is_empty().unwrap());
        let nickname: Vec<u16> = serde_json::from_value(row["nickname"].clone()).unwrap();
        let key: Vec<u16> = serde_json::from_value(row["identity_key"].clone()).unwrap();
        // Explicit native text-table fixture seam. Production entry point uses
        // the real Localization::display source instead of this closure.
        let p = owner
            .create_first_local_player_with(&nickname, &key, || {
                Ok(VWorldState::with_caption(
                    "All Units".encode_utf16().collect(),
                ))
            })
            .unwrap();
        assert_eq!(p.id(), 1);
        assert_eq!(owner.player_status(&p).unwrap(), 0);
        assert_eq!(owner.host(), PeerLink::Null);
        assert!(owner.kingdom_name(&p).unwrap().is_empty());
        assert_eq!(owner.player_name(&p).unwrap(), nickname);
        assert_projection(
            &serde_json::to_value(owner.snapshot()).unwrap(),
            &row["after_player"],
        );
        let got = owner.player_view_snapshot(&p).unwrap();
        let mut expected = row["player_view"].clone();
        assert_eq!(expected["view_word_0c"], json!(0xa5a5a5a5u32));
        // Allocation capacity/unused bytes are not authored gameplay values.
        expected
            .as_object_mut()
            .unwrap()
            .remove("player_vector_capacity");
        expected["group"]
            .as_object_mut()
            .unwrap()
            .remove("capacity");
        expected["view_word_0c"] = Value::Null; // typed uninitialized, never allocator-zero claim
        expected["view_kingdom"] = Value::Null; // native null pointer
        let zeros = expected
            .as_object_mut()
            .unwrap()
            .remove("view_zero_fields")
            .unwrap();
        assert_eq!(zeros.as_array().unwrap()[..2], [json!(0), json!(0)]);
        expected["mask_words_10_1c"] = json!(&zeros.as_array().unwrap()[2..6]);
        assert_eq!(zeros[6], 0); // actual native+20 is a null group pointer
        expected["group_is_null"] = json!(true);
        expected["look_at_28_2c"] = json!(&zeros.as_array().unwrap()[7..]);
        expected["selection_empty"] = json!(true);
        assert!(owner.selection(&p).unwrap().head().is_none());
        assert!(owner.selection(&p).unwrap().tail().is_none());
        assert_eq!(got, expected);
        for replace in row["vworld_replacements"].as_array().unwrap() {
            let kingdom = replace["kingdom_identity"]
                .as_u64()
                .map(|v| aa_runtime::load_tail_world::KingdomId::from_raw(v as u32));
            owner
                .replace_kingdom_with(&p, kingdom, || {
                    Ok(VWorldState::with_kingdom_caption(
                        kingdom,
                        "All Units".encode_utf16().collect(),
                    ))
                })
                .unwrap();
            assert_eq!(
                aa_runtime::load_tail_world::SessionRead::player_kingdom_28(&owner, &p).unwrap(),
                kingdom
            );
            assert!(owner.selection(&p).unwrap().is_empty());
            assert!(replace["stored_pointer_matches"].as_bool().unwrap());
            assert!(replace["new_view_allocation"].as_bool().unwrap());
            assert_eq!(replace["view_word_0c"], json!(0xa5a5a5a5u32));
        }
        assert!(owner.require_host_launch().is_err());
    }
}

#[test]
fn os_failures_keep_the_native_committed_prefix_without_supplying_defaults() {
    let source = InitialProperties::from_native_constructor();
    for failure in ["tick", "guid"] {
        let mut owner = LocalBootstrap::from_native_constructors();
        let queue = Notifications::new(0);
        let mut os = os(0x12345678, &[0; 16]);
        os.fail = Some(failure);
        assert!(owner.reset_fresh(&mut os, &source, &queue).is_err());
        assert_eq!(owner.progress(), Progress::Resetting);
        assert_eq!(queue.pop().unwrap().unwrap().kind, 2);
        assert!(queue.is_empty().unwrap());
        if failure == "tick" {
            assert!(owner.last_player_id().is_err());
            assert_eq!(owner.rng_state(), NativeRng::default().state);
        } else {
            assert_eq!(owner.last_player_id().unwrap(), 0);
            assert_eq!(
                owner.rng_state(),
                NativeRng::from_native_seed(0x12345678).state
            );
            assert_eq!(owner.snapshot().changed, [true; 3]);
        }
        assert!(owner.reset_fresh(&mut os, &source, &queue).is_err());
    }
}

#[test]
fn actual_localization_is_required_and_player_handles_belong_to_the_live_owner() {
    let source = InitialProperties::from_native_constructor();
    let queue = Notifications::new(0);
    let mut owner = LocalBootstrap::from_native_constructors();
    owner
        .reset_fresh(&mut os(1, &[0; 16]), &source, &queue)
        .unwrap();
    assert!(
        owner
            .create_first_local_player(&[65], &[66], &Localization::default())
            .is_err()
    );
    assert_eq!(owner.progress(), Progress::CreatingPlayer);
    assert_eq!(owner.last_player_id().unwrap(), 1); // E8 increment precedes failing ctor
    assert!(owner.local_player().unwrap().is_none());
    assert_eq!(owner.snapshot().players.len(), 0); // constructor never appended
    assert!(owner.snapshot().node_active);
    let mut a = LocalBootstrap::from_native_constructors();
    let mut b = LocalBootstrap::from_native_constructors();
    a.reset_fresh(&mut os(0, &[0; 16]), &source, &queue)
        .unwrap();
    b.reset_fresh(&mut os(0, &[0; 16]), &source, &queue)
        .unwrap();
    let p = a
        .create_first_local_player_with(&[], &[], || Ok(VWorldState::with_caption(Vec::new())))
        .unwrap();
    b.create_first_local_player_with(&[], &[], || Ok(VWorldState::with_caption(Vec::new())))
        .unwrap();
    assert!(a.kingdom_name(&p).unwrap().is_empty());
    assert!(b.kingdom_name(&p).is_err());
    assert_eq!(a.current_phase().unwrap(), 0);
    assert_eq!(a.application_state().unwrap(), 0);
    assert_eq!(
        serde_json::to_value(constructor_launch_state()).unwrap()["field_0c"],
        json!(u32::MAX)
    );
    let candidate = Some(aa_runtime::load_tail_world::KingdomId::from_raw(0x01000055));
    assert!(
        a.replace_kingdom_with(&p, candidate, || Err(missing(
            "injected view text source failure"
        )))
        .is_err()
    );
    assert_eq!(a.player(&p).unwrap().core.field_28, candidate); // store precedes replacement failure
    assert!(a.player(&p).unwrap().view.is_none());
    assert!(aa_runtime::load_tail_world::SessionRead::player_kingdom_28(&a, &p).is_err());
}

#[test]
fn ordering_selection_borrows_actual_admin_and_disjoint_session_fields() {
    use aa_runtime::player_selection::View;
    let mut owner = LocalBootstrap::from_native_constructors();
    owner
        .reset_fresh(
            &mut os(17, &[0; 16]),
            &InitialProperties::from_native_constructor(),
            &Notifications::new(0),
        )
        .unwrap();
    let p = owner
        .create_first_local_player_with(&[65, 66], &[], || {
            Ok(VWorldState::with_caption(Vec::new()))
        })
        .unwrap();
    // Explicit fixture Admin owner. Constructor-null must not inherit Node8.
    let mut admin = aa_orders::net::admin::AdminNode::new(0).unwrap();
    assert!(owner.ordering_selection_borrow(&admin).is_err());
    admin.client.field_284 = p.id().wrapping_add(1);
    assert!(owner.ordering_selection_borrow(&admin).is_err());
    admin.client.field_284 = p.id();
    owner.session.phase = 7; // explicit live-owner mutation probe, not startup default
    owner.session.state = 8;
    owner.session.multiplayer = true;
    {
        let (mut view, session) = owner.ordering_selection_and_session_borrow(&admin).unwrap();
        assert_eq!(View::player_id(&view), p.id());
        assert_eq!(session.current_phase().unwrap(), 7);
        assert_eq!(session.application_state().unwrap(), 8);
        assert!(session.multiplayer());
        assert_eq!(session.local_player_name_8().unwrap(), Some(&[65, 66][..]));
        assert_eq!(session.ordering_player_name_8(), &[65, 66]);
        assert!(View::list(&view).is_empty());
        assert!(View::group_is_null(&view).unwrap());
        View::mask_words_mut(&mut view)[2] = 0x89abcdef;
        View::look_at_mut(&mut view)[1] = 0x12345678;
        View::clear_group(&mut view);
    }
    let v = owner.player(&p).unwrap().view.as_ref().unwrap();
    assert_eq!(v.mask_words_10_1c[2], 0x89abcdef);
    assert_eq!(v.look_at_28_2c[1], 0x12345678);
    assert!(v.current_group.is_null());
    // Explicit second-player owner fixture: the DevMode name is still Node8,
    // while selection follows the different live Admin284 player.
    let mut second = PlayerFields::from_constructor(2, PeerLink::Node);
    second.name = vec![90];
    owner.session.players.push(OwnedPlayer {
        allocation_lifetime: Arc::new(()),
        core: second,
        vector_44: Vec::new(),
        view: Some(VWorldState::with_caption(Vec::new())),
    });
    admin.client.field_284 = 2;
    {
        let (view, session) = owner.ordering_selection_and_session_borrow(&admin).unwrap();
        assert_eq!(view.player_id(), 2);
        assert_eq!(session.ordering_player_name_8(), &[90]);
        assert_eq!(session.local_player_name_8().unwrap(), Some(&[65, 66][..]));
    }
    owner.local_player = Some(99); //corrupt actual local id, with lazy error
    let (_, session) = owner.ordering_selection_and_session_borrow(&admin).unwrap();
    assert_eq!(session.current_phase().unwrap(), 7);
    assert!(session.local_player_name_8().is_err());
}

#[test]
fn native_self_host_composition_retains_one_actual_admin_and_log() {
    struct Timer;
    impl crate::local_host::MillisecondTimer for Timer {
        fn milliseconds(&mut self) -> std::result::Result<u32, String> {
            Ok(1000)
        }
    }
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-local-bootstrap/host/native-local-host.json"
    ))
    .unwrap();
    for row in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["behind"] == 0)
    {
        let pc = match row["precision"].as_str().unwrap() {
            "Single" => aa_numeric::Precision::Single24,
            "Double" => aa_numeric::Precision::Double53,
            _ => aa_numeric::Precision::Extended64,
        };
        let mut owner = LocalBootstrap::from_native_constructors();
        owner
            .reset_node_fresh(
                &mut os(
                    0x12345678,
                    &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
                ),
                &InitialProperties::from_native_constructor(),
                &Notifications::new(0),
            )
            .unwrap();
        let admin = owner.order_node().unwrap() as *const aa_orders::net::admin::AdminNode;
        let sync = owner.sync_a_cell() as *const RefCell<Synchronizer>;
        let player = owner
            .create_first_local_player_with(
                &"Local Host".encode_utf16().collect::<Vec<_>>(),
                &[],
                || {
                    Ok(VWorldState::with_caption(
                        "All Units".encode_utf16().collect(),
                    ))
                },
            )
            .unwrap();
        let mut time = crate::local_host::NodeTime::from_native_constructor(Timer);
        time.begin_update_prefix(pc).unwrap();
        owner
            .finish_fresh_self_host_with(&mut time, || Ok(row["rate"].as_u64().unwrap() as u32), pc)
            .unwrap();
        assert_eq!(owner.progress(), Progress::LocalHostCreated);
        assert_eq!(owner.host(), PeerLink::Node);
        assert_eq!(owner.snapshot().peer_list, &[PeerLink::Node]);
        assert_eq!(owner.snapshot().sync_a_hash, 0);
        assert_eq!(
            json!(owner.snapshot().sync_a_end),
            row["after"]["sync_a_end"]
        );
        assert_eq!(owner.snapshot().sync_a_end, 1); //native77D900(index0) retires to index+1
        assert_eq!(
            admin,
            owner.order_node().unwrap() as *const aa_orders::net::admin::AdminNode
        );
        assert_eq!(sync, owner.sync_a_cell() as *const RefCell<Synchronizer>);
        assert!(
            owner
                .finish_fresh_self_host_with(&mut time, || Ok(0x41800000), pc)
                .is_err()
        );
        // Actual canonical slot loan, no copied Admin owner or selected list.
        let mut loan = owner.runtime_borrow().unwrap();
        assert!(!loan.players.film_mode_24()); //native76FAE7
        assert!(loan.local_client.is_host_self());
        assert_eq!(loan.local_client.order_version(), 1);
        assert!(!loan.local_client.mirror_enabled()); //native75D074
        assert_eq!(loan.local_client.player_2c(), Some(player.id()));
        let borrowed_player = aa_runtime::load_tail_world::SessionRead::local_player(&loan.players)
            .unwrap()
            .unwrap();
        assert_eq!(borrowed_player.slot, player.slot);
        assert!(
            aa_runtime::load_tail_world::SessionRead::player_name_24(
                &loan.players,
                &borrowed_player
            )
            .unwrap()
            .is_empty()
        );
        assert!(
            aa_runtime::load_tail_world::SessionRead::player_kingdom_28(
                &loan.players,
                &borrowed_player
            )
            .is_err()
        );
        loan.admin.client.field_284 = player.id(); //explicit dispatch fixture input
        let (selection, session) = loan
            .players
            .ordering_selection_and_session_for_slot(loan.admin.ordering_player_slot())
            .unwrap();
        assert_eq!(selection.player_id(), player.id());
        assert_eq!(session.current_phase().unwrap(), 0);
        selection.mask_words_10_1c[0] = 0x1234;
        loan.sync_a.borrow_mut().hash = 27; //probe same retained A allocation
        drop((selection, session));
        drop(loan);
        assert_eq!(
            owner
                .player(&player)
                .unwrap()
                .view
                .as_ref()
                .unwrap()
                .mask_words_10_1c[0],
            0x1234
        );
        assert_eq!(owner.snapshot().sync_a_hash, 27);
    }
}

#[test]
fn scoped_order_lookup_preserves_film_discriminator_and_constructor_fields() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-local-bootstrap/host/native-admin-default.json"
    ))
    .unwrap();
    for row in fixture["cases"].as_array().unwrap() {
        let mut owner = LocalBootstrap::from_native_constructors();
        owner
            .reset_node_fresh(
                &mut os(1, &[0; 16]),
                &InitialProperties::from_native_constructor(),
                &Notifications::new(0),
            )
            .unwrap();
        let first = owner
            .create_first_local_player_with(&[], &[], || Ok(VWorldState::with_caption(Vec::new())))
            .unwrap();
        // Explicit lookup-owner mutation probe, not a completed film startup.
        let mut other = PlayerFields::from_constructor(first.id(), PeerLink::Node);
        other.film = true;
        owner.session.players.push(OwnedPlayer {
            allocation_lifetime: Arc::new(()),
            core: other,
            vector_44: Vec::new(),
            view: Some(VWorldState::with_caption(Vec::new())),
        });
        let mut loan = owner.runtime_borrow().unwrap();
        assert_eq!(
            u8::from(loan.players.film_mode_24()) as u64,
            row["constructor_fields"]["session_film_24"]
                .as_u64()
                .unwrap()
        );
        assert_eq!(
            u8::from(loan.local_client.mirror_enabled()) as u64,
            row["constructor_fields"]["node_mirror_268"]
                .as_u64()
                .unwrap()
        );
        let live = loan.players.find_order_player(first.id(), false).unwrap();
        let film = loan.players.find_order_player(first.id(), true).unwrap();
        assert_ne!(live.slot, film.slot);
        assert!(!loan.players.player_film(&live).unwrap());
        assert!(loan.players.player_film(&film).unwrap());
        assert_eq!(loan.players.player_link(&live).unwrap(), PeerLink::Node);
        assert!(loan.players.find_order_player(99, false).is_none());
        loan.local_client.bind_player_74b350(film.id());
        assert_eq!(loan.local_client.player_2c(), Some(film.id()));
        let foreign = PlayerHandle {
            owner: Weak::new(),
            allocation: Weak::new(),
            id: live.id,
            slot: live.slot,
        };
        assert!(loan.players.player_link(&foreign).is_err());
        loan.sync_s.hash = 0x1234;
        drop(loan);
        assert_eq!(owner.sync_s.borrow().hash, 0x1234);
    }
}
