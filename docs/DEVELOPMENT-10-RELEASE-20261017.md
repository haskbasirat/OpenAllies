# Development-10 test release — 2026-10-17

The unfinished Windows x64 preview is in `release/Development-10`.

- EXE: `AxisAlliesRust.exe` — `e84889706013c4a110d65d6e1cb02df0176be05e811980d092b2355b0aa0e161`.
- ZIP: `AxisAlliesRust-Development-10-DIAGNOSTIC.zip` — 102,621,142 bytes, SHA256 `9a20c76ab27565a9ba42f5f3f4d2e89b5c9ffc04585e26b6fda24c3bd1b2e791`.
- Uses the existing original game/reconstruction assets; this is not a standalone whole-game copy.

Run the EXE and choose **Single Player → Custom** for the labelled skirmish setup/startup preview. `Start Asset Viewer.cmd` and `Start Live Preview.cmd` open existing isolated previews. Most new code is startup/simulation plumbing and is not yet available as live battle gameplay.

Accepted preview checks: release build with 2,070 Cargo/crate source fingerprints unchanged; final copied EXE CLI8/8; selected mechanics55/55 and world fragments24/24; structures for80 maps/225,851 placements indexed;2,386 Firefight records restored without record failures; three preview renders using Vulkan/DX12; main menu/setup/name entry; independent review; all ZIP CRC/payload hashes and3,754 previous-release files verified. Test profiles stay under QA. Original/v14 AA.exe hashes match their baselines.

No complete playable battle is accepted. Frozen runtime attempt45 creates HQ and two bunkers, then stops in the post-preparation global Fog adapter requesting staged placement from already-published actors. Headless record restoration is not activation. Actual View readiness, native picking/orders, live scheduled movement/combat/AI/victory, compatible whole saves, multiplayer and campaign still need completion. The packaged GUI startup button itself was not clicked in the accepted UI runs. No reliable overall completion percentage or finish date is established.

Exact scope and untested cases: `release/Development-10/evidence/validation.json`. Durable continuation: `research/development10/ACTIVE-SESSION-20261017.md`, then latest per-agent research findings. Source freeze was for this release; unpublished terrain-ray/timed-list/Engage/orphan/Object successors must be integrated coherently when development resumes. Preserve the sealed release.
