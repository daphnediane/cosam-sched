#!/usr/bin/env pwsh

# PowerShell equivalent of export-schedules.sh
# Helper script to export all schedule files
# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
#
# NOTE: When updating this script, also update export-schedules.sh to maintain parity
#
# Usage: scripts/export-schedules.ps1
#   Reads from input/<YEAR> Schedule.xlsx
#   Creates output/<YEAR>/schedule.cosam (via cosam-convert if new, cosam-modify --merge-xlsx if existing)
#   Writes to output/<YEAR>/{schedule.xlsx,public.json,private.json,embed.html,embed-head.html,embed-body.html,test.html,style-embed.html,style-page.html}
#   embed-head.html / embed-body.html are the split form for Ajax-navigation hosts
#   (Squarespace 7.0): paste embed-head.html into site-wide Code Injection Header, embed-body.html into the page Code Block
#   For current year, also writes CSV files to output/<CURRENT_YEAR>/csv/
#   Also generates layout to output/<CURRENT_YEAR>/layout/ via schedule-layout (built into cosam-convert)

param(
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

# Get script directory and project root
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent $ScriptDir
$InputDir = Join-Path $RootDir "input"
$OutputDir = Join-Path $RootDir "output"

function Write-Status {
    param([string]$Message)
    $timestamp = Get-Date -Format "HH:mm:ss"
    Write-Host "[$timestamp] $Message" -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    $timestamp = Get-Date -Format "HH:mm:ss"
    Write-Host "[$timestamp] WARNING: $Message" -ForegroundColor Yellow
}

try {
    Write-Status "Rebuilding schedule output files..."
    Write-Host "Script directory: $ScriptDir"
    Write-Host "Input directory:  $InputDir"
    Write-Host "Output directory: $OutputDir"
    Write-Host "Project root:     $RootDir"
    Write-Host ""

    if (-not (Test-Path $OutputDir)) {
        New-Item -ItemType Directory -Path $OutputDir | Out-Null
    }

    # Build cosam-convert (schedule-layout is linked in via the 'layout' feature)
    # and cosam-modify (used to merge XLSX into existing .cosam files)
    Write-Status "Building cosam-convert and cosam-modify..."
    Push-Location $RootDir
    & cargo build -p cosam-convert -p cosam-modify --release
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to build cosam-convert and cosam-modify"
    }
    $ConvertBin = Join-Path $RootDir "target/release/cosam-convert"
    $ModifyBin = Join-Path $RootDir "target/release/cosam-modify"
    Pop-Location

    $built = @()
    $failed = @()
    $conflictYears = @()
    $currentYear = (Get-Date).Year

    Write-Host ""
    Write-Status "Validating all schedules..."
    for ($year = 2016; $year -le $currentYear; $year++) {
        $srcFile = Join-Path $InputDir "${year} Schedule.xlsx"

        if (-not (Test-Path $srcFile)) {
            Write-Warn "Skipping ${year} - file not found"
            continue
        }

        Write-Host "  Validating ${year}..."
        $null = & $ConvertBin --input $srcFile --check 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Host "    ${year} - OK"
        }
        else {
            Write-Host "    ${year} has conflicts"
            $conflictYears += $year
        }
    }

    if ($conflictYears.Count -gt 0) {
        Write-Host ""
        Write-Warn "Schedules with conflicts: $($conflictYears -join ', ')"
        Write-Host ""
    }

    Write-Status "Building all output files..."

    $currentYear = (Get-Date).Year

    for ($year = 2016; $year -le $currentYear; $year++) {
        $yearDir = Join-Path $OutputDir "$year"
        if (-not (Test-Path $yearDir)) {
            New-Item -ItemType Directory -Path $yearDir | Out-Null
        }
        $srcFile = Join-Path $InputDir "${year} Schedule.xlsx"

        if (-not (Test-Path $srcFile)) {
            Write-Warn "Skipping ${year} - file not found"
            continue
        }

        $cosamFile = Join-Path $yearDir "schedule.cosam"
        $copy = Join-Path $yearDir "schedule.xlsx"
        $dest = Join-Path $yearDir "public.json"
        $privateDest = Join-Path $yearDir "private.json"
        $embed = Join-Path $yearDir "embed.html"
        $embedHead = Join-Path $yearDir "embed-head.html"
        $embedBody = Join-Path $yearDir "embed-body.html"
        $testHtml = Join-Path $yearDir "test.html"
        $styleEmbed = Join-Path $yearDir "style-embed.html"
        $stylePage = Join-Path $yearDir "style-page.html"
        $layoutDir = Join-Path $yearDir "layout"
        $csvDir = Join-Path $yearDir "csv"

        # Create or update the .cosam binary from the XLSX source
        Write-Host "  Updating ${year} schedule binary..."
        try {
            if (-not (Test-Path $cosamFile)) {
                Write-Host "    Creating new $cosamFile from xlsx..."
                & $ConvertBin --input $srcFile --output $cosamFile
                if ($LASTEXITCODE -ne 0) {
                    throw "cosam-convert exited with $LASTEXITCODE"
                }
            }
            else {
                Write-Host "    Merging xlsx into existing $cosamFile..."
                & $ModifyBin --file $cosamFile --merge-xlsx $srcFile
                if ($LASTEXITCODE -ne 0) {
                    throw "cosam-modify exited with $LASTEXITCODE"
                }
            }
            $built += $cosamFile
        }
        catch {
            Write-Warn "Failed to update schedule binary for ${year}: $($_.Exception.Message)"
            $failed += $cosamFile
            continue
        }

        Write-Status "Building ${year} output files..."

        $convertArgs = @(
            "--input", $cosamFile,
            "--title", "Cosplay America ${year} Schedule",
            "--output", $copy,
            "--export", $dest,
            "--private",
            "--export", $privateDest,
            "--public",
            "--export-embed", $embed,
            "--export-embed-head", $embedHead,
            "--export-embed-body", $embedBody,
            "--export-test", $testHtml,
            "--style-page",
            "--export-embed", $styleEmbed,
            "--export-test", $stylePage
        )
        $files = @($copy, $dest, $privateDest, $embed, $embedHead, $embedBody, $testHtml, $styleEmbed, $stylePage)

        # For current year, also export layout and CSV in the same pass
        if ($year -eq $currentYear) {
            $convertArgs += "--export-layout", $layoutDir
            $files += $layoutDir
            $convertArgs += "--export-csv-dir", $csvDir
            $files += $csvDir
        }

        try {
            & $ConvertBin @convertArgs

            if ($LASTEXITCODE -eq 0) {
                $built += $files
                Write-Host "    Built all files for ${year}"
            }
            else {
                $failed += $files
                Write-Warn "Failed to build files for ${year} (exit $LASTEXITCODE)"
            }
        }
        catch {
            $failed += $files
            Write-Warn "Failed to build files for ${year}: $($_.Exception.Message)"
        }

        Write-Host ""
    }

    Write-Status "Done!"
    Write-Host ""

    if ($built.Count -gt 0) {
        Write-Host "Files built:"
        foreach ($file in $built) {
            Write-Host "  - $file"
        }
    }

    if ($conflictYears.Count -gt 0) {
        Write-Host ""
        Write-Warn "Schedules with conflicts (still exported):"
        foreach ($year in $conflictYears) {
            Write-Host "  - ${year}"
        }
    }

    if ($failed.Count -gt 0) {
        Write-Host ""
        Write-Warn "Files that failed to build:"
        foreach ($file in $failed) {
            Write-Host "  - $file"
        }
        exit 10
    }
}
catch {
    Write-Error "Script failed: $_"
    exit 1
}
