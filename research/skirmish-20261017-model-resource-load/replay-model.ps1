$ErrorActionPreference='Stop'
$taskRoot=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
Push-Location $taskRoot
try{
 $env:CARGO_MANIFEST_DIR=Join-Path $taskRoot 'crates/aa-runtime'
 $json=(Resolve-Path target/s-borrowed-leader/release/deps/libserde_json-3f0b3bb60c709793.rlib).Path
 $numeric=(Resolve-Path target/s-borrowed-leader/release/deps/libaa_numeric-55927aa222c0a1ba.rlib).Path
 & rustc --edition=2024 --test research/skirmish-20261017-model-resource-load/replay_model.rs -L dependency=target/s-borrowed-leader/release/deps --extern "serde_json=$json" --extern "aa_numeric=$numeric" -o target/s-borrowed-leader/model-resource-tests.exe
 if($LASTEXITCODE-ne0){throw 'Exact model source compilation failed'}
 & target/s-borrowed-leader/model-resource-tests.exe --nocapture
 if($LASTEXITCODE-ne0){throw 'Exact model source replay failed'}
}finally{Pop-Location}
