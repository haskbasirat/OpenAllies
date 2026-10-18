//! VWorld130's native constructor/43FCA4 slot-array bookkeeping.
//! Unwritten capacity words stay distinct from actual initialized null words.
type Result<T> = std::result::Result<T, String>;
enum Slot {
    Unwritten,
    Null,
}
pub(crate) struct State {
    words: Vec<Slot>,
    count: usize,
}
impl State {
    pub(crate) fn constructed() -> Self {
        Self {
            words: std::iter::repeat_with(|| Slot::Unwritten).take(3).collect(),
            count: 0,
        }
    }
    pub(crate) fn count(&self) -> usize {
        self.count
    }
    pub(crate) fn capacity(&self) -> usize {
        self.words.len()
    }
    pub(crate) fn initialize(&mut self, count: usize) -> Result<()> {
        let count32 = i32::try_from(count)
            .map_err(|_| "Terrain registry count exceeds the native signed field representation")?;
        let old = i32::try_from(self.words.len())
            .map_err(|_| "Terrain slot capacity exceeds the native signed field representation")?;
        let doubled = old.wrapping_mul(2);
        let capacity = if count32 > doubled {
            count32
        } else if count32 > old {
            doubled
        } else {
            old
        };
        if capacity < 0 {
            return Err("Terrain slot capacity reached native signed allocation overflow".into());
        }
        let capacity = capacity as usize;
        if capacity > self.words.len() {
            self.words
                .try_reserve(capacity - self.words.len())
                .map_err(|_| "Terrain slot allocation failed")?;
            self.words.resize_with(capacity, || Slot::Unwritten);
        }
        self.count = count;
        for word in self.words.iter_mut().take(count) {
            *word = Slot::Null;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_registry_count_grows_without_a_fixed_cap_and_zeroes_only_active_slots() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/development10/view-init-world/native-tail-fields.json"
        ))
        .unwrap();
        let mut cases = 0;
        for row in fixture["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["kind"] == "slots")
        {
            let mut state = State::constructed();
            state
                .initialize(row["input"].as_u64().unwrap() as usize)
                .unwrap();
            let out = &row["output"];
            assert_eq!(state.count(), out["count"].as_u64().unwrap() as usize);
            assert_eq!(state.capacity(), out["capacity"].as_u64().unwrap() as usize);
            for (i, w) in state.words.iter().enumerate() {
                match w {
                    Slot::Null => assert_eq!(out["words"][i], 0),
                    Slot::Unwritten => assert_eq!(out["words"][i], 0xa5a5a5a5u32),
                }
            }
            cases += 1;
        }
        assert_eq!(cases, 27);
    }
}
