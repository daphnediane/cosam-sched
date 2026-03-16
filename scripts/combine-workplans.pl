#!/usr/bin/env perl

# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
# See LICENSE file for full license text
use strict;
use warnings;
use File::Find;
use JSON::MaybeXS;

# Configuration
my $workplan_dir = 'work-plan';
my $output_file  = 'WORK_PLAN.md';

# Priority order
my %priority_order = (
    'High'   => 1,
    'Medium' => 2,
    'Low'    => 3,
);

# Read all work plan files
my @files;
find(
    sub {
        return unless -f && /\.md$/;
        return if $File::Find::name =~ /combine_workplans\.pl$/;
        push @files, $File::Find::name;
    },
    $workplan_dir
);

# Parse each file
my @items;
for my $file ( sort @files ) {
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
        file         => $file,
        prefix       => $prefix,
        num          => $num,
        title        => $title,
        summary      => $summary,
        status       => $status,
        priority     => $priority,
        description  => $description,
        full_content => $content,
    };
} ## end for my $file ( sort @files)

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

# Generate WORK_PLAN.md
open my $out, '>', $output_file or die "Can't write $output_file: $!";

print $out "# Cosplay America Schedule - Work Plan\n\n";
print $out "Generated on: " . scalar( localtime ) . "\n\n";

# Separate completed and open items
my @completed = grep { $_->{ status } eq 'Completed' } @items;
my @open      = grep { $_->{ status } ne 'Completed' } @items;

# Output completed items as a simple list
if ( @completed ) {
    print $out "## Completed\n\n";

    for my $item (
        sort {
            $a->{ prefix } cmp $b->{ prefix } || $a->{ num } <=> $b->{ num }
        } @completed
    ) {
        my $relative_file = $item->{ file };
        print $out
            "* [$item->{prefix}-$item->{num}]($relative_file) $item->{summary}\n";
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
            my $relative_file = $item->{ file };
            print $out
                "  * [$item->{prefix}-$item->{num}]($relative_file) $item->{summary}\n";
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
        print $out "### [$item->{prefix}-$item->{num}] $item->{title}\n\n";

        print $out "**Status:** $item->{status}\n\n";
        print $out "**Summary:** $item->{summary}\n\n";
        print $out "**Description:** $item->{description}\n\n";

        # Add link to full file
        my $relative_file = $item->{ file };
        print $out
            "*See full details in: [$relative_file]($relative_file)*\n\n";

        # Add separator, but not after the last item
        if ( $item != $by_priority{ $priority }[ -1 ] ) {
            print $out "---\n\n";
        }
    } ## end for my $item ( @{ $by_priority...})
} ## end for my $priority ( qw(High Medium Low))

close $out;

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
