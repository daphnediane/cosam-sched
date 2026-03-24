#!/usr/bin/env perl

# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
# See LICENSE file for full license text
use strict;
use warnings;
use File::Find;
use File::Copy qw(move);
use FindBin;
use File::Spec;
use File::Path qw(make_path);

# Get script directory and project root
my $script_dir = $FindBin::Bin;
my $root_dir   = "$script_dir/..";

# Configuration
my $workplan_dir = "$root_dir/docs/work-plan";
my $output_file  = "$root_dir/docs/WORK_PLAN.md";

# Subdirectories for organization
my %status_dirs = (
    'Completed'   => 'done',
    'In Progress' => 'medium',
    'Blocked'     => 'high',
    'Not Started' => 'low',
);

# Default priority mapping for items without explicit status
my %priority_defaults = (
    'High'   => 'high',
    'Medium' => 'medium',
    'Low'    => 'low',
);

# Calculate relative path from output file to workplan directory
my $relative_workplan_path
    = File::Spec->abs2rel( $workplan_dir, "$root_dir/docs" );

# Convert forward slashes to forward slashes for markdown consistency
$relative_workplan_path =~ s/\\/\//g;

# Priority order
my %priority_order = (
    'High'   => 1,
    'Medium' => 2,
    'Low'    => 3,
);

# Read all work plan files using recursive search
my @files;
find(
    sub {
        return unless -f && /\.md$/;
        return if $File::Find::name =~ /combine-workplans\.pl$/;

        # Determine which subdirectory this file is in
        my $relative_path = $File::Find::name;
        $relative_path =~ s/^\Q$workplan_dir\E//;
        my $subdir = '';
        if ( $relative_path =~ m{^[/\\]([^/\\]+)} ) {
            $subdir = $1;
        }

        # Store the subdirectory info for later use
        push @files, {
            path   => $File::Find::name,
            subdir => $subdir,
        };
    },
    $workplan_dir
);

# Parse each file
my @items;
for my $file_info ( sort { $a->{ path } cmp $b->{ path } } @files ) {
    my $file    = $file_info->{ path };
    my $content = do {
        local $/;
        open my $fh, '<', $file or die "Can't read $file: $!";
        <$fh>;
    };

    # Extract metadata
    my ( $title )   = $content =~ /^#\s+(.+)$/m or die "No title in $file";
    my ( $summary ) = $content =~ /## Summary\s*\n(.+?)(?=\n##|\z)/s
        or die "No summary in $file";
    my ( $status ) = $content =~ /## Status\s*\n(.+?)(?=\n##|\z)/s
        or die "No status in $file";
    my ( $priority ) = $content =~ /## Priority\s*\n(.+?)(?=\n##|\z)/s
        or die "No priority in $file";
    my ( $description ) = $content =~ /## Description\s*\n(.+?)(?=\n##|\z)/s
        or die "No description in $file";

    # Clean up whitespace
    $summary     =~ s/^\s+|\s+$//g;
    $status      =~ s/^\s+|\s+$//g;
    $priority    =~ s/^\s+|\s+$//g;
    $description =~ s/^\s+|\s+$//g;

    # Extract prefix and number from filename
    my ( $prefix, $num ) = $file =~ m{/([^/]+)-(\d+)\.md$};

    push @items, {
        file           => $file,
        prefix         => $prefix,
        num            => $num,
        title          => $title,
        summary        => $summary,
        status         => $status,
        priority       => $priority,
        description    => $description,
        full_content   => $content,
        current_subdir => $file_info->{ subdir },
    };
} ## end for my $file_info ( sort...)

# Sort by priority, then by prefix and number
@items = sort {
    my $prio_a   = $priority_order{ $a->{ priority } } || 999;
    my $prio_b   = $priority_order{ $b->{ priority } } || 999;
    my $prio_cmp = $prio_a <=> $prio_b;
    return $prio_cmp if $prio_cmp;

    # If same priority, sort by prefix then number
    return $a->{ prefix } cmp $b->{ prefix }
        if $a->{ prefix } ne $b->{ prefix };
    return $a->{ num } <=> $b->{ num };
} @items;

# Reorganize files to correct directories first
reorganize_files();

# Generate WORK_PLAN.md with updated file paths
generate_work_plan( @items );

# Trim trailing whitespace from the output file
my $content = do {
    local $/;
    open my $fh, '<', $output_file or die "Can't read $output_file: $!";
    <$fh>;
};
$content =~ s/\n\s*$/\n/;
open my $fh_out, '>', $output_file or die "Can't write $output_file: $!";
print $fh_out $content;
close $fh_out;

print STDERR "Generated $output_file with "
    . scalar( @items )
    . " work items\n";

sub reorganize_files {
    print STDERR "Reorganizing files to correct directories...\n";

    # Ensure target directories exist
    for my $dir ( qw(done high medium low) ) {
        my $full_dir = "$workplan_dir/$dir";
        unless ( -d $full_dir ) {
            make_path( $full_dir )
                or die "Cannot create directory $full_dir: $!";
            print STDERR "Created directory: $full_dir\n";
        }
    } ## end for my $dir ( qw(done high medium low))

    # Process each item and move if needed
    for my $item ( @items ) {
        my $target_subdir = determine_target_directory( $item );
        my $target_dir    = "$workplan_dir/$target_subdir";

        # Skip if already in correct location
        if ( $item->{ current_subdir } eq $target_subdir ) {
            print STDERR
                "Already in correct location: $item->{prefix}-$item->{num}.md\n";
            next;
        }

        my $filename    = "$item->{prefix}-$item->{num}.md";
        my $source_path = $item->{ file };
        my $target_path = "$target_dir/$filename";

        # Check if target file already exists
        if ( -f $target_path ) {
            print STDERR
                "WARNING: Target file already exists: $target_path\n";
            print STDERR "Skipping move of: $source_path\n";
            next;
        } ## end if ( -f $target_path )

        # Move the file
        if ( move( $source_path, $target_path ) ) {
            print STDERR "Moved: $filename -> $target_subdir/\n";

            # Update the file path in the item for correct linking
            $item->{ file }           = $target_path;
            $item->{ current_subdir } = $target_subdir;
        } ## end if ( move( $source_path...))
        else {
            print STDERR
                "ERROR: Failed to move $source_path to $target_path: $!\n";
        }
    } ## end for my $item ( @items )
} ## end sub reorganize_files

sub generate_work_plan {
    my ( @items ) = @_;

    # Open output file
    open my $out, '>', $output_file
        or die "Can't write $output_file: $!";

    print $out "# Cosplay America Schedule - Work Plan\n\n";
    print $out "Generated on: " . scalar( localtime ) . "\n\n";

    # Separate completed and open items
    my @completed = grep { $_->{ status } eq 'Completed' } @items;
    my @open      = grep { $_->{ status } ne 'Completed' } @items;

    # Collect all links for glossary
    my %all_links;

    # Output completed items as a simple list
    if ( @completed ) {
        print $out "## Completed\n\n";

        for my $item (
            sort {
                       $a->{ prefix } cmp $b->{ prefix }
                    || $a->{ num } <=> $b->{ num }
            } @completed
        ) {
            my $link_id = "$item->{prefix}-$item->{num}";
            $all_links{ $link_id } = get_relative_path( $item );
            print $out "* [$link_id] $item->{summary}\n";
        } ## end for my $item ( sort { $a...})

        print $out "\n---\n\n";
    } ## end if ( @completed )

    # Add summary of todo items as nested list
    if ( @open ) {
        print $out "## Summary of Open Items\n\n";

        print $out "**Total open items:** " . scalar( @open ) . "\n\n";

        # Group open items by priority for summary list
        my %by_priority;
        for my $item ( @open ) {
            push @{ $by_priority{ $item->{ priority } } }, $item;
        }

        # Output summary list by priority as nested list
        for my $priority ( qw(High Medium Low) ) {
            next unless exists $by_priority{ $priority };

            print $out "* **$priority Priority**\n";

            for my $item (
                sort {
                           $a->{ prefix } cmp $b->{ prefix }
                        || $a->{ num } <=> $b->{ num }
                } @{ $by_priority{ $priority } }
            ) {
                my $link_id = "$item->{prefix}-$item->{num}";
                $all_links{ $link_id } = get_relative_path( $item );
                print $out "  * [$link_id] $item->{summary}\n";
            } ## end for my $item ( sort { $a...})

            print $out "\n";
        } ## end for my $priority ( qw(High Medium Low))

        print $out "---\n\n";
    } ## end if ( @open )

    # Group open items by priority
    my %by_priority;
    for my $item ( @open ) {
        push @{ $by_priority{ $item->{ priority } } }, $item;
    }

    # Output each priority section with different heading to avoid duplicates
    for my $priority ( qw(High Medium Low) ) {
        next unless exists $by_priority{ $priority };

        print $out "## Open $priority Priority Items\n\n";

        for my $item ( @{ $by_priority{ $priority } } ) {
            my $link_id = "$item->{prefix}-$item->{num}";
            $all_links{ $link_id } = get_relative_path( $item );

            print $out "### [$link_id] $item->{title}\n\n";

            print $out "**Status:** $item->{status}\n\n";
            print $out "**Summary:** $item->{summary}\n\n";
            print $out "**Description:** $item->{description}\n\n";

            # Add separator, but not after the last item
            if ( $item != $by_priority{ $priority }[ -1 ] ) {
                print $out "---\n\n";
            }
        } ## end for my $item ( @{ $by_priority...})
    } ## end for my $priority ( qw(High Medium Low))

    # Add link glossary at the end (no header to avoid rendering issues)
    print $out "---\n\n";

    for my $link_id ( sort keys %all_links ) {
        print $out "[$link_id]: $all_links{$link_id}\n";
    }

    close $out;
} ## end sub generate_work_plan

sub get_relative_path {
    my ( $item ) = @_;

    # Build relative path based on current subdirectory
    my $filename = "$item->{prefix}-$item->{num}.md";
    if ( $item->{ current_subdir } ) {
        return "work-plan/$item->{current_subdir}/$filename";
    }
    else {
        return "work-plan/$filename";
    }
} ## end sub get_relative_path

sub determine_target_directory {
    my ( $item ) = @_;

    # Use status mapping first
    if ( exists $status_dirs{ $item->{ status } } ) {
        return $status_dirs{ $item->{ status } };
    }

    # Fall back to priority mapping
    if ( exists $priority_defaults{ $item->{ priority } } ) {
        return $priority_defaults{ $item->{ priority } };
    }

    # Default to medium for unknown cases
    return 'medium';
} ## end sub determine_target_directory
