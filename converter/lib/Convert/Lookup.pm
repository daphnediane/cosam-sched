# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
# See LICENSE file for full license text

package Convert::Lookup;

use base qw{Exporter};

use v5.38.0;
use utf8;
use warnings;

use Convert::SheetUtil qw{ find_sheet get_rows };

our @EXPORT_OK = qw{
    find_data_source
    get_data_rows
    get_current_year
    get_year_variants
};

our %EXPORT_TAGS = (
    all => [ @EXPORT_OK ],
);

# Find data source with fallback hierarchy
sub find_data_source ( $wb, $config, $data_type ) {
    my $table_name = $config->{$data_type . '_table'};
    
    # Try table name first (for Google Sheets Tables API)
    if ($table_name) {
        my $data = _try_table_lookup($wb, $table_name);
        return $data if $data;
    }
    
    # Fallback to sheet names based on data type
    my @sheet_names = _get_sheet_fallbacks($data_type);
    
    for my $sheet_name (@sheet_names) {
        my $sheet = find_sheet($wb, $sheet_name);
        if ($sheet) {
            say "Reading Worksheet '${sheet_name}'";
            return $sheet;
        }
    }
    
    # Final fallback for schedule data: use first worksheet
    if ($data_type eq 'schedule') {
        my @worksheets = $wb->worksheets();
        if (@worksheets) {
            say "Reading Worksheet '" . $worksheets[0]->{Name} . "' (first worksheet)";
            return $worksheets[0];
        }
    }
    
    return undef;
}

# Try to find data using Google Sheets Tables API or XLSX named tables
sub _try_table_lookup ( $wb, $table_name ) {
    # Check if the workbook supports table lookup (most flexible approach)
    if ($wb->can('find_table')) {
        my $table = $wb->find_table($table_name);
        if ($table) {
            say "Reading Table '${table_name}'";
            return $table;
        }
    }
    
    # Fall back to sheet lookup
    return find_sheet($wb, $table_name);
}

# Get sheet name fallbacks based on data type
sub _get_sheet_fallbacks ($data_type) {
    if ($data_type eq 'schedule') {
        return qw{ Schedule };
    }
    elsif ($data_type eq 'roommap') {
        return qw{ RoomMap Rooms };
    }
    elsif ($data_type eq 'prefix') {
        return qw{ Prefix PanelTypes };
    }
    
    return ();
}

# Get data rows from a source (sheet or table)
sub get_data_rows ($source) {
    return [] unless $source;
    
    # If it's a table (hash with data), extract table data
    if (ref($source) eq 'HASH' && $source->{data}) {
        return $source->{data};
    }
    
    # Otherwise, treat it as a worksheet and get all rows
    # Note: get_rows returns an array, so we need to call it in list context
    my @rows = get_rows($source);
    return \@rows;
}

# Get current year for year-based fallbacks
sub get_current_year () {
    return (localtime)[5] + 1900;
}

# Generate year-based sheet name variants
sub get_year_variants ($base_name) {
    my $year = get_current_year();
    return (
        "${base_name} ${year}",
        "${year} ${base_name}",
        $base_name,
    );
}

1;
