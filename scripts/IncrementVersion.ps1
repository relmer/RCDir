# IncrementVersion.ps1
# Automatically increments the build number in Version.toml before each build.
# Called by Build.ps1, NOT by build.rs — this ensures rust-analyzer never triggers
# version increments because it never invokes Build.ps1.
#
# Ported from TCDir's IncrementVersion.ps1 (adapted for TOML format).

$repoRoot    = Split-Path $PSScriptRoot -Parent
$versionFile = "$repoRoot\Version.toml"
$tempFile    = "$versionFile.tmp"
$backupFile  = "$versionFile.bak"

# Check if the version file exists
if (-not (Test-Path $versionFile)) {
    Write-Error "Version file not found: $versionFile"
    exit 1
}

# Read the file content
try {
    $content = Get-Content $versionFile -Raw -ErrorAction Stop
} catch {
    Write-Error "Failed to read $versionFile : $_"
    exit 1
}

# Validate the file has expected content before proceeding
if ([string]::IsNullOrWhiteSpace($content) -or $content -notmatch 'build\s*=\s*\d+') {
    Write-Warning "Version.toml appears corrupted or empty. Attempting to restore from git..."

    try {
        Push-Location $repoRoot
        git checkout HEAD -- "Version.toml" 2>&1 | Out-Null
        Pop-Location

        # Re-read after restore
        $content = Get-Content $versionFile -Raw -ErrorAction Stop

        if ($content -notmatch 'build\s*=\s*\d+') {
            Write-Error "Failed to restore Version.toml from git"
            exit 1
        }
        Write-Host "Successfully restored Version.toml from git" -ForegroundColor Yellow
    } catch {
        Write-Error "Failed to restore Version.toml: $_"
        exit 1
    }
}

# Find and increment the build number
if ($content -match 'build\s*=\s*(\d+)') {
    $buildNumber = [int]$matches[1] + 1
    $newContent  = $content -replace 'build\s*=\s*\d+', "build = $buildNumber"

    # Write to temp file first (atomic write pattern)
    try {
        Set-Content -Path $tempFile -Value $newContent -NoNewline -ErrorAction Stop
    } catch {
        Write-Error "Failed to write temp file: $_"
        if (Test-Path $tempFile) { Remove-Item $tempFile -Force -ErrorAction SilentlyContinue }
        exit 1
    }

    # Verify temp file was written correctly
    $verifyContent = Get-Content $tempFile -Raw -ErrorAction SilentlyContinue
    if ($verifyContent -notmatch "build\s*=\s*$buildNumber") {
        Write-Error "Temp file verification failed"
        Remove-Item $tempFile -Force -ErrorAction SilentlyContinue
        exit 1
    }

    # Backup original, replace with new (with retry for locked files)
    $maxRetries = 3
    $retryDelay = 500  # milliseconds

    for ($i = 0; $i -lt $maxRetries; $i++) {
        try {
            # Create backup
            Copy-Item $versionFile $backupFile -Force -ErrorAction Stop

            # Replace original with temp
            Move-Item $tempFile $versionFile -Force -ErrorAction Stop

            # Success — remove backup
            Remove-Item $backupFile -Force -ErrorAction SilentlyContinue

            Write-Host "Build number incremented to: $buildNumber" -ForegroundColor Green
            exit 0
        } catch {
            Write-Warning "Attempt $($i + 1) failed: $_"
            Start-Sleep -Milliseconds $retryDelay
        }
    }

    # All retries failed — restore from backup if it exists
    Write-Error "Failed to update Version.toml after $maxRetries attempts"
    if (Test-Path $backupFile) {
        Copy-Item $backupFile $versionFile -Force -ErrorAction SilentlyContinue
        Remove-Item $backupFile -Force -ErrorAction SilentlyContinue
    }
    Remove-Item $tempFile -Force -ErrorAction SilentlyContinue
    exit 1
} else {
    Write-Error "Could not find 'build' key in $versionFile"
    exit 1
}
