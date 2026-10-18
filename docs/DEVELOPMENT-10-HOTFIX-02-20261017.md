# Development-10 Hotfix-02 (2026-10-17)

DIAGNOSTIC / UNFINISHED. BATTLE STATUS: UNAVAILABLE.

## Delivery

- Folder: `release/Development-10-Hotfix-02`.
- EXE SHA-256: `48ed9a7f9f4f4112f851b87daa4e041004d2dab49ba6813d5e6e31a230148e51` (34702848 bytes).
- ZIP: `AxisAlliesRust-Development-10-Hotfix-02-DIAGNOSTIC.zip` (120423552 bytes).
- ZIP SHA-256: `a8c8221a915208e1622a35c4ec633ac205e9f7724b77e52f1717a37aef2a66d2`.
- All 10,775 previous Dev08/09/10/HF01 files verify unchanged.

Payload hashes, exact ZIP membership and CRC are verified. Original
Data.rwd/reconstruction assets, x64 MSVC runtime and compatible graphics
drivers remain required. This package is not standalone or a complete game.

## Fixes and evidence

The reported visibility-memory timestamp failure now receives the actual
retained Staged.clock owner explicitly; no copied/default time substitutes
for GWorld+E0. WorldStart uses the proven Source::New class3 Actor vtable+B4
result after actual identity/storage/source checks, preserving saved/Group8
behavior. The final candidate also forwards the captured MapResource load argument
through activation and lends the actual retained current MapResource mode
through explicit FX callback context. Targeted reviews in final validation define the exact
accepted domains. Prior native stores and partial failures are preserved.
Candidate3 corrects the client Loading resource-name literal to the original
`loading_resources` value; no alternate asset lookup or readiness fallback
is introduced.

Unsealed candidate1 is retained in
`backups/dev10-hotfix02-unsealed-candidate01`. Candidate2 GUI observations
and its preserved unsealed candidate are also historical. Neither earlier
GUI is relabelled as candidate3 acceptance; final observations below come
from fresh GUI03 runs. Only the explicitly identified identical QA artifact
reuses its original strict-map results.

Default-profile GUI build: 288.447s, 2,076 unchanged
Cargo/crate source pins. Every one of 29 changed existing files
has an exact sealed-HF01 predecessor; 1 new source file(s) are
listed separately in `backups/source-checkpoint-20261017-hotfix02/pre-hotfix-source-index-v4.json`. These fingerprints do not attest
every external compiler include.

## Actual observations

Separate strict QA executable: `509da2d70b46f439b5690b56bd6286f7809e42a3bbeb7e25d7a992a40c7b9a5c`.
The Berlin/Firefight executions below are the original candidate2 results
at `qa-output/development10/hotfix02-native-startup02/`. They were not rerun or relabelled as candidate3.
Fresh QA03 compilation produced the exact same executable. Its current
Cargo/crate fingerprint matches GUI03; the only tracked change from the
original strict build is `crates/aa-client/src/battle_notifications.rs`,
which the QA test does not import. The dependency manifest and explicit
review are pinned in validation. This does not attest every external
compiler input.

| Map | Bounded result | Elapsed |
|---|---|---|
| berlin | Original candidate2 strict startup; identical QA artifact reused | 306.875 s |
| firefight | Original candidate2 strict startup; identical QA artifact reused | 130.828 s |

StartedMission is startup scope. It does not establish frontend activation,
native View/task readiness, first battle frame, native controls or scheduled
movement/combat/AI/victory. These timings are not a performance guarantee.

Final packaged-EXE GUI observations, from the sealed semantic review:

```json
[
  "Root directly viewed Berlin beforeStart,190s and240s; source hash confirms300s image identical to240s. Fall of Berlin/(2v2)Berlin selection verified.",
  "Root directly viewed Firefight beforeStart, menu-during-loading, resumed-same-loading,125s and240s. Requested DeveloperMaps/(3v3)Firefight verified.",
  "Both packaged actual Start actions clear the previous clock, fresh Actor WorldStart, SAI captured-mode and incorrect status-key errors, then reach the distinct View refusals above.",
  "Firefight menu round trip retains the existing loading attempt at elapsed11 seconds. Both owned GUI processes close with exit0 and no timeout.",
  "Firefight stderr also records native404FC0 level10 missing Texture EnvironmentMap.tga; no complete graphics/texture acceptance."
]
```

Observed remaining GUI blocker: Berlin: actual source Aspect allocation table not prepared; Firefight: terrain palette identity varies within one loaded palette slot

- Playable battle unavailable: no battle frame, native controls, scheduled movement/combat/AI or victory acceptance.
- The pending source Aspect preparation and terrain palette identity cases remain unresolved; no guard was weakened to pass them.
- Final GUI timings are bounded diagnostic observations, not loading/performance benchmarks.
- Only these two menu startup paths were exercised. Full saves, multiplayer and campaign were not tested.
- Existing installation assets and runtime dependencies remain required; not standalone.

The exact client passed the 8 recorded CLI checks.
Static Vulkan/DX12 preview observations retain all visual-review warnings;
they are not native whole-Model/View or gameplay acceptance. Detailed checks,
observed limits and untested domains are in `evidence/validation.json`.

## Continuation

Read `docs/NATIVE-PORT-RULES.md` first, then
`research/development10/ACTIVE-SESSION-20261017.md` and the latest actual
observations. Continue from the explicit GUI blocker above, preserving
same-owner lifetimes, native callback order and failure prefixes. Do not
reset a failed graph, treat a missing child as success or use a viewer as
a battle. Finish actual World/Session/View activation, native picking and
orders, scheduled movement, combat, AI and victory before accepting a match.

- Final evidence: `qa-output/development10/hotfix02-acceptance03/validation.json` and `gui-review.json`.
- GUI artifacts: `qa-output/development10/hotfix02-gui03/`.
- Separate strict startup: `qa-output/development10/hotfix02-native-startup02/`.
- Current no-run QA03 compilation: `qa-output/development10/hotfix02-native-startup03/build-record.json`.
- Default-profile build: `qa-output/development10/hotfix02-build03/build-record.json`.
- Source-change index: `backups/source-checkpoint-20261017-hotfix02/pre-hotfix-source-index-v4.json`.

No whole-game saves/campaign, real second-PC/internet multiplayer, clean-PC
standalone deployment, 150% laptop DPI, every-map or performance acceptance
is inferred. Native cleanup may still wait; no interruptible-loader claim.
Skirmish and multiplayer remain ahead of campaign. No defensible overall
completion percentage or delivery date has been established.

This writer changes only its new result/document and continuation journal.
Original game/profile data and all sealed releases remain untouched.
