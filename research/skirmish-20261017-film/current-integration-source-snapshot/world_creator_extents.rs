//! World+3C/+40 written by4F28A6..4F28B1 from the actual Creator passed to
//! World::Init. This value copy precedes5A2FCF..5A3013's optional replacement
//! of MapResource+88; the final selected Creator is not the source of these
//! fields. Terrain's ceiled GMap extents are a separate native owner.

/// Actual World scalar storage. There is no public raw/default/clone
/// constructor, and no floating-point conversion touches these payloads.
#[derive(Debug)]
pub struct State {
    words: [u32; 2],
}
impl State {
    /// Called at the same managed World::Init boundary, before resource+88
    /// selection. Creator+3C and+40 are copied with native DWORD MOVs.
    pub(crate) fn from_world_init_creator(creator: &crate::resource_creator::Creator) -> Self {
        Self::copy_words(creator.extents_bits())
    }
    fn copy_words(words: [u32; 2]) -> Self {
        Self { words }
    }
    pub fn bits(&self) -> [u32; 2] {
        self.words
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_world_init_creator_raw_copy_and_partial_stores() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-film/native-world-creator-extents.json"
        ))
        .unwrap();
        let rows = fixture["cases"].as_array().unwrap();
        assert_eq!(rows.len(), 162);
        for row in rows {
            let input = &row["input"];
            let source: [u32; 2] = serde_json::from_value(input["creator"].clone()).unwrap();
            let before: [u32; 2] = serde_json::from_value(input["world_before"].clone()).unwrap();
            let stop = input["stop"].as_u64().map(|v| v as usize);
            let complete = State::copy_words(source).bits();
            // Explicit native instruction cutoffs: first MOV load, width
            // store, height load, unrelated x87 pop, height store.
            let expected = [
                if stop.is_some_and(|v| v <= 1) {
                    before[0]
                } else {
                    complete[0]
                },
                if stop.is_some_and(|v| v <= 4) {
                    before[1]
                } else {
                    complete[1]
                },
            ];
            assert_eq!(
                serde_json::to_value(expected).unwrap(),
                row["output"]["world"]
            );
            assert_eq!(
                row["output"]["fpu_top"],
                if stop.is_some_and(|v| v <= 3) { 7 } else { 0 }
            );
        }
    }
}
