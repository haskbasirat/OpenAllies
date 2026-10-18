//!Canonical typed current SVar cells.5065D0 constructs135 fixed-index Vars;
//!4B3C10 appends missing source-created Vars, then474C50 supplies WorldRules.
//!There is one value allocation per registration, shared by gameplay and dump.
use crate::load_view_variables::{CurrentValue, Registration};
#[path = "svar_constructors.rs"]
mod constructors;
struct Constructor {
    name: &'static str,
    kind: u32,
    flags: u32,
    word: u32,
    point: [u32; 3],
    bounds: [u32; 2],
}
#[derive(Debug)]
pub(crate) enum Value {
    Bool(u8),
    Fixed(u32),
    Int(u32),
    CString(Vec<u16>),
    Point3([u32; 3]),
}
impl Value {
    pub(crate) fn kind(&self) -> u32 {
        match self {
            Self::Bool(_) => 1,
            Self::Fixed(_) => 2,
            Self::Int(_) => 3,
            Self::CString(_) => 4,
            Self::Point3(_) => 5,
        }
    }
}
pub(crate) struct Entry {
    name: String,
    value: Value,
    flags: u32,
    bounds: [u32; 2],
}
pub(crate) struct Store {
    entries: Vec<Entry>,
}
impl Store {
    pub(crate) fn constructors() -> Self {
        Self {
            entries: constructors::CONSTRUCTORS
                .iter()
                .map(|c| Entry {
                    name: c.name.into(),
                    flags: c.flags,
                    bounds: c.bounds,
                    value: match c.kind {
                        1 => Value::Bool(c.word as u8),
                        2 => Value::Fixed(c.word),
                        3 => Value::Int(c.word),
                        4 => Value::CString(Vec::new()),
                        5 => Value::Point3(c.point),
                        _ => unreachable!("private constructor kind"),
                    },
                })
                .collect(),
        }
    }
    pub(crate) fn registrations(&self) -> impl Iterator<Item = Registration<'_>> {
        self.entries.iter().enumerate().map(|(i, e)| Registration {
            ordinal: i as u32,
            name: &e.name,
            kind: e.value.kind(),
        })
    }
    pub(crate) fn current(&self, index: usize) -> Result<CurrentValue<'_>, String> {
        let e = self.entries.get(index).ok_or("current SVar index absent")?;
        Ok(match &e.value {
            Value::Bool(v) => CurrentValue::Bool(v),
            Value::Fixed(v) => CurrentValue::Fixed(v),
            Value::Int(v) => CurrentValue::Int(v),
            Value::CString(v) => CurrentValue::CString(v),
            Value::Point3(v) => CurrentValue::Point3(v),
        })
    }
    pub(crate) fn index(&self, name: &str) -> Option<usize> {
        self.entries
            .iter()
            .position(|e| e.name.eq_ignore_ascii_case(name))
    }
    pub(crate) fn get(&self, name: &str) -> &Value {
        &self.entries[self
            .index(name)
            .expect("canonical SVar binding was validated")]
        .value
    }
    pub(crate) fn get_mut(&mut self, name: &str) -> &mut Value {
        let i = self
            .index(name)
            .expect("canonical SVar binding was validated");
        &mut self.entries[i].value
    }
    pub(crate) fn flags(&self, name: &str) -> Option<u32> {
        self.index(name).map(|i| self.entries[i].flags)
    }
    ///4B3470 adds exactly once, retaining native registration spelling/order.
    pub(crate) fn add(
        &mut self,
        name: String,
        value: Value,
        flags: u32,
        bounds: [u32; 2],
    ) -> Result<(), String> {
        if self.index(&name).is_some() {
            return Err(format!("duplicate canonical SVar {name}"));
        }
        self.entries.push(Entry {
            name,
            value,
            flags,
            bounds,
        });
        Ok(())
    }
    ///Closed actual flags0/2/20 source/default setter domain. Nonconstant policy
    ///gates must bind their current process owner before they can be admitted.
    pub(crate) fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
        let i = self
            .index(name)
            .ok_or_else(|| format!("current SVar {name} absent"))?;
        let e = &mut self.entries[i];
        if e.value.kind() != value.kind() {
            return Err(format!("current SVar {name} typed setter differs"));
        }
        if e.flags & 2 != 0 {
            return Ok(());
        }
        if e.flags & !0x22 != 0 {
            return Err(format!(
                "current SVar {name} process policy flags require owner"
            ));
        }
        e.value = match value {
            Value::Int(v) if e.flags & 32 != 0 => {
                Value::Int((v as i32).clamp(e.bounds[0] as i32, e.bounds[1] as i32) as u32)
            }
            Value::Fixed(v) if e.flags & 32 != 0 => {
                let f = f32::from_bits(v);
                let lo = f32::from_bits(e.bounds[0]);
                let hi = f32::from_bits(e.bounds[1]);
                if !f.is_finite() || !lo.is_finite() || !hi.is_finite() {
                    return Err(
                        "nonfinite bounded Fixed SVar assignment requires native comparison domain"
                            .into(),
                    );
                }
                Value::Fixed(if f < lo {
                    e.bounds[0]
                } else if f > hi {
                    e.bounds[1]
                } else {
                    v
                })
            }
            v => v,
        };
        Ok(())
    }
    pub(crate) fn set_enum(&mut self, name: &str, count: usize) -> Result<(), String> {
        if count == 0 || count > i32::MAX as usize {
            return Err("SVar enum range representation".into());
        }
        let i = self.index(name).ok_or("SVar enum allocation absent")?;
        let e = &mut self.entries[i];
        if e.value.kind() != 3 {
            return Err("SVar enum allocation has wrong native type".into());
        }
        e.flags |= 32;
        e.bounds = [0, count as u32 - 1];
        Ok(())
    }
    pub(crate) fn word(&self, name: &str) -> u32 {
        match self.get(name) {
            Value::Fixed(v) | Value::Int(v) => *v,
            _ => panic!("validated canonical SVar word kind"),
        }
    }
    pub(crate) fn bool_ref(&self, name: &str) -> &u8 {
        match self.get(name) {
            Value::Bool(v) => v,
            _ => panic!("validated canonical SVar Bool kind"),
        }
    }
    pub(crate) fn assign_word_raw(&mut self, name: &str, v: u32) {
        match self.get_mut(name) {
            Value::Fixed(w) | Value::Int(w) => *w = v,
            _ => panic!("validated canonical SVar word setter kind"),
        }
    }
    pub(crate) fn assign_bool_raw(&mut self, name: &str, v: u8) {
        match self.get_mut(name) {
            Value::Bool(w) => *w = v,
            _ => panic!("validated canonical SVar Bool setter kind"),
        }
    }
}
impl Store {
    pub(crate) fn require_kind(&self, name: &str, kind: u32) -> Result<(), String> {
        let i = self
            .index(name)
            .ok_or_else(|| format!("canonical SVar {name} is absent"))?;
        if self.entries[i].value.kind() != kind {
            return Err(format!("canonical SVar {name} kind differs"));
        }
        Ok(())
    }
    pub(crate) fn publish_rule(
        &mut self,
        row: &crate::load_world_rules::Record,
    ) -> Result<(), String> {
        let value = match row.kind() {
            0 => Value::Bool(row.value() as u8),
            1 | 3 => Value::Int(row.value()),
            2 => Value::Fixed(row.value()),
            _ => return Err("unbound WorldRule native setter kind".into()),
        };
        self.assign(row.ids(), value)
    }
    fn apply_default(&mut self, row: &crate::load_world_rules::Record) -> Result<(), String> {
        let value = match row.kind() {
            0 => Value::Bool(row.default_value() as u8),
            1 | 3 => Value::Int(row.default_value()),
            2 => Value::Fixed(row.default_value()),
            _ => return Err("unbound WorldRule native default kind".into()),
        };
        if self.index(row.ids()).is_none() {
            let bounds = if row.kind() == 3 {
                [
                    0,
                    row.enum_values()
                        .len()
                        .checked_sub(1)
                        .ok_or("empty WorldRule enum")? as u32,
                ]
            } else if row.kind() == 1 {
                [0x80000000, 0x7fffffff]
            } else {
                [0xff7fffff, 0x7f7fffff]
            };
            self.add(
                row.ids().into(),
                value,
                if row.kind() == 3 { 32 } else { 0 },
                bounds,
            )?;
        } else {
            if row.kind() == 3 {
                self.set_enum(row.ids(), row.enum_values().len())?;
            }
            self.assign(row.ids(), value)?;
        }
        Ok(())
    }
}
pub(crate) fn from_fresh_source(
    catalog: &crate::catalog::Catalog<'_>,
    pc: aa_numeric::Precision,
) -> Result<Store, String> {
    let mut store = Store::constructors();
    apply_source_files(catalog, pc, &mut store)?;
    //474C50 runs before the native source manager snapshots current Vars.
    for row in crate::load_world_rules::descriptors(catalog, pc)? {
        store.apply_default(&row)?;
    }
    Ok(store)
}
fn apply_source_files(
    catalog: &crate::catalog::Catalog<'_>,
    pc: aa_numeric::Precision,
    store: &mut Store,
) -> Result<(), String> {
    let settings = catalog
        .settings
        .as_ref()
        .ok_or("canonical SVar Settings source owner absent")?;
    let nodes = catalog
        .actors
        .resolved_definitions("SVars")
        .map_err(|e| e.to_string())?;
    if nodes.len() != 2 {
        return Err("canonical SVar fresh source requires both actual SVars files".into());
    }
    let mut matched_public = 0usize;
    for path in ["Game/SVars.tgi", "Game/SVars_AA.tgi"] {
        let matches: Vec<_> = nodes
            .iter()
            .filter(|node| {
                let sources = node.sources();
                sources.len() == 1 && sources[0].path.eq_ignore_ascii_case(path)
            })
            .collect();
        let [node] = matches.as_slice() else {
            return Err(format!(
                "canonical SVar source order/identity differs at {path}"
            ));
        };
        for field in node.native_svar_fields().map_err(|e| e.to_string())? {
            //Public Settings is only a projection. Private source/span/value
            //authority is retained by this actual lazy field token.
            let rows = settings
                .declarations
                .get(&field.name().to_ascii_lowercase())
                .ok_or("canonical SVar public source row absent")?;
            let matching: Vec<_> = rows
                .iter()
                .filter(|r| r.source == *field.source())
                .collect();
            let [row] = matching.as_slice() else {
                return Err("canonical SVar public/private source span is ambiguous".into());
            };
            if row.field_type.as_deref() != field.declared_type()
                || row.raw != field.raw().map_err(|e| e.to_string())?
            {
                return Err(format!(
                    "canonical SVar {} public projection differs from actual source",
                    field.name()
                ));
            }
            matched_public += 1;
            let source_kind = field.source_kind().map_err(|e| e.to_string())?;
            if source_kind == 9 {
                continue;
            }
            //4B3FD4 normalLoad+24==0: native hardcoded flags2 never writes.
            //Their actual fixed literal getter remains checked when reached.
            if store
                .flags(field.name())
                .is_some_and(|flags| flags & 3 != 0)
            {
                if store.flags(field.name()) != Some(2) {
                    return Err("canonical SVar source policy owner is unbound".into());
                }
                if source_kind == 4 {
                    let _ = field.value(pc).map_err(|e| e.to_string())?;
                }
                continue;
            }
            let value = typed(field.value(pc).map_err(|e| e.to_string())?)?;
            store.apply_source(field.name(), value)?;
        }
    }
    if matched_public != settings.declarations.values().map(Vec::len).sum::<usize>() {
        return Err("canonical SVar public Settings has foreign or unmatched source rows".into());
    }
    Ok(())
}

fn typed(value: aa_rules::native::TypedValue) -> Result<Value, String> {
    use aa_rules::native::TypedValue as T;
    Ok(match value {
        T::Integer { value, .. } => Value::Int(value as u32),
        T::Boolean { value, .. } => Value::Bool(u8::from(value)),
        T::Scalar(v) => Value::Fixed(v.bits),
        T::String { value, .. } => Value::CString(value.encode_utf16().collect()),
        T::Point { bits, .. } => Value::Point3(
            bits.try_into()
                .map_err(|_| "current SVar Point3 native arity differs")?,
        ),
    })
}
impl Store {
    fn apply_source(&mut self, name: &str, value: Value) -> Result<(), String> {
        if self.index(name).is_some() {
            self.assign(name, value)
        } else {
            let bounds = if value.kind() == 3 {
                [0x80000000, 0x7fffffff]
            } else {
                [0xff7fffff, 0x7f7fffff]
            };
            self.add(name.into(), value, 0, bounds)
        }
    }
}
#[cfg(test)]
#[path = "canonical_svars_tests.rs"]
mod tests;
