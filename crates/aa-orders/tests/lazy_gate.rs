use aa_orders::{
    Error, GateSource, Packet, Position, Registry, Result, commands::WireWriter, dispatch,
    net::transport::finalize,
};
use std::{cell::Cell, rc::Rc};
struct Current {
    session: Rc<Cell<u8>>,
    admin: Rc<Cell<u8>>,
    reads: Vec<&'static str>,
    fail_admin: bool,
}
impl GateSource for Current {
    fn session_byte(&mut self) -> Result<u8> {
        self.reads.push("session");
        Ok(self.session.get())
    }
    fn admin_byte(&mut self) -> Result<u8> {
        self.reads.push("admin");
        if self.fail_admin {
            Err(Error::SchemaContext("actual Node byte unavailable"))
        } else {
            Ok(self.admin.get())
        }
    }
    fn store_session_byte(&mut self, v: u8) -> Result<()> {
        self.session.set(v);
        Ok(())
    }
}
fn packet() -> Packet {
    let mut w = WireWriter::new(Position::PACKET_START).unwrap();
    w.write_uint(0, 1).unwrap();
    w.write_uint(1, 1).unwrap();
    Packet::new(finalize(&w).unwrap()).unwrap()
}
#[test]
fn actual_byte_gate_is_lazy_and_observes_callback_writes_before_next_handler() {
    let r = Registry::new(vec!["a".into(), "b".into()]).unwrap();
    for initial in [0, 1, 128, 255] {
        let session = Rc::new(Cell::new(initial));
        let admin = Rc::new(Cell::new(0));
        let mut g = Current {
            session: session.clone(),
            admin: admin.clone(),
            reads: Vec::new(),
            fail_admin: false,
        };
        let d = dispatch(Some(&packet()), None, &r, false, &mut g, |i, _, _| {
            assert_eq!(i, 0);
            session.set(128);
            admin.set(255);
            Ok(())
        })
        .unwrap();
        assert_eq!(d.executed, 1);
        assert_eq!(d.stopped_before_execute, Some(1));
        assert_eq!(
            g.reads,
            if initial == 0 {
                vec!["session", "session", "admin"]
            } else {
                vec!["session", "admin", "session", "admin"]
            }
        );
    }
}
#[test]
fn missing_admin_is_unobserved_until_session_nonzero_and_keeps_decode_prefix() {
    let r = Registry::new(vec!["a".into(), "b".into()]).unwrap();
    let mut g = Current {
        session: Rc::new(Cell::new(0)),
        admin: Rc::new(Cell::new(0)),
        reads: Vec::new(),
        fail_admin: true,
    };
    let mut calls = 0;
    let e = dispatch(Some(&packet()), None, &r, false, &mut g, |_, _, gate| {
        calls += 1;
        gate.store_session_byte(128).unwrap();
        Ok(())
    })
    .unwrap_err();
    assert!(e.to_string().contains("actual Node byte unavailable"));
    assert_eq!(calls, 1);
    assert_eq!(g.reads, ["session", "session", "admin"]);
}
#[test]
fn original_v14_raw_gate_read_order_for_all_supported_precision_modes() {
    let d: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/development10/live-frame-host/native-order-gate.json"
    ))
    .unwrap();
    assert_eq!(d["cases"].as_array().unwrap().len(), 96);
    for c in d["cases"].as_array().unwrap() {
        let mut gate = Current {
            session: Rc::new(Cell::new(c["session"].as_u64().unwrap() as u8)),
            admin: Rc::new(Cell::new(c["admin"].as_u64().unwrap() as u8)),
            reads: Vec::new(),
            fail_admin: false,
        };
        let stopped = gate.session_byte().unwrap() != 0 && gate.admin_byte().unwrap() != 0;
        assert_eq!(serde_json::to_value(gate.reads).unwrap(), c["reads"]);
        assert_eq!(stopped, c["stopped"].as_bool().unwrap());
    }
}
