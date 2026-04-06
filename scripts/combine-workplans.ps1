#!/usr/bin/env pwsh

# PowerShell equivalent of combine-workplans.pl
# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause

# Mark sure to also update combine-workplans.pl when making changes here

param(
    [string]$WorkplanDir = "docs/work-plan",
    [string]$OutputFile = "docs/WORK_PLAN.md",
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

# Get script directory and project root
if ($MyInvocation.MyCommand.Path) {
    $ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    $RootDir = Split-Path -Parent $ScriptDir
}
else {
    # Fallback when running interactively
    $ScriptDir = $PSScriptRoot
    $RootDir = Split-Path -Parent $ScriptDir
}

# Resolve relative paths to absolute paths
$WorkplanDir = Resolve-Path (Join-Path $RootDir $WorkplanDir) -ErrorAction SilentlyContinue
if (-not $WorkplanDir) {
    $WorkplanDir = Join-Path $RootDir $WorkplanDir
}

$OutputFile = Join-Path $RootDir $OutputFile
$OutputFile = [System.IO.Path]::GetFullPath($OutputFile)

# Subdirectories for organization
$StatusDirs = @{
    'Completed'   = 'done'
    'In Progress' = 'medium'
    'Blocked'     = 'high'
    'Not Started' = 'low'
}

# Default priority mapping for items without explicit status
$PriorityDefaults = @{
    'High'   = 'high'
    'Medium' = 'medium'
    'Low'    = 'low'
}

# Priority order
$PriorityOrder = @{
    'High'   = 1
    'Medium' = 2
    'Low'    = 3
}

function Write-Status {
    param([string]$Message)
    $timestamp = Get-Date -Format "HH:mm:ss"
    Write-Host "[$timestamp] $Message" -ForegroundColor Green
}

function Get-WorkplanFiles {
    param([string]$Directory)
    
    Write-Status "Scanning for workplan files in $Directory and subdirectories"
    
    $files = Get-ChildItem -Path $Directory -Filter "*.md" -File -Recurse | 
    Where-Object { $_.Name -ne "combine-workplans.ps1" } |
    ForEach-Object { 
        # Determine which subdirectory this file is in
        $relativePath = $_.FullName.Substring($Directory.Length)
        $subdir = if ($relativePath -match '^[/\\]([^/\\]+)') { 
            $matches[1] 
        }
        else { 
            '' 
        }
        
        # Add current subdirectory as custom property
        $_ | Add-Member -NotePropertyName "CurrentSubdir" -NotePropertyValue $subdir -PassThru 
    }
    
    $files = $files | Sort-Object FullName
    
    if ($Verbose) {
        Write-Host "Found $($files.Count) markdown files"
        foreach ($file in $files) {
            Write-Host "  - $($file.Name) (in $($file.CurrentSubdir))"
        }
    }
    
    return $files
}

function Import-WorkplanFile {
    param([System.IO.FileInfo]$File)
    
    if ($Verbose) {
        Write-Host "Parsing $($file.Name)..."
    }
    
    $content = Get-Content $File.FullName -Raw
    
    # Extract metadata using regex (multiline mode)
    $title = if ($content -match '(?m)^#\s+(.+)$') { $matches[1] } else { throw "No title found in $($File.Name)" }
    $summary = if ($content -match '(?ms)## Summary\s*\n(.+?)(?=\n##|\z)') { $matches[1].Trim() } else { throw "No summary found in $($File.Name)" }
    $status = if ($content -match '(?ms)## Status\s*\n(.+?)(?=\n##|\z)') { $matches[1].Trim() } else { throw "No status found in $($File.Name)" }
    $priority = if ($content -match '(?ms)## Priority\s*\n(.+?)(?=\n##|\z)') { $matches[1].Trim() } else { throw "No priority found in $($File.Name)" }
    $description = if ($content -match '(?ms)## Description\s*\n(.+?)(?=\n##|\z)') { $matches[1].Trim() } else { throw "No description found in $($File.Name)" }
    
    # Extract prefix and number from filename
    if ($File.BaseName -match '([^-]+)-(\d+)$') {
        $prefix = $matches[1]
        $num = $matches[2]  # Keep as string to preserve leading zeros
    }
    else {
        throw "Cannot parse filename format: $($File.Name). Expected format: prefix-number.md"
    }
    
    # Calculate relative path from output file to workplan directory
    $outputDir = Split-Path $OutputFile -Parent
    $relativeWorkplanPath = [System.IO.Path]::GetRelativePath($outputDir, $WorkplanDir).Replace('\', '/')
    
    return @{
        File          = $File.FullName
        RelativeFile  = "$relativeWorkplanPath/$($File.Name)"
        Prefix        = $prefix
        Number        = $num
        Title         = $title
        Summary       = $summary
        Status        = $status
        Priority      = $priority
        Description   = $description
        FullContent   = $content
        CurrentSubdir = $File.CurrentSubdir
    }
}

function Set-WorkplanOrder {
    param([array]$Items)

    # Add sort properties to each item
    foreach ($item in $Items) {
        $prio = if ($PriorityOrder.ContainsKey($item.Priority)) { $PriorityOrder[$item.Priority] } else { 999 }
        $item | Add-Member -NotePropertyName 'SortPriority' -NotePropertyValue $prio -Force
        $item | Add-Member -NotePropertyName 'SortNumber' -NotePropertyValue ([int]$item.Number) -Force
    }

    # Sort using Sort-Object with scriptblock syntax to avoid single-item array bug
    $sorted = @($Items | Sort-Object { $_.SortPriority }, { $_.Prefix }, { $_.SortNumber })
    return $sorted
}

function New-WorkPlanContent {
    param([array]$Items)
    
    $content = @()
    $content += "# Cosplay America Schedule - Work Plan"
    $content += ""
    $content += "Generated on: $(Get-Date)"
    $content += ""
    
    # Separate completed and open items
    $completed = $Items | Where-Object { $_.Status -eq 'Completed' }
    $open = $Items | Where-Object { $_.Status -ne 'Completed' }
    
    # Collect all links for glossary
    $allLinks = @{}
    
    # Track numbering conflicts and used IDs
    $conflicts = @{}
    $usedIds = @{}
    
    foreach ($item in $Items) {
        $id = $item.Number
        $prefix = $item.Prefix
        
        # Track used IDs
        if ($usedIds.ContainsKey($id)) {
            if (-not $conflicts.ContainsKey($id)) {
                $conflicts[$id] = @()
            }
            $conflicts[$id] += $item
        }
        else {
            $usedIds[$id] = $item
        }
    }
    
    # Output completed items
    if ($completed) {
        $content += "## Completed"
        $content += ""
        
        # Add SortNumber and sort
        foreach ($item in $completed) {
            $item | Add-Member -NotePropertyName 'SortNumber' -NotePropertyValue ([int]$item.Number) -Force
        }
        $sortedCompleted = @($completed | Sort-Object { $_.Prefix }, { $_.SortNumber })
        foreach ($item in $sortedCompleted) {
            $linkId = "$($item.Prefix)-$($item.Number)"
            $allLinks[$linkId] = Get-RelativePath -Item $item
            $content += "* [$linkId] $($item.Summary)"
        }
        
        $content += ""
        $content += "---"
        $content += ""
    }
    
    # Add summary of open items
    if ($open) {
        $content += "## Summary of Open Items"
        $content += ""
        $content += "**Total open items:** $($open.Count)"
        $content += ""
        
        # Group open items by priority
        $byPriority = $open | Group-Object -Property Priority
        
        # Output summary list by priority
        foreach ($priority in @('High', 'Medium', 'Low')) {
            $group = $byPriority | Where-Object { $_.Name -eq $priority }
            if (-not $group) { continue }
            
            $content += "* **$priority Priority**"
            
            # Add SortNumber and sort
            foreach ($item in $group.Group) {
                $item | Add-Member -NotePropertyName 'SortNumber' -NotePropertyValue ([int]$item.Number) -Force
            }
            $sortedItems = @($group.Group | Sort-Object { $_.Prefix }, { $_.SortNumber })
            foreach ($item in $sortedItems) {
                $linkId = "$($item.Prefix)-$($item.Number)"
                $allLinks[$linkId] = Get-RelativePath -Item $item
                $content += "  * [$linkId] $($item.Summary)"
            }
            
            $content += ""
        }
        
        $content += "---"
        $content += ""
    }
    
    # Add next available IDs section
    $content += "## Next Available IDs"
    $content += ""
    $content += "The following ID numbers are available for new items:"
    $content += ""
    
    # Find max ID used across all items
    $maxId = 0
    $allUsedIds = @{}
    foreach ($item in $Items) {
        $id = [int]$item.Number
        $allUsedIds[$id] = $true
        if ($id -gt $maxId) { $maxId = $id }
    }
    
    # Calculate how many IDs to show (at least 10 more than number of conflicts)
    $conflictCount = $conflicts.Keys.Count
    $minCount = $conflictCount + 10
    $countToShow = if ($minCount -gt 10) { $minCount } else { 10 }
    
    # Find available IDs with zero padding
    $available = @()
    $checkId = 1
    while ($available.Count -lt $countToShow) {
        if (-not $allUsedIds.ContainsKey($checkId)) {
            $available += "{0:D3}" -f $checkId
        }
        $checkId++
    }
    
    $content += "**Available:** $($available -join ', ')"
    $content += ""
    $content += "**Highest used:** $maxId"
    $content += ""
    $content += "---"
    $content += ""
    
    # Add numbering conflicts section if any exist
    if ($conflicts.Keys.Count -gt 0) {
        # Filter to only show actual conflicts (IDs with 2+ items) and exclude completed-only conflicts
        $actualConflicts = @{}
        foreach ($conflictId in $conflicts.Keys) {
            $totalItems = 1 + $conflicts[$conflictId].Count
            if ($totalItems -ge 2) {
                # Check if any of the conflicting items are open
                $hasOpenItems = $false
                if ($usedIds[$conflictId].Status -ne 'Completed') {
                    $hasOpenItems = $true
                }
                foreach ($item in $conflicts[$conflictId]) {
                    if ($item.Status -ne 'Completed') {
                        $hasOpenItems = $true
                        break
                    }
                }
                # Only include conflicts that have at least one open item
                if ($hasOpenItems) {
                    $actualConflicts[$conflictId] = @($usedIds[$conflictId]) + $conflicts[$conflictId]
                }
            }
        }
        
        if ($actualConflicts.Keys.Count -gt 0) {
            $content += "### Numbering Conflicts"
            $content += ""
            $content += "The following ID numbers are used by multiple items:"
            $content += ""
            
            $sortedConflictIds = $actualConflicts.Keys | ForEach-Object { [int]$_ } | Sort-Object | ForEach-Object { "{0}" -f $_ }
            foreach ($conflictId in $sortedConflictIds) {
                $content += "#### Suffix `"{0:D3}`"" -f $conflictId
                $content += ""
                foreach ($item in $actualConflicts[$conflictId]) {
                    $statusIcon = if ($item.Status -eq 'Completed') { '✓' } else { '○' }
                    $displayId = "{0}-{1:D3}" -f $item.Prefix, $item.Number
                    $content += "* $statusIcon [$displayId] $($item.Title)"
                }
                $content += ""
            }
            $content += "---"
            $content += ""
        }
    }
    
    # Group open items by prefix for detailed view
    $openByPrefix = @{}
    foreach ($item in $open) {
        if (-not $openByPrefix.ContainsKey($item.Prefix)) {
            $openByPrefix[$item.Prefix] = @()
        }
        $openByPrefix[$item.Prefix] += $item
    }
    
    # Output each prefix section
    foreach ($prefix in ($openByPrefix.Keys | Sort-Object)) {
        $content += "## Open $prefix Items"
        $content += ""

        # Add SortNumber and sort
        $groupItems = $openByPrefix[$prefix]

        foreach ($item in $groupItems) {
            $item | Add-Member -NotePropertyName 'SortNumber' -NotePropertyValue ([int]$item.Number) -Force
        }
        # Sort by priority first, then by number (matches Perl behavior)
        $sortedItems = @($groupItems | Sort-Object { $_.SortPriority }, { $_.SortNumber })

        for ($i = 0; $i -lt $sortedItems.Count; $i++) {
            $item = $sortedItems[$i]
            
            $linkId = "$($item.Prefix)-$($item.Number)"
            $allLinks[$linkId] = Get-RelativePath -Item $item
            
            $content += "### [$linkId] $($item.Title)"
            $content += ""
            $content += "**Status:** $($item.Status)"
            $content += ""
            $content += "**Priority:** $($item.Priority)"
            $content += ""
            $content += "**Summary:** $($item.Summary)"
            $content += ""
            $content += "**Description:** $($item.Description)"
            $content += ""
            
            # Add separator, but not after the last item in this prefix
            if ($i -lt $sortedItems.Count - 1) {
                $content += "---"
                $content += ""
            }
        }
        $content += "---"
        $content += ""
    }
    
    # Add link glossary at the end (no header to avoid rendering issues)
    $content += "---"
    $content += ""
    
    foreach ($linkId in ($allLinks.Keys | Sort-Object)) {
        $content += "[$linkId]: $($allLinks[$linkId])"
    }
    
    return $content -join "`r`n"
}

function Get-RelativePath {
    param([hashtable]$Item)
    
    # Build relative path based on current subdirectory
    $filename = "$($Item.Prefix)-$($Item.Number).md"
    if ($Item.CurrentSubdir) {
        return "work-plan/$($Item.CurrentSubdir)/$filename"
    }
    else {
        return "work-plan/$filename"
    }
}

function Invoke-FileReorganization {
    param([array]$Items)
    
    Write-Status "Reorganizing files to correct directories..."
    
    # Ensure target directories exist
    $targetDirs = @('done', 'high', 'medium', 'low')
    foreach ($dir in $targetDirs) {
        $fullDir = Join-Path $WorkplanDir $dir
        if (-not (Test-Path $fullDir)) {
            New-Item -ItemType Directory -Path $fullDir -Force | Out-Null
            Write-Status "Created directory: $fullDir"
        }
    }
    
    # Process each item and move if needed
    foreach ($item in $Items) {
        $targetSubdir = Get-TargetDirectory -Item $item
        $targetDir = Join-Path $WorkplanDir $targetSubdir
        
        # Skip if already in correct location
        if ($item.CurrentSubdir -eq $targetSubdir) {
            if ($Verbose) { Write-Host "Already in correct location: $($item.Prefix)-$($item.Number).md" }
            continue
        }
        
        $filename = "$($item.Prefix)-$($item.Number).md"
        $sourcePath = $item.File
        $targetPath = Join-Path $targetDir $filename
        
        # Check if target file already exists
        if (Test-Path $targetPath) {
            Write-Warning "Target file already exists: $targetPath"
            Write-Warning "Skipping move of: $sourcePath"
            continue
        }
        
        # Move the file
        try {
            Move-Item -Path $sourcePath -Destination $targetPath -Force
            Write-Status "Moved: $filename -> $targetSubdir/"
            
            # Update the file path in the item for correct linking
            $item.File = $targetPath
            $item.CurrentSubdir = $targetSubdir
        }
        catch {
            $errorMsg = "Failed to move {0} to {1}: {2}" -f $sourcePath, $targetPath, $_.Exception.Message
            Write-Error $errorMsg
        }
    }
}

function Get-TargetDirectory {
    param([hashtable]$Item)
    
    # Completed items always go to done directory
    if ($Item.Status -eq 'Completed') {
        return 'done'
    }
    
    # Use priority mapping for non-completed items
    if ($PriorityDefaults.ContainsKey($Item.Priority)) {
        return $PriorityDefaults[$Item.Priority]
    }
    
    # Fall back to status mapping for remaining cases
    if ($StatusDirs.ContainsKey($Item.Status)) {
        return $StatusDirs[$Item.Status]
    }
    
    # Default to medium for unknown cases
    return 'medium'
}

# Main execution
try {
    Write-Status "Generating combined work plan..."
    
    # Get all workplan files
    $files = Get-WorkplanFiles -Directory $WorkplanDir
    if (-not $files) {
        Write-Warning "No workplan files found in $WorkplanDir"
        exit 0
    }
    
    # Parse all files
    $items = @()
    foreach ($file in $files) {
        try {
            $item = Import-WorkplanFile -File $file
            $items += $item
        }
        catch {
            Write-Warning "Failed to parse $($file.Name): $($_.Exception.Message)"
        }
    }
    
    if (-not $items) {
        Write-Warning "No valid workplan items found"
        exit 0
    }
    
    # Sort items
    $sortedItems = Set-WorkplanOrder -Items $items
    
    # Generate content (will be regenerated after file moves)
    $content = New-WorkPlanContent -Items $sortedItems
    
    # Ensure output directory exists
    $outputDir = Split-Path $OutputFile -Parent
    if (-not (Test-Path $outputDir)) {
        New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
    }
    
    # Write output file with LF line endings (add trailing newline to match Perl)
    $contentLf = ($content -replace "`r`n", "`n") + "`n"
    $contentLf | Out-File -FilePath $OutputFile -Encoding UTF8 -NoNewline
    
    Write-Status "Generated $OutputFile with $($items.Count) work items"
    
    # Reorganize files to correct directories first
    Invoke-FileReorganization -Items $items
    
    # Regenerate WORK_PLAN.md with updated file paths
    $content = New-WorkPlanContent -Items $items
    $contentLf = ($content -replace "`r`n", "`n") + "`n"
    $contentLf | Out-File -FilePath $OutputFile -Encoding UTF8 -NoNewline
    
    Write-Status "File reorganization complete"
}
catch {
    Write-Error "Failed to generate work plan: $_"
    exit 1
}
