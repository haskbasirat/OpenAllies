# Contributing to OpenAllies

Thank you for helping. OpenAllies recreates **Axis & Allies (2004)** in Rust, and the goal is a game a player can't tell apart from the original. That goal decides almost everything below.

By taking part you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## The ground rules

1. **The original is the specification.** Every rule, number, screen and behaviour comes from the original game: its program (`AA.exe`) and its data files. If you don't know what the original does, find out before you write the code.
2. **Show your evidence.** Cite the native function addresses (for example `0x4CA400`) your change follows, in a short code comment next to the code and in the pull request. For visible behaviour, a screenshot or a short description of the running original helps a lot.
3. **No invented features.** Don't add mechanics, options, buttons or screens the original doesn't have, even good ones. Requests for new features will be closed. Ideas that make the original's limits go away, such as fixed-size pools or 32-bit memory limits, are welcome if gameplay stays the same.
4. **Never commit the original game's files.** That means no art, audio, video, maps, rules data, executables, decompiled or disassembled code, memory dumps, traces or recordings of the original program, and no data extracted or converted from them. Tests and tools that need such files read them from the player's own installed copy, or from local, git-ignored folders made from it.
5. **Keep the game playable.** Each change should leave the game building and running. Large ports can land behind a switch until they work end to end.

The detailed porting rules are in [`docs/NATIVE-PORT-RULES.md`](docs/NATIVE-PORT-RULES.md).

## Getting set up

You need:

- Windows 10 or 11 (x64) with a Vulkan or DirectX 12 driver; other platforms are planned;
- a recent stable [Rust](https://rustup.rs/) toolchain (Rust 2024 edition);
- your own installed copy of Axis & Allies (2004).

Build and run as described in the [README](README.md#building-and-running). The renderer, `crates/aa-bevy`, is a separate Cargo workspace:

```bash
cargo build --release --manifest-path crates/aa-bevy/Cargo.toml
```

Run the game logic tests with `AA_GAME_ROOT` pointing at your installed game:

```bash
cargo test -p aa-play --release
```

Some tests need locally generated research data that isn't in the repository. Run what you can, and say in your pull request which tests you ran.

## Finding something to work on

- [`research/byte-exact/NEXT-ROUND.md`](research/byte-exact/NEXT-ROUND.md) lists the open items of each part of the game.
- [`research/byte-exact/ROADMAP.md`](research/byte-exact/ROADMAP.md) explains how each system becomes byte-for-byte identical to the original.
- Issues labelled as differences from the original are a good start: each describes something that should look or behave like the original and doesn't yet.

If you plan a large change, open an issue first so the work isn't duplicated.

## Writing a change

- **Follow the original's steps.** Use its order of operations, iteration order, random number draws and arithmetic. The `aa-numeric` crate reproduces the original's floating-point behaviour; use it where the result must match bit for bit.
- **Match the surrounding code.** Keep its naming, comment density and idiom. Keep each file's line endings (LF or CRLF): `git diff --stat` and `git diff --stat --ignore-cr-at-eol` should show the same thing.
- **Test against the original where you can.** The exact ports in the `aa-*` crates are checked against fixtures recorded from the original code. New ports should be too.
- **Say what is left.** Note anything that still has to be compared with the running original.

## Pull requests

- Keep one topic per pull request.
- Describe the original behaviour your change matches, with its addresses.
- Add screenshots for visible changes, ideally ours next to the original's.
- List the tests you ran.
- Fill in the pull request template's checklist.

## Reporting bugs and differences

Use the issue templates:

- **Bug report:** a crash, an error, or something that is broken.
- **Difference from the original:** something that looks, sounds or behaves differently from the original game.

Never attach the original game's files to an issue. Screenshots and short descriptions are fine.

## License

OpenAllies is licensed under the [GNU General Public License, version 3 or later](LICENSE). By contributing, you agree that your contributions are licensed under the same terms.
