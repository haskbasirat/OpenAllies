//! Concrete manager-local546AF0 operations. World-dependent opcodes must be
//! dispatched by their actual recovered world implementation; this evaluator
//! returns an explicit error for those operations instead of reporting success.
use crate::{
    decode::{Element, Parameter, Value, Variable},
    manager::{Elements, Error, Manager, Result},
};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Diagnostic {
    MissingVariable(Vec<u16>),
    EmptyVariableName,
    ExpectedVariable,
    ActionFailed { trigger: u32, element: String },
    /// Original logging call (404ED0/404F30/404FC0/405CB0) at this point.
    NativeLog(u32),
}
pub struct CoreProgram {
    pub time_bits: u32,
    pub application_mode: u32,
    pub precision: Precision,
    pub diagnostics: Vec<Diagnostic>,
}
fn numeric(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(|e| Error::State(e.to_string()))
}
fn arithmetic<T>(value: std::result::Result<T, aa_numeric::Error>) -> Result<T> {
    value.map_err(|e| Error::State(e.to_string()))
}
pub(crate) fn text(parameter: &Parameter) -> Result<&[u16]> {
    match &parameter.value {
        Value::Text(value) => Ok(value),
        _ => Err(Error::State(
            "native text parameter context requires text storage".into(),
        )),
    }
}
pub(crate) fn word(parameter: &Parameter) -> Result<u32> {
    match parameter.value {
        Value::Word(value) => Ok(value),
        _ => Err(Error::State(
            "native word parameter context requires word storage".into(),
        )),
    }
}
fn key(value: &[u16]) -> Result<Vec<u16>> {
    if value.iter().any(|c| *c > 127 || *c == 0) {
        return Err(Error::State(
            "non-ASCII or embedded-null variable IDS requires native locale/string rules".into(),
        ));
    }
    Ok(value
        .iter()
        .map(|c| if (97..=122).contains(c) { c - 32 } else { *c })
        .collect())
}
fn variable(manager: &Manager, name: &[u16]) -> Result<Option<usize>> {
    let wanted = key(name)?;
    for (i, v) in manager.variables().iter().enumerate() {
        if key(&v.name)? == wanted {
            return Ok(Some(i));
        }
    }
    Ok(None)
}
/// Manager-local opcodes implemented by `CoreProgram::evaluate_core`.
pub const CORE: &[u32] = &[0, 1, 16, 80, 81, 91, 92, 93];

/// Actual541900 validity: count and native argument classes only. A false
/// result skips the opcode. Storage/finite-domain checks happen at actual use.
pub fn valid(element: &Element) -> bool {
    let Some((_, arguments)) = crate::registrations::schema(element.opcode) else {
        return false;
    };
    arguments.len() == element.parameters.len()
        && arguments
            .iter()
            .zip(&element.parameters)
            .all(|(kind, p)| crate::registrations::argument(&p.argument) == Some(*kind))
}
pub fn compare(left: u32, kind: u32, right: u32) -> Result<bool> {
    let order = numeric(left)?.compare(&numeric(right)?);
    Ok(match kind {
        0 => order == Ordering::Less,
        1 => order != Ordering::Greater,
        2 => order == Ordering::Equal,
        3 => order != Ordering::Less,
        4 => order == Ordering::Greater,
        5 => order != Ordering::Equal,
        _ => false,
    })
}
impl CoreProgram {
    pub fn new(time_bits: u32, application_mode: u32, precision: Precision) -> Self {
        Self {
            time_bits,
            application_mode,
            precision,
            diagnostics: vec![],
        }
    }
    /// Native54D9E0 followed by the caller's binary32 store. Missing variables
    /// produce the native diagnostic and actual zero return, not guessed data.
    /// Native 54D9E0 numeric register (literal, float or trigger variable),
    /// returned as the binary32 bits its callers store.
    pub fn value(&mut self, manager: &Manager, parameter: &Parameter) -> Result<u32> {
        match parameter.item_index {
            1 => arithmetic(Binary::from_i32(word(parameter)? as i32).store_f32()),
            2 => {
                let value = word(parameter)?;
                numeric(value)?;
                Ok(value)
            }
            3 => {
                let name = text(parameter)?;
                if name.is_empty() {
                    return Ok(0);
                }
                if let Some(i) = variable(manager, name)? {
                    let value = manager.variables()[i].bits;
                    numeric(value)?;
                    Ok(value)
                } else {
                    self.diagnostics
                        .push(Diagnostic::MissingVariable(name.to_vec()));
                    Ok(0)
                }
            }
            _ => Ok(0),
        }
    }
    pub(crate) fn comparison(parameter: &Parameter) -> Result<u32> {
        if parameter.item_index == 6 {
            word(parameter)
        } else {
            Ok(2)
        }
    }
    /// Return None only when the opcode needs a distinct actual world service.
    /// The caller must not turn None into successful execution.
    pub fn evaluate_core(
        &mut self,
        manager: &mut Manager,
        ordinal: usize,
        element: &Element,
    ) -> Result<Option<bool>> {
        if crate::registrations::schema(element.opcode).is_none() {
            return Err(Error::State(format!(
                "unknown native element opcode {}",
                element.opcode
            )));
        }
        if !valid(element) {
            return Ok(Some(false));
        }
        manager.convert_registers(element);
        let p = &element.parameters;
        let value = match element.opcode {
            0 => true,
            1 => {
                // Native evaluates the right numeric argument before the left.
                let right = self.value(manager, &p[2])?;
                let kind = Self::comparison(&p[1])?;
                let left = self.value(manager, &p[0])?;
                compare(left, kind, right)?
            }
            16 => {
                let right = self.value(manager, &p[1])?;
                let kind = Self::comparison(&p[0])?;
                let elapsed = arithmetic(numeric(self.time_bits)?.sub(
                    &numeric(manager.trigger(ordinal)?.activation_bits)?,
                    self.precision,
                ))?;
                compare(arithmetic(elapsed.store_f32())?, kind, right)?
            }
            80 | 81 => {
                let identity = word(&p[0])?;
                let found = (0..manager.definitions().len())
                    .find(|i| manager.trigger(*i).is_ok_and(|t| t.id == identity));
                if let Some(index) = found {
                    let enabled = manager.trigger(index)?.enabled != 0;
                    let requested = element.opcode == 80;
                    if enabled != requested {
                        manager.set_enabled(index, u8::from(requested), self.time_bits)?;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            91 => {
                let amount = self.value(manager, &p[1])?;
                let name = text(&p[0])?;
                if name.is_empty() {
                    self.diagnostics.push(Diagnostic::EmptyVariableName);
                } else if let Some(i) = variable(manager, name)? {
                    manager.variables_mut()[i].bits = amount;
                } else {
                    key(name)?;
                    manager.variables_mut().push(Variable {
                        name: name.to_vec(),
                        bits: amount,
                    });
                }
                true
            }
            92 | 93 => {
                let amount = self.value(manager, &p[1])?;
                let found = if p[0].item_index == 3 {
                    let name = text(&p[0])?;
                    let found = variable(manager, name)?;
                    if found.is_none() && !name.is_empty() {
                        self.diagnostics
                            .push(Diagnostic::MissingVariable(name.to_vec()));
                    }
                    found
                } else {
                    if [1, 2].contains(&p[0].item_index) {
                        self.diagnostics.push(Diagnostic::ExpectedVariable);
                    }
                    None
                };
                if let Some(i) = found {
                    let old = numeric(manager.variables()[i].bits)?;
                    let amount = numeric(amount)?;
                    let result = if element.opcode == 92 {
                        arithmetic(amount.add(&old, self.precision))?
                    } else {
                        arithmetic(old.sub(&amount, self.precision))?
                    };
                    manager.variables_mut()[i].bits = arithmetic(result.store_f32())?;
                    true
                } else {
                    false
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(value))
    }
}
impl Elements for CoreProgram {
    fn evaluate(
        &mut self,
        manager: &mut Manager,
        ordinal: usize,
        element: &Element,
    ) -> Result<bool> {
        self.evaluate_core(manager, ordinal, element)?
            .ok_or_else(|| {
                Error::State(format!(
                    "opcode{} ({}) requires its actual world service",
                    element.opcode, element.name
                ))
            })
    }
    fn world_time_bits(&self) -> u32 {
        self.time_bits
    }
    fn action_failed(
        &mut self,
        manager: &mut Manager,
        ordinal: usize,
        element: &Element,
    ) -> Result<()> {
        if self.application_mode == 7 {
            self.diagnostics.push(Diagnostic::ActionFailed {
                trigger: manager.trigger(ordinal)?.id,
                element: element.name.clone(),
            });
        }
        Ok(())
    }
}
