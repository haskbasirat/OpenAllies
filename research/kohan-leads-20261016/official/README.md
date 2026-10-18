# Kohan II official source-access lead, checked 2026-10-16 EDT

The lead is real. The game-specific official announcement is [Kohan II: Kings of War Multiplayer & Modernization Update is Live!](https://store.steampowered.com/news/app/97130/view/672876020537229477). Its publisher-side Steam event API identifies app **97130**, event **672876020537229477**, announcement body **672876020537229478**, and post timestamp **1785285026**: **2026-07-29 00:30:26 UTC / July 28, 2026 20:30:26 EDT**. The [community announcement](https://steamcommunity.com/games/97130/announcements/detail/672876020537229478) is the same post. The broad Steam feed supplied by the user also contained it, but is not a game-filtered permalink.

The announcement says **“Rebuilt from the original source code”**. It advertises Steamworks replacing GameSpy, internet room browsing and invitations, corrected Windows scaling at 125%–250%, and widescreen/4K rendering. These are the developers' published claims, not features tested by this research. This supports pursuing an authorized source or documentation lead. It does **not** offer a public engine repository, source archive, SDK, or reuse license.

The July beta's default-branch description is historical: the official [August 25 event](https://store.steampowered.com/news/app/97130/view/674003823632582284) says 1.3.68 became default and the old release remained on a separate branch. The [September 3 event](https://store.steampowered.com/news/app/97130/view/705530288588980948) describes a further cross-hardware multiplayer desynchronization correction involving fixed trigonometric tables and a memory fix. This is a useful determinism lead, not an Axis & Allies implementation specification.

## Attribution and official sites

The [Steam app metadata](https://store.steampowered.com/api/appdetails?appids=97130&l=english) lists **Lootmax Games** and **TimeGate Studios** as developers, **Big Bad Wolf, Inc.** as publisher, and [lootmaxgames.com](https://www.lootmaxgames.com/) as the product website. The legal notice names TimeGate's copyright and Augustus Intellectual Properties, LLC's Kohan II trademark. These fields do not independently establish ownership of Axis & Allies source or authority to license its middleware.

The [Lootmax home](https://www.lootmaxgames.com/), [about](https://www.lootmaxgames.com/about), [news](https://www.lootmaxgames.com/news), and [contact](https://www.lootmaxgames.com/contact) pages were downloaded as documentation only. No public engine/SDK download or license was found on those reviewed pages. The contact page provides a general contact address; **UNSENT-SOURCE-REQUEST.txt is only a local draft**. No contact form, email, account, or game download was used. A separate Big Bad Wolf, Inc. website was not verified; similarly named companies were not assumed to be this publisher.

## Public source search boundary

The bounded searches covered the official pages above, game/source/SDK searches for Kohan II and Axis & Allies (2004), and public GitHub repository searches for `kohan timegate`, `"lootmax games"`, and `axis allies rts`. No authorized original TimeGate game-source or public SDK release was located. This is a statement about those checks, not proof that no such release exists.

The GitHub search exposed an additional [Axis & Allies fan patch project](https://github.com/ibm2187/axis-and-allies-2004-remastered), pinned for this review at **7fd8ebe9a838e634c9886c4778fb22c1178570ab**. Its README describes installer/patch scripts that require the original game; it is not the original engine source. Its code/documentation have an MIT license; the README separates game-derived HD art and third-party components from that grant. Only commit metadata, README, and LICENSE were downloaded. No scripts, binaries, or art were executed, integrated, or tested. Its technical claims remain review leads.

## Limits and next useful evidence

Source access by a modernization team does not grant public access or permission for this rewrite. Cross-game compatibility of TimeGate-specific actor, simulation, animation, material, save, or network code remains unverified. Matching a Gamebryo family or release year cannot establish an ABI or behavior match. No original Kohan II executable was downloaded or analyzed here.

The useful next step would be an explicitly authorized documentation/source inquiry, followed by checking any supplied rights and actual version boundaries. This research does not replace the existing AA.exe native proofs or justify importing later SDK implementation. It changes neither production code nor Dev09/release packages and provides no completed-game percentage or ETA.

## Evidence layout

- Raw references: `E:\Games\Axis and Alies\reconstruction\references\kohan-leads-20261016\official`.
- Every downloaded payload has a `.source.json` with requested/final URL, retrieval time, byte count, and SHA-256. `download-manifest.json` verifies and indexes both payloads and sidecars.
- `kohan2-announcement.html` is an exploratory redirect to the announcement list caused by using a different ISteamNews identifier. It is deliberately retained and **not** treated as the exact post. The two `kohan2-exact-*` captures and events API establish the canonical IDs.
- `findings.json`, `reference-manifest.json`, and the local unsent request provide the handoff. No missing search result is represented as proof of public-source unavailability.
