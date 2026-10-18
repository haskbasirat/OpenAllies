# Battleborn reference review — 2026-10-16

The reviewed project is a **Kohan II gameplay mod and its development tools**,
not the original Kohan II or Gamebryo engine source. Its README says the mod is
built against the player's retail `Data.rwd` and runs on a separate Modern
Rebuild executable. That executable is not supplied here. The tree at the
pinned commit has 170 files and no `.c`, `.cpp`, `.h`, `.hpp`, `.rs`, `.asm`,
`.dll`, or `.exe` files. This extension census does not substitute for reading
every file; the selected project documents and tools establish the mod scope.

Primary reference: [Ensrick/kohan2-battleborn at the reviewed commit](https://github.com/Ensrick/kohan2-battleborn/tree/7f911fa8da5d53c1b7d0264f1a03d0cfab88f250).

## Pinned reference material

- Commit: `7f911fa8da5d53c1b7d0264f1a03d0cfab88f250`.
- Commit date: `2026-10-12T00:07:05Z`; version metadata: `0.2.4-dev`.
- Tree: `f486540655236cb190d3ddaf4c809db92ee2cca1`.
- Saved directory: `reconstruction/references/kohan-leads-20261016/battleborn`.
- Download: 13 selected UTF-8 text files, 127,809 bytes, plus public GitHub
  metadata/tree/commit JSON. Every text file passed size, Git blob SHA-1 and
  local SHA-256 verification. The manifest pins each immutable raw URL.
- Nothing from the repository was executed, installed or integrated. No
  archive, DLL, executable, map, patch, texture or original game asset was
  downloaded. Only our independent fetcher ran.

The actual [LICENSE](https://github.com/Ensrick/kohan2-battleborn/blob/7f911fa8da5d53c1b7d0264f1a03d0cfab88f250/LICENSE)
contains the MIT grant for Ensrick's own repository work and a separate notice
excluding TimeGate game content from that grant. GitHub's API labels this
`NOASSERTION`; the saved license text is the basis for the manual reading.
There is no verified permission here to redistribute TimeGate or Gamebryo
implementation/assets. The upstream contribution instructions are reference
data and do not change this rewrite's instructions.

## Useful, qualified leads

| Lead | What the reference actually provides | Relevance to this AA rewrite |
| --- | --- | --- |
| Ability animation references | `tools/animations.py` checks inherited ability `animation1`–`animation5` and `idle_animation` names against actor animation IDs. Its introductory comments connect missing animation references with the author's K2 startup logs. | **Independently confirmed in AA:** original `004BF990` contains the same missing idle/ability animation diagnostics. It also checks equal animation durations, which the mod validator does not establish. Existing `aa-rules/src/abilities.rs` already extracts these fields. Future diagnostics can report the actual AA failure path without borrowing K2 defaults. |
| Strict template fields and child placement | `tools/template_fields.py` describes inherited required declarations, unknown fields, and brace-less node placement. `CHANGELOG.md` records a `can_raze` assignment accidentally belonging to the preceding Upgrade node. | A useful regression-test theme for source loading. AA already has a native-proved parser and historical template binding; the mod's regex checker is not an authoritative replacement. No parser change was made. |
| Load-time reference ordering | `tools/load_order.py` separates immediate references from allowed forward references and uses retail data to reduce diagnostic noise. | Useful triage idea only. Its comments expressly say the exact K2 sort rule is not pinned and try two case orders. AA's existing native loader has proven ordering and must remain authoritative. Do not transplant the heuristic or suppress real AA errors as baseline noise. |
| Resource depot/startup clues | README and `tools/bb.py` describe K2 startup script enumeration, later-depot priority, `addmod`, and connection checksums. | K2 author claims, with no addresses or native fixtures in these selected files. They concern resource mounting, not completion of AA Actor Init/PostCreate. Verify against AA before using any corresponding multiplayer or startup behavior. |
| RWD layout | `tools/rwd.py` documents TimeGate TGCK sections, a 96-byte section record and relative file offsets. | Corroborates the broad archive family already supported by `aa-formats/src/rwd.rs`. Its K2-specific Header game ID/stamp and packer assumptions must not overwrite AA metadata. No new archive-format implementation is needed from this review. |

## What this does not resolve

- No `4BBF90`, `4BB590`, combat-value formula, Gamebryo implementation, NIF
  decoder, skeletal skinning, material shader, or billboard/controller code was
  found in the 13 selected documents/tools. This is a scoped negative finding,
  not a claim that every patch in the repository was examined.
- The mod's settlement costs, production, garrisons, health, unit limits and
  economic adjustments are deliberately changed K2 gameplay. They provide no
  authority for AA combat-value calculations or original gameplay rules.
- The checklist includes unverified assumptions and old design inventory;
  the later changelog's reported main-menu success is the author's K2 mod
  result. We did not run or independently reproduce it and it is not proof of
  a completed battle or AA compatibility.
- There is no native evidence here that removes AA's current Actor startup,
  spatial placement, state-effect, scheduler, or live-world dependencies.

The native AA animation observation was sent directly to the agent working on
settlement/combat-value completion. No gameplay rule, implementation or mod
balance value was copied into the Rust rewrite. The ongoing native AA evidence
remains the source of truth.

The AA `004BF990` observation above is static inspection of the existing AA
export, not a new native execution or full loader parity test. Existing
`research/phase7-definitions` evidence also keeps full `004BF990` completion
outside its isolated fragment claims.

## Files inspected

`README.md`, `LICENSE`, `CONTRIBUTING.md`, `CHECKLIST.md`, `CHANGELOG.md`,
`mod.json`, and `tools/{animations,load_order,template_fields,validate,rwd,bbpatch,bb}.py`.
Downloaded copies preserve their original bytes and notices. The adjacent
`download-manifest.json`, `commit.json`, `tree.json` and
`repository-metadata.json` record provenance; the authored review and verification
record live separately under this rewrite's `research/kohan-leads-20261016/battleborn`.
