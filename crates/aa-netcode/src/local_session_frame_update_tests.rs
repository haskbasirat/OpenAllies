use super::*;
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-session-frame-update/native-session-frame-update.json"
    ))
    .unwrap()
}

#[test]
fn native_recurring_update_order_and_required_prefixes_match_same_owner() {
    let fixture = fixture();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 438);
    for row in fixture["rows"].as_array().unwrap() {
        let input = &row["input"];
        let mut owner = LocalBootstrap::from_native_constructors();
        owner.host_node = Some(crate::local_host::Node::before_session_reset().unwrap());
        owner.node_host = if input["host"].as_bool().unwrap() {
            PeerLink::Node
        } else {
            PeerLink::Null
        };
        owner.session.host_countdown = input["countdown"].as_i64().unwrap() as i32;
        owner.session.state = input["state"].as_u64().unwrap() as u32;
        owner.session.flags_168_16e[4] = Some(input["sent"].as_bool().unwrap());
        owner.host_node.as_mut().unwrap().takeover_2e0 = input["takeover"].as_u64().unwrap() as u32;
        let session = &owner.session as *const _;
        let node = owner.host_node.as_ref().unwrap() as *const _;
        let status = input["task_status"].as_u64().map(|x| x as u32);
        //Raw positive status rows are an explicit native getter projection,
        //not a fabricated Task/StartedMission. Public NULL rows exercise run.
        let result = {
            let mut loan = owner.runtime_borrow().unwrap();
            if let Some(status) = status {
                leave_step::check_status_775db0(status, false)
                    .and_then(|_| after_task(loan.players.session, &loan.local_client))
            } else {
                run(&mut loan.players, &mut loan.local_client)
            }
        };
        let stop = row["stop"].as_str();
        match stop {
            None => assert!(result.is_ok(), "{input}: {result:?}"),
            Some("finished_task_diagnostic") => assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("775DB0 positive task")
            ),
            Some("countdown_readiness") => {
                assert!(result.unwrap_err().to_string().contains("actual76C960"))
            }
            Some("migration_request") => {
                assert!(result.unwrap_err().to_string().contains("actual76B570"))
            }
            _ => panic!("unrecognized boundary {stop:?}"),
        }
        assert_eq!(
            owner.session.flags_168_16e[4],
            Some(row["sent_after"].as_bool().unwrap())
        );
        assert_eq!(
            owner.session.host_countdown,
            row["countdown_after"].as_i64().unwrap() as i32
        );
        assert!(row["task_retained"].as_bool().unwrap());
        assert!(owner.task_is_null_124());
        assert_eq!(&owner.session as *const _, session);
        assert_eq!(owner.host_node.as_ref().unwrap() as *const _, node);
        //Native fixture records actual call/read order. NULL still reached
        //both remaining native bodies, except when readiness stopped first.
        if status.is_none() && stop != Some("countdown_readiness") {
            assert!(
                row["calls"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v == "76C4B0")
            );
        }
        //The native read footprint pins short-circuit order independently
        //of a final return value, including the2E0 getter before SessionE4.
        let mut reads = vec!["Session124"];
        if status.is_some() {
            reads.push("Task14");
        }
        if stop != Some("finished_task_diagnostic") {
            reads.push("Node2C4");
            let host = input["host"].as_bool().unwrap();
            if host {
                reads.push("Session11C");
            }
            if stop != Some("countdown_readiness") {
                reads.push("Session16C");
                if !input["sent"].as_bool().unwrap() {
                    reads.push("Node2C4");
                    if host {
                        reads.extend(["Node2C4", "Node2E0"]);
                        if input["takeover"].as_u64().unwrap() != 0 {
                            reads.push("SessionE4");
                        }
                    }
                }
            }
        }
        assert_eq!(serde_json::json!(reads), row["reads"], "{input}");
    }
}

#[test]
fn constructor_word_foreign_owner_and_late_unbound_byte_are_preserved() {
    let mut owner = LocalBootstrap::from_native_constructors();
    owner.host_node = Some(crate::local_host::Node::before_session_reset().unwrap());
    assert_eq!(owner.host_node.as_ref().unwrap().takeover_2e0, 0);
    let fixture = fixture();
    assert_eq!(fixture["constructors"].as_array().unwrap().len(), 12);
    for row in fixture["constructors"].as_array().unwrap() {
        assert_eq!(row["takeover_after"], 0);
    }
    owner.node_host = PeerLink::Node;
    owner.session.flags_168_16e[4] = None; //Explicit unbound-field error probe.
    owner.session.host_countdown = 1;
    {
        let mut loan = owner.runtime_borrow().unwrap();
        //Readiness occurs before the missing16C read.
        assert!(
            run(&mut loan.players, &mut loan.local_client)
                .unwrap_err()
                .to_string()
                .contains("actual76C960")
        );
    }
    owner.session.host_countdown = 0;
    {
        let mut loan = owner.runtime_borrow().unwrap();
        assert!(
            run(&mut loan.players, &mut loan.local_client)
                .unwrap_err()
                .to_string()
                .contains("Session16C")
        );
    }
    let mut foreign = LocalBootstrap::from_native_constructors();
    foreign.host_node = Some(crate::local_host::Node::before_session_reset().unwrap());
    let mut first = owner.runtime_borrow().unwrap();
    let mut second = foreign.runtime_borrow().unwrap();
    assert!(
        run(&mut first.players, &mut second.local_client)
            .unwrap_err()
            .to_string()
            .contains("different owners")
    );
}
