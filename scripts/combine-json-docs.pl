#!/usr/bin/env perl

# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
# See LICENSE file for full license text
use strict;
use warnings;
use FindBin;
use File::Spec;
use File::Path qw(make_path);

# Get script directory and project root
my $script_dir = $FindBin::Bin;
my $root_dir   = "$script_dir/..";

# Configuration
my $json_schedule_dir = "$root_dir/docs/json-schedule";
my $output_dir        = "$root_dir/docs";

# Calculate relative path from output file to json-schedule directory
my $relative_json_path
    = File::Spec->abs2rel( $json_schedule_dir, $output_dir );

# Convert forward slashes to forward slashes for markdown consistency
$relative_json_path =~ s/\\/\//g;

# Discover versions and their structures dynamically
my %versions = discover_versions( $json_schedule_dir );

# Process each version
for my $version ( sort keys %versions ) {
    my $config = $versions{ $version };

    print STDERR "Processing $version...\n";

    # Read entry file
    my $entry_file    = "$json_schedule_dir/$config->{entry_file}";
    my $entry_content = read_file( $entry_file );

    # Extract sections from entry file
    my ( $top_level_section ) = $entry_content =~ /```json\s*\n(.+?)\n```/s;
    my ( $structures_section )
        = $entry_content =~ /## Structures\s*\n(.+?)(?=\n##|\z)/s;

    # Read structure files and extract summaries
    my @structure_summaries;
    for my $structure_file ( @{ $config->{ structures } } ) {
        my $file_path = "$json_schedule_dir/$structure_file";
        if ( -f $file_path ) {
            my $content = read_file( $file_path );

            # Extract structure name and description
            my ( $structure_name ) = $content =~ /^#\s+`([^`]+)`/m;
            my ( $first_paragraph )
                = $content =~ /`[^`]+`.*?\n\n(.+?)(?=\n##|\n#|\z)/s;

            if ( $structure_name && $first_paragraph ) {
                $first_paragraph =~ s/^\s+|\s+$//g;

                push @structure_summaries, {
                    name        => $structure_name,
                    file        => $structure_file,
                    description => $first_paragraph,
                    content     => $content
                };
            } ## end if ( $structure_name &&...)
        } ## end if ( -f $file_path )
    } ## end for my $structure_file ...

    # Generate combined document
    my $output_file = "$output_dir/$config->{output_file}";
    open my $out, '>', $output_file
        or die "Can't write $output_file: $!";

    # Write header
    print $out "# $config->{title}\n\n";
    print $out "$config->{description}\n\n";
    print $out
        "This document is generated from the structured documentation in [$relative_json_path]($relative_json_path).\n\n";
    print $out "---\n\n";

    # Write top-level structure
    if ( $top_level_section ) {
        print $out "## Top-Level Structure\n\n";
        print $out "```json\n";
        print $out $top_level_section;
        print $out "\n```\n\n";
    } ## end if ( $top_level_section)

    # Write structures overview
    if ( $structures_section ) {
        print $out "## Structures Overview\n\n";
        print $out $structures_section;
        print $out "\n";
    }

    # Write detailed structure summaries
    print $out "## Structure Details\n\n";

    for my $structure ( @structure_summaries ) {
        my $relative_file = "$relative_json_path/$structure->{file}";

        print $out "### [`$structure->{name}`]($relative_file)\n\n";
        print $out "$structure->{description}\n\n";

        # Extract key information from the structure content
        my ( $access )
            = $structure->{ content }
            =~ /## Access\s*\n(.+?)(?=\n##|\n#|\z)/s;
        my ( $status )
            = $structure->{ content }
            =~ /## Status\s*\n(.+?)(?=\n##|\n#|\z)/s;

        if ( $access && $status ) {
            $access =~ s/^\s+|\s+$//g;
            $status =~ s/^\s+|\s+$//g;

            print $out "**Access:** $access\n\n";
            print $out "**Status:** $status\n\n";
        } ## end if ( $access && $status)

        # Extract field table if present
        my ( $fields_section )
            = $structure->{ content }
            =~ /## Fields\s*\n(.+?)(?=\n##|\n#|\z)/s;
        if ( $fields_section ) {

            # Clean up the fields section for display
            $fields_section =~ s/^\s+|\s+$//g;
            print $out "**Key Fields:**\n\n";

            # Extract just the table part (everything from the header row to end of table)
            my ( $table_only )
                = $fields_section
                =~ /(\| Field[^\n]*\n(?:\|[-\s\|]+\n)?(?:\|.*\n)*)/s;
            if ( $table_only ) {
                print $out $table_only;
            }
            else {
                # Fallback: just print the whole section
                print $out $fields_section;
            }
            print $out "\n";
        } ## end if ( $fields_section )

        print $out
            "*See full details in: [`$structure->{file}`]($relative_file)*\n\n";
    } ## end for my $structure ( @structure_summaries)

    # Add examples section if available in entry file
    my ( $examples_section )
        = $entry_content =~ /## Example(.+?)(?=\n##|\z)/s;
    if ( $examples_section ) {
        print $out "## Complete Example\n\n";
        print $out $examples_section;
        print $out "\n";
    }

    # Add migration notes if available
    my ( $migration_section )
        = $entry_content =~ /## Migration Notes(.+?)(?=\n##|\z)/s;
    if ( $migration_section ) {
        print $out "## Migration Notes\n\n";
        print $out $migration_section;
        print $out "\n";
    }

    # Add footer
    print $out "---\n\n";
    print $out "## Related Documentation\n\n";
    print $out
        "- [JSON Schedule Documentation]($relative_json_path/) - Complete structured documentation\n";

    # Link to other versions
    for my $other_version ( sort keys %versions ) {
        next if $other_version eq $version;
        my $other_config = $versions{ $other_version };
        print $out
            "- [$other_config->{title}]($other_config->{output_file}) - $other_config->{description}\n";
    } ## end for my $other_version (...)

    print $out "\n";
    print $out
        "*This document is automatically generated. Do not edit directly.*\n";

    close $out;

    # Clean up the generated file to fix markdown lint issues
    cleanup_markdown( $output_file );

    print STDERR "Generated $output_file\n";
} ## end for my $version ( sort ...)

print STDERR "JSON format documentation generation complete.\n";

# Helper function to read file
sub read_file {
    my ( $file ) = @_;
    open my $fh, '<', $file or die "Can't read $file: $!";
    local $/;
    my $content = <$fh>;
    close $fh;
    return $content;
} ## end sub read_file

# Discover versions and their structures dynamically
sub discover_versions {
    my ( $json_schedule_dir ) = @_;

    my %versions;

    # Find all version entry files (v#.md and v#-*.md)
    opendir my $dh, $json_schedule_dir
        or die "Can't open $json_schedule_dir: $!";
    my @files = readdir $dh;
    closedir $dh;

    for my $file ( @files ) {

        # Match version entry files: v4.md, v5-private.md, v5-public.md, etc.
        if ( $file =~ /^v(\d+)(?:-.+)?\.md$/ ) {
            my $version_num = $1;
            my $version_key
                = $file =~ /^(v\d+(?:-.+))\.md$/ ? $1 : "v$version_num";

            my $file_path = "$json_schedule_dir/$file";
            my $content   = read_file( $file_path );

            # Extract title and description
            my ( $title )       = $content =~ /^#\s+(.+)$/m;
            my ( $description ) = $content =~ /^#\s+.+?\n\n(.+?)(?=\n##|\z)/s;

            # Clean up description
            $description =~ s/^\s+|\s+$//g if $description;

            # Extract structures from the Structures section
            my @structures;
            my ( $structures_section )
                = $content =~ /## Structures\s*\n(.+?)(?=\n##|\z)/s;

            if ( $structures_section ) {

                # Extract structure references like "- [meta](meta-v5.md)"
                while (
                    $structures_section =~ /\[([^\]]+)\]\(([^)]+\.md)\)/g ) {
                    my $structure_name = $1;
                    my $structure_file = $2;

                    # Only include if the file exists
                    if ( -f "$json_schedule_dir/$structure_file" ) {
                        push @structures, $structure_file;
                    }
                } ## end while ( $structures_section...)
            } ## end if ( $structures_section)

            # Determine output file name
            my $output_file;
            if ( $version_key =~ /^v\d+$/ ) {

                # Simple version like v4 -> json-format-v4.md
                $output_file = "json-format-$version_key.md";
            }
            elsif ( $version_key =~ /^v\d+-private$/ ) {

                # Private variant -> json-private-v5.md
                $output_file = "json-private-v$version_num.md";
            }
            elsif ( $version_key =~ /^v\d+-public$/ ) {

                # Public variant -> json-public-v5.md
                $output_file = "json-public-v$version_num.md";
            }
            else {
                # Fallback: use the version key
                $output_file = "json-$version_key.md";
            }

            $versions{ $version_key } = {
                entry_file  => $file,
                output_file => $output_file,
                title       => $title
                    || "Schedule JSON Format $version_key",
                description => $description
                    || "JSON format documentation for $version_key",
                structures => \@structures
            };

            print STDERR
                "Discovered version: $version_key -> $output_file with "
                . scalar( @structures )
                . " structures\n";
        } ## end if ( $file =~ /^v(\d+)(?:-.+)?\.md$/)
    } ## end for my $file ( @files )

    return %versions;
} ## end sub discover_versions

# Clean up markdown to fix lint issues
sub cleanup_markdown {
    my ( $file ) = @_;

    # Read the file
    my $content = read_file( $file );

    # Fix multiple consecutive blank lines (MD012)
    $content =~ s/\n{3,}/\n\n/g;

    # Remove trailing whitespace
    $content =~ s/[ \t]+$//gm;

    # Ensure file ends with single newline
    $content =~ s/\n*$/\n/;

    # Write back the cleaned content
    open my $fh_out, '>', $file or die "Can't write $file: $!";
    print $fh_out $content;
    close $fh_out;
} ## end sub cleanup_markdown
