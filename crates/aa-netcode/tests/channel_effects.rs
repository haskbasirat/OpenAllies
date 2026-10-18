//! The heartbeat port (SetServer 0x759790 / SetHost 0x757010) emits the
//! channel calls of 0x752300 and 0x752490 as `Effect::Channel`; applied
//! through `packets::apply_channel_op` they must leave the channel objects
//! exactly as the packet layer's own bindings (proven by
//! native-packets.json) do.
use aa_netcode::heartbeat::{Channel as C, ChannelOp as O, HistoryRef as H};
use aa_netcode::packets::{apply_channel_op, AdminConn, NetworkAvars, NoFileTransfer, NodeHistories};
use aa_orders::{History, Packet};

struct Node {
    requests: History,
    backup_requests: History,
    decrees: History,
    backup_decrees: History,
    next: i32,
}
impl Node {
    fn new() -> Self {
        let mut decrees = History::new(5, 8).unwrap();
        for i in 5..8 {
            decrees.put(i, Some(Packet::new(vec![0x05, i as u8]).unwrap())).unwrap();
        }
        Self {
            requests: History::new(0, 8).unwrap(),
            backup_requests: History::new(0, 8).unwrap(),
            decrees,
            backup_decrees: History::new(3, 8).unwrap(),
            next: 0,
        }
    }
    fn view(&mut self) -> NodeHistories<'_> {
        NodeHistories {
            requests: &mut self.requests,
            backup_requests: &mut self.backup_requests,
            decrees: &mut self.decrees,
            backup_decrees: &mut self.backup_decrees,
            backup_decree_next: &mut self.next,
        }
    }
}

fn snapshot(c: &AdminConn) -> String {
    format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
        c.decree_out,
        (c.decree_in.bound, &c.decree_in.blocks),
        c.reverse_out,
        (c.reverse_in.bound, &c.reverse_in.blocks),
        c.request_out,
        (c.request_in.bound, &c.request_in.blocks),
        c.private_out,
        (c.private_in.bound, &c.private_in.blocks),
        c.backup_decree_out,
        (c.backup_decree_in.bound, &c.backup_decree_in.blocks),
        c.backup_request_out,
        (c.backup_request_in.bound, &c.backup_request_in.blocks),
    )
}

#[test]
fn heartbeat_channel_effects_match_packet_bindings() {
    let avars = NetworkAvars::default();
    let (mut fs, mut fr) = (NoFileTransfer::new(&avars), NoFileTransfer::new(&avars));
    let i = 3;

    // 0x752300 as bind_backup_server emits it.
    let mut node = Node::new();
    let mut a = AdminConn::new().unwrap();
    let start = node.backup_decrees.end();
    for (ch, op) in [
        (C::BackupDecreeSender, O::SenderBind { history: H::NodeBackupDecrees, start }),
        (C::BackupDecreeReceiver, O::ReceiverReset),
        (C::BackupRequestSender, O::SenderReset),
        (C::BackupRequestReceiver, O::ReceiverBind { history: H::ConnBackupRequests(i) }),
        (C::ReverseDecreeSender, O::SenderReset),
        (C::ReverseDecreeReceiver, O::ReceiverBind { history: H::ConnReverseDecrees(i) }),
    ] {
        apply_channel_op(&mut a, i, &mut node.view(), ch, op, &mut fs, &mut fr).unwrap();
    }
    let mut node2 = Node::new();
    let mut b = AdminConn::new().unwrap();
    b.reset_backup_senders_to_client(&mut node2.view()).unwrap();
    assert_eq!(snapshot(&a), snapshot(&b));

    // 0x752490(start) as bind_host_channels emits it.
    let start = 6;
    for (ch, op) in [
        (C::PrivateDecreeSender, O::SenderBind { history: H::ConnPrivateDecrees(i), start: 0 }),
        (C::PrivateDecreeReceiver, O::ReceiverReset),
        (C::BroadcastDecreeSender, O::SenderBind { history: H::NodeDecrees, start }),
        (C::BroadcastDecreeReceiver, O::ReceiverReset),
        (C::BroadcastRequestSender, O::SenderReset),
        (C::BroadcastRequestReceiver, O::ReceiverBind { history: H::ConnRequests(i) }),
    ] {
        apply_channel_op(&mut a, i, &mut node.view(), ch, op, &mut fs, &mut fr).unwrap();
    }
    b.reset_senders_to_client(start).unwrap();
    assert_eq!(snapshot(&a), snapshot(&b));
    assert_eq!(a.base.requests.begin(), b.base.requests.begin());

    // A binding the packet layer does not use is refused, not applied.
    let err = apply_channel_op(
        &mut a,
        i,
        &mut node.view(),
        C::BroadcastDecreeSender,
        O::SenderBind { history: H::NodeBackupDecrees, start: 0 },
        &mut fs,
        &mut fr,
    );
    assert!(err.is_err());
    let err = apply_channel_op(
        &mut a,
        i,
        &mut node.view(),
        C::BroadcastRequestReceiver,
        O::ReceiverBind { history: H::ConnRequests(i + 1) },
        &mut fs,
        &mut fr,
    );
    assert!(err.is_err());
}
