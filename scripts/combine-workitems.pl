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
my $workitem_dir = "$root_dir/docs/work-item";
my $output_file  = "$root_dir/docs/WORK_ITEMS.md";

# Track newest modification time for "Updated on" line
my $newest_mtime = ( stat( $0 ) )[ 9 ];    # Start with script's own mtime

# Default priority mapping for items without explicit status
my %priority_defaults = (
    'High'   => 'high',
    'Medium' => 'medium',
    'Low'    => 'low',
);

# Subdirectories for META prefix items (always go to meta/ regardless of priority)
my %meta_prefixes = map { $_ => 1 } qw(META);

# Subdirectories for organization
my %status_dirs = (
    'Completed'   => 'done',
    'Superseded'  => 'rejected',
    'Rejected'    => 'rejected',
    'In Progress' => \%priority_defaults,
    'Blocked'     => \%priority_defaults,
    'Not Started' => \%priority_defaults,
    'Open'        => \%priority_defaults,
);

# Calculate relative path from output file to workitem directory
my $relative_workitem_path
    = File::Spec->abs2rel( $workitem_dir, "$root_dir/docs" );

# Convert forward slashes to forward slashes for markdown consistency
$relative_workitem_path =~ s/\\/\//g;

# Priority order
my %priority_order = (
    'High'   => 1,
    'Medium' => 2,
    'Low'    => 3,
);

# Read all work item files using recursive search
my @files;
find(
    sub {
        return unless -f && /\.md$/;

        # Track newest modification time
        my $mtime = ( stat( $File::Find::name ) )[ 9 ];
        $newest_mtime = $mtime if $mtime > $newest_mtime;

        # Determine which subdirectory this file is in
        my $relative_path = $File::Find::name;
        $relative_path =~ s/^\Q$workitem_dir\E//;
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
    $workitem_dir
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
    # Normalize bullets to asterisks in entire content
    $content =~ s/^(\s*)[-*](?=\s)/$1\*/gm;

    my ( $title )   = $content =~ /^#\s+(.+)$/m or die "No title in $file";
    my ( $summary ) = $content =~ /## Summary\s*\n(.+?)(?=\n##|\z)/s
        or die "No summary in $file";
    my ( $status ) = $content =~ /## Status\s*\n(.+?)(?=\n##|\z)/s
        or die "No status in $file";
    my ( $priority ) = $content =~ /## Priority\s*\n(.+?)(?=\n##|\z)/s
        or die "No priority in $file";
    my ( $description ) = $content =~ /## Description\s*\n(.+?)(?=\n##|\z)/s
        or die "No description in $file";

    # Extract optional sections (non-fatal if missing)
    my ( $blocked_by_raw )
        = $content =~ /## Blocked By\s*\n(.+?)(?=\n##|\z)/s;
    my ( $work_list ) = $content =~ /## Work Items\s*\n(.+?)(?=\n##|\z)/s;

    # Parse Blocked By into list of IDs — only the leading ID on each bullet line
    my @blocked_by_ids;
    if ( $blocked_by_raw ) {
        while ( $blocked_by_raw =~ /^[-*]\s+([A-Z]+-\d+)\b/mg ) {
            push @blocked_by_ids, $1;
        }
    }

    # Clean up whitespace
    $summary     =~ s/^\s+|\s+$//g;
    $status      =~ s/^\s+|\s+$//g;
    $priority    =~ s/^\s+|\s+$//g;
    $description =~ s/^\s+|\s+$//g;

    # Normalize status and priority to canonical values
    $status   = normalize_status( $status );
    $priority = normalize_priority( $priority );

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
        blocked_by_ids => \@blocked_by_ids,
        work_list      => $work_list,
        full_content   => $content,
        current_subdir => $file_info->{ subdir },
    };
} ## end for my $file_info ( sort...)

# Sort META items first, then by priority, then by prefix and number
@items = sort {
    my $meta_a   = $meta_prefixes{ $a->{ prefix } } ? 0 : 1;
    my $meta_b   = $meta_prefixes{ $b->{ prefix } } ? 0 : 1;
    my $meta_cmp = $meta_a <=> $meta_b;
    return $meta_cmp if $meta_cmp;

    my $prio_a   = $priority_order{ $a->{ priority } } || 999;
    my $prio_b   = $priority_order{ $b->{ priority } } || 999;
    my $prio_cmp = $prio_a <=> $prio_b;
    return $prio_cmp if $prio_cmp;

    # If same priority, sort by prefix then number
    return $a->{ prefix } cmp $b->{ prefix }
        if $a->{ prefix } ne $b->{ prefix };
    return $a->{ num } <=> $b->{ num };
} @items;

# Build two maps from each open META item's Work Items list:
#   %meta_parent_of  : child_id => [parent META IDs]
#                      Used to label non-META items with their phase tracker.
#   %open_item_ids   : quick lookup of all open item IDs
my %meta_parent_of;
my %open_item_ids;
for my $item ( @items ) {
    next if is_closed_status( $item->{ status } );
    my $id = sprintf( "%s-%03d", $item->{ prefix }, $item->{ num } );
    $open_item_ids{ $id } = $item;
}
for my $item ( @items ) {
    next unless $meta_prefixes{ $item->{ prefix } };
    next if is_closed_status( $item->{ status } );
    next unless $item->{ work_list };
    my $parent_id = sprintf( "%s-%03d", $item->{ prefix }, $item->{ num } );
    while ( $item->{ work_list } =~ /^[-*]\s+([A-Z]+-\d+)\b/mg ) {
        my $child_id = $1;
        push @{ $meta_parent_of{ $child_id } }, $parent_id;
    }
} ## end for my $item ( @items )

# Annotate items using the maps:
# - Non-META items: store their parent META IDs as 'meta_parent_ids' (label only)
# - META items: open work-list children that are also META block the parent
#   (e.g. META-001 is blocked by META-025..031); inject into blocked_by_ids.
for my $item ( @items ) {
    my $item_id = sprintf( "%s-%03d", $item->{ prefix }, $item->{ num } );
    if ( !$meta_prefixes{ $item->{ prefix } } ) {

        # Non-META: label with parent phase tracker(s)
        my @parents = @{ $meta_parent_of{ $item_id } // [] };
        $item->{ meta_parent_ids } = \@parents if @parents;
    } ## end if ( !$meta_prefixes{ ...})
    else {
        # META: inject open META work-list children as Blocked By
        next unless $item->{ work_list };
        my %existing = map { $_ => 1 } @{ $item->{ blocked_by_ids } };
        while ( $item->{ work_list } =~ /^[-*]\s+([A-Z]+-\d+)\b/mg ) {
            my $child_id = $1;
            my ( $cpfx ) = $child_id =~ /^([A-Z]+)-/;
            next unless $meta_prefixes{ $cpfx };        # only META children
            next unless $open_item_ids{ $child_id };    # only open ones
            push @{ $item->{ blocked_by_ids } }, $child_id
                unless $existing{ $child_id }++;
        } ## end while ( $item->{ work_list...})
    } ## end else [ if ( !$meta_prefixes{ ...})]
} ## end for my $item ( @items )

# Reorganize files to correct directories first
reorganize_files();

# Generate WORK_ITEMS.md content
my $new_content = generate_work_item_content( @items );

# Check if existing file is identical (ignoring "Updated on" timestamp line)
my $write_needed = 1;
if ( -f $output_file ) {
    my $existing_content = do {
        local $/;
        open my $fh, '<', $output_file or die "Can't read $output_file: $!";
        <$fh>;
    };

    # Normalize both contents: remove "Updated on" line and trailing whitespace
    my $normalized_existing = $existing_content;
    $normalized_existing =~ s/^Updated on: .*\n\n//m;
    $normalized_existing =~ s/\n\s*$/\n/;

    my $normalized_new = $new_content;
    $normalized_new =~ s/^Updated on: .*\n\n//m;
    $normalized_new =~ s/\n\s*$/\n/;

    if ( $normalized_existing eq $normalized_new ) {
        $write_needed = 0;
    }
} ## end if ( -f $output_file )

if ( $write_needed ) {
    open my $fh_out, '>', $output_file
        or die "Can't write $output_file: $!";
    print $fh_out $new_content;
    close $fh_out;
    print STDERR "Updated $output_file with "
        . scalar( @items )
        . " work items\n";
} ## end if ( $write_needed )
else {
    print STDERR "$output_file is up to date ("
        . scalar( @items )
        . " work items)\n";
}

sub reorganize_files {
    print STDERR "Reorganizing files to correct directories...\n";

    # Ensure target directories exist
    for my $dir ( qw(done rejected meta high medium low) ) {
        my $full_dir = "$workitem_dir/$dir";
        unless ( -d $full_dir ) {
            make_path( $full_dir )
                or die "Cannot create directory $full_dir: $!";
            print STDERR "Created directory: $full_dir\n";
        }
    } ## end for my $dir ( qw(done rejected meta high medium low))

    # Process each item and move if needed
    for my $item ( @items ) {
        my $target_subdir = determine_target_directory( $item );
        my $target_dir    = "$workitem_dir/$target_subdir";

        # Skip if already in correct location
        if ( $item->{ current_subdir } eq $target_subdir ) {
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

sub generate_work_item_content {
    my ( @items ) = @_;

    my $content = '';

    $content .= "# Cosplay America Schedule - Work Item\n\n";
    $content
        .= "Updated on: " . scalar( localtime( $newest_mtime ) ) . "\n\n";

    # Separate completed, superseded/rejected, and open items
    my @completed = grep { $_->{ status } eq 'Completed' } @items;
    my @rejected  = grep {
               $_->{ status } eq 'Superseded'
            || $_->{ status } eq 'Rejected'
    } @items;
    my @open = grep {
               $_->{ status } ne 'Completed'
            && $_->{ status } ne 'Superseded'
            && $_->{ status } ne 'Rejected'
    } @items;

    # Collect all links for glossary
    my %all_links;

    # Track numbering conflicts and used IDs
    my %conflicts;
    my %used_ids;
    my %prefix_groups;

    for my $item ( @items ) {
        my $id     = $item->{ num };
        my $prefix = $item->{ prefix };

        # Track used IDs
        if ( exists $used_ids{ $id } ) {
            push @{ $conflicts{ $id } }, $item;
        }
        else {
            $used_ids{ $id } = $item;
        }

        # Group by prefix
        push @{ $prefix_groups{ $prefix } }, $item;
    } ## end for my $item ( @items )

    # Output completed items as a simple list
    if ( @completed ) {
        $content .= "## Completed\n\n";

        for my $item (
            sort {
                       $a->{ prefix } cmp $b->{ prefix }
                    || $a->{ num } <=> $b->{ num }
            } @completed
        ) {
            my $link_id = "$item->{prefix}-$item->{num}";
            $all_links{ $link_id } = get_relative_path( $item );
            $content .= "* [$link_id] $item->{summary}\n";
        } ## end for my $item ( sort { $a...})

        $content .= "\n---\n\n";
    } ## end if ( @completed )

    # Output superseded/rejected items as a simple list
    if ( @rejected ) {
        $content .= "## Superseded / Rejected\n\n";

        for my $item (
            sort {
                       $a->{ prefix } cmp $b->{ prefix }
                    || $a->{ num } <=> $b->{ num }
            } @rejected
        ) {
            my $link_id = "$item->{prefix}-$item->{num}";
            $all_links{ $link_id } = get_relative_path( $item );
            $content .= "* [$link_id] ($item->{status}) $item->{summary}\n";
        } ## end for my $item ( sort { $a...})

        $content .= "\n---\n\n";
    } ## end if ( @rejected )

    # Add summary of todo items as nested list
    if ( @open ) {
        $content .= "## Summary of Open Items\n\n";

        $content .= "**Total open items:** " . scalar( @open ) . "\n\n";

        # Group open items by priority for summary list
        my %by_priority;
        for my $item ( @open ) {
            push @{ $by_priority{ $item->{ priority } } }, $item;
        }

        # Separate META items for their own section in summary
        my @meta_open
            = grep { $meta_prefixes{ $_->{ prefix } } } @open;
        my @nonmeta_open
            = grep { !$meta_prefixes{ $_->{ prefix } } } @open;

        # Output META items first
        if ( @meta_open ) {
            $content .= "* **Meta / Project-Level**\n";
            for my $item ( sort { $a->{ num } <=> $b->{ num } } @meta_open ) {
                my $link_id = "$item->{prefix}-$item->{num}";
                $all_links{ $link_id } = get_relative_path( $item );
                my $blocked_suffix = '';
                if ( @{ $item->{ blocked_by_ids } } ) {
                    my @refs = map { "[$_]" } @{ $item->{ blocked_by_ids } };
                    $blocked_suffix
                        = ' (Blocked by ' . join( ', ', @refs ) . ')';
                }
                $content
                    .= "  * [$link_id] $item->{summary}$blocked_suffix\n";
            } ## end for my $item ( sort { $a...})
            $content .= "\n";
        } ## end if ( @meta_open )

        # Output summary list by priority as nested list
        my %nonmeta_by_priority;
        for my $item ( @nonmeta_open ) {
            push @{ $nonmeta_by_priority{ $item->{ priority } } }, $item;
        }

        for my $priority ( qw(High Medium Low) ) {
            next unless exists $nonmeta_by_priority{ $priority };

            $content .= "* **$priority Priority**\n";

            for my $item (
                sort {
                           $a->{ prefix } cmp $b->{ prefix }
                        || $a->{ num } <=> $b->{ num }
                } @{ $nonmeta_by_priority{ $priority } }
            ) {
                my $link_id = "$item->{prefix}-$item->{num}";
                $all_links{ $link_id } = get_relative_path( $item );
                my $parent_prefix = '';
                if ( @{ $item->{ meta_parent_ids } // [] } ) {
                    my @refs = map { "[$_]" } @{ $item->{ meta_parent_ids } };
                    $parent_prefix = '(' . join( ', ', @refs ) . ') ';
                }
                $content .= "  * [$link_id] $parent_prefix$item->{summary}\n";
            } ## end for my $item ( sort { $a...})

            $content .= "\n";
        } ## end for my $priority ( qw(High Medium Low))

        $content .= "---\n\n";
    } ## end if ( @open )

    # Add next available IDs section
    $content .= "## Next Available IDs\n\n";
    $content .= "The following ID numbers are available for new items:\n\n";

    # Find max ID used across all items
    my $max_id = 0;
    my %all_used_ids;
    for my $item ( @items ) {
        my $id = $item->{ num };

        # Convert to integer for proper comparison
        my $int_id = int( $id );
        $all_used_ids{ $int_id } = 1;
        $max_id = $int_id if $int_id > $max_id;
    } ## end for my $item ( @items )

    # Calculate how many IDs to show (at least 10 more than number of conflicts)
    my $conflict_count = scalar( keys %conflicts );
    my $min_count      = $conflict_count + 10;
    my $count_to_show  = $min_count > 10 ? $min_count : 10;

    # Find available IDs with zero padding
    my @available;
    my $check_id = 1;
    while ( @available < $count_to_show ) {
        if ( !exists $all_used_ids{ $check_id } ) {
            push @available, sprintf( "%03d", $check_id );
        }
        $check_id++;
    } ## end while ( @available < $count_to_show)

    $content .= "**Available:** " . join( ", ", @available ) . "\n\n";
    $content .= "**Highest used:** $max_id\n\n";
    $content .= "---\n\n";

    # Add numbering conflicts section if any exist
    if ( %conflicts ) {

        # Filter to only show actual conflicts (IDs with 2+ items)
        my %actual_conflicts;
        for my $conflict_id ( keys %conflicts ) {

            # Count total items with this ID (first item in used_ids + conflicts)
            my $total_count = 1 + scalar( @{ $conflicts{ $conflict_id } } );
            if ( $total_count >= 2 ) {

                # Check if any of the conflicting items are open
                my $has_open_items = 0;
                if ( !is_closed_status(
                    $used_ids{ $conflict_id }->{ status }
                ) ) {
                    $has_open_items = 1;
                }
                for my $item ( @{ $conflicts{ $conflict_id } } ) {
                    if ( !is_closed_status( $item->{ status } ) ) {
                        $has_open_items = 1;
                        last;
                    }
                } ## end for my $item ( @{ $conflicts...})

                # Only include conflicts that have at least one open item
                if ( $has_open_items ) {
                    $actual_conflicts{ $conflict_id } = [
                        $used_ids{ $conflict_id },        # Add the first item
                        @{ $conflicts{ $conflict_id } }   # Add the rest
                    ];
                } ## end if ( $has_open_items )
            } ## end if ( $total_count >= 2)
        } ## end for my $conflict_id ( keys...)

        if ( %actual_conflicts ) {
            $content .= "### Numbering Conflicts\n\n";
            $content
                .= "The following ID numbers are used by multiple items:\n\n";

            for my $conflict_id ( sort { $a <=> $b } keys %actual_conflicts )
            {
                $content .= "#### Suffix `"
                    . sprintf( "%03d", $conflict_id ) . "`\n\n";
                for my $item ( @{ $actual_conflicts{ $conflict_id } } ) {
                    my $status_icon
                        = is_closed_status( $item->{ status } )
                        ? '✓'
                        : '○';
                    my $display_id = sprintf(
                        "%s-%03d", $item->{ prefix },
                        $item->{ num }
                    );

                    $content
                        .= "* $status_icon [$display_id] $item->{title}\n";
                } ## end for my $item ( @{ $actual_conflicts...})
                $content .= "\n";
            } ## end for my $conflict_id ( sort...)
            $content .= "---\n\n";
        } ## end if ( %actual_conflicts)
    } ## end if ( %conflicts )

    # Group open items by prefix for detailed view
    my %open_by_prefix;
    for my $item ( @open ) {
        push @{ $open_by_prefix{ $item->{ prefix } } }, $item;
    }

    # Output each prefix section
    for my $prefix ( sort keys %open_by_prefix ) {
        $content .= "## Open $prefix Items\n\n";

        for my $item ( @{ $open_by_prefix{ $prefix } } ) {
            my $link_id = "$item->{prefix}-$item->{num}";
            $all_links{ $link_id } = get_relative_path( $item );

            $content .= "### [$link_id] $item->{title}\n\n";

            $content .= "**Status:** $item->{status}\n\n";
            $content .= "**Priority:** $item->{priority}\n\n";
            $content .= "**Summary:** $item->{summary}\n\n";
            if ( @{ $item->{ meta_parent_ids } // [] } ) {
                my @refs = map { "[$_]" } @{ $item->{ meta_parent_ids } };
                $content .= "**Part of:** " . join( ', ', @refs ) . "\n\n";
            }
            elsif ( @{ $item->{ blocked_by_ids } } ) {
                my @refs = map { "[$_]" } @{ $item->{ blocked_by_ids } };
                $content .= "**Blocked By:** " . join( ', ', @refs ) . "\n\n";
            }
            $content .= "**Description:** $item->{description}\n\n";
            if ( $item->{ work_list } ) {
                ( my $wl = $item->{ work_list } ) =~ s/\s+$//;
                $content .= "**Work Items:**\n\n$wl\n\n";
            }

            # Add separator, but not after the last item in this prefix
            if ( $item != $open_by_prefix{ $prefix }[ -1 ] ) {
                $content .= "---\n\n";
            }
        } ## end for my $item ( @{ $open_by_prefix...})
        $content .= "---\n\n";
    } ## end for my $prefix ( sort keys...)

    # Add link glossary at the end (no header to avoid rendering issues)
    $content .= "---\n\n";

    for my $link_id ( sort keys %all_links ) {
        $content .= "[$link_id]: $all_links{$link_id}\n";
    }

    # Trim trailing whitespace and ensure single trailing newline
    $content =~ s/\n\s*$/\n/;

    return $content;
} ## end sub generate_work_item_content

sub get_relative_path {
    my ( $item ) = @_;

    # Build relative path based on current subdirectory
    my $filename = "$item->{prefix}-$item->{num}.md";
    if ( $item->{ current_subdir } ) {
        return "work-item/$item->{current_subdir}/$filename";
    }
    else {
        return "work-item/$filename";
    }
} ## end sub get_relative_path

sub determine_target_directory {
    my ( $item ) = @_;

    # META-prefix items always go to meta/ (unless closed)
    if ( $meta_prefixes{ $item->{ prefix } } ) {
        my $closed_dir = $status_dirs{ $item->{ status } };
        return $closed_dir if defined $closed_dir && !ref $closed_dir;
        return 'meta';
    }

    my $target_dir = $status_dirs{ $item->{ status } } // \%priority_defaults;
    $target_dir = $target_dir->{ $item->{ priority } } if ref $target_dir;
    return $target_dir // 'medium';
} ## end sub determine_target_directory

sub is_closed_status {
    my ( $status ) = @_;
    my $dir = $status_dirs{ $status };
    return defined $dir && !ref $dir;
}

{
    # Case-insensitive normalization maps for status and priority
    my %status_normalize = map { lc $_ => $_ } (
        'Completed',   'done',    'finished', 'complete',
        'In Progress', 'started', 'working',
        'Not Started', 'open',    'new', 'todo',
        'Blocked',     'waiting',
        'Superseded',  'replaced',
        'Rejected',    'declined', 'wontfix',
    );

    # Map synonyms to canonical values
    @status_normalize{ 'done', 'finished', 'complete' } = ( 'Completed' ) x 3;
    @status_normalize{ 'started', 'working' } = ( 'In Progress' ) x 2;
    @status_normalize{ 'open', 'new', 'todo' } = ( 'Not Started' ) x 3;
    @status_normalize{ 'waiting' }  = 'Blocked';
    @status_normalize{ 'replaced' } = 'Superseded';
    @status_normalize{ 'declined', 'wontfix' } = ( 'Rejected' ) x 2;

    my %priority_normalize = map { lc $_ => $_ } ( 'High', 'Medium', 'Low' );
    @priority_normalize{ 'hi', 'critical', 'urgent' } = ( 'High' ) x 3;
    @priority_normalize{ 'mid', 'med', 'normal' }     = ( 'Medium' ) x 3;
    @priority_normalize{ 'lo', 'minor' }              = ( 'Low' ) x 2;

    sub normalize_status {
        my ( $raw ) = @_;

        # Try exact match first, then case-insensitive lookup
        return $raw if exists $status_dirs{ $raw };
        my $key = lc $raw;

        # Strip trailing punctuation and " - ..." suffixes for matching
        ( my $base = $key ) =~ s/\s*[-:].*//;
        return $status_normalize{ $key } // $status_normalize{ $base }
            // $raw;
    } ## end sub normalize_status

    sub normalize_priority {
        my ( $raw ) = @_;
        return $raw if exists $priority_order{ $raw };
        my $key = lc $raw;
        return $priority_normalize{ $key } // $raw;
    } ## end sub normalize_priority
}
