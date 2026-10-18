//! Actual42D0D0 helper/current-alias fields, retained once per application.
//! Safe Weak aliases replace native addresses; constructor/Drop retain the
//! same ordered alias semantics, including an older helper clearing a newer
//! registration. This is not a VWorld/renderer readiness capability.
use std::sync::{Arc, Mutex, Weak};
type Result<T> = std::result::Result<T, String>;
struct Identity;
struct Fields {
    root_8c: Weak<Identity>,
    alias_90: bool,
    current_94: Weak<Identity>,
}
pub struct State {
    fields: Arc<Mutex<Fields>>,
}
impl State {
    ///The three fresh-process PE fields are zero. Only the once-installed
    ///application Inputs factory invokes this constructor; never per View.
    pub(crate) fn fresh_application() -> Self {
        Self {
            fields: Arc::new(Mutex::new(Fields {
                root_8c: Weak::new(),
                alias_90: false,
                current_94: Weak::new(),
            })),
        }
    }
    pub fn construct_helper_42d0d0(&mut self) -> Result<Helper> {
        let identity = Arc::new(Identity);
        let mut f = self
            .fields
            .lock()
            .map_err(|_| "view helper global owner is poisoned")?;
        f.current_94 = Arc::downgrade(&identity);
        f.alias_90 = true;
        f.root_8c = Arc::downgrade(&identity);
        Ok(Helper {
            identity,
            fields: Arc::downgrade(&self.fields),
        })
    }
    pub fn current_helper(&self) -> Result<Option<HelperWitness>> {
        let f = self
            .fields
            .lock()
            .map_err(|_| "view helper global owner is poisoned")?;
        Ok(f.current_94.upgrade().map(|identity| HelperWitness {
            identity: Arc::downgrade(&identity),
            fields: Arc::downgrade(&self.fields),
        }))
    }
}
///The actual VWorld10C allocation, retained until its real owner is destroyed.
pub struct Helper {
    identity: Arc<Identity>,
    fields: Weak<Mutex<Fields>>,
}
impl Helper {
    pub fn witness(&self) -> HelperWitness {
        HelperWitness {
            identity: Arc::downgrade(&self.identity),
            fields: self.fields.clone(),
        }
    }
}
#[derive(Clone)]
pub struct HelperWitness {
    identity: Weak<Identity>,
    fields: Weak<Mutex<Fields>>,
}
impl HelperWitness {
    pub fn is_current(&self, state: &State) -> Result<bool> {
        let Some(fields) = self.fields.upgrade() else {
            return Ok(false);
        };
        if !Arc::ptr_eq(&fields, &state.fields) || self.identity.upgrade().is_none() {
            return Ok(false);
        }
        let f = fields
            .lock()
            .map_err(|_| "view helper global owner is poisoned")?;
        Ok(Weak::ptr_eq(&f.current_94, &self.identity) && f.current_94.upgrade().is_some())
    }
}
impl Drop for Helper {
    fn drop(&mut self) {
        //Rust poisoned-mutex failure is outside the admitted normal native
        //destructor domain. It never fabricates a replacement registration.
        if let Some(fields) = self.fields.upgrade() {
            if let Ok(mut f) = fields.lock() {
                f.current_94 = Weak::new();
                if f.alias_90 {
                    f.root_8c = Weak::new();
                }
            }
        }
    }
}

///86C490's VWorld12C constructor fields. Unknown padding remains unknown;
///no guessed class label, complete Ni ABI or initialized rendering is implied.
pub struct Auxiliary12c {
    bytes: [Option<u8>; 0x38],
}
impl Auxiliary12c {
    pub fn from_native_constructor() -> Self {
        let mut s = Self {
            bytes: [None; 0x38],
        };
        for (o, w) in [
            (0x14, 0),
            (0x18, 0xab71ac),
            (0x20, 0),
            (0x2c, 1),
            (0x24, 0),
            (0x28, 0),
            (0x1c, 0),
            (0, 0),
            (4, 0),
            (8, 1),
            (0xc, 1),
        ] {
            s.word(o, w);
        }
        s.bytes[0x10] = Some(1);
        s.bytes[0x11] = Some(0);
        for o in [0x34, 0x35, 0x36, 0x37] {
            s.bytes[o] = Some(0);
        }
        s.word(0x30, 0);
        s
    }
    fn word(&mut self, o: usize, w: u32) {
        for (i, b) in w.to_le_bytes().into_iter().enumerate() {
            self.bytes[o + i] = Some(b);
        }
    }
    pub fn byte_at(&self, o: usize) -> Option<u8> {
        self.bytes.get(o).copied().flatten()
    }
    pub fn word_at(&self, o: usize) -> Option<u32> {
        let b = self.bytes.get(o..o.checked_add(4)?)?;
        Some(u32::from_le_bytes([b[0]?, b[1]?, b[2]?, b[3]?]))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    #[test]
    fn actual_helper_alias_constructor_and_older_destructor_clearing_newer_owner() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../research/development10/view-init-world/native-tail-fields.json"
        ))
        .unwrap();
        let mut count = 0;
        for row in fixture["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["kind"] == "helper")
        {
            let mut state = State::fresh_application();
            let mut owners = std::collections::BTreeMap::new();
            let mut identities = std::collections::BTreeMap::new();
            let mut output = Vec::new();
            for operation in row["input"].as_array().unwrap() {
                let operation = operation.as_str().unwrap();
                let (kind, name) = operation.split_once(':').unwrap();
                if kind == "new" {
                    let helper = state.construct_helper_42d0d0().unwrap();
                    identities.insert(name, helper.witness());
                    owners.insert(name, helper);
                } else {
                    drop(owners.remove(name).unwrap());
                }
                let f = state.fields.lock().unwrap();
                let tag = |ptr: &Weak<Identity>| -> Option<&str> {
                    if ptr.upgrade().is_none() {
                        return None;
                    }
                    identities
                        .iter()
                        .find(|(_, w)| Weak::ptr_eq(ptr, &w.identity))
                        .map(|(n, _)| *n)
                };
                output.push(json!({"operation":operation,"root":tag(&f.root_8c),"alias":f.alias_90,"current":tag(&f.current_94),"vtable":if kind=="new"{"0x9fd158"}else{"0x9fd154"}}));
            }
            assert_eq!(json!(output), row["output"], "{row}");
            count += 1;
        }
        assert_eq!(count, 12);
    }
    #[test]
    fn actual_auxiliary_constructor_preserves_unknown_padding() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../research/development10/view-init-world/native-tail-fields.json"
        ))
        .unwrap();
        let mut count = 0;
        for row in fixture["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["kind"] == "auxiliary")
        {
            let s = Auxiliary12c::from_native_constructor();
            let fill = row["input"].as_u64().unwrap() as u8;
            for i in 0..0x38 {
                let expected = row["output"][i].as_u64().unwrap() as u8;
                match s.byte_at(i) {
                    Some(b) => assert_eq!(b, expected, "{i:X} {row}"),
                    None => assert_eq!(fill, expected, "unknown padding{i:X} {row}"),
                }
            }
            assert!(s.word_at(0x10).is_none());
            count += 1;
        }
        assert_eq!(count, 9);
    }
}
