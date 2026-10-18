//!Native522F60/523000 leaves replayed through exact current shared kernels.
use aa_mind::{
    cai_sleep::fields::{self, RefreshFields, RetryFields},
    construct::Kind,
};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/development10/cai-arrivals/native-sleep-fields.json"
    ))
    .unwrap()
}
struct W {
    retry: u32,
    pending: u8,
    flag: u8,
    mode: u32,
    primary: u32,
    attach2: bool,
    count: usize,
    mutation: Option<String>,
    fail: Option<usize>,
    calls: usize,
    active: bool,
    close: bool,
    trace: Vec<Value>,
}
impl RefreshFields for W {
    type Member = u32;
    fn retry(&mut self) -> aa_mind::Result<u32> {
        self.trace.push(json!(["read_retry", self.retry]));
        Ok(self.retry)
    }
    fn store_retry(&mut self, v: u32) -> aa_mind::Result<()> {
        self.trace.push(json!(["retry", v]));
        self.retry = v;
        Ok(())
    }
    fn next_attached(&mut self, i: &mut usize) -> aa_mind::Result<Option<u32>> {
        while *i < self.count {
            let m = [1, 0, 2, 3][*i];
            *i += 1;
            if m == 0 || (m == 2 && !self.attach2) {
                continue;
            }
            return Ok(Some(m));
        }
        Ok(None)
    }
    fn mode(&mut self) -> aa_mind::Result<u32> {
        self.trace.push(json!(["mode", self.mode]));
        Ok(self.mode)
    }
    fn primary(&mut self, m: u32) -> aa_mind::Result<bool> {
        self.trace.push(json!(["primary", m]));
        Ok(m == self.primary)
    }
    fn flag(&mut self) -> aa_mind::Result<u8> {
        self.trace.push(json!(["flag", self.flag]));
        Ok(self.flag)
    }
    fn dispatch(&mut self, m: u32, f: u8) -> aa_mind::Result<()> {
        self.trace.push(json!(["dispatch", m, f]));
        self.calls += 1;
        if self.fail == Some(self.calls) {
            return Err(aa_mind::Error::World("dispatch".into()));
        }
        match self.mutation.as_deref() {
            Some("flag") => self.flag = self.flag.wrapping_add(1),
            Some("mode") => self.mode = 2,
            Some("truncate") => self.count = 0,
            _ => {}
        }
        Ok(())
    }
    fn store_pending(&mut self, v: u8) -> aa_mind::Result<()> {
        self.trace.push(json!(["pending", v]));
        self.pending = v;
        Ok(())
    }
}
impl RetryFields for W {
    fn any_active(&mut self) -> aa_mind::Result<bool> {
        self.trace.push(json!(["active", self.active]));
        Ok(self.active)
    }
    fn all_at_slots(&mut self) -> aa_mind::Result<bool> {
        self.trace.push(json!(["close", self.close]));
        Ok(self.close)
    }
    fn retry(&mut self) -> aa_mind::Result<u32> {
        RefreshFields::retry(self)
    }
    fn store_pending(&mut self, v: u8) -> aa_mind::Result<()> {
        RefreshFields::store_pending(self, v)
    }
    fn refresh(&mut self) -> aa_mind::Result<()> {
        fields::refresh_members(self)
    }
}
#[test]
fn native_refresh_retry_lazy_fields_and_current_callback_mutations() {
    let d = fixture();
    let rows = d["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 402);
    for row in rows {
        let c = &row["input"];
        let mut w = W {
            retry: c["retry"].as_u64().unwrap() as u32,
            pending: 128,
            flag: c["flag"].as_u64().unwrap() as u8,
            mode: c["mode"].as_u64().unwrap() as u32,
            primary: c["primary"].as_u64().unwrap() as u32,
            attach2: c["attach2"].as_bool().unwrap(),
            count: 4,
            mutation: c["mutation"].as_str().map(str::to_owned),
            fail: c["fail"].as_u64().map(|v| v as usize),
            calls: 0,
            active: c["active"].as_bool().unwrap(),
            close: c["close"].as_bool().unwrap(),
            trace: vec![],
        };
        let result = if c["entry"].as_u64() == Some(0x522f60) {
            fields::refresh_members(&mut w)
        } else {
            fields::retry_refresh(&mut w)
        };
        let out = json!({"trace":w.trace,"retry":w.retry,"pending":w.pending,"flag":w.flag,"mode":w.mode,"error":result.err().map(|_|w.fail.unwrap())});
        assert_eq!(out, row["output"], "{c}")
    }
}
#[test]
fn native_member_busy_inherited_false_does_not_read_element_state() {
    let d = fixture();
    let rows = d["getters"].as_array().unwrap();
    assert_eq!(rows.len(), 36);
    for row in rows {
        let c = &row["input"];
        let kind = match c["kind"].as_str().unwrap() {
            "Element" => Kind::Element,
            "Regiment" => Kind::Regiment,
            "Aircraft" => Kind::Aircraft,
            _ => panic!(),
        };
        let mut reads = 0;
        let value = fields::member_busy(kind, || {
            reads += 1;
            Ok(c["status"].as_u64().unwrap() as u32)
        })
        .unwrap();
        assert_eq!(value, u32::from(row["output"].as_u64().unwrap() as u8) != 0);
        assert_eq!(reads, usize::from(kind == Kind::Element));
    }
    assert!(
        fields::member_busy(Kind::Base, || panic!(
            "purecall must not read Element fields"
        ))
        .is_err()
    );
}
