param([ValidateSet('actors','economy','mapworld','runtime','integration-remaining','element','compile','compile-final')][string]$Phase='runtime')
$ErrorActionPreference='Stop'
$taskRoot=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
Set-Location -LiteralPath $taskRoot
$env:AA_GAME_ROOT=Split-Path $taskRoot -Parent
$env:CARGO_TARGET_DIR=Join-Path $taskRoot 'target/s-owner-recruitment'
$taskOutput=Join-Path $taskRoot 'qa-output/economy-lifecycle-20261016'
function Invoke-ScopedCargo([string]$Name,[string[]]$Arguments) {
    $taskLog=Join-Path $taskOutput ($Name+'.log')
    if(Test-Path -LiteralPath $taskLog){throw "Validation log already exists: $taskLog"}
    & cargo @Arguments *> $taskLog
    $taskCode=$LASTEXITCODE
    Get-Content -LiteralPath $taskLog | Select-Object -Last 22
    if($taskCode -ne 0){throw "Scoped cargo failed: $Name (exit $taskCode); see $taskLog"}
}
switch($Phase){
    'actors' {
        Invoke-ScopedCargo 'accepted-actors' @('test','-p','aa-actors','--release','--locked','--offline','-j','4',
            '--test','economy_values_native','--test','economy_lazy_backref','--','--include-ignored')
    }
    'economy' {
        Invoke-ScopedCargo 'accepted-economy' @('test','-p','aa-economy','--release','--locked','--offline','-j','4',
            '--test','native_borrowed_flows','--test','kingdom_init','--','--include-ignored')
    }
    'mapworld' {
        Invoke-ScopedCargo 'accepted-mapworld' @('test','-p','aa-mapworld','--release','--locked','--offline','-j','4',
            '--test','native_team_init','--','--include-ignored')
    }
    'runtime' {
        Invoke-ScopedCargo 'accepted-runtime-lib' @('test','-p','aa-runtime','--lib','--release','--locked','--offline','-j','4',
            '--','--include-ignored')
        Invoke-ScopedCargo 'accepted-runtime-integration' @('test','-p','aa-runtime','--release','--locked','--offline','-j','4',
            '--test','managed_restore','--test','economy_values_world','--test','economy_refresh_native',
            '--test','saved_kingdom_flows','--test','restore_lifecycle_native','--test','map_replacement_queue',
            '--test','skirmish_restore','--test','element_attach_restore','--test','organization_layout_bindings',
            '--test','skirmish_start','--','--include-ignored')
    }
    'compile' {
        $env:CARGO_TARGET_DIR=Join-Path $taskRoot 'target/s-definitions'
        Invoke-ScopedCargo 'accepted-workspace-check' @('check','--workspace','--all-targets','--release','--locked','--offline','-j','4')
    }
    'compile-final' {
        $env:CARGO_TARGET_DIR=Join-Path $taskRoot 'target/s-definitions'
        Invoke-ScopedCargo 'accepted-workspace-final' @('check','--workspace','--all-targets','--release','--locked','--offline','-j','4')
    }
    'integration-remaining' {
        Invoke-ScopedCargo 'accepted-runtime-remaining' @('test','-p','aa-runtime','--release','--locked','--offline','-j','4',
            '--test','managed_restore','--test','saved_kingdom_flows','--test','restore_lifecycle_native',
            '--test','map_replacement_queue','--test','skirmish_restore','--test','organization_layout_bindings',
            '--test','skirmish_start','--','--include-ignored')
    }
    'element' {
        Invoke-ScopedCargo 'accepted-element-attachment' @('test','-p','aa-runtime','--release','--locked','--offline','-j','4',
            '--test','element_attach_restore','--','--include-ignored')
    }
}
