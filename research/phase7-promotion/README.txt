Native owned ExperienceComponent and rank transitions
===================================================

aa-body::promotion owns ExperienceState, RankTable and the XP/rank transition
state. Constructor571690 sets owner, XP+8=positive zero and rank+C=null. The
factory must subsequently assign a real rank via5716B0; apply does not guess a
rank for an uninitialized component because original45DB60 dereferences it.

571A60 increments XP with a binary32 store, clamps only to a positive cap from
the actor's Element+80/+14 definition+18, then repeatedly selects the next rank
using current rank's signed index+4 plusone in the definition4F8/4FC table.
Each threshold is compared to current XP. Crossed ranks remove old modifiers,
write current rank, apply new modifier(kind7), dispatch rank-change effects and
optionally notify, in that order. Callback changes of XP/current rank/class
are respected at their original read points. Incoming notification kind is
cached from Organization/regiment+7C before the actor-label callback.

4CEB70 experience propagation receives the original award amount even if XP
was capped. A fresh allegiance is then queried. For positive awards, original
classifiers select score indices6/7 for regiments or17/18 for elements with an
experience component; totals add the original amount and maxima compare the
current XP. Negative awards do not demote. Reset571C50 writes XPzero and uses
the same transition operation. Rank setter5716B0 also preserves callback order
and rereads the post-add current rank for rank-change dispatch.

Native validation:141 original executions atPC24/PC53/PC64. Constructor, XP
math/caps, next-rank table, classifications and statistics execute unchanged.
Modifier lifecycle, rank-change effects, strings, allegiance virtuals,
presentation and member/allegiance propagation are required recorded seams.
Those effects must be bound to real world components; a recorded event alone
is not a complete promotion. Selected callbacks mutate concrete state. Two
Rust tests compare all state/events/stats and verify failure after native writes.

Rank data must be loaded in original order with real native indices/modifiers.
Null current rank, missing definitions, invalid indices, unsupported nonfinite
arithmetic and missing score storage are explicit errors. Errors preserve earlier
XP/rank/modifier effects, so automatic retries are unsafe. Heap/string lifetime,
native invalid pointers, exception flags and process-wide FPU mode are not
emulated. No game, release, asset or save file was modified.

Reproduce: python research/phase7-promotion/inspect_native.py
           python research/phase7-promotion/original_promotion.py
           python research/phase7-promotion/record_findings.py
