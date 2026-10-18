Phase6 kill credit and statistics
================================

Production: crates/aa-body/src/credit.rs
Original: actor kill-credit4D57C0, classification58EA30/58EA50/58EA80/
58EAB0/58EAE0/58EDD0/58EE00, definition iterator45ECA0, ordinary score58E870,
relative score58D950, source-or-leader4CE6C0, experience571820/571960.

The implementation preserves actor+8 credited bit4 and session-mode3 early
returns. The bit is written before choosing Body+28 or component7C+90 damage
history; missing history still leaves it set. KillCreditTimeout uses the
unrounded world-time minus damage-time result. Source and allegiance generation
lookups happen before source eligibility and hostility virtual callbacks, so
both identities are cached even if callbacks subsequently change history.

Notification gating retains both outer-actor calls, distinct virtual20(true)
and virtual20(false), player iteration/hostility/visibility order, and the
victim-label callback before rereading the current owner for its label. Required
callbacks preserve owner text generation and presentation event order without
recreating original CString heap/refcount addresses.

The four owner/attacker statistics component identities are cached before
classification. Component presence, definition list presence and virtual
callbacks implement each classification in original order. A nonzero ignored
58EAE0 return has no side effect; its native body is a pure +98 presence read.
Related-owner arguments are reread at the original places. Each ordinary score
increment adds at the selected precision and stores binary32. Relative scores
own logical keyed entries, insert missing entries as zero and compare the
extended sum against the old maximum after a nonpopping binary32 store. This
can change the leader when the rounded new value ties the old maximum.

Experience caches its global multiplier before the two required derived-value
callbacks. Victim value has a binary32 store; zero uses the exact definition
-1 marker or the resolved fallback setting. Source value remains extended.
The ratio, half-scale and half-offset use the original arithmetic sequence;
the factor product is stored before minimum/maximum clamping. Clamp settings
are read after the callbacks. The source-or-leader identity can determine the
ratio while the actual award still goes to the original cached source.
Organization/component7C/componentC4 reward gates are reread after the award.
The world death trigger runs last, unless an earlier required operation fails.

Validation
----------
original_credit.py executes186 original cases:62 cases at PC24/PC53/PC64.
It executes the real classification routines, generation-checked lookup,
score increments, map insert/update, running-maximum comparison and experience
formula. It supplies registry data, actor virtual results, player iteration,
string construction, derived extended actor values, reward/world callbacks,
map node allocation and disabled diagnostic logging as explicit fixtures.
Callbacks record complete ordering and current state; selected callbacks mutate
owner, history, component presence and experience settings synchronously.
Native stack/nonvolatile-register preservation is checked in every execution.
Original/v14 function bodies and all visited instruction-byte windows agree.
Visited pins include intercepted entry bytes and do not prove execution of the
replaced leaves. A relation insertion case executes actual7BA3C0 with only its
node allocator intercepted.

The Rust test compares every event, flag, ordinary/related score and returned
source with those186 runs. Additional tests cover the extended maximum tie and
partial state after an award callback error. No fake native exception is used
as an expected failure result. The public Result error path is not transactional:
the credited bit and already applied scores remain. Retrying by clearing only
that bit would double-count earlier successful work.

Remaining dependencies / unsupported boundaries
------------------------------------------------
The world must bind real actor/owner/statistic identities and provide complete
consistent component views, ordered player iteration, actor virtuals, labels,
and loaded settings. A component7C alternate damage record must describe that
actual component; missing native components may not be silently fabricated.
Required derived actor value4D01E0 still needs real component aggregation.
Actual experience application4CE770, organization reward4CEB40, component7C
reward4D10F0, componentC4 reward4D3E20, campaign observer7A06A0 and death-trigger
54BE10 remain explicit callbacks. This module does not claim the whole ownership,
promotion, campaign, or playable combat world is connected.

Native invalid-pointer/null-dereference domains produce explicit errors where
representable, not fake parity. Heap layout, hash bucket order, failed native
allocation, diagnostics/CString allocation and exception unwinding are outside
the gameplay contract. Values are finite round-to-nearest with explicit
precision; process-wide FPU defaults/exception flags are not inferred.
No original game executable, release file, asset or save was changed.

Reproduction:
  python research/phase6-credit/inspect_native.py
  python research/phase6-credit/original_credit.py
  python research/phase6-credit/record_findings.py
Peer review is a separate independent-review.json, never this producer.
