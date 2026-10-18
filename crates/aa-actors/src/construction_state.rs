//! Original565FA0 construction storage with the source-proven empty predecessor
//! chain. Cost vectors come from the actual null-modifier45F4D0 getter.
use crate::{
    ActorId,
    ground_states::{ComponentHeader, Error, Result},
};
#[derive(Clone, Debug)]
pub struct Definition {
    /// None means no EconomyComponent, so5657B0 performs no vector assignment.
    pub base_cost_words: Option<Vec<u32>>,
    /// Must be proved from completed native definition linking. A nonempty
    /// chain requires actual owner-modifier evaluation and remains unsupported.
    pub empty_predecessor_chain: bool,
}
#[derive(Clone, Debug)]
pub struct ConstructionState {
    pub header: ComponentHeader,
    pub words_10: Vec<u32>,
    pub words_1c: Vec<u32>,
    pub capacity_hints: [u32; 2],
    /// Portable definition identity once saved28 has been resolved.
    pub definition_28: Option<u32>,
}
impl ConstructionState {
    pub fn new(owner: ActorId, definition: &Definition) -> Result<Self> {
        if !definition.empty_predecessor_chain {
            return Err(Error::Definition(
                "Construction predecessor costs require live owner modifiers",
            ));
        }
        let values = definition.base_cost_words.as_deref().unwrap_or(&[]);
        let count = u32::try_from(values.len()).map_err(|_| Error::Allocation)?;
        let capacity = if count > 16 {
            count
        } else if count > 8 {
            16
        } else {
            8
        };
        let mut primary = Vec::new();
        primary
            .try_reserve_exact(capacity as usize)
            .map_err(|_| Error::Allocation)?;
        primary.extend_from_slice(values);
        let mut secondary = Vec::new();
        secondary
            .try_reserve_exact(capacity as usize)
            .map_err(|_| Error::Allocation)?;
        secondary.resize(values.len(), 0);
        Ok(Self {
            header: ComponentHeader::new(owner),
            words_10: primary,
            words_1c: secondary,
            capacity_hints: [capacity; 2],
            definition_28: None,
        })
    }
    pub fn reset_base_cost(&mut self, definition: &Definition) -> Result<()> {
        if !definition.empty_predecessor_chain {
            return Err(Error::Definition(
                "Construction predecessor costs require live owner modifiers",
            ));
        }
        if let Some(values) = &definition.base_cost_words {
            let count = u32::try_from(values.len()).map_err(|_| Error::Allocation)?;
            if count > self.capacity_hints[0] {
                let capacity = count.max(
                    self.capacity_hints[0]
                        .checked_mul(2)
                        .ok_or(Error::Allocation)?,
                );
                self.words_10
                    .try_reserve_exact((capacity as usize).saturating_sub(self.words_10.len()))
                    .map_err(|_| Error::Allocation)?;
                self.capacity_hints[0] = capacity;
            }
            self.words_10.clear();
            self.words_10.extend_from_slice(values);
        }
        Ok(())
    }
}
