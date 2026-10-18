# Rewrite status and intended finished game

The next useful milestone is a complete playable skirmish. Development-09 is
an asset preview and native map-loading diagnostic, not that milestone.

The slowest part is connecting the original game's complete runtime lifecycle:
load every saved object, initialize its real components, attach it to the
world, run native state/formation/economy/AI callbacks, and start deterministic
simulation. Many individual functions are ported and tested, but their required
world services and owners are not all connected. A successful isolated test
does not stand in for those connections.

The original TimeGate game source is unavailable. Decompiled machine code,
original data and differential tests provide the behavior evidence. The
matching Gamebryo headers/docs help identify graphics types and interfaces;
they do not provide the game's combat, AI, object lifecycle or session logic.
Agents can work on independent ports and reviews, while dependencies still
have to integrate in the correct order.

## Schedule

There is no reliable delivery date for either the first complete skirmish or
the full rewrite yet. As a planning assessment, the faithful full game is a
multi-month undertaking, not something that should be promised in a few more
sessions. That broad assessment is not a measured forecast or a delivery
commitment; remaining subsystem integration can change it substantially.

A useful estimate needs an initialized real world and sustained ordered ticks,
then a complete match. Those expose the actual reachable gaps far better than
function counts, test counts or lines of Rust. Current evidence cannot justify
an overall completion percentage or a calendar deadline.

The measurable order is:

1. One original map completes real initialization, conversions, PostCreate,
   PostLoad, mission startup and world registration, with no fake callbacks.
2. That world runs ordered ticks with real commands, component scheduling,
   clock/RNG and subsystem services.
3. A human-versus-AI skirmish completes: movement, combat, construction,
   recruitment, resources, fog, victory/defeat, save/reload and restart.
4. Multiplayer passes deterministic local tests, then host/join/synchronization
   and save transfers on separate PCs and separate internet connections.
5. Campaign missions, scripted objectives and WWII/metagame progression pass.

Skirmish remains the first priority, multiplayer second, campaign last. The
earlier answer that no second PC/different connection is available still
limits real internet acceptance; local tests cannot substitute for it.

## What the finished rewrite is intended to include

This is the target scope, not a list of currently finished features.

* A playable **64-bit Windows game** with an EXE and a complete documented
  package of required data, replacing the development-only viewer/diagnostics.
* Original nations, units, buildings, maps, controls, combat, supply, morale,
  pathfinding, formations, economy, recruitment and AI behavior, preserving the
  chosen modified-game baseline and gameplay rules.
* The accepted v14 modifications: unit/building capacity settings of 100,
  additional 768/1024 map sizes and expanded 16-side/player configuration,
  with the existing costs and attachment rules retained. These settings are
  targets to preserve, not proof of a tested 16-person multiplayer session.
* Complete single-player skirmish and AI opponents, then LAN/internet
  multiplayer with working lobbies, hosting/joining, command synchronization
  and save transfers, followed by campaign and WWII/metagame play.
* Correct original models, textures, transparency, effects, skeletal/building
  animations, interface, audio and music, validated against the original.
  The rejected HD texture pack stays retired; original artwork is the baseline.
* Fullscreen/windowed controls, high-DPI layouts and mouse alignment, the
  requested WASD camera controls and extended zoom, integrated into gameplay.
* Removal of the old 32-bit process address-space ceiling, growable engine
  storage and configurable capacities where appropriate. Gameplay rules and
  compatible file/network layouts remain deliberate constraints. Physical
  RAM/VRAM, disk, CPU/GPU and network limits still exist.
* Multithreading for independent work such as asset loading and rendering
  preparation; authoritative simulation order and RNG remain deterministic.
  Actual loading speed, frame pacing and multiplayer smoothness require
  benchmarks after the live systems work. Rust alone guarantees no FPS gain.
* Compatible original saves where native-format coverage is proven, useful
  diagnostics, maintainable Rust source, reproducible builds and documented
  settings so future fixes and modifications are easier.

Free internet service/relay availability and automatic operation without port
forwarding are separate networking requirements. Rewriting the engine does
not itself supply unlimited public relay capacity or an always-online server.
No paid hosting/subscription is planned; any service choice still requires
actual connection testing and a clear account of external dependencies.

For the exact source work and verification from this continuation, read the
dated skirmish continuation referenced at the top of `HANDOFF.txt`. The
startup audit is in
`research/skirmish-20261016-startup-critical-path/README.md`.
