use super::*;
use serde_json::{Value, json};

struct Probe<'a> {
    input: &'a Value,
    trace: Vec<String>,
    initialized: bool,
    counters: u32,
}
impl Probe<'_> {
    fn step(&mut self, name: String) -> Result<()> {
        self.trace.push(name.clone());
        if self.input["cutoff"].as_str() == Some(&name) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl Constructor for Probe<'_> {
    fn allocate(&mut self, size: u32) -> Result<bool> {
        self.step(format!("allocate:{size}"))?;
        Ok(self.input["fail_alloc"].as_u64() != Some(size as u64))
    }
    fn timer(&mut self) -> Result<TimerAnchor> {
        if !self.initialized {
            self.step("frequency".into())?;
            self.initialized = true;
        }
        self.step(format!("counter:{}", self.counters))?;
        let counter =
            self.input["counter"].as_i64().unwrap_or(123456789) + i64::from(self.counters);
        self.counters += 1;
        TimerAnchor::from_os_counter(
            counter,
            self.input["frequency"].as_i64().unwrap_or(10_000_000),
        )
    }
    fn period(&mut self, p: Period) -> Result<i32> {
        self.step(p.name().into())?;
        Ok(self.input[match p {
            Period::Browser => "sb",
            Period::Query => "qr",
        }]
        .as_i64()
        .unwrap_or(50) as i32)
    }
    fn before_key(&mut self, id: u32) -> Result<()> {
        self.step(format!("key:{id}"))
    }
}
fn words(timer: &Option<TimerAnchor>) -> (u32, u32) {
    timer
        .as_ref()
        .map(|v| (v.counter as u32, (v.counter as u64 >> 32) as u32))
        .unwrap_or((0xa5a5a5a5, 0xa5a5a5a5))
}
fn output(s: &Standalone, p: &Probe<'_>, error: Option<String>) -> Value {
    let sb=s.browser.as_ref().map(|v|{let(a,b)=words(&v.timer);json!({"0x0":0xa9be04u32,"0x8":a,"0xc":b,"0x10":v.period_10.unwrap_or(0xa5a5a5a5),"0x14":v.field_14.unwrap_or(0xa5a5a5a5)})});
    let qr=s.query.as_ref().map(|v|{let(a,b)=words(&v.timer);json!({"0x0":0xa9b974u32,"0x4":v.field_04,"0x8":a,"0xc":b,
        "0x10":v.flag_10.map(|v|0xa5a5a500u32|u32::from(v)).unwrap_or(0xa5a5a5a5),"0x14":v.period_14.unwrap_or(0xa5a5a5a5),"0x18":v.field_18.unwrap_or(0xa5a5a5a5),
        "0x1c":v.flag_1c.map(|v|0xa5a5a500u32|u32::from(v)).unwrap_or(0xa5a5a5a5),"0x20":v.field_20.unwrap_or(0xa5a5a5a5),
        "0x24":v.word_24.map(|v|0xa5a50000u32|u32::from(v)).unwrap_or(0xa5a5a5a5)})});
    let hash=s.hash.as_ref().map(|v|json!({"0x0":0xa9be48u32,"0x8":v.count_08,"0xc":v.size_0c,"0x10":0xa5a5a500u32|u32::from(v.flag_10)}));
    let frequency = if p.initialized {
        p.input["frequency"].as_i64().unwrap_or(10_000_000)
    } else {
        0
    };
    let quotient = frequency as u32 / 1000;
    json!({"error":error,"trace":p.trace,"standalone_flag":s.standalone_flag,"online":p.input["prior_online"].as_u64().unwrap_or(0),
        "sb":sb,"qr":qr,"hash":hash,"bucket_count":s.hash.as_ref().and_then(|v|v.buckets_04.as_ref()).map(|v|v.iter().filter(|&&v|v==0).count()),
        "keys":s.keys,"timer":{"initialized":u32::from(p.initialized),"frequency_low":frequency as u32,"frequency_high":(frequency as u64>>32)as u32,
        "quotient":quotient,"inverse":if quotient==0 {0}else{u32::MAX/quotient}}})
}
#[test]
fn native_all_three_constructors_and_required_error_prefixes_replay() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/development10/standalone-network/native-standalone.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 126);
    for row in fixture["rows"].as_array().unwrap() {
        let mut s = Standalone::pending();
        let mut p = Probe {
            input: &row["input"],
            trace: vec![],
            initialized: false,
            counters: 0,
        };
        let result = construct(&mut s, &mut p);
        let error = result.err();
        assert_eq!(output(&s, &p, error.clone()), row["output"], "{row}");
        assert_eq!(s.complete, error.is_none());
    }
    for row in fixture["globals"].as_array().unwrap() {
        assert_eq!(row["value"], 0);
    }
}
#[test]
fn undefined_native_timer_arithmetic_is_refused() {
    assert!(TimerAnchor::from_os_counter(1, 999).is_err());
    assert!(TimerAnchor::from_os_counter(1, 0).is_err());
    let value = TimerAnchor::from_os_counter(123456789, 10_000_000).unwrap();
    assert_eq!(value.counter(), 123456789);
    for value in [0, 50, -50, 16777216, i32::MIN] {
        assert!(exact_binary32_integer(value));
    }
    for value in [16777217, -16777217, i32::MAX] {
        assert!(!exact_binary32_integer(value));
    }
}
struct SourceClock(u32);
impl Clock for SourceClock {
    fn start_timer(&mut self) -> Result<TimerAnchor> {
        self.0 += 1;
        TimerAnchor::from_os_counter(123000000 + i64::from(self.0), 10_000_000)
    }
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source files"]
fn actual_fresh_application_choice_and_same_retained_owner() {
    use crate::restore_lifecycle::{ScopedManagedLoad, TaskKind};
    use aa_rules::{actors::ActorCatalog, sources::SourceLimits};
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let catalog = Catalog::new(&actors);
    let mut app = ScopedManagedLoad::from_sources(&mut files, 256).unwrap();
    let mut clock = SourceClock(0);
    assert!(app.standalone().is_err());
    app.select_local_offline(
        &catalog,
        &mut files,
        NativeOverrides::ControlledNone,
        &mut clock,
    )
    .unwrap();
    let native = app.standalone().unwrap();
    assert!(!native.online_manager_present().unwrap());
    assert_eq!(native.browser_period().unwrap(), Some(50));
    assert_eq!(native.query_period_bits().unwrap(), Some(50f32.to_bits()));
    assert_eq!(native.registered_key(54).unwrap(), Some("locale"));
    assert!(native.registered_key(1).is_err());
    assert_eq!(clock.0, 2);
    let buckets = native
        .hash
        .as_ref()
        .unwrap()
        .buckets_04
        .as_ref()
        .unwrap()
        .as_ptr();
    assert!(
        app.select_local_offline(
            &catalog,
            &mut files,
            NativeOverrides::ControlledNone,
            &mut clock
        )
        .is_err()
    );
    let mut load = app.begin(TaskKind::Normal).unwrap();
    let result: Result<()> =
        load.with_startup_engine(|_| Err("explicit laboratory activation stop".into()));
    assert!(result.is_err());
    let startup = load.into_startup_owner().unwrap();
    assert_eq!(
        startup
            .standalone()
            .unwrap()
            .hash
            .as_ref()
            .unwrap()
            .buckets_04
            .as_ref()
            .unwrap()
            .as_ptr(),
        buckets
    );
    let mut failed = ScopedManagedLoad::from_sources(&mut files, 256).unwrap();
    assert!(
        failed
            .select_local_offline(&catalog, &mut files, NativeOverrides::Unbound, &mut clock)
            .is_err()
    );
    assert!(failed.standalone().is_err());
    assert!(
        failed
            .select_local_offline(
                &catalog,
                &mut files,
                NativeOverrides::ControlledNone,
                &mut clock
            )
            .is_err()
    );
    let inputs = startup.standalone().unwrap().inputs.as_ref().unwrap();
    let report = json!({"browser_assignments":inputs.browser.as_ref().unwrap().assignments,"query_assignments":inputs.query.as_ref().unwrap().assignments,"files":inputs.files,
        "browser_period":50,"query_period_bits":50f32.to_bits(),"bucket_count":997,"same_bucket_allocation_after_transfer":true,
        "scope":"Laboratory OS samples; actual Windows producer is separate. No successful mission activation asserted."});
    std::fs::write(
        root.join(
            "rust-rebuild/research/development10/standalone-network/actual-source-inputs.json",
        ),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
