#!/usr/bin/env pwsh

# PowerShell equivalent of sync-schedule.sh
# Build the current-year layout PDFs and data files in a temp dir, then
# sync them to OneDrive.
# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
#
# Usage: scripts/sync-schedule.ps1 [--dry-run|-n] [--year YYYY] [--output-dir DIR]
#   Reads from input/<YEAR> Schedule.xlsx
#   Builds into a temporary <tmp>/{pdf,generated}/ tree (ramdisk preferred)
#   Syncs both pdf/ and generated/ to the OneDrive CosAm schedule folder in one
#   operation, leaving any sibling files in that folder untouched

param(
    [switch]$DryRun,
    [string]$Year = "2026",
    [string]$OutputDir = ""
)

# Also support bash-style arguments for compatibility
for ($i = 0; $i -lt $args.Count; $i++) {
    switch ($args[$i]) {
        { $_ -eq "--dry-run" -or $_ -eq "-n" } {
            $DryRun = $true
        }
        "--year" {
            if ($i + 1 -lt $args.Count) {
                $Year = $args[$i + 1]
                $i++
            }
        }
        "--output-dir" {
            if ($i + 1 -lt $args.Count) {
                $OutputDir = $args[$i + 1]
                $i++
            }
        }
        default {
            Write-Error "Unknown argument: $($args[$i])"
            exit 1
        }
    }
}

$ErrorActionPreference = "Stop"

# Get script directory and project root
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent $ScriptDir

function Write-Status {
    param([string]$Message)
    $timestamp = Get-Date -Format "HH:mm:ss"
    Write-Host "[$timestamp] $Message" -ForegroundColor Green
}

function Invoke-Command {
    param([string[]]$Arguments)
    Write-Host "+ $($Arguments -join ' ')"
    if (-not $DryRun) {
        $exe = $Arguments[0]
        $cmdArgs = $Arguments[1..($Arguments.Count - 1)]
        & $exe @cmdArgs
        if ($LASTEXITCODE -ne 0) {
            throw "Command failed with exit code $LASTEXITCODE"
        }
    }
}

function Get-TempRoot {
    # Check for ramdisk first (Windows: R: drive or similar)
    $ramdisk = "R:\"
    if (Test-Path $ramdisk) {
        return $ramdisk.TrimEnd('\')
    }
    
    # Fall back to standard temp locations
    $tempDirs = @($env:TEMP, $env:TMP, [System.IO.Path]::GetTempPath())
    foreach ($dir in $tempDirs) {
        if ($dir -and (Test-Path $dir) -and (Test-Path $dir -PathType Container)) {
            return $dir.TrimEnd('\')
        }
    }
    
    throw "No usable temp directory found"
}

$WorkDir = $null

try {
    Push-Location $RootDir
    
    $inputFile = "input/${Year} Schedule.xlsx"
    
    # Determine OneDrive base path
    if ($env:OneDriveConsumer) {
        $oneDriveBase = $env:OneDriveConsumer
    }
    else {
        $oneDriveBase = Join-Path $env:USERPROFILE "OneDrive"
    }
    
    $schedBase = Join-Path $oneDriveBase "Cosplay America - CosAm\CosAm - Schedule\${Year} - CosAm - Schedule"
    
    # Use custom output directory if specified
    if ($OutputDir) {
        $schedBase = $OutputDir
    }
    
    if (-not (Test-Path $inputFile)) {
        throw "input not found: ${inputFile}"
    }

    # Build into a temporary tree; clean it up on exit
    $tmproot = Get-TempRoot
    $WorkDir = Join-Path $tmproot "cosam-sched-${Year}-$(Get-Random)"
    New-Item -ItemType Directory -Path $WorkDir -Force | Out-Null
    Write-Status "Working in $WorkDir"
    
    # Subdir name matches the OneDrive destination so sync maps cleanly
    $generatedDir = Join-Path $WorkDir "generated"
    New-Item -ItemType Directory -Path $generatedDir -Force | Out-Null
    
    # Generate the public schedule layout PDFs and data files
    $convertArgs = @(
        "cargo", "run", "--release", "-p", "cosam-convert", "--",
        "--input", $inputFile,
        "--title", "Cosplay America ${Year} Schedule",
        "--public",
        "--embed-as-html",
        "--output", "${generatedDir}/cos${Year}.xlsx",
        "--export-xlsx-grid", "${generatedDir}/cos${Year}grid.xlsx",
        "--export-embed", "${generatedDir}/embed.html",
        "--export-embed-head", "${generatedDir}/embed-head.html",
        "--export-embed-body", "${generatedDir}/embed-body.html",
        "--export-test", "${generatedDir}/preview.html",
        "--layout-config", "config/layout.toml",
        "--export-layout", $generatedDir
    )
    
    Invoke-Command $convertArgs
    
    # Sync generated/ to OneDrive using Robocopy
    # To preserve directory structure like rsync --relative, we copy to schedBase/generated
    # /E - copy subdirectories including empty ones
    # /PURGE - delete dest files that no longer exist in source (like --delete-after)
    # /XO - exclude older files (similar to --checksum logic, but uses timestamps)
    # /COPYALL - copy all file info (data, attributes, timestamps, security, ownership) like rsync -aAX
    # /DCOPY:T - copy directory timestamps
    # /IT - include tweaked files (more thorough comparison, closer to checksum behavior)
    # /R:0 /W:0 - no retries on failure
    # /L - list only (dry-run mode)
    # Note: robocopy doesn't support hard links (-H) or true checksum comparison (--checksum)
    $destDir = Join-Path $schedBase "generated"
    $robocopyArgs = @(
        $generatedDir,
        $destDir,
        "/E",
        "/PURGE",
        "/XO",
        "/COPYALL",
        "/DCOPY:T",
        "/IT",
        "/R:0",
        "/W:0"
    )
    
    if ($DryRun) {
        $robocopyArgs += "/L"
        Write-Host "Dry-run mode: showing what would be synced"
        Write-Host "+ robocopy $($robocopyArgs -join ' ')"
        & robocopy @robocopyArgs
        # Robocopy returns 0 for success, 1 for files copied, 2 for some mismatches
        if ($LASTEXITCODE -gt 7) {
            throw "robocopy dry-run to ${schedBase} failed with exit code $LASTEXITCODE"
        }
    }
    else {
        & robocopy @robocopyArgs | Out-Null
        # Robocopy returns 0 for success, 1 for files copied, 2 for some mismatches
        if ($LASTEXITCODE -gt 7) {
            throw "robocopy to ${schedBase} failed with exit code $LASTEXITCODE"
        }
        Write-Status "Synced to $schedBase"
    }
}
finally {
    # Clean up temp directory
    if ($WorkDir -and (Test-Path $WorkDir)) {
        Remove-Item -Path $WorkDir -Recurse -Force -ErrorAction SilentlyContinue
    }
    Pop-Location
}
