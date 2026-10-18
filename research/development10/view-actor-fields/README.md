# VActor453F10 scalar tail

The new client `view_actor_fields::Fields` owns onlyBC..154 of one actual
VActor allocation, plus the growable128/134 lists. The existing455050 base
remains a separate retained owner in the same ViewDirectory entry. This module
does not construct/reconstruct that base, bind source/Thing identity, perform
451F90 derived Init, publish Thing+C, upload a model or certify view readiness.

All nine current global reads stay lazy: B322A4/A8/AC twice, then B43CF8/CFC/D00.
The caller must supply the real current owners, not a chosen white/zero vector.
Data virtual38 then controls all reached source reads. If false,140 stays
unwritten. D4..EC are never written by this tail and remain unknown in every
case.14C is written only at the final return prefix, so an earlier source
refusal leaves it unknown while144/148/150/154 already retain zero.

The turret list appends its actual zero-initialized record BEFORE57DBE0 obtains
the pitch. That helper executes45DC90 and loads Data1F8[index]+14. A missing
pitch retains the appended record with its constructor pitch0, along with the
actual increased count. Initial and post-iteration source1FC counts are read
separately. The244 list also rereads its source count after every append; its
meaning remains offset-neutral because tracks are a different later field.
Source23C is copied to140 after both loops even when their counts skip.

Each list has one item owner/count and growable allocation metadata. Initial
native capacities2/3 and native doubling are allocation behavior, not caps.
The portable Vec can grow until hardware allocation fails. Positive native
array pointers are opaque; their numeric getter returns unknown rather than
inventing addresses. Actual constructor-null pointers can return0. Records
retain the native byte8=0 but padding9..B stays unreadable as a complete word.
Portable Rust allocation/refcount/pointer ABI and out-of-memory behavior are
not claimed equivalent to native heap handling.

The pitch slice copies finite binary32 through the native FLD/FSTP spill;
there is no arithmetic precision choice in the Rust API. Signed zero and
subnormal values are covered at all three native x87 precision controls.
Nonfinite pitch is an explicit portable refusal; no Unicorn signalling-NaN
behavior is accepted as hardware evidence. Other scalar copies preserve their
raw bits exactly, including the laboratory NaN color/position words.

`native-actor-fields.json` has99 paired rows/198 original-v14 runs, with269
individually decoded equal instruction pins.455050 is an explicit separately
owned base seam; allocation/free are the only other transports. Native source
virtual38,57DBE0/45DC90,766DE0/4849F0 and both loops execute. All23 reached
Sources cutoffs of a nontrivial case preserve known fields, list records and
source order. Source count changes in some cases are explicit external storage
interventions; they do not assert native application reentrancy.

Two tests compile the exact production file and replay every fixture row,
including source order, masks, partial owners, capacities, pointer opacity and
record padding. The module is exported under live_scene for the SDK author's
same-source integration. Whole client/actual-data source binding remains that
separate caller's acceptance; no successful source callback is invented here.

```powershell
python research/development10/view-actor-fields/native_actor.py
python research/development10/view-actor-fields/audit_actor.py
research/development10/view-actor-fields/isolated-actor-final.exe --nocapture
```
