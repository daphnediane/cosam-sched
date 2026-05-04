#!/usr/bin/env perl

# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
# See LICENSE file for full license text

use v5.38;
use feature qw{ signatures };
use utf8;

use open qw[ :std :encoding(UTF-8) ];
use File::Find;
use File::Copy qw[ move ];
use FindBin;
use File::Spec;
use File::Path   qw[ make_path ];
use Getopt::Long qw[ GetOptions ];
use Readonly;

# Get script directory and project root
Readonly my $SCRIPT_DIR => $FindBin::Bin;
Readonly my $ROOT_DIR =>
    File::Spec->catdir( $SCRIPT_DIR, File::Spec->updir() );

# Configuration
Readonly my $WORKITEM_DIR =>
    File::Spec->catdir( $ROOT_DIR, qw[ docs work-item ] );
Readonly my $OUTPUT_FILE_WORK =>
    File::Spec->catdir( $ROOT_DIR, qw[ docs WORK_ITEMS.md ] );
Readonly my $OUTPUT_FILE_IDEAS =>
    File::Spec->catdir( $ROOT_DIR, qw[ docs FUTURE_IDEAS.md ] );

# All valid subdirectories
Readonly my $SUBDIR_STATUS_DONE        => q{done};
Readonly my $SUBDIR_STATUS_REJECTED    => q{rejected};
Readonly my $SUBDIR_STATUS_SUPERSEDED  => q{superseded};
Readonly my $SUBDIR_STATUS_PLACEHOLDER => q{new};
Readonly my $SUBDIR_PREFIX_META        => q{meta};
Readonly my $SUBDIR_PREFIX_IDEA        => q{idea};
Readonly my $SUBDIR_PRIORITY_HIGH      => q{high};
Readonly my $SUBDIR_PRIORITY_MEDIUM    => q{medium};
Readonly my $SUBDIR_PRIORITY_LOW       => q{low};

Readonly::Hash my %SUBDIR_RANK => (
    $SUBDIR_STATUS_REJECTED    => 8,
    $SUBDIR_STATUS_SUPERSEDED  => 7,
    $SUBDIR_STATUS_DONE        => 6,
    $SUBDIR_STATUS_PLACEHOLDER => 5,
    $SUBDIR_PREFIX_META        => 4,
    $SUBDIR_PREFIX_IDEA        => 3,
    $SUBDIR_PRIORITY_HIGH      => 2,
    $SUBDIR_PRIORITY_MEDIUM    => 1,
    $SUBDIR_PRIORITY_LOW       => 0,
);

# All known valid prefixes for --create validation
Readonly::Hash my %PREFIXES => (
    (   map { uc $_ => { name => $_, work => 1 } }
            qw[
            FEATURE BUGFIX UI EDITOR CLI DEPLOY CLEANUP
            PERFORMANCE DOCS REFACTOR TEST
            ]
    ),
    META => {
        name => q{META},
        work => 1,
        meta => 1,
    },
    IDEA => {
        name => q{IDEA},
        idea => 1
    }
);
Readonly::Hash my %PREFIX_TO_DIR => (
    META => $SUBDIR_PREFIX_META,
    IDEA => $SUBDIR_PREFIX_IDEA,
);

# All known valid status values
Readonly my $STATUS_DONE        => q{completed};
Readonly my $STATUS_SUPERSEDED  => q{superseded};
Readonly my $STATUS_REJECTED    => q{rejected};
Readonly my $STATUS_PLACEHOLDER => q{placeholder};
Readonly my $STATUS_IN_PROGRESS => q{in progress};
Readonly my $STATUS_BLOCKED     => q{blocked};
Readonly my $STATUS_OPEN        => q{open};

Readonly::Hash my %ALIAS_TO_STATUS => _populate_alias_hash( {
    $STATUS_DONE        => [ qw{done finished complete closed} ],
    $STATUS_SUPERSEDED  => [ qw{replaced} ],
    $STATUS_REJECTED    => [ qw{declined wontfix} ],
    $STATUS_PLACEHOLDER => [ qw{stub template} ],
    $STATUS_IN_PROGRESS => [ qw{started working} ],
    $STATUS_BLOCKED     => [ qw{waiting} ],
    $STATUS_OPEN        => [ qw{new todo}, q{not started} ],
} );
Readonly::Hash my %STATUS_TO_OPEN => _populate_over_alias(
    \%ALIAS_TO_STATUS,
    {   $STATUS_IN_PROGRESS => 1,
        $STATUS_BLOCKED     => 1,
        $STATUS_OPEN        => 1,
    }
);
Readonly::Hash my %STATUS_TO_DIR => _populate_over_alias(
    \%ALIAS_TO_STATUS,
    {   $STATUS_DONE        => $SUBDIR_STATUS_DONE,
        $STATUS_SUPERSEDED  => $SUBDIR_STATUS_SUPERSEDED,
        $STATUS_REJECTED    => $SUBDIR_STATUS_REJECTED,
        $STATUS_PLACEHOLDER => $SUBDIR_STATUS_PLACEHOLDER,
    }
);

Readonly::Hash my %STATUS_VALUES => (
    map { $_ => { name => $_ } } (
        map { $_ => { name => $_ } } $STATUS_DONE, $STATUS_SUPERSEDED,
        $STATUS_REJECTED,
        $STATUS_PLACEHOLDER

    ),
    (   map { $_ => { name => $_, open => 1 } } $STATUS_IN_PROGRESS,
        $STATUS_BLOCKED, $STATUS_OPEN
    ),
);

# Default priority mapping for items with open status
Readonly my $PRIORITY_HIGH   => q{high};
Readonly my $PRIORITY_MEDIUM => q{medium};
Readonly my $PRIORITY_LOW    => q{low};

Readonly::Hash my %ALIAS_TO_PRIORITY => _populate_alias_hash( {
    $PRIORITY_HIGH   => [ qw[ hi critical urgent raise ] ],
    $PRIORITY_MEDIUM => [ qw[ mid med normal default ] ],
    $PRIORITY_LOW    => [ qw[ minor low ] ],
} );
Readonly::Hash my %PRIORITY_TO_DIR => _populate_over_alias(
    \%ALIAS_TO_PRIORITY,
    {   $PRIORITY_HIGH   => $SUBDIR_PRIORITY_HIGH,
        $PRIORITY_MEDIUM => $SUBDIR_PRIORITY_MEDIUM,
        $PRIORITY_LOW    => $SUBDIR_PRIORITY_LOW,
    }
);
Readonly::Hash my %PRIORITY_ORDER => _populate_over_alias(
    \%ALIAS_TO_PRIORITY,
    {   $PRIORITY_HIGH   => 1,
        $PRIORITY_MEDIUM => 2,
        $PRIORITY_LOW    => 3,
    }
);

# Parse command-line options
my @create_tags;
GetOptions( q{create=s} => \@create_tags )
    or die qq{Usage: $0 [--create PREFIX] [--create PREFIX] ...\n};

# Expand comma-separated tags (--create FEATURE,BUGFIX is also allowed)
@create_tags = map { split /,/, $_ } @create_tags;

# Validate tags before doing anything else
my @unknown = grep { !defined $PREFIXES{ uc $_ } } @create_tags;
if ( @unknown ) {
    my @sorted = sort keys %PREFIXES;
    print STDERR q{Unknown prefix(es): }
        . join( q{, }, map { uc $_ } @unknown ) . qq{\n};
    print STDERR q{Supported prefixes: } . join( q{, }, @sorted ) . qq{\n};
    exit 1;
} ## end if ( @unknown )

# Track newest modification time for "Updated on" line
my $newest_mtime = ( stat( $0 ) )[ 9 ];    # Start with script's own mtime

# Calculate relative path from output file to workitem directory
my $relative_workitem_path = File::Spec->abs2rel(
    $WORKITEM_DIR,
    File::Spec->catdir( $ROOT_DIR, qw[ docs ] )
);

# Convert forward slashes to forward slashes for markdown consistency
$relative_workitem_path =~ s/\\/\//g;

# Create placeholder work item files if --create was requested
create_placeholders( @create_tags ) if @create_tags;

# Read all work item files using recursive search
my @files;
find(
    sub {
        # Skip the template directory entirely
        if ( -d && $_ eq 'template' ) {
            $File::Find::prune = 1;
            return;
        }
        return unless -f && /\.md$/;

        # Track newest modification time
        my $mtime = ( stat( $File::Find::name ) )[ 9 ];
        $newest_mtime = $mtime if $mtime > $newest_mtime;

        # Determine which subdirectory this file is in
        my $relative_path = $File::Find::name;
        $relative_path =~ s/^\Q$WORKITEM_DIR\E//;
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
    $WORKITEM_DIR
);

# Parse each file
my @items;
for my $file_info ( sort { $a->{ path } cmp $b->{ path } } @files ) {
    my $file    = $file_info->{ path };
    my $content = do {
        local $/;
        open my $fh, '<', $file or die qq{Can't read $file: $!};
        <$fh>;
    };

    # Extract metadata
    # Normalize bullets to asterisks in entire content
    $content =~ s/^(\s*)[-*](?=\s)/$1\*/gm;

    my ( $title )   = $content =~ /^#\s+(.+)$/m or die qq{No title in $file};
    my ( $summary ) = $content =~ /## Summary\s*\n(.+?)(?=\n##|\z)/s
        or die qq{No summary in $file};
    my ( $status ) = $content =~ /## Status\s*\n(.+?)(?=\n##|\z)/s
        or die qq{No status in $file};
    my ( $priority ) = $content =~ /## Priority\s*\n(.+?)(?=\n##|\z)/s
        or die qq{No priority in $file};
    my ( $description ) = $content =~ /## Description\s*\n(.+?)(?=\n##|\z)/s
        or die qq{No description in $file};

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
    my $meta_a   = $PREFIXES{ uc $a->{ prefix } }->{ meta } ? 0 : 1;
    my $meta_b   = $PREFIXES{ uc $b->{ prefix } }->{ meta } ? 0 : 1;
    my $meta_cmp = $meta_a <=> $meta_b;
    return $meta_cmp if $meta_cmp;

    my $prio_a   = $PRIORITY_ORDER{ $a->{ priority } } || 999;
    my $prio_b   = $PRIORITY_ORDER{ $b->{ priority } } || 999;
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
    next if is_closed_status( $item );
    my $id = sprintf( q{%s-%03d}, $item->{ prefix }, $item->{ num } );
    $open_item_ids{ $id } = $item;
}
for my $item ( @items ) {
    next unless $PREFIXES{ uc $item->{ prefix } }->{ meta };
    next if is_closed_status( $item );
    next unless $item->{ work_list };
    my $parent_id = sprintf( q{%s-%03d}, $item->{ prefix }, $item->{ num } );
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
    my $item_id = sprintf( q{%s-%03d}, $item->{ prefix }, $item->{ num } );
    if ( !$PREFIXES{ uc $item->{ prefix } }->{ meta } ) {

        # Non-META: label with parent phase tracker(s)
        my @parents = @{ $meta_parent_of{ $item_id } // [] };
        $item->{ meta_parent_ids } = \@parents if @parents;
    } ## end if ( !$PREFIXES{ uc $item...})
    else {
        # META: inject open META work-list children as Blocked By
        next unless $item->{ work_list };
        my %existing = map { $_ => 1 } @{ $item->{ blocked_by_ids } };
        while ( $item->{ work_list } =~ /^[-*]\s+([A-Z]+-\d+)\b/mg ) {
            my $child_id = $1;
            my ( $child_prefix ) = $child_id =~ /^([A-Z]+)-/;
            next
                unless $PREFIXES{ uc $child_prefix }->{ meta }
                ;    # only META children
            next unless $open_item_ids{ $child_id };    # only open ones
            push @{ $item->{ blocked_by_ids } }, $child_id
                unless $existing{ $child_id }++;
        } ## end while ( $item->{ work_list...})
    } ## end else [ if ( !$PREFIXES{ uc $item...})]
} ## end for my $item ( @items )

# Reorganize files to correct directories first
reorganize_files();

# Separate IDEA items from regular work items for separate output
my @idea_items = grep { $PREFIXES{ uc $_->{ prefix } }->{ idea } } @items;
my @workitems  = grep { !$PREFIXES{ uc $_->{ prefix } }->{ idea } } @items;

# Generate WORK_ITEMS.md content (excludes IDEA items from display,
# but uses all items for ID pool tracking)
my $new_content = generate_work_item_content( \@items, @workitems );

# Check if existing file is identical (ignoring "Updated on" timestamp line)
my $write_needed = 1;
if ( -f $OUTPUT_FILE_WORK ) {
    my $existing_content = do {
        local $/;
        open my $fh, '<', $OUTPUT_FILE_WORK
            or die qq{Can't read $OUTPUT_FILE_WORK: $!};
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
} ## end if ( -f $OUTPUT_FILE_WORK)

if ( $write_needed ) {
    open my $fh_out, '>', $OUTPUT_FILE_WORK
        or die qq{Can't write $OUTPUT_FILE_WORK: $!};
    print $fh_out $new_content;
    close $fh_out;
    print STDERR qq{Updated $OUTPUT_FILE_WORK with }
        . scalar( @workitems )
        . qq{ work items\n};
} ## end if ( $write_needed )
else {
    print STDERR qq{$OUTPUT_FILE_WORK is up to date (}
        . scalar( @workitems )
        . qq{ work items)\n};
}

# Generate FUTURE_IDEAS.md content
my $new_ideas_content = generate_ideas_content( @items );

# Check if existing ideas file is identical
my $ideas_write_needed = 1;
if ( -f $OUTPUT_FILE_IDEAS ) {
    my $existing_ideas = do {
        local $/;
        open my $fh, '<', $OUTPUT_FILE_IDEAS
            or die qq{Can't read $OUTPUT_FILE_IDEAS: $!};
        <$fh>;
    };
    my $normalized_existing = $existing_ideas;
    $normalized_existing =~ s/^Updated on: .*\n\n//m;
    $normalized_existing =~ s/\n\s*$/\n/;

    my $normalized_new = $new_ideas_content;
    $normalized_new =~ s/^Updated on: .*\n\n//m;
    $normalized_new =~ s/\n\s*$/\n/;

    $ideas_write_needed = 0 if $normalized_existing eq $normalized_new;
} ## end if ( -f $OUTPUT_FILE_IDEAS)

if ( $ideas_write_needed ) {
    open my $fh_out, '>', $OUTPUT_FILE_IDEAS
        or die qq{Can't write $OUTPUT_FILE_IDEAS: $!};
    print $fh_out $new_ideas_content;
    close $fh_out;
    print STDERR qq{Updated $OUTPUT_FILE_IDEAS with }
        . scalar( @idea_items )
        . qq{ ideas\n};
} ## end if ( $ideas_write_needed)
else {
    print STDERR qq{$OUTPUT_FILE_IDEAS is up to date (}
        . scalar( @idea_items )
        . qq{ ideas)\n};
}

sub create_placeholders {
    my ( @tags ) = @_;

    # Read all existing items to find the next available ID
    my %used_ids;
    find(
        sub {
            # Skip the template directory
            if ( -d && $_ eq q{template} ) {
                $File::Find::prune = 1;
                return;
            }
            return unless -f && /\.md$/;
            if ( /([A-Z]+)-0*(\d+)\.md$/ ) {
                $used_ids{ int( $2 ) } = 1;
            }
        },
        $WORKITEM_DIR
    );

    # Ensure the workitem directory exists
    make_path( $WORKITEM_DIR ) unless -d $WORKITEM_DIR;

    # Ensure subdirs exist
    for my $dir ( qw(done rejected meta idea new high medium low) ) {
        my $full_dir = "$WORKITEM_DIR/$dir";
        make_path( $full_dir ) unless -d $full_dir;
    }

    my @created;
    for my $tag ( @tags ) {
        $tag = uc $tag;

        # Find next available ID
        my $next_id = 1;
        $next_id++ while exists $used_ids{ $next_id };
        $used_ids{ $next_id } = 1;    # Reserve it for subsequent tags

        my $num_str  = sprintf( q{%03d}, $next_id );
        my $filename = qq{$tag-$num_str.md};

        # All placeholders start in new/ regardless of prefix
        my $subdir = q{new};
        my $target_path
            = File::Spec->catfile( $WORKITEM_DIR, $subdir, $filename );

        if ( -f $target_path ) {
            print STDERR qq{WARNING: $target_path already exists, skipping\n};
            next;
        }

        # Load template: prefer PREFIX-template.md, fall back to default-template.md
        my $template_dir
            = File::Spec->catdir( $WORKITEM_DIR, qw[ template ] );
        my $prefix_tmpl
            = File::Spec->catfile( $template_dir, qq{$tag-template.md} );
        my $default_tmpl
            = File::Spec->catfile( $template_dir, q{default-template.md} );
        my $tmpl_file
            = -f $prefix_tmpl  ? $prefix_tmpl
            : -f $default_tmpl ? $default_tmpl
            :                    undef;

        my $content;
        if ( $tmpl_file ) {
            $content = do {
                local $/;
                open my $tfh, '<', $tmpl_file
                    or die qq{Can't read template $tmpl_file: $!};
                <$tfh>;
            };

            # Replace placeholder PREFIX-### in title line with actual ID
            $content =~ s{^# [A-Z]+-###:}{# $tag-$num_str:}m;
        } ## end if ( $tmpl_file )
        else {
            $content = <<"END_TEMPLATE";
# $tag-$num_str: Brief title

## Summary

One-line summary

## Status

Placeholder

## Priority

Medium

## Description

[Detailed description]
END_TEMPLATE
        } ## end else [ if ( $tmpl_file ) ]

        open my $fh, '>', $target_path
            or die qq{Can't create $target_path: $!};
        print $fh $content;
        close $fh;

        push @created, $target_path;
        print STDERR qq{Created: $target_path\n};
    } ## end for my $tag ( @tags )

    # Print created filenames to STDOUT for easy capture
    print qq{$_\n} for @created;
} ## end sub create_placeholders

sub reorganize_files {
    print STDERR qq{Reorganizing files to correct directories...\n};

    # Ensure target directories exist
    for my $dir ( qw{done rejected meta idea new high medium low} ) {
        my $full_dir = File::Spec->catdir( $WORKITEM_DIR, $dir );
        unless ( -d $full_dir ) {
            make_path( $full_dir )
                or die qq{Cannot create directory $full_dir: $!};
            print STDERR qq{Created directory: $full_dir\n};
        }
    } ## end for my $dir ( qw{done rejected meta idea new high medium low})

    # Process each item and move if needed
    for my $item ( @items ) {
        my $target_subdir = determine_target_directory( $item );
        my $target_dir = File::Spec->catdir( $WORKITEM_DIR, $target_subdir );

        # Skip if already in correct location
        if ( $item->{ current_subdir } eq $target_subdir ) {
            next;
        }

        my $filename    = qq{$item->{prefix}-$item->{num}.md};
        my $source_path = $item->{ file };
        my $target_path = File::Spec->catfile( $target_dir, $filename );

        # Check if target file already exists
        if ( -f $target_path ) {
            print STDERR
                qq{WARNING: Target file already exists: $target_path\n};
            print STDERR qq{Skipping move of: $source_path\n};
            next;
        } ## end if ( -f $target_path )

        # Move the file
        if ( move( $source_path, $target_path ) ) {
            print STDERR qq{Moved: $filename -> $target_subdir/\n};

            # Update the file path in the item for correct linking
            $item->{ file }           = $target_path;
            $item->{ current_subdir } = $target_subdir;
        } ## end if ( move( $source_path...))
        else {
            print STDERR
                qq{ERROR: Failed to move $source_path to $target_path: $!\n};
        }
    } ## end for my $item ( @items )
} ## end sub reorganize_files

sub generate_work_item_content {
    my ( $all_items_ref, @items ) = @_;

    my $content = '';

    $content .= qq{# Cosplay America Schedule - Work Item\n\n};
    $content
        .= qq{Updated on: } . scalar( localtime( $newest_mtime ) ) . qq{\n\n};

    # Separate completed, superseded/rejected, and open items
    my @completed = grep { is_completed_status( $_ ) } @items;
    my @rejected  = grep { is_rejected_status( $_ ) } @items;
    my @open      = grep { is_open_status( $_ ) } @items;

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
        $content .= qq{## Completed\n\n};

        for my $item (
            sort {
                       $a->{ prefix } cmp $b->{ prefix }
                    || $a->{ num } <=> $b->{ num }
            } @completed
        ) {
            $content .= get_item_summary( $item, \%all_links ) . qq{\n};
        } ## end for my $item ( sort { $a...})

        $content .= qq{\n---\n\n};
    } ## end if ( @completed )

    # Output superseded/rejected items as a simple list
    if ( @rejected ) {
        $content .= qq{## Superseded / Rejected\n\n};

        for my $item (
            sort {
                       $a->{ prefix } cmp $b->{ prefix }
                    || $a->{ num } <=> $b->{ num }
            } @rejected
        ) {
            $content
                .= get_item_summary( $item, \%all_links, status => 1 )
                . qq{\n};
        } ## end for my $item ( sort { $a...})

        $content .= qq{\n---\n\n};
    } ## end if ( @rejected )

    # Add summary of todo items as nested list
    if ( @open ) {
        $content .= qq{## Summary of Open Items\n\n};

        $content .= qq{**Total open items:** } . scalar( @open ) . qq{\n\n};

        # Group open items by priority for summary list
        my %by_priority;
        for my $item ( @open ) {
            push @{ $by_priority{ $item->{ priority } } }, $item;
        }

        # Separate META items for their own section in summary
        my @meta_open
            = grep { $PREFIXES{ uc $_->{ prefix } }->{ meta } } @open;
        my @non_meta_open
            = grep { !$PREFIXES{ uc $_->{ prefix } }->{ meta } } @open;

        # Output META items first
        if ( @meta_open ) {
            $content .= qq{* **Meta / Project-Level**\n};
            for my $item ( sort { $a->{ num } <=> $b->{ num } } @meta_open ) {
                my $suffix;
                if ( @{ $item->{ blocked_by_ids } } ) {
                    my @refs
                        = map { qq{[$_]} } @{ $item->{ blocked_by_ids } };
                    $suffix = q{(Blocked by } . join( q{, }, @refs ) . q{)};
                }
                $content .= q{  } . get_item_summary(
                    $item, \%all_links,
                    suffix => $suffix
                ) . qq{\n};
            } ## end for my $item ( sort { $a...})

            $content .= qq{\n};
        } ## end if ( @meta_open )

        # Output summary list by priority as nested list
        my %non_meta_by_priority;
        for my $item ( @non_meta_open ) {
            push @{ $non_meta_by_priority{ $item->{ priority } } }, $item;
        }

        for my $priority ( $PRIORITY_HIGH, $PRIORITY_MEDIUM, $PRIORITY_LOW ) {
            my $title_priority = to_title_case( $priority );
            next unless exists $non_meta_by_priority{ $priority };

            $content .= qq{* **$title_priority Priority**\n};

            for my $item (
                sort {
                           $a->{ prefix } cmp $b->{ prefix }
                        || $a->{ num } <=> $b->{ num }
                } @{ $non_meta_by_priority{ $priority } }
            ) {
                my $parent_prefix;
                if ( @{ $item->{ meta_parent_ids } // [] } ) {
                    my @refs
                        = map { qq{[$_]} } @{ $item->{ meta_parent_ids } };
                    $parent_prefix = q{(} . join( q{, }, @refs ) . q{)};
                }
                $content .= q{  } . get_item_summary(
                    $item, \%all_links,
                    prefix => $parent_prefix
                ) . qq{\n};
            } ## end for my $item ( sort { $a...})

            $content .= qq{\n};
        } ## end for my $priority ( $PRIORITY_HIGH...)

        $content .= qq{---\n\n};
    } ## end if ( @open )

    # Add placeholder items section
    my @placeholders = grep { is_placeholder_status( $_ ) } @$all_items_ref;
    @placeholders
        = grep { !$PREFIXES{ uc $_->{ prefix } }->{ idea } } @placeholders;
    $content .= qq{## Placeholders\n\n};
    if ( @placeholders ) {
        $content
            .= qq{Stub items in `docs/work-item/new/` awaiting details:\n\n};
        for my $item (
            sort {
                       $a->{ prefix } cmp $b->{ prefix }
                    || $a->{ num } <=> $b->{ num }
            } @placeholders
        ) {
            $content .= get_item_summary( $item, \%all_links ) . qq{\n};
        } ## end for my $item ( sort { $a...})

        $content .= qq{\n};
    } ## end if ( @placeholders )
    else {
        $content .= qq{*No placeholders — all stubs have been promoted.*\n\n};
    }
    $content
        .= qq{Use `perl scripts/work-item-update.pl --create <PREFIX>` to add new stubs.\n\n};
    $content .= qq{---\n\n};

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
                if ( is_open_status( $used_ids{ $conflict_id } ) ) {
                    $has_open_items = 1;
                }
                for my $item ( @{ $conflicts{ $conflict_id } } ) {
                    if ( is_open_status( $item ) ) {
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
            $content .= qq{### Numbering Conflicts\n\n};
            $content
                .= qq{The following ID numbers are used by multiple items:\n\n};

            for my $conflict_id ( sort { $a <=> $b } keys %actual_conflicts )
            {
                $content
                    .= qq{#### Suffix `}
                    . sprintf( q{%03d}, $conflict_id )
                    . qq{`\n\n};
                for my $item ( @{ $actual_conflicts{ $conflict_id } } ) {
                    my $status_icon
                        = is_closed_status( $item )
                        ? '✓'
                        : '○';
                    my $display_id = sprintf(
                        q{%s-%03d}, $item->{ prefix },
                        $item->{ num }
                    );

                    $content
                        .= qq{* $status_icon [$display_id] $item->{title}\n};
                } ## end for my $item ( @{ $actual_conflicts...})

                $content .= qq{\n};
            } ## end for my $conflict_id ( sort...)

            $content .= qq{---\n\n};
        } ## end if ( %actual_conflicts)
    } ## end if ( %conflicts )

    # Group open items by prefix for detailed view
    my %open_by_prefix;
    for my $item ( @open ) {
        push @{ $open_by_prefix{ $item->{ prefix } } }, $item;
    }

    # Output each prefix section
    for my $prefix ( sort keys %open_by_prefix ) {
        $content .= qq{## Open $prefix Items\n\n};

        for my $item ( @{ $open_by_prefix{ $prefix } } ) {
            my $link_ref = get_item_link_ref( $item, \%all_links );

            $content .= qq{### $link_ref $item->{title}\n\n};

            $content
                .= qq{**Status:** }
                . to_title_case( $item->{ status } )
                . qq{\n\n};
            $content
                .= qq{**Priority:** }
                . to_title_case( $item->{ priority } )
                . qq{\n\n};
            $content .= qq{**Summary:** } . $item->{ summary } . qq{\n\n};
            if ( @{ $item->{ meta_parent_ids } // [] } ) {
                my @refs = map { qq{[$_]} } @{ $item->{ meta_parent_ids } };
                $content
                    .= qq{**Part of:** } . join( q{, }, @refs ) . qq{\n\n};
            }
            elsif ( @{ $item->{ blocked_by_ids } } ) {
                my @refs = map { qq{[$_]} } @{ $item->{ blocked_by_ids } };
                $content
                    .= qq{**Blocked By:** } . join( q{, }, @refs ) . qq{\n\n};
            }
            $content .= qq{**Description:** $item->{description}\n\n};
            if ( $item->{ work_list } ) {
                ( my $wl = $item->{ work_list } ) =~ s/\s+$//;
                $content .= qq{**Work Items:**\n\n$wl\n\n};
            }

            # Add separator, but not after the last item in this prefix
            if ( $item != $open_by_prefix{ $prefix }[ -1 ] ) {
                $content .= qq{---\n\n};
            }
        } ## end for my $item ( @{ $open_by_prefix...})

        $content .= qq{---\n\n};
    } ## end for my $prefix ( sort keys...)

    # Add link glossary at the end (no header to avoid rendering issues)
    $content .= qq{---\n\n};

    for my $link_id ( sort keys %all_links ) {
        $content .= qq{[$link_id]: $all_links{$link_id}\n};
    }

    # Trim trailing whitespace and ensure single trailing newline
    $content =~ s/\n\s*$/\n/;

    return $content;
} ## end sub generate_work_item_content

sub generate_ideas_content {
    my ( @all_items ) = @_;

    my @ideas = grep { $PREFIXES{ uc $_->{ prefix } }->{ idea } } @all_items;

    my $content = '';
    $content .= qq{# Future Ideas and Design Notes\n\n};
    $content
        .= qq{Updated on: } . scalar( localtime( $newest_mtime ) ) . qq{\n\n};
    $content
        .= qq{Open design questions, unexplored alternatives, and deferred ideas.\n};
    $content
        .= qq{An IDEA item can be promoted to a work item by renaming it to another prefix\n};
    $content
        .= qq{(e.g. `IDEA-033.md` \x{2192} `REFACTOR-033.md`) while keeping the same number.\n\n};

    my %idea_links;

    my @open_ideas   = grep { is_open_status( $_ ) } @ideas;
    my @closed_ideas = grep { is_closed_status( $_ ) } @ideas;

    if ( @open_ideas ) {
        $content .= qq{## Open Ideas\n\n};
        for my $item ( sort { $a->{ num } <=> $b->{ num } } @open_ideas ) {
            my $link_ref = get_item_link_ref( $item, \%idea_links );
            $content .= qq{### $link_ref $item->{title}\n\n};
            $content .= qq{**Summary:** $item->{summary}\n\n};
            $content .= qq{**Description:** $item->{description}\n\n};
            if ( $item != $open_ideas[ -1 ] ) {
                $content .= qq{---\n\n};
            }
        } ## end for my $item ( sort { $a...})

        $content .= qq{---\n\n};
    } ## end if ( @open_ideas )

    if ( @closed_ideas ) {
        $content .= qq{## Closed Ideas\n\n};
        for my $item ( sort { $a->{ num } <=> $b->{ num } } @closed_ideas ) {
            $content .= get_item_summary( $item, \%idea_links, status => 1 )
                . qq{\n};
        }

        $content .= qq{\n---\n\n};
    } ## end if ( @closed_ideas )

    if ( !@ideas ) {
        $content .= qq{*No ideas recorded yet.*\n\n};
    }

    # Add IDEA placeholder items section
    my @idea_placeholders = grep { is_placeholder_status( $_ ) } @ideas;
    $content .= qq{## Placeholders\n\n};
    $content
        .= qq{Rename `IDEA-###.md` to another prefix to promote an idea.\n\n};
    if ( @idea_placeholders ) {
        $content
            .= qq{Stub ideas in `docs/work-item/new/` awaiting details:\n\n};
        for my $item ( sort { $a->{ num } <=> $b->{ num } }
            @idea_placeholders ) {
            $content .= get_item_summary( $item, \%idea_links ) . qq{\n};
        }

        $content .= qq{\n};
    } ## end if ( @idea_placeholders)
    else {
        $content .= qq{*No IDEA placeholders.*\n\n};
    }
    $content
        .= qq{Use `perl scripts/work-item-update.pl --create IDEA` to add new stubs.\n\n};
    $content .= qq{---\n\n};

    for my $link_id ( sort keys %idea_links ) {
        $content .= qq{[$link_id]: $idea_links{$link_id}\n};
    }

    $content =~ s/\n\s*$/\n/;
    return $content;
} ## end sub generate_ideas_content

sub get_relative_path {
    my ( $item ) = @_;

    # Build relative path based on current subdirectory
    my $filename = qq{$item->{prefix}-$item->{num}.md};
    if ( $item->{ current_subdir } ) {
        return qq{work-item/$item->{current_subdir}/$filename};
    }
    else {
        return qq{work-item/$filename};
    }
} ## end sub get_relative_path

sub to_bracket ( @stuff ) {
    return unless @stuff;
    my $stuff = join q{ }, grep { defined && length $_ } @stuff;
    return q{[} . $stuff . q{]} if $stuff ne q{};
    return;
} ## end sub to_bracket

sub to_paren ( @stuff ) {
    return unless @stuff;
    my $stuff = join q{ }, grep { defined && length $_ } @stuff;
    return q{(} . $stuff . q{)} if $stuff ne q{};
    return;
} ## end sub to_paren

sub get_item_link_ref ( $item, $glossary_ref ) {
    my $link_id = join q{-}, $item->{ prefix }, $item->{ num };

    return to_bracket( $link_id ) . to_paren( get_relative_path( $item ) )
        unless defined $glossary_ref;

    $glossary_ref->{ $link_id } //= get_relative_path( $item );
    return to_bracket( $link_id );

} ## end sub get_item_link_ref

sub get_item_summary ( $item, $glossary_ref, %flags ) {
    my @fields;

    push @fields, q{*};
    push @fields, get_item_link_ref( $item, $glossary_ref );
    push @fields,
        to_paren( to_title_case( $item->{ status } ) )
        if $flags{ status };
    push @fields, $flags{ prefix } if defined $flags{ prefix };
    push @fields, $item->{ summary };
    push @fields, $flags{ suffix } if defined $flags{ suffix };

    $_ =~ s{\A\s+}{}xmsg for @fields;
    $_ =~ s{\s+\z}{}xmsg for @fields;

    return join q{ }, @fields;
} ## end sub get_item_summary

sub determine_target_directory {
    my ( $item ) = @_;

    my @check_dirs = grep { defined } (
        $PREFIX_TO_DIR{ uc $item->{ prefix } },
        $STATUS_TO_DIR{ $item->{ status } },
        $PRIORITY_TO_DIR{ $item->{ priority } },
    );
    push @check_dirs, $STATUS_PLACEHOLDER unless @check_dirs;
    my ( $dir )
        = sort { $SUBDIR_RANK{ $b } <=> $SUBDIR_RANK{ $a } } @check_dirs;
    return $dir;
} ## end sub determine_target_directory

sub to_title_case ( $value ) {
    $value =~ s{\b{wb}(\w)}{\u$1}xms;
    return $value;
}

sub is_open_status ( $status ) {
    $status = $status->{ status } if ref $status;
    $status = lc $status;
    return 1 if $STATUS_TO_OPEN{ $status };
    return 0;
} ## end sub is_open_status

sub is_closed_status ( $status ) {
    $status = $status->{ status } if ref $status;
    $status = lc $status;
    return 1 unless $STATUS_TO_OPEN{ $status };
    return 0;
} ## end sub is_closed_status

sub is_completed_status ( $status ) {
    $status = $status->{ status } if ref $status;
    $status = lc $status;
    return 0 if $STATUS_TO_OPEN{ $status };
    return 1 if $STATUS_TO_DIR{ $status } eq $SUBDIR_STATUS_DONE;
    return 0;
} ## end sub is_completed_status

sub is_rejected_status ( $status ) {
    $status = $status->{ status } if ref $status;
    $status = lc $status;
    return 0 if $STATUS_TO_OPEN{ $status };
    return 1 if $STATUS_TO_DIR{ $status } eq $SUBDIR_STATUS_REJECTED;
    return 0;
} ## end sub is_rejected_status

sub is_placeholder_status ( $status ) {
    $status = $status->{ status } if ref $status;
    $status = lc $status;
    return 0 if $STATUS_TO_OPEN{ $status };
    return 1 unless defined $STATUS_TO_DIR{ $status };
    return 1 if $STATUS_TO_DIR{ $status } eq $SUBDIR_STATUS_PLACEHOLDER;
    return 0;
} ## end sub is_placeholder_status

sub _populate_alias_hash( $mapping ) {
    my @res;
    foreach my $status ( keys %{ $mapping } ) {
        push @res,
            ( $status => $status ),
            map { $_ => $status } @{ $mapping->{ $status } };
    }
    @res;
} ## end sub _populate_alias_hash

sub _populate_over_alias( $alias_to_status, $mapping ) {
    my @res;
    foreach my $alias ( keys %{ $alias_to_status } ) {
        my $status = $alias_to_status->{ $alias };
        next unless exists $mapping->{ $status };
        my $value = $mapping->{ $status };
        push @res, ( $alias => $value );
    } ## end foreach my $alias ( keys %{...})
    @res;
} ## end sub _populate_over_alias

sub normalize_status {
    my ( $status ) = @_;
    return $ALIAS_TO_STATUS{ lc $status } || $status;
}

sub normalize_priority {
    my ( $priority ) = @_;
    return $ALIAS_TO_PRIORITY{ lc $priority } || $priority;
}

1;
