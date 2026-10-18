"""Root-authored, task-only preparation. Does not mutate game source or releases."""
import json
import subprocess
from pathlib import Path
import package_animation01 as p
import package_animation01_checks as c


def main():
    prior = p.read(p.ROOT / "backups/source-checkpoint-20261017-animations01/pre-animation-source-index-v1.json")
    command = ["python", str(p.HERE / "package_animation01_source_index.py"), "--output",
               str(p.ROOT / "backups/source-checkpoint-20261017-animations01/pre-animation-source-index-v2.json")]
    for row in prior["files"]:
        command += ["--predecessor", row["source"] + "=" + row["backup"]["path"]]
    subprocess.run(command, cwd=p.ROOT, check=True)
    out = p.HERE / "release-documents01"
    out.mkdir(exist_ok=False)
    extra = """
HOW TO TEST
Run Animation Browser.cmd to open a British infantry HQ death immediately.
Or run AxisAlliesRust.exe, choose Editor, then Animations.
Type HQ, CorpsDeath, Pack, UnPack, or a unit name into Filter original assets.
Click a listed rig: it now loads automatically. Choose a clip and use Play,
Loop and the time slider. Loaded shows the actual original model identity.
Other animation sources lists files whose timelines cannot yet be played.
Reconstruction notes lists unsupported effect controllers on loaded rigs.

This build publishes 786 model rigs and 3,316 model/clip bindings. All 20 HQ
death models, five Corps deaths, 77 building deaths and 119 building
Pack/UnPack source models are represented. The full catalog covers 2,481
original NIF/KF files; 240 animated sources remain explicitly unavailable.
47 nonlinear quaternion clips were excluded rather than played incorrectly.
139 published rigs also contain unsupported controllers; particles, UV,
look-at and other effects can remain incomplete. These counts describe
source/playback coverage, not completion of the game.
"""
    for name, content in [("README.txt", c.README + extra), ("CHECKPOINT.txt", c.CHECKPOINT + extra)]:
        with (out / name).open("x", encoding="utf-8", newline="\n") as f:
            f.write(content)
    p.write_new(out / "launchers.json", [
        {"name": "Start.cmd", "arguments": []},
        {"name": "Animation Browser.cmd", "arguments": ["--animated", "GBRHQInfantryDeath"]},
    ])
    manifest = p.read(p.HERE / "export/cache-v2-final/animation-browser-manifest.json")
    rigs = {r["source"].lower(): r for r in manifest["rigs"]}
    cases = []
    for name, source, times, backend, clip in [
        ("gbr-hq", "Buildings/GBR/GBRHQInfantry/GBRHQInfantryDeath.nif", [0, 2.8, 5.733333587646484], "vulkan", None),
        ("gbr-corps", "Buildings/GBR/GBRCorps/GBRCorpsDeath.nif", [0, 3.2, 6.633333683013916], "vulkan", None),
        ("usa-pack", "Buildings/USA/USACorps/USACorpsPack.nif", [0, 5, 10], "vulkan", None),
        ("usa-infantry", "Units/USA/USAInfantry/USAInfantry.NIF", [0, 0.5], "dx12", "InfantryJog"),
        ("ger-artillery", "Buildings/GER/GERBunkerArtillery/GERBunkerArtillery.nif", [0.3], "dx12", "Recoil1"),
    ]:
        rig = rigs[source.lower()]
        idx = 0 if clip is None else next(i for i, c in enumerate(rig["clips"]) if c["name"] == clip)
        for i, time in enumerate(times):
            cases.append(dict(name=f"{name}-{i}", model=rig["source"], clip_index=idx, time=time, backend=backend))
    p.write_new(out / "render-cases.json", cases)
    print(out)


if __name__ == "__main__":
    main()
