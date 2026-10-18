//! Original UPDATABLES decoding and binding to the completed saved-ID table.
//! Whole queue Load paths are verified against every original map's bytes.
use crate::updatables::Table;
use aa_world::queues::{Kind, Owner, Queues, restoration::SavedQueue};
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("saved queue data: {0}")]
    Source(&'static str),
    #[error(transparent)]
    Identity(#[from] crate::updatables::Error),
    #[error(transparent)]
    Queue(#[from] aa_world::queues::Error),
    #[error("saved queue allocation failed")]
    Allocation,
}
pub type Result<T> = std::result::Result<T, Error>;
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn word(&mut self) -> Result<u32> {
        let bytes = self
            .bytes
            .get(self.at..self.at + 4)
            .ok_or(Error::Source("truncated word"))?;
        self.at += 4;
        Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
    }
}
#[derive(Debug)]
pub struct Decoded {
    pub queues: Vec<SavedQueue>,
    pub discarded_updates: usize,
    pub boundaries: Vec<(u8, usize)>,
}
pub fn decode<T>(
    bytes: &[u8],
    world_version: u32,
    table: &Table<T>,
    mut owner: impl FnMut(&T) -> Owner,
) -> Result<Decoded> {
    if !(11..=28).contains(&world_version) {
        return Err(Error::Source("world queue format version"));
    }
    let mut r = Reader { bytes, at: 0 };
    let count = r.word()? as i32;
    if count < 0 || count as usize != table.len() {
        return Err(Error::Source("table count differs from bound table"));
    }
    let mut out = Decoded {
        queues: Vec::new(),
        discarded_updates: 0,
        boundaries: Vec::new(),
    };
    let order: &[u8] = if world_version >= 22 {
        &[3, 4, 0, 1, 2]
    } else {
        &[3, 4]
    };
    for &id in order {
        if r.word()? != 0 {
            return Err(Error::Source("queue version"));
        }
        let current = r.word()? as i32;
        let count = r.word()? as i32;
        if count <= 0
            || current < 0
            || current >= count
            || count as usize > (bytes.len() - r.at) / 4
        {
            return Err(Error::Source("queue dimensions"));
        }
        let mut buckets = Vec::new();
        buckets
            .try_reserve_exact(count as usize)
            .map_err(|_| Error::Allocation)?;
        for _ in 0..count {
            let count = r.word()? as i32;
            let mut bucket = Vec::new();
            // The original signed list loop skips a negative count.
            if count > 0 {
                if count as usize > (bytes.len() - r.at) / 8 {
                    return Err(Error::Source("list extent"));
                }
                bucket
                    .try_reserve_exact(count as usize)
                    .map_err(|_| Error::Allocation)?;
                for _ in 0..count {
                    let source = r.word()? as i32;
                    let data = r.word()?;
                    if let Some(target) = table.resolve(source)? {
                        bucket.push((owner(target), data));
                    } else {
                        out.discarded_updates += 1;
                    }
                }
            }
            buckets.push(bucket);
        }
        out.queues.push(SavedQueue {
            id,
            kind: if id < 3 {
                Kind::Periodic
            } else {
                Kind::Deferred
            },
            current: current as usize,
            buckets,
        });
        out.boundaries.push((id, r.at));
    }
    if r.at != bytes.len() {
        return Err(Error::Source("trailing bytes"));
    }
    Ok(out)
}
impl Decoded {
    /// Called only after all actual target components are restored. Returns
    /// discarded entries (native null-ID/null-slot behavior), not live actors.
    pub fn install(self, queues: &mut Queues) -> Result<usize> {
        for queue in self.queues {
            queues.restore_saved(queue)?;
        }
        Ok(self.discarded_updates)
    }
}
