"""Record observations directly inspected by Root, separate from screenshot existence."""
import package_hotfix01 as p

root=p.ROOT/"qa-output/development10/hotfix01-gui"
results={name:p.read(root/name/"result.json") for name in ("berlin","firefight")}
digest=p.sha(p.RELEASE/p.EXE)
for result in results.values():
    assert result["executable_sha256"]==digest and not result["timed_out"] and result["exit_code"]==0
evidence=[]
for name in ("berlin","firefight"):
    for file in ("before-start.png","after-001s.png","after-030s.png","after-125s.png","after-155s.png","result.json","stdout.txt","stderr.txt","script.json"):
        evidence.append(p.record(root/name/file))
for file in ("after-010s.png","menu-during-loading.png","resumed-same-loading.png","after-220s.png"):
    evidence.append(p.record(root/"firefight"/file))
p.write_new(p.ROOT/"qa-output/development10/hotfix01-acceptance/gui-review.json",dict(
    passed=True,executable_sha256=digest,
    scope="Root directly inspected actual setup, pending and failure screenshots through the application's real GUI input hook. Success means responsive diagnostic/error UI only.",
    maps={
        "berlin":dict(map_id="(2v2)Berlin",actual_start_clicked=True,
            observations=["Before-start image shows Fall of Berlin selected with Regression Tester and enabled Test skirmish startup.",
                "Pending images show actual timer progression1/10/30/125/155s and source-preparation to selected-world status transition.",
                "GUI serviced its next scripted screenshots beyond the sealed predecessor's25s starvation window.",
                "Still loading at155s. Script requested close afterward; process exited0 at187.875s, with a native-worker shutdown wait. No completed startup or native error observed before close."],
            final_observed_startup="pending at155s before scripted close",process_result=results["berlin"]),
        "firefight":dict(map_id="DeveloperMaps/(3v3)Firefight",actual_start_clicked=True,
            observations=["Before-start image shows (3v3)Firefight selected and actual Test skirmish startup button.",
                "At10s, Loading selected world. Actual Main menu click opened the menu, and Single Player > Custom returned to same Firefight attempt at11s; no second startup was queued.",
                "Loading continued to91s. By125s the live GUI displays Skirmish could not start with the genuine WorldStart PostLoad classifier error.",
                "Failure UI includes Change setup, Main menu and Discard stopped attempt. Discard/retry was not clicked in this test."],
            observed_error='economy laboratory: World("post-load world operation failed: a fresh object exists before the PostLoad pass (record sections create none)")',
            final_observed_startup="genuine unsupported WorldStart classification",process_result=results["firefight"]),
    },evidence_files=evidence,playable_battle=False,full_game_parity=False,
    limitations=["No map completed a playable battle; native failure stays a failure.",
        "Berlin was bounded to155s of live capture and then closed while its native worker was pending; no Berlin completion/error acceptance.",
        "Native resource/simulation destruction still joins. A close request can wait for native cleanup, as observed on Berlin.",
        "No hard frame latency, faster map loading or general performance claim. CLI/static preview checks overlapped part of Firefight run.",
        "Own isolated APPDATA/LOCALAPPDATA used; scripts inject only this application's egui events, without sending desktop-wide input.",
        "No forced readiness/native activation, fake simulation, or original-engine fallback.",
        "The GUI helper was extended only for Firefight while Berlin's earlier JSON script was already loaded; each actual script and process result is retained."]
))
print("GUI semantic review recorded")
