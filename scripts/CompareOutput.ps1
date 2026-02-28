<#
.SYNOPSIS
    Compare RCDir and TCDir output for ad-hoc tree-mode verification.

.DESCRIPTION
    Runs both rcdir and tcdir with the same arguments and compares their
    output line-by-line.  Skips timing lines and free-space lines that
    vary between runs.  Reports matching percentage and first N diffs.

.PARAMETER Arguments
    The arguments to pass to both rcdir and tcdir (e.g., "/Tree C:\Users\*").

.PARAMETER MaxDiffs
    Maximum number of diff lines to display. Default: 20.

.PARAMETER TcDirExe
    Path to TCDir.exe.  If not specified, checks TCDIR_EXE env var,
    then default build locations.

.EXAMPLE
    .\CompareOutput.ps1 "/Tree C:\Users\relmer\repos\relmer\RCDir\*"

.EXAMPLE
    .\CompareOutput.ps1 "/Tree /Depth=2 /Icons C:\Users\relmer\repos\relmer\RCDir\*"

.EXAMPLE
    .\CompareOutput.ps1 -Arguments "/Tree /os *.rs" -MaxDiffs 50

.NOTES
    Requires both rcdir (cargo build) and tcdir to be available.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Arguments,

    [int]$MaxDiffs = 20,

    [string]$TcDirExe = ""
)

$ErrorActionPreference = "Stop"

# Locate TCDir
if ($TcDirExe -eq "") {
    $TcDirExe = $env:TCDIR_EXE
}
if (-not $TcDirExe -or -not (Test-Path $TcDirExe)) {
    $candidates = @(
        "c:\Users\relmer\source\repos\relmer\TCDir\ARM64\Debug\TCDir.exe",
        "c:\Users\relmer\source\repos\relmer\TCDir\x64\Debug\TCDir.exe"
    )
    foreach ($c in $candidates) {
        if (Test-Path $c) {
            $TcDirExe = $c
            break
        }
    }
}
if (-not $TcDirExe -or -not (Test-Path $TcDirExe)) {
    Write-Error "TCDir.exe not found.  Set TCDIR_EXE env var or pass -TcDirExe."
    exit 1
}

# Locate RCDir
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptDir
$rcDirExe = Join-Path $repoRoot "target\debug\rcdir.exe"
if (-not (Test-Path $rcDirExe)) {
    $rcDirExe = Join-Path $repoRoot "target\release\rcdir.exe"
}
if (-not (Test-Path $rcDirExe)) {
    Write-Error "rcdir.exe not found.  Run 'cargo build' first."
    exit 1
}

Write-Host "TCDir: $TcDirExe" -ForegroundColor Cyan
Write-Host "RCDir: $rcDirExe" -ForegroundColor Cyan
Write-Host "Args:  $Arguments" -ForegroundColor Cyan
Write-Host ""

# Split arguments and run both tools
$argArray = $Arguments -split '\s+'

$tcOutput = & $TcDirExe @argArray 2>&1 | Out-String
$rcOutput = & $rcDirExe @argArray 2>&1 | Out-String

# Filter lines — skip timing and free-space lines
function Filter-Lines([string]$output) {
    $output -split "`n" | Where-Object {
        $line = $_ -replace '\x1b\[[0-9;]*[a-zA-Z]', ''
        $trimmed = $line.Trim()
        if ($trimmed -match '^(RCDir|TCDir) time elapsed:') { return $false }
        if ($trimmed -match 'bytes free on volume$') { return $false }
        if ($trimmed -match 'bytes available to user$') { return $false }
        return $true
    }
}

$tcLines = @(Filter-Lines $tcOutput)
$rcLines = @(Filter-Lines $rcOutput)

$maxLines = [Math]::Max($tcLines.Count, $rcLines.Count)
$matching = 0
$diffs = @()

for ($i = 0; $i -lt $maxLines; $i++) {
    $tcLine = if ($i -lt $tcLines.Count) { $tcLines[$i] } else { "<missing>" }
    $rcLine = if ($i -lt $rcLines.Count) { $rcLines[$i] } else { "<missing>" }

    if ($tcLine -eq $rcLine) {
        $matching++
    }
    elseif ($diffs.Count -lt $MaxDiffs) {
        $diffs += "Line $($i+1):`n  TC: [$tcLine]`n  RC: [$rcLine]"
    }
}

# Report results
$pct = if ($maxLines -gt 0) { [math]::Round(($matching / $maxLines) * 100, 1) } else { 100 }

Write-Host ""
Write-Host "Results: $matching/$maxLines lines match ($pct%)" -ForegroundColor $(if ($pct -ge 95) { "Green" } else { "Red" })

if ($diffs.Count -gt 0) {
    Write-Host ""
    Write-Host "First $($diffs.Count) differences:" -ForegroundColor Yellow
    foreach ($d in $diffs) {
        Write-Host $d
    }
}
elseif ($maxLines -gt 0) {
    Write-Host "Output is byte-identical (excluding filtered lines)." -ForegroundColor Green
}
