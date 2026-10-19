//! The VWorld per-frame update (AA.exe, image base 0x400000).
//!
//! Per UI frame the UI update 5C92F0 calls, while VWorld+AD is set,
//! 43AB30 (the VWorld view time, [`view_time_43ab30`] over the KKC_Sim render
//! time [`sim_time_76b120`]) and later, through the widget tree, the VWorld
//! update 43FF60 (vtable 9FDFF8+C, [`update_43ff60`]):
//!
//! 1. 5EEC70 (the widget base update of the VWorld widget);
//! 2. return while VWorld+AD is clear;
//! 3. a null +140 is replaced by a new 0x88-byte object (6F7810 + 4341A0,
//!    the KKC minimap owner);
//! 4. +174 > 0 (the 43C150 RandomLookAt interval): the function-static timer
//!    B39F68 (constructed once under the B39F70 bit-0 guard: 59AA70, 59AB60,
//!    96CAB0 exit registration) fires once its 59ABA0 milliseconds * 0.001
//!    reach +174: two 85D840 draws on B39714 scale World+30+C/+10 for 6FF130
//!    on the current camera, then 59AB60 restarts the timer;
//! 5. 701590 (the current camera's per-frame dynamics);
//! 6. 43DA00 prunes the VWorld+98 deferred-destroy list ([`prune_43da00`]);
//! 7. while B36518 is set, 43C4B0 writes the cursor world point to
//!    +184..+18C (and its normal to +178);
//! 8. 41C0E0(5), 43EC60 ([`world_view_43ec60`]), 41C110(5). The profiler
//!    calls only store for arguments 1..4, so argument 5 is no operation
//!    (proved by the oracle: no store, no 866060 call);
//! 9. 43A1C0([+13C]), 71FAC0([B36518]), 4361B0([+140]).
//!
//! 43EC60 (WorldView.cpp): the IsGWorldValid() assert (non-fatal 855D30 under
//! the B39F64 latch), then 50C750 on World+130, 43A400 on VWorld+108, 4318E0
//! on VWorld+BC, 4312F0 on VWorld+94, 725340 on B36518, the VWorld+208
//! request segment and the E0/E4/E8 FX command loop (count read first).
//!
//! Children owned elsewhere are [`Services`] methods; a child without a Rust
//! owner must return an explicit error. [`Refusals`] decides what a refused
//! child does: the product stops the update at the first refusal (earlier
//! effects stay committed: native has no unwinding there); the diagnostic
//! look-ahead (startup census) records it and continues with the native
//! successor, which is never native state.
//!
//! Proofs: research/phase8-vworld-update (original/v14 instructions at
//! PC24/53/64; every child seam recorded).
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

pub type Result<T> = std::result::Result<T, String>;

/// `[9FDA90]` binary32 1e-4: 43AB30 divides only when World+E4-E0 exceeds it.
pub const VIEW_TIME_SPAN_THRESHOLD_BITS: u32 = 0x38d1_b717;
/// `[9F240C]` binary32 2^32: the unsigned correction after FILD.
const TWO_POW_32_F32: u32 = 0x4f80_0000;
/// `[9F4580]` binary32 0.001 (milliseconds to seconds).
const MILLI_F32: u32 = 0x3a83_126f;
/// `[9FDAA0]` binary64 2^32 and `[9FDA98]` binary64 2^-32 (85D840 fractions).
const TWO_POW_32_F64: u64 = 0x41f0_0000_0000_0000;
const TWO_POW_M32_F64: u64 = 0x3df0_0000_0000_0000;
/// 6F7810 size of the +140 object (4341A0).
pub const MINIMAP_140_BYTES: u32 = 0x88;
/// 855D30 arguments at 43ECC1..43ECCB.
pub const WORLD_VALID_EXPRESSION: &str = "IsGWorldValid()";
pub const WORLD_VALID_FILE: &str = ".\\Source\\View\\WorldView.cpp";
pub const WORLD_VALID_LINE: u32 = 0x43b;

fn b(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(|e| format!("binary32 {bits:#010x}: {e}"))
}
fn b64(bits: u64) -> Result<Binary> {
    Binary::from_f64_bits(bits).map_err(|e| format!("binary64 {bits:#018x}: {e}"))
}
fn n<T>(r: aa_numeric::Result<T>) -> Result<T> {
    r.map_err(|e| e.to_string())
}
/// `fild dword` of an unsigned word, then `jge` past the 2^32 correction.
fn unsigned(word: u32, correction: &Binary, pc: Precision) -> Result<Binary> {
    let value = Binary::from_i32(word as i32);
    if (word as i32) < 0 { n(value.add(correction, pc)) } else { Ok(value) }
}

// ---------------------------------------------------------------- 43AB30
/// 43AB30 on the VWorld words `[+84, +88, +8C, +90]` with the 76B120 result
/// `t` (an exact binary32 ST0). Store order is native: +90 = t - old +8C,
/// +8C = t, +88 = t - World+E0 (FST keeps the unrounded difference), then
/// +84 = (t - E0) / (E4 - E0) when that span exceeds 1e-4 (ordered), else 0.
/// Every subtraction and the division run at `pc`. An error leaves the
/// stores already made.
pub fn view_time_43ab30(words: &mut [u32; 4], t: u32, world_e0: u32, world_e4: u32, pc: Precision) -> Result<()> {
    let time = b(t)?;
    words[3] = n(n(time.sub(&b(words[2])?, pc))?.store_f32())?;
    words[2] = t;
    let since = n(time.sub(&b(world_e0)?, pc))?;
    words[1] = n(since.store_f32())?;
    let span = n(b(world_e4)?.sub(&b(world_e0)?, pc))?;
    if span.compare(&b(VIEW_TIME_SPAN_THRESHOLD_BITS)?) == Ordering::Greater {
        words[0] = n(n(since.div(&span, pc))?.store_f32())?;
    } else {
        words[0] = 0;
    }
    Ok(())
}

// ---------------------------------------------------------------- 76B120
/// Reads and the one store of 76B120 on KKC_Sim `[B36554]`, at their native
/// positions. Each read is lazy: a source that is not reached is not read.
pub trait SimTime {
    /// Session `[B36550]`+F4.
    fn session_f4(&mut self) -> Result<bool>;
    /// Application word `[B396C4]` (bit 1 forces the timeline on).
    fn flags_b396c4(&mut self) -> Result<u32>;
    /// UVar SinglePlayerAutoPause (`[[B39940]+350]` byte).
    fn single_player_auto_pause(&mut self) -> Result<bool>;
    /// Sim+28 paused and Sim+29 frozen bytes.
    fn paused_28(&mut self) -> Result<u8>;
    fn frozen_29(&mut self) -> Result<u8>;
    /// UVar RenderDumpAVI (`[[B39940]+1C8]` byte).
    fn render_dump_avi(&mut self) -> Result<bool>;
    /// 76B080 (the RenderDumpAVI frame-locked time), its ST0 result.
    fn avi_time_76b080(&mut self) -> Result<Binary>;
    /// Sim+10 begin game, +14 begin admin, +18 end game, +1C end admin.
    fn time_words(&mut self) -> Result<[u32; 4]>;
    /// 74B180(argument) on `[B36558]` (the admin clock estimate), its ST0.
    fn admin_clock_74b180(&mut self, argument: u32) -> Result<Binary>;
    /// Sim+34 and its store.
    fn current_34(&mut self) -> Result<u32>;
    fn store_current_34(&mut self, bits: u32) -> Result<()>;
    /// World+E0 and World+E4 (read after the +34 update).
    fn world_e0(&mut self) -> Result<u32>;
    fn world_e4(&mut self) -> Result<u32>;
}

/// 76B120(argument): the Sim render time. While the timeline is enabled
/// (Session+F4, or B396C4 bit 1, or SinglePlayerAutoPause clear) and Sim is
/// neither paused (+28) nor frozen (+29): RenderDumpAVI stores 76B080's
/// binary32 in +34; otherwise v = binary32((binary32(74B180) - begin_admin)
/// * binary32((end_game - begin_game) / (end_admin - begin_admin)) +
/// begin_game) raises +34 when greater, and +34 is clamped to World+E0 (when
/// below) else to World+E4 (when above). The result is always +34.
pub fn sim_time_76b120(sim: &mut impl SimTime, argument: u32, pc: Precision) -> Result<u32> {
    let enabled = sim.session_f4()? || (sim.flags_b396c4()? >> 1) & 1 != 0 || !sim.single_player_auto_pause()?;
    if !enabled || sim.paused_28()? != 0 || sim.frozen_29()? != 0 {
        return sim.current_34();
    }
    if sim.render_dump_avi()? {
        let value = n(sim.avi_time_76b080()?.store_f32())?;
        sim.store_current_34(value)?;
        return Ok(value);
    }
    let [begin_game, begin_admin, end_game, end_admin] = sim.time_words()?;
    let game = n(b(end_game)?.sub(&b(begin_game)?, pc))?;
    let admin = n(b(end_admin)?.sub(&b(begin_admin)?, pc))?;
    let slope = n(n(game.div(&admin, pc))?.store_f32())?;
    let now = n(sim.admin_clock_74b180(argument)?.store_f32())?;
    let value = n(n(n(n(b(now)?.sub(&b(begin_admin)?, pc))?.mul(&b(slope)?, pc))?.add(&b(begin_game)?, pc))?.store_f32())?;
    if f32::from_bits(value) > f32::from_bits(sim.current_34()?) {
        sim.store_current_34(value)?;
    }
    let current = sim.current_34()?;
    let e0 = sim.world_e0()?;
    let e4 = sim.world_e4()?;
    if f32::from_bits(current) < f32::from_bits(e0) {
        sim.store_current_34(e0)?;
        return Ok(e0);
    }
    if f32::from_bits(current) > f32::from_bits(e4) {
        sim.store_current_34(e4)?;
        return Ok(e4);
    }
    Ok(current)
}

// ---------------------------------------------------------------- fields
/// VWorld words of the update owned by this module, with the two function
/// statics of its children (one VWorld per process: 442C00 publishes B36538
/// once). 442C00 stores 0 into +98, +140, +174..+18C; the statics start 0.
pub struct Fields<V> {
    /// +98: 43D190 defers the destruction of a view whose +54 has 0xC0000
    /// set (head first; 43D190 pushes at the head). 43D190 is not ported,
    /// so no Rust producer exists yet.
    deferred_98: Vec<V>,
    /// +140 is non-null (the 4341A0 object exists).
    minimap_140: bool,
    /// +174 RandomLookAt interval (binary32); only the unported 43C150
    /// command handler writes it.
    look_at_174: u32,
    /// +178 normal and +184 point written through 43C4B0.
    normal_178: [u32; 3],
    cursor_184: [u32; 3],
    /// B39F64: 43EC60's assert latch (855D30's AL).
    assert_latch_b39f64: u8,
    /// B39F70: 43FF60's B39F68 timer construction guard.
    timer_guard_b39f70: u32,
}
impl<V> Fields<V> {
    pub fn from_native_constructor() -> Self {
        Self { deferred_98: Vec::new(), minimap_140: false, look_at_174: 0, normal_178: [0; 3], cursor_184: [0; 3],
            assert_latch_b39f64: 0, timer_guard_b39f70: 0 }
    }
    pub fn deferred_98(&self) -> &[V] {
        &self.deferred_98
    }
    /// 43D190's deferral (push at the head) for its future port.
    pub fn defer_destroy_43d190(&mut self, view: V) -> Result<()> {
        self.deferred_98.try_reserve(1).map_err(|e| e.to_string())?;
        self.deferred_98.insert(0, view);
        Ok(())
    }
    pub fn minimap_140(&self) -> bool {
        self.minimap_140
    }
    pub fn look_at_174(&self) -> u32 {
        self.look_at_174
    }
    pub fn normal_178(&self) -> [u32; 3] {
        self.normal_178
    }
    pub fn cursor_184(&self) -> [u32; 3] {
        self.cursor_184
    }
    pub fn assert_latch_b39f64(&self) -> u8 {
        self.assert_latch_b39f64
    }
    pub fn timer_guard_b39f70(&self) -> u32 {
        self.timer_guard_b39f70
    }
}

/// The IsGWorldValid() words of 43EC60, read in native order only when the
/// B39F64 latch is clear.
pub struct WorldValid {
    /// VWorld+B8 == B3CC94 (the current World revision).
    pub revision_matches_b8: bool,
    /// VWorld+AD.
    pub initialized_ad: bool,
    /// World+D8.
    pub world_d8: u32,
}

/// The children of 43FF60/43EC60 that this module does not own, in native
/// order. `fields` lends this module's VWorld words.
pub trait Services {
    type View: Clone;
    fn fields(&mut self) -> Result<&mut Fields<Self::View>>;
    // 43FF60
    /// 5EEC70 on the VWorld widget (UI+D0): fade +64/+68, DummyVE vt+30
    /// (5E25C0 state bits), child widgets +28 vt+C.
    fn widget_base_update_5eec70(&mut self) -> Result<()>;
    /// VWorld+AD (43FF80).
    fn vworld_ad(&mut self) -> Result<bool>;
    /// 6F7810(0x88) + 4341A0 stored at +140. An allocation failure is an
    /// error (native would store null and fault in 4361B0).
    fn construct_minimap_4341a0(&mut self) -> Result<()>;
    /// B39F68 construction (59AA70, 59AB60) and its 96CAB0 exit registration.
    fn look_at_timer_construct_59aa70(&mut self) -> Result<()>;
    fn look_at_timer_restart_59ab60(&mut self) -> Result<()>;
    fn look_at_timer_exit_96cab0(&mut self) -> Result<()>;
    fn look_at_timer_milliseconds_59aba0(&mut self) -> Result<u32>;
    /// 85D840 on the B39714 state.
    fn random_b39714_85d840(&mut self) -> Result<u32>;
    /// `[[GWorld+30]+C]` and `[[GWorld+30]+10]` (binary32).
    fn map_extent_30(&mut self) -> Result<[u32; 2]>;
    /// 6FF130(x, y) on the current camera `*[B40048]`.
    fn camera_look_at_6ff130(&mut self, x: u32, y: u32) -> Result<()>;
    /// 701590 on the current camera.
    fn camera_update_701590(&mut self) -> Result<()>;
    /// `[B36518]` (the TerrainMap) is non-null.
    fn visual_map_b36518(&mut self) -> Result<bool>;
    /// VWorld+AE (43DA00's first test).
    fn uninitializing_ae(&mut self) -> Result<bool>;
    /// 43DA00's reads of one deferred view: +54 flags, vt+BC, +19C.
    fn deferred_flags_54(&mut self, view: &Self::View) -> Result<u32>;
    fn deferred_presenting_bc(&mut self, view: &Self::View) -> Result<bool>;
    fn deferred_count_19c(&mut self, view: &Self::View) -> Result<i32>;
    /// 43CF00(view) on this VWorld.
    fn destroy_view_43cf00(&mut self, view: Self::View) -> Result<()>;
    /// 43C4B0(&point, [B3653C]+58+4 cursor, &VWorld+178): the world point
    /// under the cursor; `normal` is the +178 out parameter.
    fn cursor_world_point_43c4b0(&mut self, normal: &mut [u32; 3]) -> Result<[u32; 3]>;
    /// 43A1C0 on VWorld+13C (UIWorldState).
    fn ui_world_state_43a1c0(&mut self) -> Result<()>;
    /// 71FAC0 on `[B36518]`.
    fn terrain_map_71fac0(&mut self) -> Result<()>;
    /// 4361B0 on +140.
    fn minimap_4361b0(&mut self) -> Result<()>;
    // 43EC60
    fn world_valid(&mut self) -> Result<WorldValid>;
    /// 855D30(expression, file, line); returns the new B39F64 byte.
    fn assert_855d30(&mut self, expression: &str, file: &str, line: u32) -> Result<u8>;
    /// 50C750 on World+130.
    fn world_messages_50c750(&mut self) -> Result<()>;
    /// 43A400 on VWorld+108.
    fn scene_light_43a400(&mut self) -> Result<()>;
    /// 4318E0 on VWorld+BC.
    fn heightfield_rects_4318e0(&mut self) -> Result<()>;
    /// 4312F0 on VWorld+94.
    fn terrain_visibility_4312f0(&mut self) -> Result<()>;
    /// 725340 on `[B36518]`.
    fn terrain_map_725340(&mut self) -> Result<()>;
    /// 43ED1A..43ED53: the VWorld+208 request segment.
    fn view_requests_208(&mut self) -> Result<()>;
    /// VWorld+E8 (read once at 43ED53) and the 43ED5D loop over that count.
    fn fx_command_count_e8(&mut self) -> Result<i32>;
    fn fx_commands_e0(&mut self, count: i32) -> Result<()>;
}

/// What a refused child does; see the module documentation.
pub struct Refusals {
    look_ahead: bool,
    list: Vec<(&'static str, String)>,
}
impl Refusals {
    /// The product: the first refusal stops the update.
    pub fn strict() -> Self {
        Self { look_ahead: false, list: Vec::new() }
    }
    /// Diagnostic only (AA_STARTUP_CENSUS + AA_STARTUP_CENSUS_CONTINUE).
    pub fn look_ahead() -> Self {
        Self { look_ahead: true, list: Vec::new() }
    }
    pub fn is_look_ahead(&self) -> bool {
        self.look_ahead
    }
    /// The refusals recorded in look-ahead mode, in update order.
    pub fn list(&self) -> &[(&'static str, String)] {
        &self.list
    }
    /// One child's result: Ok(None) after recording a look-ahead refusal,
    /// the step-named error in the product.
    pub fn step<T>(&mut self, step: &'static str, result: Result<T>) -> Result<Option<T>> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(error) if self.look_ahead => {
                self.list.push((step, error));
                Ok(None)
            }
            Err(error) => Err(format!("{step}: {error}")),
        }
    }
}

// ---------------------------------------------------------------- 43DA00
/// 43DA00 over the +98 list, head first: a view is kept while VWorld+AE is
/// clear, +54 & 0xC0000, vt+BC and +19C > 0 (signed), in that order and
/// short-circuited; otherwise 43CF00(view) runs before its node is unlinked
/// and freed. A refused 43CF00 keeps the node.
pub fn prune_43da00<S: Services>(s: &mut S) -> Result<()> {
    let mut index = 0;
    while index < s.fields()?.deferred_98.len() {
        let view = s.fields()?.deferred_98[index].clone();
        let keep = !s.uninitializing_ae()?
            && s.deferred_flags_54(&view)? & 0xc0000 != 0
            && s.deferred_presenting_bc(&view)?
            && s.deferred_count_19c(&view)? > 0;
        if keep {
            index += 1;
        } else {
            s.destroy_view_43cf00(view)?;
            s.fields()?.deferred_98.remove(index);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- 43FF60
/// 43FF60 on this VWorld; see the module documentation for the order.
pub fn update_43ff60<S: Services>(s: &mut S, refusals: &mut Refusals, pc: Precision) -> Result<()> {
    refusals.step("5EEC70 VWorld widget base update", s.widget_base_update_5eec70())?;
    match refusals.step("43FF80 VWorld+AD", s.vworld_ad())? {
        Some(true) => {}
        Some(false) | None => return Ok(()),
    }
    if !s.fields()?.minimap_140 && refusals.step("43FF98 6F7810+4341A0 (+140)", s.construct_minimap_4341a0())?.is_some() {
        s.fields()?.minimap_140 = true;
    }
    refusals.step("43FFCE +174 RandomLookAt", random_look_at(s, pc))?;
    refusals.step("4400E7 701590 camera update", s.camera_update_701590())?;
    refusals.step("4400EE 43DA00 deferred views", prune_43da00(s))?;
    if refusals.step("4400F3 B36518", s.visual_map_b36518())? == Some(true) {
        let mut normal = s.fields()?.normal_178;
        if let Some(point) = refusals.step("440117 43C4B0 cursor point", s.cursor_world_point_43c4b0(&mut normal))? {
            let fields = s.fields()?;
            fields.normal_178 = normal;
            fields.cursor_184 = point;
        }
    }
    //41C0E0(5) at 44013A: no store (arguments 1..4 only).
    world_view_43ec60(s, refusals)?;
    //41C110(5) at 44014E: no store.
    refusals.step("440159 43A1C0 UIWorldState", s.ui_world_state_43a1c0())?;
    refusals.step("440164 71FAC0 TerrainMap", s.terrain_map_71fac0())?;
    refusals.step("44016F 4361B0 +140", s.minimap_4361b0())?;
    Ok(())
}

/// 43FFCE..4400DA.
fn random_look_at<S: Services>(s: &mut S, pc: Precision) -> Result<()> {
    let interval = s.fields()?.look_at_174;
    //fcomp 0.0; test ah,41h: only an ordered positive interval proceeds.
    if !(f32::from_bits(interval) > 0.0) {
        return Ok(());
    }
    if s.fields()?.timer_guard_b39f70 & 1 == 0 {
        s.fields()?.timer_guard_b39f70 |= 1;
        s.look_at_timer_construct_59aa70()?;
        s.look_at_timer_restart_59ab60()?;
        s.look_at_timer_exit_96cab0()?;
    }
    let milliseconds = s.look_at_timer_milliseconds_59aba0()?;
    let elapsed = n(unsigned(milliseconds, &b(TWO_POW_32_F32)?, pc)?.mul(&b(MILLI_F32)?, pc))?;
    //fcomp +174; test ah,1: below (or an infinite interval) waits.
    if f32::from_bits(interval).is_infinite() || elapsed.compare(&b(interval)?) == Ordering::Less {
        return Ok(());
    }
    let correction = b64(TWO_POW_32_F64)?;
    let scale = b64(TWO_POW_M32_F64)?;
    let first = s.random_b39714_85d840()?;
    let u = n(n(unsigned(first, &correction, pc)?.mul(&scale, pc))?.store_f32())?;
    let second = s.random_b39714_85d840()?;
    let v = n(unsigned(second, &correction, pc)?.mul(&scale, pc))?;
    let [width, height] = s.map_extent_30()?;
    let x = n(n(b(u)?.mul(&b(width)?, pc))?.store_f32())?;
    let y = n(n(v.mul(&b(height)?, pc))?.store_f32())?;
    s.camera_look_at_6ff130(x, y)?;
    s.look_at_timer_restart_59ab60()
}

// ---------------------------------------------------------------- 43EC60
/// 43EC60 on this VWorld; see the module documentation for the order.
pub fn world_view_43ec60<S: Services>(s: &mut S, refusals: &mut Refusals) -> Result<()> {
    if s.fields()?.assert_latch_b39f64 == 0 {
        if let Some(valid) = refusals.step("43EC76 IsGWorldValid words", s.world_valid())? {
            if !valid.revision_matches_b8 || !valid.initialized_ad || valid.world_d8 == 6 {
                if let Some(latch) = refusals.step("43ECD0 855D30 IsGWorldValid() assert",
                    s.assert_855d30(WORLD_VALID_EXPRESSION, WORLD_VALID_FILE, WORLD_VALID_LINE))? {
                    s.fields()?.assert_latch_b39f64 = latch;
                }
            }
        }
    }
    refusals.step("43ECE9 50C750 World+130 messages", s.world_messages_50c750())?;
    refusals.step("43ECF4 43A400 VWorld+108 light", s.scene_light_43a400())?;
    refusals.step("43ECFF 4318E0 VWorld+BC rects", s.heightfield_rects_4318e0())?;
    refusals.step("43ED0A 4312F0 VWorld+94 visibility", s.terrain_visibility_4312f0())?;
    refusals.step("43ED15 725340 TerrainMap", s.terrain_map_725340())?;
    refusals.step("43ED1A VWorld+208 requests", s.view_requests_208())?;
    if let Some(count) = refusals.step("43ED53 VWorld+E8", s.fx_command_count_e8())? {
        if count > 0 {
            refusals.step("43ED5D VWorld+E0 FX commands", s.fx_commands_e0(count))?;
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests_fixture {
    /// research/phase8-vworld-update/native-vworld-update.json, read at test
    /// run time (2.5 MB; not compiled into every test binary).
    pub(crate) fn load() -> serde_json::Value {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-vworld-update/native-vworld-update.json");
        serde_json::from_str(&std::fs::read_to_string(path).expect("vworld fixture")).expect("vworld fixture json")
    }
}
#[cfg(test)]
#[path = "vworld_update_tests.rs"]
mod tests;
