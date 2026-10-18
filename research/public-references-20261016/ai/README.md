# Public AI architecture reference review

Reviewed October 16, 2026. This task downloaded reference material and inspected
existing Rust/decompiled code. It made no production edits, builds, installs or
native AI execution claims. The actual AI crate is `aa-mind`; there is no
`aa-ai` crate in this workspace.

## Reference identity and notices

[Publisher article](https://ojs.aaai.org/index.php/AIIDE/article/view/18712):
*A Goal-Based Architecture for Opposing Player AI*, Kevin Dill and Denis Papp,
AIIDE 1(1), June 1, 2005, pp. 33-38,
[DOI](https://doi.org/10.1609/aiide.v1i1.18712).
[Original publisher PDF](https://ojs.aaai.org/index.php/AIIDE/article/download/18712/18490/22343).

The printed PDF and PDF author metadata spell the second author's name **Denis
Papp**; the current publisher HTML metadata says **Denis Pap**. Both observations
are retained, without silently treating the typo as a different author. The
six-page PDF is 469,221 bytes, SHA256
`1f77270ec6488b90284792712c5a8ba5e14f945d7441f4ea599e39f1fee88296`.

The PDF has an AAAI 2005 copyright notice and says "All rights reserved."
No Creative Commons or code-reuse license was found on the article or current
[submission page](https://ojs.aaai.org/index.php/AIIDE/about/submissions).
An openly downloadable paper is not an open-source release of TimeGate's AI.
The original notice remains inside the reference PDF. It is not added to any
game package or used as a source-code license.

## Short architecture summary (leads only)

The authors describe a slower strategic goal engine paired with reactive
updates. Goals have eligibility/completion state, base and resource-adjusted
priorities; fuzzy variation and inertia stabilize choices. A cycle prunes goals,
calculates/sorts priorities, assigns resources, optimizes actor allocation, then
emits commands. Commitment can reserve resources, and some assignments remain
locked. Data-driven profiles and egos control adaptation. Influence information
and exploration bonuses deliberately exceed player-visible knowledge. The paper
spans both games and notes campaign limitations. Its approximate intervals and
example strength ratios are investigation leads, not verified AA constants.
[PDF, pp. 33-37](https://ojs.aaai.org/index.php/AIIDE/article/download/18712/18490/22343).

## Mapping to independently recovered native code

The following are **existing local source observations**, not new behavior
implemented from the paper. They make the paper useful as a search/index aid.
The native addresses, precise branches, reads/stores, data and prior oracle
evidence remain authoritative if prose is broader or the two games differ.

| Topic / paper pages | Existing Rust and native addresses | Remaining concrete verification/integration boundary |
| --- | --- | --- |
| Strategic/reactive layers, 33-35 | `crates/aa-mind/src/sai/manager.rs`: constructor `7A0AA0`, Think `7A0080`, goal-engine continuation `79F660`, RAI `79EF60`, MGAI `79E830`, yield `79DF70`. `sai/mod.rs` documents the split. `007a0080.c` was inspected. | Native deadlines and continuation services must be supplied. The saved-load inactive-SAI capability deliberately cannot stand in for an active initialized player. Do not hardcode the paper's approximate cycle times. |
| Priorities and inertia, 34 | `sai/goal_priority.rs`: affordability `7A3860`, finalization `7A5200`, per-type ego records and SAI RNG draws. `007a5200.c` was inspected. `sai/base_priority*.rs` contains actual class-specific formulas. | Current source/profile/goal ownership and reached child services are required. Native binary32 stores, explicit precision and the existing separate SAI RNG take precedence over a generic utility implementation. |
| Goal lifecycle, 34 | `sai/engine.rs`: `7A7700` reevaluation, `7A93B0` selection, native comparator `7A30B0` and CRT qsort; both exported decompiles were inspected. The controller has 23 authored goal-type buckets. | Real goal construction/destruction, current actor/goal lists, costs, and yielding remain required host ownership. The paper's simplified goal list must not replace the actual native enum. |
| Resource/actor allocation, 34-35 | `sai/assign.rs`: evaluate `7A9150`, assign `7A7E30`, gather `7A7EB0`, lower/higher-goal transfers `7A3E10/7A3F30`, trade `7A50E0`, swap `7A33E0`. `sai/goal_base.rs`: `7A3B00`, `7A6930/7A6A90`, travel weighting `4A7130`. | Caller-owned native-order actor pools, current goals, class virtuals and route services are not replaced by paper pseudocode. `sai/bridge.rs` explicitly refuses linked actor-list conversion; list nodes cannot be invented. |
| Egos/profiles, 35-36 | `sai/profile.rs`, `ego.rs`, `ego_lists.rs`, `rules_data.rs`: profile `4AE3E0`, ego `4B0560`, filters `4AD980`, per-goal priorities `4AF500`, actor preferences and data references. Existing `research/phase8-ai` records shipped profile/ego fixtures. | Resolve the real inherited TGI/provenance/profile state and delayed references, preserving loader errors and source order. No new profile defaults or balance tuning were introduced by this review. |
| Influence/exploration, 36-37 | `sai/map_analysis.rs`: `7AC0D0`, region propagation `7AA620`, learning `7AE350`, static/mobile spreads `7ABA70/7AB940`; `base_priority_misc.rs` contains native exploration formulas. | Bind real topology, kingdom slots, actor CV, visibility and source ego fields. The paper alone does not authorize adding omniscience or changing fog behavior. Use native loops and source fields to determine what AA actually reads. |
| Commands and startup | `sai/command.rs`: `7AF220` emits ordinary order requests. `crates/aa-navigation/src/startup.rs`: `7A0D40` outer startup and mandatory world services. `aa-runtime/src/restore_lifecycle.rs` explicitly rejects the active transition. | Real world startup, ownership and ordinary command/admin transport must run before strategic decisions can affect a live match. |

Existing bounded proof inventory: `research/phase8-ai/README.txt`,
`inventory.json`, `findings.json` and `native-*.json`. Those proofs were **read,
not reexecuted** in this public-reference review. Their previous case counts
must not be presented as newly run tests. `aa-mind` code contains substantial
native AI already; this paper does not provide the missing initialized-world
adapter, solve actor combat-source completion, or make a battle playable.

## Practical next use

After the first initialized world and ordered simulation exist, use the paper's
vocabulary to organize a native AI trace: real profile/ego choice, goal list
status, priority stores, resource reservations, actor moves between pools,
yield/resume points and resulting orders. Compare identical native inputs at
the correct precision. Distinguish SAI's host-timed RNG from deterministic unit
Mind/world RNG. Preserve campaign-specific native paths instead of extrapolating
skirmish heuristics to missions. No change is justified merely because the
paper describes a cleaner or more general architecture.

## Files and reproduction

Downloads and notices:
`E:/Games/Axis and Alies/reconstruction/references/public-rewrite-20261016/ai/`.
The downloaded publisher HTML is the server's original gzip-encoded response;
`publisher-article-decoded.html` is a separately preserved decoded derivative.
The PDF was checked with `pdfinfo` and its first page rendered/visually inspected
with Poppler (`paper-page1.png` here). Poppler reported missing Symbol and
ArialUnicode display-font warnings, but the inspected author/title/notice text
is legible. No PDF was edited or executed; it reports no PDF JavaScript.

`reference-manifest.json` pins downloads, local source/decompile leads, preserved
native images/release artifacts and this report. The companion
`FOV-REVIEW.md` covers the separately downloaded public camera fix. The review
script `review_native_fov.py` only reads PE bytes; ordinary replay verifies its
frozen static result without writing files.
