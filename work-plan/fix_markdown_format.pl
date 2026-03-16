#!/usr/bin/env perl
use strict;
use warnings;
use File::Find;

# Find all markdown files in work-plan directory
my @files;
find(sub {
    return unless -f && /\.md$/;
    return if $File::Find::name =~ /fix_markdown_format\.pl$/;
    push @files, $File::Find::name;
}, 'work-plan');

for my $file (@files) {
    print "Fixing $file...\n";
    
    # Read the file
    my $content = do {
        local $/;
        open my $fh, '<', $file or die "Can't read $file: $!";
        <$fh>;
    };
    
    # Add blank lines before headings (##)
    $content =~ s/([^\n])\n(## )/$1\n\n$2/g;
    
    # Add blank lines after headings if not already present
    $content =~ s/(## .+?)\n(?!\n)/$1\n\n/g;
    
    # Add blank lines before lists if not already present
    $content =~ s/([^\n])\n(\d+\. )/$1\n\n$2/g;
    
    # Write back
    open my $fh, '>', $file or die "Can't write $file: $!";
    print $fh $content;
    close $fh;
}

print "Fixed " . scalar(@files) . " files\n";
