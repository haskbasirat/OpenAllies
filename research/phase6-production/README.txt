Phase 6: original structure production queue

aa-production implements a stateful native production path, with required world
services rather than a fabricated playable simulation. It includes recruit
requests through payment/reservation and queue insertion; queue activation;
research/recruit/location item duration; polling and pause behavior; item removal,
cancellation, completion callbacks and failed-completion refunds.

Native entry points

* 0x5836E0: recruit request, organization admission, temporary cost/upkeep vectors.
* 0x582AE0 and 0x582950: recruitability, affordability, pricing, payment and reserve.
* 0x589E00 and 0x5895B0/0x5896E0: recruit/location queue-item creation and append.
* 0x589420: start next item, actor-state selection, deadline, immediate processing,
  then zero-delay scheduling only if the component has no current scheduler item.
* 0x588D60: organization recruit time, reciprocal solo recruit rate, research time,
  or half the native delta for instant/default production.
* 0x589F90: due completion before all supply/disabled checks; limited-resource
  reservation; pause reasons 0/1/2; deadline extension while remaining paused;
  creation/research completion and failure refunds; next-head advancement.
* 0x5894C0/0x588CD0: queue removal, timer cancellation for head removal, resource
  upkeep removal and organization-slot detachment.
* 0x589BD0: first matching item-ID cancellation, refund, resource/item cleanup.
* 0x581D40/0x581D90/0x583F30: native refund wrappers, with resource credit executed.

The queue stores original discriminator values, opaque definition/property/layout
identities, raw binary32 values and dense resource vectors in explicit catalog
order. No arbitrary queue capacity, worker cadence, or wall-clock rate is added.
The owner supplies Context now/delta and x87 precision. This context must be
stable during the operation; the native world time is not guessed or advanced.

Ordering details preserved

Affordability computes one cost quote, then payment computes it again. A changed
second price is debited without another affordability check, even if that creates
a negative stockpile. Payment/reservation precedes item allocation. The native
kingdom handle captured for payment is retained separately from the handle used
inside the affordability helper.

For a default organization layout, the admission helper resolves a local layout
for pricing/reservation. Queue insertion still copies the caller's original
layout reference. The API preserves this difference.

The duration is stored as binary32 for presentation, but deadline addition uses
the unpopped extended duration, matching the FST/FADD/FSTP sequence. Positive solo
recruit rate is fetched a second time before its reciprocal is computed. The
native half-delta constant is binary32 0x3F000000.

The process operation checks now >= deadline before testing disabled/supply or
limited-resource pause conditions. A blocked operation extends the deadline only
when the previously recorded pause timestamp is nonnegative. Returning to active
state clears the pause fields without inventing a separate resume-time update.
Notifications obey the previous pause state/reason and reservation result.

Head removal cancels the scheduler before removing upkeep. Cancellation itself
does not start the next head: that is the native caller's responsibility. By
contrast, due completion always starts the next head. When an instant item rounds
to an already-due deadline, nested native processing can empty the queue before
the outer start operation schedules again; that ordering is retained.

Public boundaries and economy integration

World and Recruitment have required methods without permissive defaults. The
caller supplies actual ownership, authored data, pricing modifiers, availability,
organization registry behavior, supply state, fulfillment, notifications and
scheduler operations. Fulfillment is not replaced with a fake actor spawn.
Prepared research items can be queued and completed, but research request/pricing
admission is not implemented in this crate.

The source-bound integration harness uses the existing aa-economy Economy for
affordability, stock debit/credit and coupled consumption updates. It compares
whole economy state against real original instructions, including the native
shortage remove/mutate/reapply order. Runtime consumers must use the real catalog
order and owner handle; they must not substitute placeholder resource defaults.

The limited_upkeep callback must produce the 0x4CDA40 projection: original values
only for LimitedConsumed rows and zero for all other resource kinds. Native
4CCE70 compares numeric float equality, including equality of positive/negative
zero. Current upkeep is copied before its accounting callback. Real capacity
calculation, population/availability registries, price formulas and supply-region
ownership are not implemented by this crate.

Callback ownership contract: while start/poll retains a selected queued item,
callbacks must preserve queue membership/order and stored item fields. They may
change scheduler/timer fields and external world state. After item removal,
completion/refund/release callbacks may enqueue new items, which the following
start operation observes. Recursive processing of the same queue and arbitrary
external aliases that mutate native item storage are not supported. Rust owns
item values and does not reproduce raw-pointer aliasing. The head-ID check after
the capacity callback catches only an obvious violation; a reused/wrapped ID is
not a proof of allocation identity. No unrestricted callback mutation parity is
claimed by the fixtures or the API.

Scheduling is exposed as original 0x50B250 cancel and 0x50AC00(0) calls. The outer
aa-world queue is not modified or connected by this work. Production command
routing/authorization, actor factory publication, campaign progress, UI commands,
saves and networking remain separate work.

Evidence and supported domain

native_production.py executes original AA.exe in isolated x86 memory. It never
launches a native game or writes original assets/saves. Every visited original
instruction window is checked against the accepted v14 executable. The byte
ledger includes intercepted boundary entries; their function bodies are not
misrepresented as executed. Allocation, free, actor/research creation, scheduler
links, predicates, reservation registries and presentation leaves are explicit
fixtures. Original queue decisions, constructors, vector filtering/equality/copy,
duration/timestamp arithmetic, stock debit/credit and shortage updates execute.

231 sequences and 558 operations run across PC24/PC53/PC64. They cover admission
success/rejection, two price quotes, successful/failed fulfillment, head/non-head
cancellation, FIFO advancement, all pause causes, default organization layouts,
organization-slot registration/detachment, location recruits, rate fallbacks,
instant/zero/negative durations, fractional deadlines, signed-zero resources and
ID wrapping. Each native top-level operation verifies return stack and preserved
EBX/ESI/EDI/EBP registers. Rust compares complete queue state, exact float bits,
economy arrays, ID counter, return results and ordered semantic callbacks.

Four Rust tests include the complete corpus, a callback error after payment,
an error after removal/accounting, and a nonfinite clock. Errors stop further
work but do not claim rollback of earlier state/effects; automatic retry is unsafe.
Inputs are finite arithmetic values with consistent resolved resource vectors.
Unsupported/malformed numeric domains fail explicitly rather than emulating x86
invalid arithmetic or memory corruption.

The native item constructors do not initialize the +0x4C organization-slot word.
The oracle allocator zeroes fresh memory, and safe Rust initializes the unused
word to zero. This is an explicit defined initialization, not a claim that every
original heap allocation contained zero. Organization removal later replaces it
with the actual detachment result before relevant fulfillment.

Reproduce:
    python research/phase6-production/native_production.py
    cargo test -p aa-production -- --nocapture
    cargo clippy -p aa-production --all-targets -- -D warnings
    python research/phase6-production/record_evidence.py
