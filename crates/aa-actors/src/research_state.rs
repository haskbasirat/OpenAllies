//! Original Research component (slot 21, 0x18 bytes): constructor 584210
//! (4DB0C0 base, empty in-progress list), saved Load 5849E0, writer 584060.
//! Oracle: research/phase8-restoration/native-research.json.
use crate::{ActorId, ground_states::ComponentHeader};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchState {
    pub header: ComponentHeader,
    /// +10/+14 in-progress ResearchItems (canonical Property IDS), head
    /// first; +14 (the tail) is the oldest entry.
    pub items: Vec<String>,
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("ResearchComponent: research property '{0}' not found (native 858A00)")]
    NotFound(String),
}
impl ResearchState {
    pub fn new(owner: ActorId) -> Self {
        Self {
            header: ComponentHeader::new(owner),
            items: Vec::new(),
        }
    }
    /// 5849E0 per saved reference: registry lookup (case-insensitive) then
    /// 45EE00 over the ActorData research list; the property's +54 FX list
    /// fires through 42A590 (`fx`), then the item is head-inserted. Earlier
    /// items stay inserted when a later reference fails.
    pub fn load(
        &mut self,
        saved: &[String],
        research_items: &[String],
        mut fx: impl FnMut(&str),
    ) -> Result<(), Error> {
        self.header.scheduled_head = None;
        for reference in saved {
            let item = research_items
                .iter()
                .find(|item| item.eq_ignore_ascii_case(reference))
                .ok_or_else(|| Error::NotFound(reference.clone()))?;
            fx(item);
            self.items.insert(0, item.clone());
        }
        Ok(())
    }
}
