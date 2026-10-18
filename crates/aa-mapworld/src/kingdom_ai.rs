//! Concrete Kingdom tactical/strategic storage and the bounded empty-table
//! source load path. This module does not execute strategic AI decisions.
use crate::{Error, Result, restore::NamedDefinition};
use aa_actors::ActorId;

#[derive(Clone, Debug)]
pub struct Entry<K> {
    pub key: K,
    pub value: u32,
    pub next: Option<Box<Entry<K>>>,
}
#[derive(Clone, Debug)]
pub struct Table<K> {
    pub buckets: Vec<Option<Box<Entry<K>>>>,
    pub count: u32,
    pub owns_entries: bool,
}
impl<K> Table<K> {
    fn new(bucket_count: usize) -> Result<Self> {
        let mut buckets = Vec::new();
        buckets
            .try_reserve_exact(bucket_count)
            .map_err(|_| Error::Allocation)?;
        buckets.resize_with(bucket_count, || None);
        Ok(Self {
            buckets,
            count: 0,
            owns_entries: true,
        })
    }
}
fn tables<K>(buckets: usize) -> Result<[Table<K>; 5]> {
    let mut values = Vec::new();
    for _ in 0..5 {
        values.push(Table::new(buckets)?);
    }
    values
        .try_into()
        .map_err(|_| Error::Format("invalid fixed tactical table count".into()))
}
#[derive(Clone, Debug)]
pub struct TacticalState {
    pub definitions: [Table<NamedDefinition>; 5],
    pub handles: [Table<ActorId>; 5],
    /// OriginalA0/B4/C8/DC counters; their tactical interpretation stays opaque.
    pub counters: [[u32; 5]; 4],
}
impl TacticalState {
    pub fn new() -> Result<Self> {
        Ok(Self {
            definitions: tables(97)?,
            handles: tables(13)?,
            counters: [[0; 5]; 4],
        })
    }
    /// Source tutorial tables are empty. Other records must use their native
    /// definition/identity/hash insertion path, which remains an explicit gate.
    pub fn apply_empty_saved(&mut self, source: &crate::kingdoms::TacticalState) -> Result<()> {
        if source.version > 2 {
            return Err(Error::Format("unsupported tactical state version".into()));
        }
        let count = 4 + usize::from(source.version > 1);
        if source.definition_groups.len() != count
            || source.handle_groups.len() != if source.version > 0 { count } else { 0 }
        {
            return Err(Error::Format(
                "tactical group count differs from native version schema".into(),
            ));
        }
        if source
            .definition_groups
            .iter()
            .any(|v| !v.values.is_empty())
            || source.handle_groups.iter().any(|v| !v.values.is_empty())
        {
            return Err(Error::Format(
                "nonempty tactical tables require native hash/reference binding".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct StrategicState {
    /// None represents original uninitialized storage, not a fabricated zero.
    pub words: [Option<u32>; 66],
}
impl StrategicState {
    pub fn new(initialize: bool) -> Self {
        let mut words = [None; 66];
        if initialize {
            words[..57].fill(Some(0));
        }
        Self { words }
    }
    pub fn apply_saved(&mut self, source: &crate::kingdoms::StrategicState) -> Result<()> {
        match source.version {
            0 => {
                if source.words.len() != 60 {
                    return Err(Error::Format(
                        "strategic v0 requires60serialized words".into(),
                    ));
                }
                self.words[..57].fill(Some(0));
                for (i, v) in source.words[..52].iter().enumerate() {
                    self.words[i] = Some(*v);
                }
                for (i, v) in source.words[52..].iter().enumerate() {
                    self.words[57 + i] = Some(*v);
                }
            }
            1 => {
                if source.words.len() != 66 {
                    return Err(Error::Format(
                        "strategic v1 requires66serialized words".into(),
                    ));
                }
                for (i, v) in source.words.iter().enumerate() {
                    self.words[i] = Some(*v);
                }
            }
            _ => return Err(Error::Format("unsupported strategic state version".into())),
        }
        Ok(())
    }
}
