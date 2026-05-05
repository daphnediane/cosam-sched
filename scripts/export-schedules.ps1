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
#   Writes to output/<YEAR>/{schedule.xlsx,public.json,embed.html,test.html,style-embed.html,style-page.html}

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

    # Build cosam-convert once at the start
    Write-Status "Building cosam-convert..."
    Push-Location $RootDir
    & cargo build -p cosam-convert --release
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to build cosam-convert"
    }
    $ConvertBin = Join-Path $RootDir "target/release/cosam-convert"
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

        Write-Status "Building ${year} files..."

        $copy       = Join-Path $yearDir "schedule.xlsx"
        $dest       = Join-Path $yearDir "public.json"
        $embed      = Join-Path $yearDir "embed.html"
        $testHtml   = Join-Path $yearDir "test.html"
        $styleEmbed = Join-Path $yearDir "style-embed.html"
        $stylePage  = Join-Path $yearDir "style-page.html"

        try {
            & $ConvertBin `
                --input $srcFile `
                --title "Cosplay America ${year} Schedule" `
                --output $copy `
                --export $dest `
                --export-embed $embed `
                --export-test $testHtml `
                --style-page `
                --export-embed $styleEmbed `
                --export-test $stylePage

            if ($LASTEXITCODE -eq 0) {
                $built += $copy, $dest, $embed, $testHtml, $styleEmbed, $stylePage
                Write-Host "    Built all files for ${year}"
            }
            else {
                $failed += $copy, $dest, $embed, $testHtml, $styleEmbed, $stylePage
                Write-Warn "Failed to build files for ${year} (exit $LASTEXITCODE)"
            }
        }
        catch {
            $failed += $copy, $dest, $embed, $testHtml, $styleEmbed, $stylePage
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
