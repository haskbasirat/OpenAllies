# Fresh environment effect constructor and update

`view_texture_effect::Core` is one retained numeric/context owner for the real
environment114 allocation. The outer State keeps the actual source Texture110
and source texture154 handle. Scene118 remains a separate allocation. No local
transform, world transform, projection, plane or D0 counter is copied back from
a second effect instance.

The constructor ports the fields reached by this fresh update. Its local/world
transforms use the actual8751C0 immediate identity stores and nine ordered
current B43CF8/B43CFC/B43D00 reads (including the bounding-center reads whose
values this numeric projection does not expose). Constructor refusal returns
the same partial Core. It retains actual86F710 constructor-null controller,
property, parent and bound links,89CA00 D0=1/C4=1, and86B940 projection/plane
fields. Source matrix B32264 and unit-X B3206C table bytes are pinned in both
images; this does not claim mutable globals remain immutable forever.

The caller uses the actual43EA50 order: configure160/158/15C/164; explicit
86BA10; store C4=1;86E8D0(0,1). The update executes the constructor-proven empty
866BE0 traversals, the13 local→world read/store pairs in8A2C70, the actual mode2
projection and unconditional plane tail, then rereads parent24 after the pure
vf64 return. Vtable loads are typed class reads, preserving their ordered
required boundaries without inventing raw ABI addresses. Actual AB70A0 slots
48/68/64 point to86EED0/86BC30/4B2830.

Only the fresh constructor can establish the null-link capability. There is no
caller-provided empty flag, raw positive pointer or successful child callback.
Future positive parent/controller/property/bound writers must replace that
typed state and bind their real services; this method cannot remain a silent
empty-list fallback. Other projection modes explicitly refuse.

Rust ownership deliberately replaces native heap, vtable/refcount and C8/
B32450 identity accounting. C8 is not exposed as a fabricated scalar. This is
the supported numeric/current-link slice, not full Ni object/ABI accounting or
destruction parity. Finite successful arithmetic and masked round-nearest
explicit x87 precision are the supported numeric domain. There is no AD/view
completion or application activation capability.

`native-effect.json` contains234 paired rows/468 original-v14 executions at all
three precisions. All actual86B940 constructor and86E8D0 update child bodies run;
the harness asserts zero seams/callbacks and no null accesses. It includes all
nine constructor source-read failures for two seed vectors, local/world
differences, controller argument bytes0/1/128 and all52 update-read failure
prefixes.343 pins are individually decoded/equal across both images. The
combined exact-source4/4 Rust suite covers Core plus the prior projection
kernel and all resulting fields/read/store prefixes.

The initial new update test exposed omitted typed vtable-read boundaries: the
numeric results agreed, but the native read trace included three class reads.
effect-tests-01.log is retained. The kernel now performs typed dispatch reads
at those exact positions; the final replay preserves those failure prefixes.
No fixture output or scope check was weakened.

```powershell
python research/development10/view-environment-projection/native_effect.py
python research/development10/view-environment-projection/audit_effect.py
research/development10/view-environment-projection/isolated-effect.exe --nocapture
```
