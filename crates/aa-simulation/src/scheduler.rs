use crate::{FloatBits, NativeClock, NativeRng, ProviderState, RuleError};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// These are laboratory commands, not a native decree/wire protocol decoder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    AccumulateProvider { slot: usize, delta: FloatBits },
    DrawRandom,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Batch {
    pub sequence: u64,
    pub commands: Vec<Command>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandResult {
    Provider { slot: usize, state: ProviderState },
    Random(u32),
}

type ProviderJobResult = Result<(ProviderState, Vec<(usize, CommandResult)>), RuleError>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchReceipt {
    pub sequence: u64,
    /// Same order as the submitted commands, irrespective of worker completion.
    pub results: Vec<CommandResult>,
    pub snapshot_sha256: [u8; 32],
}

/// A versioned laboratory snapshot, NOT RSG/RMP or native sync serialization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema: u32,
    pub next_sequence: u64,
    pub clock: NativeClock,
    pub rng: NativeRng,
    pub providers: Vec<ProviderState>,
}

impl Snapshot {
    /// Explicit little-endian format; no serde map order, pointers, timestamps,
    /// worker counts or platform usize widths enter the digest.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = b"AA-RUST-LAB\0".to_vec();
        out.extend(self.schema.to_le_bytes());
        out.extend(self.next_sequence.to_le_bytes());
        for word in [
            self.clock.tick,
            self.clock.begin.0,
            self.clock.end.0,
            self.clock.step.0,
            self.rng.state,
        ] {
            out.extend(word.to_le_bytes());
        }
        out.extend((self.providers.len() as u64).to_le_bytes());
        for row in &self.providers {
            out.extend(row.raw.0.to_le_bytes());
            out.extend(row.effective.0.to_le_bytes());
            out.push(u8::from(row.limited));
            out.extend(row.maximum.to_le_bytes());
        }
        out
    }
    pub fn sha256(&self) -> [u8; 32] {
        Sha256::digest(self.canonical_bytes()).into()
    }
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("worker count must be at least one")]
    NoWorkers,
    #[error("could not create worker pool: {0}")]
    WorkerPool(String),
    #[error("expected authoritative sequence {expected}, received {received}")]
    Sequence { expected: u64, received: u64 },
    #[error("authoritative sequence exhausted")]
    SequenceExhausted,
    #[error("unknown provider slot {0}")]
    UnknownSlot(usize),
    #[error("unsupported or inconsistent laboratory snapshot")]
    InvalidSnapshot,
    #[error(transparent)]
    Rule(#[from] RuleError),
}

pub struct Core {
    state: Snapshot,
    pool: rayon::ThreadPool,
}

impl Core {
    pub fn new(
        seed: u32,
        providers: Vec<ProviderState>,
        workers: usize,
    ) -> Result<Self, CoreError> {
        Self::restore(
            Snapshot {
                schema: 1,
                next_sequence: 0,
                clock: NativeClock::stock(),
                rng: NativeRng::new(seed),
                providers,
            },
            workers,
        )
    }
    pub fn restore(state: Snapshot, workers: usize) -> Result<Self, CoreError> {
        if workers == 0 {
            return Err(CoreError::NoWorkers);
        }
        if state.schema != 1
            || NativeClock::with_step(state.clock.step).is_err()
            || state.providers.iter().any(|r| {
                ProviderState::new(r.raw, r.limited, r.maximum)
                    .ok()
                    .as_ref()
                    != Some(r)
            })
        {
            return Err(CoreError::InvalidSnapshot);
        }
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()
            .map_err(|e| CoreError::WorkerPool(e.to_string()))?;
        Ok(Self { state, pool })
    }
    pub fn snapshot(&self) -> Snapshot {
        self.state.clone()
    }
    /// Advance only the proven clock-field fragment. The caller must choose the
    /// phase; this does not pretend to establish the native update phase graph.
    pub fn advance_clock_fragment(&mut self) {
        self.state.clock.advance_fragment();
    }

    pub fn apply_batch(&mut self, batch: Batch) -> Result<BatchReceipt, CoreError> {
        if batch.sequence != self.state.next_sequence {
            return Err(CoreError::Sequence {
                expected: self.state.next_sequence,
                received: batch.sequence,
            });
        }
        let next_sequence = self
            .state
            .next_sequence
            .checked_add(1)
            .ok_or(CoreError::SequenceExhausted)?;
        let mut groups = vec![Vec::<(usize, FloatBits)>::new(); self.state.providers.len()];
        for (index, command) in batch.commands.iter().enumerate() {
            if let Command::AccumulateProvider { slot, delta } = *command {
                groups
                    .get_mut(slot)
                    .ok_or(CoreError::UnknownSlot(slot))?
                    .push((index, delta));
            }
        }
        // Jobs read immutable source rows and own disjoint output. A real
        // economy is NOT safe here: native upkeep has cross-resource effects.
        let jobs: Vec<ProviderJobResult> = self.pool.install(|| {
            groups
                .par_iter()
                .enumerate()
                .map(|(slot, commands)| {
                    let mut row = self.state.providers[slot];
                    let mut results = Vec::with_capacity(commands.len());
                    for &(index, delta) in commands {
                        row = row.accumulated(delta)?;
                        results.push((index, CommandResult::Provider { slot, state: row }));
                    }
                    Ok((row, results))
                })
                .collect()
        });
        // No authoritative state is changed until every job succeeds.
        let mut candidate = self.state.clone();
        let mut results = vec![None; batch.commands.len()];
        for (slot, job) in jobs.into_iter().enumerate() {
            let (row, entries) = job?;
            candidate.providers[slot] = row;
            for (index, result) in entries {
                results[index] = Some(result);
            }
        }
        // Global RNG draws remain in original command order on this thread.
        for (index, command) in batch.commands.iter().enumerate() {
            if matches!(command, Command::DrawRandom) {
                results[index] = Some(CommandResult::Random(candidate.rng.next_u32()));
            }
        }
        candidate.next_sequence = next_sequence;
        let receipt = BatchReceipt {
            sequence: batch.sequence,
            results: results
                .into_iter()
                .map(|r| r.expect("each supported command has one result"))
                .collect(),
            snapshot_sha256: candidate.sha256(),
        };
        self.state = candidate;
        Ok(receipt)
    }
}
