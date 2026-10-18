use super::*;

#[test]
fn constructor_null_task_has_no_minted_completion_and_node_votes_are_same_owner() {
    let mut owner = LocalBootstrap::from_native_constructors();
    assert!(owner.task_is_null_124());
    owner.check_finished_task_775db0(false).unwrap();
    owner.check_finished_task_775db0(true).unwrap();
    let notifications = Notifications::new(0);
    assert!(
        owner
            .launch_step_770500(&notifications)
            .unwrap_err()
            .to_string()
            .contains("actual Node")
    );
    owner.host_node = Some(crate::local_host::Node::before_session_reset().unwrap());
    assert_eq!(owner.host_node.as_ref().unwrap().vote_count_64(), 0);
    owner.peer_list.push(PeerLink::Node); //explicit singleton native-list fixture
    owner
        .host_node
        .as_mut()
        .unwrap()
        .fixture_votes(vec![PeerLink::Node, PeerLink::Node]);
    let node = owner.host_node.as_ref().unwrap() as *const _ as usize;
    let admin = &owner.host_node.as_ref().unwrap().orders as *const _ as usize;
    owner.launch_step_770500(&notifications).unwrap();
    assert_eq!(owner.host_node.as_ref().unwrap().vote_count_64(), 0);
    assert_eq!(owner.host_node.as_ref().unwrap() as *const _ as usize, node);
    assert_eq!(
        &owner.host_node.as_ref().unwrap().orders as *const _ as usize,
        admin
    );
    assert!(notifications.is_empty().unwrap());
}

#[test]
fn positive_native_notification_is_same_queue_and_unowned_states_preserve_votes() {
    let mut owner = LocalBootstrap::from_native_constructors();
    owner.host_node = Some(crate::local_host::Node::before_session_reset().unwrap());
    owner.peer_list.push(PeerLink::Node);
    let notifications = Notifications::new(0);
    for state in [1, 2, 4, 5] {
        owner.session.state = state;
        owner
            .host_node
            .as_mut()
            .unwrap()
            .fixture_votes(vec![PeerLink::Node]);
        assert!(owner.launch_step_770500(&notifications).is_err());
        assert_eq!(owner.host_node.as_ref().unwrap().vote_count_64(), 1);
        assert!(notifications.is_empty().unwrap());
    }
    owner.session.state = 0;
    owner.session.launch_mode = 1;
    owner.session.launch_game_type = 4;
    owner.launch_step_770500(&notifications).unwrap();
    let message = notifications.pop().unwrap().unwrap();
    assert_eq!(message.kind, 3);
    assert_eq!(message.text, Vec::<u16>::new());
    assert_eq!(message.state, 0);
    assert_eq!(message.launch_state, constructor_launch_state());
    assert!(!message.flag_34);
    assert_eq!(message.word_38, 0);
    assert_eq!(owner.host_node.as_ref().unwrap().vote_count_64(), 0);
}

fn fixture(name: &str) -> serde_json::Value {
    serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/skirmish-20261017-local-leave-step")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn native_constructor_reset_default_state_and_notification_fixtures_match() {
    for row in fixture("native-leave-step.json")["rows"]
        .as_array()
        .unwrap()
    {
        let case = &row["input"];
        if case["kind"] == "task" {
            let mut owner = LocalBootstrap::from_native_constructors();
            owner
                .check_finished_task_775db0(case["block"].as_u64().unwrap() != 0)
                .unwrap();
            assert_eq!(
                serde_json::json!(owner.task_is_null_124()),
                row["output"]["task_null"]
            );
        } else if case["kind"] == "constructor" {
            let node = crate::local_host::Node::before_session_reset().unwrap();
            assert_eq!(node.vote_count_64(), 0);
            assert_eq!(row["output"]["head"], 0);
            assert_eq!(row["output"]["tail"], 0);
            assert_eq!(
                serde_json::json!(node.order_version),
                row["output"]["order_version"]
            );
        } else {
            let counts = case["counts"].as_array().unwrap();
            //Each74DA10/7514F0 list-clear body is proved, including the raw
            //multi-peer fixture. Concrete remote peer storage remains unbound.
            for (index, count) in counts.iter().enumerate() {
                let mut owner = LocalBootstrap::from_native_constructors();
                owner.host_node = Some(crate::local_host::Node::before_session_reset().unwrap());
                owner.peer_list.push(PeerLink::Node);
                owner.host_node.as_mut().unwrap().fixture_votes(vec![
                    PeerLink::Node;
                    count.as_u64().unwrap()
                        as usize
                ]);
                if case["kind"] == "step" {
                    owner.session.state = case["state"].as_u64().unwrap() as u32;
                    owner.launch_step_770500(&Notifications::new(0)).unwrap();
                } else {
                    owner.host_node.as_mut().unwrap().clear_votes_74da10();
                }
                assert_eq!(
                    owner.host_node.as_ref().unwrap().vote_count_64(),
                    row["output"]["votes"][index].as_array().unwrap().len()
                );
            }
        }
    }
    for row in fixture("native-leave-notification.json")["rows"]
        .as_array()
        .unwrap()
    {
        let case = &row["input"];
        let mut owner = LocalBootstrap::from_native_constructors();
        owner.host_node = Some(crate::local_host::Node::before_session_reset().unwrap());
        owner.peer_list.push(PeerLink::Node);
        owner.host_node.as_mut().unwrap().fixture_votes(vec![
            PeerLink::Node;
            case["votes"].as_u64().unwrap()
                as usize
        ]);
        owner.session.launch_mode = case["mode"].as_u64().unwrap() as u32;
        owner.session.launch_game_type = case["game_type"].as_u64().unwrap() as u32;
        let notifications = Notifications::new(case["application_kind"].as_u64().unwrap() as u32);
        owner.launch_step_770500(&notifications).unwrap();
        let mut messages = Vec::new();
        while let Some(m) = notifications.pop().unwrap() {
            messages.push(m);
        }
        assert_eq!(serde_json::json!(messages), row["output"]["queued"]);
        assert_eq!(owner.host_node.as_ref().unwrap().vote_count_64(), 0);
    }
}

#[test]
fn actual_fresh_reset_retains_null_task_and_same_node_without_task_result_or_new_message() {
    struct FixtureOs;
    impl ResetOs for FixtureOs {
        fn get_tick_count(&mut self) -> Result<u32> {
            Ok(7)
        }
        fn co_create_guid(&mut self) -> Result<Guid> {
            Ok(Guid {
                data1: 0,
                data2: 0,
                data3: 0,
                data4: [0; 8],
            })
        }
        fn sync_log_gate(&mut self) -> Result<bool> {
            panic!("disabled constructor synchronizer must not reach gate")
        }
    }
    let mut owner = LocalBootstrap::from_native_constructors();
    let notifications = Notifications::new(0);
    owner
        .reset_node_fresh(
            &mut FixtureOs,
            &InitialProperties::from_native_constructor(),
            &notifications,
        )
        .unwrap();
    let node = owner.host_node.as_ref().unwrap() as *const _ as usize;
    let prior = notifications.len().unwrap();
    assert_eq!(prior, 1);
    assert!(owner.task_is_null_124());
    owner.check_finished_task_775db0(true).unwrap();
    owner.launch_step_770500(&notifications).unwrap();
    assert_eq!(owner.host_node.as_ref().unwrap() as *const _ as usize, node);
    assert_eq!(owner.host_node.as_ref().unwrap().vote_count_64(), 0);
    assert_eq!(notifications.len().unwrap(), prior);
    assert!(owner.task_is_null_124());
    assert_eq!(owner.progress, Progress::Reset);
}
