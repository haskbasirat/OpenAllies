//! Fresh, explicitly selected local standalone network service785560.
//! This owns its real three child projections and shared QR2 key registrations.
//! It creates no Internet account manager and supplies no networking readiness.
use crate::{catalog::Catalog, load_view_variables::NativeOverrides};
use aa_rules::{
    native::index::native_enumerate,
    resources::{Provenance, SourceIdentity},
    sources::V14Sources,
    tgi::{Document, Item, Limits},
};
use std::collections::BTreeMap;

type Result<T> = std::result::Result<T, String>;

#[cfg(test)]
#[path = "standalone_network_tests.rs"]
mod tests;

/// Actual OS boundary of59AA70/59AB60. It is not simulation time or an RNG.
/// The Windows producer shares the existing process QPF initialization.
pub trait Clock {
    fn start_timer(&mut self) -> Result<TimerAnchor>;
}
#[derive(Debug)]
pub struct TimerAnchor {
    counter: i64,
    frequency: i64,
}
impl TimerAnchor {
    /// The platform callback supplies the actual two OS outputs. Native stores
    /// QPC before its discarded signed integer calculation. Invalid native
    /// divisors/overflow are refused rather than given a timer value.
    pub fn from_os_counter(counter: i64, frequency: i64) -> Result<Self> {
        let low = frequency as u32;
        if low / 1000 == 0 {
            return Err("59AA70 native frequency quotient would divide by zero".into());
        }
        counter
            .wrapping_mul(1000)
            .checked_div(frequency)
            .ok_or("59AB60 discarded native timer division is undefined")?;
        Ok(Self { counter, frequency })
    }
    pub fn counter(&self) -> i64 {
        self.counter
    }
    pub fn frequency(&self) -> i64 {
        self.frequency
    }
}

#[derive(Clone, Copy)]
enum Period {
    Browser,
    Query,
}
impl Period {
    fn name(self) -> &'static str {
        match self {
            Self::Browser => "GameSpySBThinkPeriod",
            Self::Query => "GameSpyQR2ThinkPeriod",
        }
    }
}
#[derive(Debug)]
struct Variable {
    value: i32,
    assignments: Vec<Provenance>,
}
#[derive(Debug)]
struct Inputs {
    browser: Option<Variable>,
    query: Option<Variable>,
    source_ids: Vec<SourceIdentity>,
    files: Vec<(String, String)>,
}
impl Inputs {
    fn bind(
        catalog: &Catalog<'_>,
        files: &mut V14Sources,
        overrides: NativeOverrides,
    ) -> Result<Self> {
        catalog.require_source_owner()?;
        if !matches!(overrides, NativeOverrides::ControlledNone) {
            return Err(
                "standalone current UVars require their real profile/console override owner".into(),
            );
        }
        let paths: Vec<_> = files
            .catalog()
            .into_iter()
            .map(|e| e.logical_path)
            .collect();
        let mut result = Self {
            browser: None,
            query: None,
            source_ids: catalog.actors.sources.clone(),
            files: Vec::new(),
        };
        for path in native_enumerate("UVars*.tgi", &paths) {
            let file = files.read(&path).map_err(|e| e.to_string())?;
            if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
                return Err("standalone UVar source hash differs".into());
            }
            result
                .files
                .push((file.logical_path.clone(), file.sha256.clone()));
            let doc =
                Document::parse_native(file.bytes, Limits::default()).map_err(|e| e.to_string())?;
            for item in &doc.items {
                let Item::Node(node) = item else { continue };
                if !doc
                    .header_name(&node.header)
                    .is_some_and(|v| v.eq_ignore_ascii_case("Vars"))
                {
                    continue;
                }
                if node.header.tokens.len() != 1 {
                    return Err(
                        "standalone UVars inherited node requires actual VarHash owner".into(),
                    );
                }
                for item in &node.items {
                    let Item::Property(p) = item else { continue };
                    let Some(name) = doc.property_name(p) else {
                        continue;
                    };
                    let target = if name.eq_ignore_ascii_case(Period::Browser.name()) {
                        &mut result.browser
                    } else if name.eq_ignore_ascii_case(Period::Query.name()) {
                        &mut result.query
                    } else {
                        continue;
                    };
                    if p.left.len() != 2
                        || !doc
                            .text(p.left[0].span)
                            .is_some_and(|v| v.eq_ignore_ascii_case("int"))
                    {
                        return Err("standalone period requires actual assigned Int UVar".into());
                    }
                    let raw = doc
                        .property_value(p)
                        .ok_or("standalone period assignment absent")?
                        .trim();
                    let value = raw.parse::<i32>().map_err(
                        |_| "standalone period is outside the proved decimal Int source scope",
                    )?;
                    let provenance = Provenance {
                        path: file.logical_path.clone(),
                        sha256: file.sha256.clone(),
                        span: p.span,
                    };
                    if let Some(current) = target {
                        current.value = value;
                        current.assignments.push(provenance)
                    } else {
                        *target = Some(Variable {
                            value,
                            assignments: vec![provenance],
                        })
                    }
                }
            }
        }
        Ok(result)
    }
    fn period(&self, which: Period) -> Result<i32> {
        let variable = match which {
            Period::Browser => &self.browser,
            Period::Query => &self.query,
        };
        let value = variable
            .as_ref()
            .map(|v| v.value)
            .ok_or_else(|| format!("{} missing at actual405370 constructor read", which.name()))?;
        //The shipped50 is exactly representable. Do not silently assume the
        //live rounding-control word for a changed, inexact custom integer.
        if matches!(which, Period::Query) && !exact_binary32_integer(value) {
            return Err(
                "inexact QR2 period requires the actual live rounding-control owner".into(),
            );
        }
        Ok(value)
    }
}
fn exact_binary32_integer(value: i32) -> bool {
    let magnitude = value.unsigned_abs();
    let shift = (31u32.saturating_sub(magnitude.leading_zeros())).saturating_sub(23);
    magnitude & ((1u32 << shift) - 1) == 0
}

#[derive(Debug)]
struct Browser {
    timer: Option<TimerAnchor>,
    period_10: Option<u32>,
    field_14: Option<u32>,
}
#[derive(Debug)]
struct Query {
    field_04: u32,
    timer: Option<TimerAnchor>,
    flag_10: Option<u8>,
    period_14: Option<u32>,
    field_18: Option<u32>,
    flag_1c: Option<u8>,
    field_20: Option<u32>,
    word_24: Option<u16>,
}
#[derive(Debug)]
struct Hash {
    count_08: u32,
    size_0c: u32,
    flag_10: u8,
    buckets_04: Option<Vec<u32>>,
}
#[derive(Debug)]
pub struct Standalone {
    standalone_flag: u32,
    browser: Option<Browser>,
    query: Option<Query>,
    hash: Option<Hash>,
    keys: BTreeMap<u32, &'static str>,
    complete: bool,
    inputs: Option<Inputs>,
    stopped: Option<String>,
}
const KEYS: [&str; 13] = [
    "fullkeys",
    "worldrules",
    "mapsessiontype",
    "mapids",
    "locale",
    "teammgmnt",
    "language",
    "observersallowed",
    "mapsize",
    "internet",
    "wins_",
    "losses_",
    "disconnects_",
];
impl Standalone {
    fn pending() -> Self {
        Self {
            standalone_flag: 0,
            browser: None,
            query: None,
            hash: None,
            keys: BTreeMap::new(),
            complete: false,
            inputs: None,
            stopped: None,
        }
    }
    /// Genuine fresh standalone785560 never constructs B40C9C. This method is
    /// available only after the complete selected constructor on this owner.
    pub fn online_manager_present(&self) -> Result<bool> {
        if !self.complete {
            return Err(self
                .stopped
                .clone()
                .unwrap_or_else(|| "standalone constructor incomplete".into()));
        }
        Ok(false)
    }
    pub fn browser_period(&self) -> Result<Option<i32>> {
        self.online_manager_present()?;
        Ok(self
            .browser
            .as_ref()
            .and_then(|v| v.period_10)
            .map(|v| v as i32))
    }
    pub fn query_period_bits(&self) -> Result<Option<u32>> {
        self.online_manager_present()?;
        Ok(self.query.as_ref().and_then(|v| v.period_14))
    }
    pub fn registered_key(&self, id: u32) -> Result<Option<&str>> {
        self.online_manager_present()?;
        if !(50..=62).contains(&id) {
            return Err("QR2 key outside the thirteen current785560 registrations requires its actual prior table owner".into());
        }
        Ok(self.keys.get(&id).copied())
    }
    pub fn source_files(&self) -> &[(String, String)] {
        self.inputs
            .as_ref()
            .map(|v| v.files.as_slice())
            .unwrap_or(&[])
    }
    pub fn require_source_owner(&self, catalog: &Catalog<'_>) -> Result<()> {
        self.online_manager_present()?;
        catalog.require_source_owner()?;
        if self
            .inputs
            .as_ref()
            .is_none_or(|v| v.source_ids != catalog.actors.sources)
        {
            return Err("standalone source owner differs".into());
        }
        Ok(())
    }
}
trait Constructor {
    fn allocate(&mut self, size: u32) -> Result<bool>;
    fn timer(&mut self) -> Result<TimerAnchor>;
    fn period(&mut self, which: Period) -> Result<i32>;
    // External observation cutoff only; production performs the real owned
    //8ADA60 table store below, rather than replacing it with an ignored call.
    fn before_key(&mut self, _id: u32) -> Result<()> {
        Ok(())
    }
}
fn construct(state: &mut Standalone, world: &mut impl Constructor) -> Result<()> {
    state.standalone_flag = 1;
    if world.allocate(0x18)? {
        state.browser = Some(Browser {
            timer: None,
            period_10: None,
            field_14: None,
        });
        let value = state.browser.as_mut().unwrap();
        value.timer = Some(world.timer()?);
        value.field_14 = Some(0);
        value.period_10 = Some(world.period(Period::Browser)? as u32);
    }
    if world.allocate(0x28)? {
        state.query = Some(Query {
            field_04: 0,
            timer: None,
            flag_10: None,
            period_14: None,
            field_18: None,
            flag_1c: None,
            field_20: None,
            word_24: None,
        });
        let value = state.query.as_mut().unwrap();
        value.timer = Some(world.timer()?);
        value.flag_10 = Some(0);
        value.field_18 = Some(0);
        value.flag_1c = Some(0);
        value.field_20 = Some(0);
        value.word_24 = Some(0);
        //79898C FILD integer,798995 FSTP binary32. No arithmetic/PC rounding
        //occurs between these two instructions.
        value.period_14 = Some(
            aa_numeric::Binary::from_i32(world.period(Period::Query)?)
                .store_f32()
                .map_err(|e| e.to_string())?,
        );
        for (index, key) in KEYS.iter().enumerate() {
            let id = 50 + index as u32;
            world.before_key(id)?;
            state.keys.insert(id, *key);
        }
    }
    if world.allocate(0x14)? {
        state.hash = Some(Hash {
            count_08: 997,
            size_0c: 0,
            flag_10: 1,
            buckets_04: None,
        });
        if !world.allocate(0xf94)? {
            return Err("79AB30 null bucket allocation reaches a native invalid write".into());
        }
        state.hash.as_mut().unwrap().buckets_04 = Some(vec![0; 997]);
    }
    state.complete = true;
    Ok(())
}

/// This choice can be created only by the existing fresh application owner;
/// it cannot adopt a previous Internet manager or be reset for each mission.
#[derive(Debug)]
pub(crate) enum ApplicationNetwork {
    Fresh,
    Local(Standalone),
}
impl ApplicationNetwork {
    pub(crate) fn fresh_application() -> Self {
        Self::Fresh
    }
    pub(crate) fn select_local(
        &mut self,
        catalog: &Catalog<'_>,
        files: &mut V14Sources,
        overrides: NativeOverrides,
        clock: &mut impl Clock,
    ) -> Result<()> {
        if !matches!(self, Self::Fresh) {
            return Err(
                "application network choice already selected (including a failed prefix)".into(),
            );
        }
        *self = Self::Local(Standalone::pending());
        let Self::Local(state) = self else {
            unreachable!()
        };
        let result = (|| {
            state.inputs = Some(Inputs::bind(catalog, files, overrides)?);
            struct Native<'a, C> {
                inputs: &'a Inputs,
                clock: &'a mut C,
            }
            impl<C: Clock> Constructor for Native<'_, C> {
                fn allocate(&mut self, _size: u32) -> Result<bool> {
                    Ok(true)
                }
                fn timer(&mut self) -> Result<TimerAnchor> {
                    self.clock.start_timer()
                }
                fn period(&mut self, which: Period) -> Result<i32> {
                    self.inputs.period(which)
                }
            }
            // Inputs were created by the earlier actual VarHash source-load
            //boundary. Temporarily loan their same storage, do not copy values.
            let inputs = state.inputs.take().unwrap();
            let outcome = construct(
                state,
                &mut Native {
                    inputs: &inputs,
                    clock,
                },
            );
            state.inputs = Some(inputs);
            outcome
        })();
        if let Err(error) = &result {
            state.stopped = Some(error.clone());
        }
        result
    }
    pub(crate) fn standalone(&self) -> Result<&Standalone> {
        match self {
            Self::Fresh => Err("application network choice has not run".into()),
            Self::Local(value) => {
                value.online_manager_present()?;
                Ok(value)
            }
        }
    }
}
