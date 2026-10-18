# Public references added to the rewrite — October 16, 2026

The supplied references were downloaded as selected source, notices and
documentation into
`E:\Games\Axis and Alies\reconstruction\references\public-rewrite-20261016`.
The per-area download manifests record exact revisions, URLs, sizes and hashes.
No downloaded executables or DLLs were run. No Gamebryo vendor source or other
external implementation was copied into the Rust runtime in this continuation.

| Reference | Useful result and limits |
|---|---|
| [NifSkope](https://github.com/niftools/nifskope/tree/3a85ac55e65cc60abc3434cc4aaca2a5cc712eef) and [Niflib](https://github.com/niftools/niflib/tree/e29166183e23c3477fb7e08d4523dd111440947e) | Checked NIF texture descriptor fields against original 10.1 files. The converter and `aa-assets::Material` now retain optional authored clamp/filter/UV words. Existing converted files, geometry, preview UV selection and GPU samplers are unchanged. Root BSD-style notices and selected file headers are preserved; bundled third-party components have separate notices. |
| [GameSpy SDK](https://github.com/nitrocaster/GameSpy/tree/d1deb2d1a951cf77933dda040b8d311cc09815a7) | Specific GP, peerchat, server-browsing, QR, NAT and HTTP files are useful protocol leads. Root BSD-3-Clause notice retained. The selected Common/Peer code is from 2007; AA's exact linked SDK version and compatibility remain unverified. It does not replace TimeGate's match, save-transfer or simulation protocols. |
| [OpenSpy core](https://github.com/openspy/openspy-core/tree/221c6a7931914362a04ffe6f374957eff2fe1bac) and [issue 33](https://github.com/openspy/openspy-core/issues/33) | Compared server protocol code with existing native AA evidence. The official-map gate is an HTTP content request, distinct from master-server map-loop requests. Fresh read-only map/rank URL checks returned 404. The exact content, rank and rated-stat requirements are recorded in the networking report. No project-wide implementation reuse grant was verified; no OpenSpy code was integrated. |
| [AA FOV fix](https://github.com/alphayellow1/AlphaYellowWidescreenFixes/tree/99a5f300685bb9a505eea7cf12e7dad3ee61875f/source/fixes/Axis%26AlliesFOVFix) | MIT source supplies concrete executable signatures and camera/aspect leads. Static review found an inverted signature-validity guard in this revision, so it was not adopted or executed. Signature matches do not prove hook compatibility or rendered FOV parity. |
| [Dill and Papp, 2005 AI paper](https://ojs.aaai.org/index.php/AIIDE/article/view/18712) | The authors describe the goal-based AI used in AA and Kohan II. Goal, ego and assignment concepts map to native-backed `aa-mind::sai` code. The paper guides discovery and cross-checks names; its prose does not supply the exact game's decisions, timing or source code. The publisher PDF copyright notice is retained; an open code-reuse license was not established. |

Detailed evidence and independent graphics review:

- `research/public-references-20261016/graphics/README.md`
- `research/public-references-20261016/graphics/independent-review.json`
- `research/public-references-20261016/networking/README.md`
- `research/public-references-20261016/ai/README.md`
- `research/public-references-20261016/ai/FOV-REVIEW.md`

The material-retention change supersedes the earlier reference review's schema
gap only for newly converted payloads. It does not fix the still-unconsumed
sampler state, billboards, full animation evaluation or all transparency issues.

These references do not supply TimeGate's original source or a completed battle
world. Native machine-code proofs and actual integration remain required.
Skirmish remains first, multiplayer second, campaign last. Internet acceptance
still needs two Windows PCs on separate connections; the existing test setup
does not provide them. No live login, advertisement or multiplayer match was
performed during this reference review.
