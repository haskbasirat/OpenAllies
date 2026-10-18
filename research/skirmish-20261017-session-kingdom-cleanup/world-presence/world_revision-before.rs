//! Process B3CC94, zero in the PE loader's uninitialized section tail.
//! World constructor4F8072..4F809C increments it with native u32 wrapping.
//! Kept by the actual managed application; each World retains its own stamp.
#[derive(Debug)]
pub(crate) struct Counter {
    value: u32,
}
impl Counter {
    pub(crate) fn fresh_application() -> Self {
        Self { value: 0 }
    }
    pub(crate) fn construct(&mut self) -> ConstructedWorldRevision {
        self.value = self.value.wrapping_add(1);
        ConstructedWorldRevision {
            value: self.value,
            identity: std::sync::Arc::new(()),
        }
    }
}
/// Native revision value created by the same application's World constructor.
/// No Clone/Default/public scalar constructor can manufacture this receipt.
#[derive(Debug)]
pub struct ConstructedWorldRevision {
    value: u32,
    identity: std::sync::Arc<()>,
}
impl ConstructedWorldRevision {
    pub fn value(&self) -> u32 {
        self.value
    }
    /// Stable pointer identity metadata. Equal numeric native revisions in
    /// two applications still name different actual World allocations.
    pub(crate) fn owner_identity(&self) -> &std::sync::Arc<()> {
        &self.identity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_constructor_interval_and_zero_tail() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/development10/world-revision/native-world-revision.json"
        ))
        .unwrap();
        assert_eq!(fixture["native_executions"], 48);
        let section = &fixture["zero_tail"];
        assert!(section["raw_size"].as_u64().unwrap() <= section["word_offset"].as_u64().unwrap());
        let mut fresh = Counter::fresh_application();
        assert_eq!(fresh.value, 0);
        assert_eq!(fresh.construct().value(), 1);
        assert_eq!(fresh.construct().value(), 2);
        for row in fixture["rows"].as_array().unwrap() {
            let mut owner = Counter {
                value: row["before"].as_u64().unwrap() as u32,
            };
            assert_eq!(u64::from(owner.construct().value()), row["after"]);
        }
    }
}
