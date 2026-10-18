//! Replays research/phase8-restoration/native-turrets.json: the original
//! 4C7660/4C7820 Turret definition loader executed under Unicorn.
use aa_numeric::Precision;
use aa_rules::{
    native::{Error, Result},
    turrets::{
        BoneIdentity, TurretDataError, TurretDefinition, TurretLoadError, TurretSource,
        load_turret, turret_index,
    },
};
use serde_json::Value;

const FIXTURE: &str = include_str!("../../../research/phase8-restoration/native-turrets.json");

struct Source<'a> {
    input: &'a Value,
    reads: Vec<String>,
}
fn canonical(name: &str) -> Option<&'static str> {
    // Seam used by the harness's 42C070 hook.
    match name {
        "turret" | "Turret" => Some("turret"),
        "turret2" => Some("turret2"),
        "turret3" => Some("turret3"),
        _ => None,
    }
}
impl TurretSource for Source<'_> {
    fn string(&mut self, name: &str) -> Result<String> {
        self.reads.push(name.into());
        Ok(self.input[name].as_str().unwrap().to_owned())
    }
    fn fixed(&mut self, name: &str) -> Result<u32> {
        self.reads.push(name.into());
        Ok(self.input[name].as_u64().unwrap() as u32)
    }
    fn flag(&mut self, name: &str) -> Result<bool> {
        self.reads.push(name.into());
        Ok(self.input[name].as_bool().unwrap())
    }
    fn point2(&mut self, name: &str) -> Result<[u32; 2]> {
        self.reads.push(name.into());
        let v = self.input[name].as_array().unwrap();
        Ok([v[0].as_u64().unwrap() as u32, v[1].as_u64().unwrap() as u32])
    }
    fn bone(&mut self, ids: &str) -> Result<Option<BoneIdentity>> {
        self.reads.push(format!("bone:{ids}"));
        Ok(canonical(ids).map(|id| BoneIdentity { id: id.into() }))
    }
}
fn existing(bone: &str) -> TurretDefinition {
    TurretDefinition {
        bone: BoneIdentity {
            id: canonical(bone).unwrap().into(),
        },
        yaw_start_bits: 0,
        yaw_rate_bits: 0,
        yaw_half_range_bits: 0,
        yaw_half_range_cos_bits: 0,
        pitch_start_bits: 0,
        pitch_rate_bits: 0,
        pitch_range_start_bits: 0,
        pitch_range_end_bits: 0,
        offset_bits: [0; 2],
        reset_time_bits: 0,
        can_reset: false,
        can_preaim: false,
    }
}
fn precision(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        _ => Precision::Extended64,
    }
}
fn error_name(e: TurretDataError) -> &'static str {
    use TurretDataError::*;
    match e {
        BoneUnrecognized => "bone_unrecognized",
        DuplicateBone => "duplicate_bone",
        YawRateRange => "yaw_rate_range",
        PitchRateRange => "pitch_rate_range",
        YawRangeRange => "yaw_range_range",
        PitchRangeStartRange => "pitch_range_start_range",
        PitchRangeEndRange => "pitch_range_end_range",
        PitchRangeEndBeforeStart => "pitch_range_end_before_start",
        YawStartRange => "yaw_start_range",
        PitchStartRange => "pitch_start_range",
    }
}
fn is_nan(bits: u32) -> bool {
    bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x7f_ffff != 0
}

#[test]
fn original_turret_loader_fixtures_replay() {
    let doc: Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() > 1500, "fixture case count {}", cases.len());
    let (mut ok, mut thrown, mut nan_excluded) = (0, 0, 0);
    for case in cases {
        let input = &case["input"];
        let native = &case["native"];
        let pc = precision(case["precision"].as_str().unwrap());
        let prior: Vec<TurretDefinition> = case["existing_bones"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| existing(b.as_str().unwrap()))
            .collect();
        let mut source = Source {
            input,
            reads: Vec::new(),
        };
        let result = load_turret(&mut source, &prior, pc);
        let native_reads: Vec<String> = native["reads"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        match result {
            Ok(turret) => {
                assert!(native["error"].is_null(), "{case}");
                assert_eq!(source.reads, native_reads, "{case}");
                let words: Vec<u32> = native["words"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect();
                let bytes: Vec<u64> = native["bytes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap())
                    .collect();
                let bone_word = match turret.bone.id.as_str() {
                    "turret" => 0x0bad_0100,
                    "turret2" => 0x0bad_0200,
                    "turret3" => 0x0bad_0300,
                    other => panic!("{other}"),
                };
                let ours = [
                    bone_word,
                    turret.yaw_start_bits,
                    turret.yaw_rate_bits,
                    turret.yaw_half_range_bits,
                    turret.yaw_half_range_cos_bits,
                    turret.pitch_start_bits,
                    turret.pitch_rate_bits,
                    turret.pitch_range_start_bits,
                    turret.pitch_range_end_bits,
                    turret.offset_bits[0],
                    turret.offset_bits[1],
                    turret.reset_time_bits,
                ];
                assert_eq!(ours.as_slice(), words.as_slice(), "{case}");
                assert_eq!(
                    [u64::from(turret.can_reset), u64::from(turret.can_preaim)].as_slice(),
                    bytes.as_slice(),
                    "{case}"
                );
                ok += 1;
            }
            Err(TurretLoadError::Data(e)) => {
                assert_eq!(native["error"].as_str(), Some(error_name(e)), "{case}");
                assert_eq!(source.reads, native_reads, "{case}");
                thrown += 1;
            }
            Err(TurretLoadError::Source(Error::Unsupported(message))) if message.contains("NaN") => {
                // Native lets NaN through validation; the port refuses it.
                let last = source.reads.last().unwrap();
                assert!(is_nan(input[last].as_u64().unwrap() as u32), "{case}");
                assert!(native_reads.starts_with(&source.reads), "{case}");
                nan_excluded += 1;
            }
            Err(other) => panic!("{other} {case}"),
        }
    }
    assert!(ok > 1000 && thrown == 63 && nan_excluded > 0);
    println!("turret fixtures: {ok} loaded, {thrown} native data errors, {nan_excluded} NaN exclusions");
}

#[test]
fn turret_index_matches_45dd50() {
    let turrets = vec![existing("turret2"), existing("turret"), existing("turret3")];
    let bone = |id: &str| BoneIdentity { id: id.into() };
    assert_eq!(turret_index(&turrets, Some(&bone("turret"))), 1);
    assert_eq!(turret_index(&turrets, Some(&bone("turret3"))), 2);
    assert_eq!(turret_index(&turrets, Some(&bone("turret9"))), -1);
    assert_eq!(turret_index(&turrets, None), -1);
    assert_eq!(turret_index(&[], Some(&bone("turret"))), -1);
}

/// Executes the native instruction sequence 4C79DD..4C79FA on the host x87:
/// fld f; fmul 0.5; fmul 9FC064; fcos; fstp dword, under the given control word.
#[cfg(target_arch = "x86_64")]
fn host_fcos(value: u32, control: u16) -> u32 {
    #[repr(align(16))]
    struct FxState([u8; 512]);
    let mut saved = FxState([0; 512]);
    let half: u32 = 0x3f00_0000;
    let degrees: u32 = 0x3c8e_fa35;
    let mut out = 0u32;
    // SAFETY: private fixed-size buffers; the FPU state is saved and restored.
    unsafe {
        core::arch::asm!(
            "fxsave64 [{saved}]", "fninit", "fldcw word ptr [{control}]",
            "fld dword ptr [{value}]", "fmul dword ptr [{half}]", "fmul dword ptr [{degrees}]",
            "fcos", "fstp dword ptr [{out}]", "fxrstor64 [{saved}]",
            saved = in(reg) saved.0.as_mut_ptr(), control = in(reg) &control,
            value = in(reg) &value, half = in(reg) &half, degrees = in(reg) &degrees,
            out = in(reg) &mut out, options(nostack));
    }
    out
}

#[cfg(target_arch = "x86_64")]
struct One(u32);
#[cfg(target_arch = "x86_64")]
impl TurretSource for One {
    fn string(&mut self, _: &str) -> Result<String> {
        Ok("turret".into())
    }
    fn fixed(&mut self, name: &str) -> Result<u32> {
        Ok(if name == "yaw_rotation_range_degrees" {
            self.0
        } else if name == "reset_time" {
            0x4120_0000
        } else {
            0
        })
    }
    fn flag(&mut self, _: &str) -> Result<bool> {
        Ok(true)
    }
    fn point2(&mut self, _: &str) -> Result<[u32; 2]> {
        Ok([0, 0])
    }
    fn bone(&mut self, _: &str) -> Result<Option<BoneIdentity>> {
        Ok(Some(BoneIdentity { id: "turret".into() }))
    }
}

/// The FCOS field against the physical x87 of this machine: every binary32
/// in [0, 360] reachable by a decimal with up to three fractional digits, plus
/// a deterministic pseudo-random sample of all binary32 values in [0, 360].
#[test]
#[cfg(target_arch = "x86_64")]
fn turret_fcos_matches_host_x87() {
    let mut values: Vec<u32> = (0..=360_000u32)
        .map(|i| (f64::from(i) / 1000.0) as f32)
        .map(f32::to_bits)
        .collect();
    let mut state = 0x1234_5678_9abc_def0u64;
    let limit = 360f32.to_bits();
    for _ in 0..400_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        values.push((state as u32) % (limit + 1));
    }
    values.extend([0, 0x8000_0000, 1, 0x007f_ffff, 0x0080_0000, limit]);
    let mut ambiguous = 0;
    for (pc, cw) in [
        (Precision::Single24, 0x7fu16),
        (Precision::Double53, 0x27f),
        (Precision::Extended64, 0x37f),
    ] {
        for &v in &values {
            match load_turret(&mut One(v), &[], pc) {
                Ok(t) => assert_eq!(
                    t.yaw_half_range_cos_bits,
                    host_fcos(v, cw),
                    "range bits {v:#x} {pc:?}"
                ),
                Err(TurretLoadError::Source(Error::Unsupported(m))) if m.contains("ambiguity") => {
                    ambiguous += 1
                }
                Err(e) => panic!("{v:#x} {e}"),
            }
        }
    }
    println!("host x87 FCOS cross-check: {} values x3 precisions, {ambiguous} refused", values.len());
    assert!(ambiguous < 16);
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn shipped_turret_and_defender_definitions_load() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
    };
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let pc = Precision::Double53;
    let mut rows = Vec::new();
    for id in [
        "rus_corps",
        "ger_corps",
        "usa_corps",
        "gbr_corps",
        "ger_bunker",
        "ger_bunker_aa",
        "gbr_bunker",
        "usa_tank_medium",
    ] {
        let turrets = actors.native_turrets(id, pc).unwrap();
        assert!(!turrets.is_empty(), "{id}");
        let defender = actors.native_loaded_defender(id, pc, None);
        let summary = defender.map(|d| {
            d.map(|d| {
                d.abilities()
                    .iter()
                    .map(|a| a.turret_index)
                    .collect::<Vec<_>>()
            })
        });
        rows.push(format!("{id}: turrets {} defender {:?}", turrets.len(), summary));
    }
    println!("{}", rows.join("\n"));
}
