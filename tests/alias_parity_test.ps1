# tests/alias_parity_test.ps1 — Automated parity test for alias commands
#
# Runs rcdir alias commands with RCDIR_ALIAS_TEST_INPUTS and compares output
# against TCDir reference fixtures. Run from the RCDir repo root.
#
# Usage: pwsh -NoProfile -File tests/alias_parity_test.ps1

param(
    [string]$RcDirExe = ".\target\debug\rcdir.exe"
)

$ErrorActionPreference = "Stop"
$script:failures = 0
$script:passed   = 0
$fixtureDir      = "$PSScriptRoot\fixtures\alias_parity"

function Test-OutputMatch {
    param(
        [string]$TestName,
        [string]$Actual,
        [string]$FixtureFile
    )

    if (-not (Test-Path $FixtureFile)) {
        Write-Host "  SKIP: $TestName — fixture file not found: $FixtureFile" -ForegroundColor Yellow
        return
    }

    $fixture = Get-Content $FixtureFile -Raw

    # Normalize: strip terminal prompt lines (timestamps, paths), strip tcdir/rcdir name differences
    $normalizeBlock = {
        param($text)
        $lines = $text -split "`n" | ForEach-Object { $_.TrimEnd("`r") }
        $lines = $lines | Where-Object {
            $_ -notmatch '^\[20\d{2}-' -and       # timestamp lines
            $_ -notmatch '^C:\\' -and              # prompt lines
            $_ -notmatch '^\s*$' -eq $false        # keep blank lines (don't filter them)
        }
        $result = ($lines -join "`n")
        $result = $result -replace 'tcdir', 'rcdir'
        $result = $result -replace 'TCDir', 'RCDir'
        $result = $result -replace 'v\d+\.\d+\.\d+', 'vX.Y.Z'  # normalize version
        $result.Trim()
    }

    $normalActual  = & $normalizeBlock $Actual
    $normalFixture = & $normalizeBlock $fixture

    if ($normalActual -eq $normalFixture) {
        Write-Host "  PASS: $TestName" -ForegroundColor Green
        $script:passed++
    } else {
        Write-Host "  FAIL: $TestName" -ForegroundColor Red
        Write-Host "    Expected (fixture):" -ForegroundColor DarkGray
        $normalFixture -split "`n" | ForEach-Object { Write-Host "      $_" -ForegroundColor DarkGray }
        Write-Host "    Actual:" -ForegroundColor DarkGray
        $normalActual -split "`n" | ForEach-Object { Write-Host "      $_" -ForegroundColor DarkGray }
        $script:failures++
    }
}

# Verify exe exists
if (-not (Test-Path $RcDirExe)) {
    Write-Host "ERROR: rcdir.exe not found at $RcDirExe" -ForegroundColor Red
    Write-Host "Run 'cargo build' first." -ForegroundColor Red
    exit 1
}

Write-Host "`nRCDir Alias Parity Tests`n" -ForegroundColor Cyan

# Test 1: --get-aliases with no aliases
Write-Host "Test: --get-aliases (no aliases)" -ForegroundColor White
$output = & $RcDirExe --get-aliases 2>&1 | Out-String
Test-OutputMatch "get-aliases-none" $output "$fixtureDir\get_aliases_none.txt"

# Test 2: --set-aliases --whatif with default inputs
Write-Host "Test: --set-aliases --whatif (defaults)" -ForegroundColor White
$env:RCDIR_ALIAS_TEST_INPUTS = "d;all;CurrentUserAllHosts;y"
$output = & $RcDirExe --set-aliases --whatif 2>&1 | Out-String
Remove-Item Env:\RCDIR_ALIAS_TEST_INPUTS -ErrorAction SilentlyContinue
Test-OutputMatch "set-aliases-whatif-default" $output "$fixtureDir\set_aliases_whatif_default.txt"

# Test 3: --set-aliases --whatif with custom root
Write-Host "Test: --set-aliases --whatif (custom root tc)" -ForegroundColor White
$env:RCDIR_ALIAS_TEST_INPUTS = "tc;all;CurrentUserAllHosts;y"
$output = & $RcDirExe --set-aliases --whatif 2>&1 | Out-String
Remove-Item Env:\RCDIR_ALIAS_TEST_INPUTS -ErrorAction SilentlyContinue
Test-OutputMatch "set-aliases-whatif-custom" $output "$fixtureDir\set_aliases_whatif_custom.txt"

# Summary
Write-Host "`n---" -ForegroundColor Cyan
Write-Host "Results: $($script:passed) passed, $($script:failures) failed`n" -ForegroundColor $(if ($script:failures -gt 0) { "Red" } else { "Green" })

exit $script:failures
