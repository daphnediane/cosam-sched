# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
# See LICENSE file for full license text

package Convert::GoogleSheets;

use v5.38.0;
use utf8;
use warnings;

use URI;
use JSON        qw{ decode_json };

# Adapter class that makes Google Sheets look like Spreadsheet::ParseXLSX workbook
sub new ( $class, %args ) {
    my $self = bless {
        sheets_api => $args{sheets_api},
        spreadsheet_id => $args{spreadsheet_id},
        worksheets => {},
    }, $class;
    
    $self->_load_worksheets();
    return $self;
}

sub _load_worksheets ($self) {
    my $spreadsheet = $self->{sheets_api}->open_spreadsheet(
        id => $self->{spreadsheet_id}
    );
    
    my $sheets = $spreadsheet->sheets();
    foreach my $sheet (@$sheets) {
        my $worksheet = $spreadsheet->open_worksheet(
            id => $sheet->{properties}->{sheetId}
        );
        $self->{worksheets}->{ $sheet->{properties}->{title} } = 
            Convert::GoogleSheets::Worksheet->new(
                worksheet => $worksheet,
                title => $sheet->{properties}->{title}
            );
    }
}

sub worksheets ($self) {
    return sort keys %{ $self->{worksheets} };
}

sub worksheet ($self, $name) {
    return $self->{worksheets}->{$name};
}

# Find a table by name across all worksheets
sub find_table ($self, $table_name) {
    for my $worksheet_name (keys %{ $self->{worksheets} }) {
        my $worksheet = $self->{worksheets}->{$worksheet_name};
        
        # Try to get tables via API for this worksheet
        my $tables = $self->_get_tables_via_api($worksheet);
        
        for my $table (@$tables) {
            if ($table->{name} && lc $table->{name} eq lc $table_name) {
                # Return a hash with table data and metadata
                return {
                    name => $table->{name},
                    data => $self->_extract_table_data($worksheet, $table),
                    worksheet => $worksheet,
                    table_id => $table->{tableId},
                };
            }
        }
    }
    
    return undef;
}

package Convert::GoogleSheets::Worksheet;

use v5.38.0;
use utf8;
use warnings;

sub new ( $class, %args ) {
    my $self = bless {
        worksheet => $args{worksheet},
        title => $args{title},
        _data => undef,
    }, $class;
    return $self;
}

sub _load_data ($self) {
    return if defined $self->{_data};
    
    my $worksheet = $self->{worksheet};
    
    # Try to get tables using the Google Sheets Tables API first
    my $tables = $self->_get_tables_via_api($worksheet);
    
    if ($tables && @$tables) {
        # Use the first table found (or we could enhance to select by name)
        my $primary_table = $tables->[0];
        $self->{_data} = $self->_extract_table_data($worksheet, $primary_table);
        $self->{_table_info} = {
            source => 'api',
            table  => $primary_table,
        };
        return;
    }
    
    # Fallback to range-based detection if no formal tables exist
    $self->_load_data_via_range_detection($worksheet);
}

# Get tables using the Google Sheets Tables API
sub _get_tables_via_api ($self, $worksheet) {
    eval {
        # Use the REST API directly to get tables for this worksheet
        my $rest_api = $self->{sheets_api}->api();
        my $sheet_id = $self->_get_worksheet_id($worksheet);
        
        return [] unless $sheet_id;
        
        my $response = $rest_api->api(
            uri => "/v4/spreadsheets/" . $self->{spreadsheet_id} . "/sheets/" . $sheet_id . "/tables",
            method => 'get',
        );
        
        return $response->{tables} || [];
    };
    
    # If API call fails, return empty array to trigger fallback
    return [];
}

# Get worksheet ID - this might need adjustment based on the actual API structure
sub _get_worksheet_id ($self, $worksheet) {
    # Try to get the sheet ID from the worksheet object
    if (ref($worksheet) && $worksheet->can('id')) {
        return $worksheet->id();
    }
    
    # Fallback: try to get from properties or other methods
    if (ref($worksheet) && $worksheet->can('properties')) {
        my $props = $worksheet->properties();
        return $props->{sheetId} if $props && $props->{sheetId};
    }
    
    return undef;
}

# Extract data from a formal table definition
sub _extract_table_data ($self, $worksheet, $table) {
    my $range = $table->{range};
    return [] unless $range;
    
    # Convert range indices to A1 notation
    my $start_col = _col_number_to_letter($range->{startColumnIndex} + 1);
    my $end_col = _col_number_to_letter($range->{endColumnIndex});
    my $start_row = $range->{startRowIndex} + 1;
    my $end_row = $range->{endRowIndex};
    
    my $range_spec = "${start_col}${start_row}:${end_col}${end_row}";
    
    my $api_range = $worksheet->range($range_spec);
    return $api_range->values() || [];
}

# Fallback method using range detection for non-table data
sub _load_data_via_range_detection ($self, $worksheet) {
    # Start with a reasonable range and expand if needed
    my $max_rows = 1000;
    my $max_cols = 26;  # A-Z
    
    # Check if we need to expand beyond 1000 rows
    my $expanded_data = _find_data_bounds($worksheet, $max_rows, $max_cols);
    
    # Get all data from the determined range
    my $range_spec = "A1:" . _col_number_to_letter($expanded_data->{cols}) . $expanded_data->{rows};
    my $range = $worksheet->range($range_spec);
    my $raw_data = $range->values();
    
    # Detect and extract the primary data table using heuristics
    my $primary_table = _get_primary_table($raw_data);
    $self->{_data} = $primary_table->{data} || [];
    
    # Store table info for potential future use
    $self->{_table_info} = {
        source => 'detected',
        table  => $primary_table,
    };
}

# Find the actual bounds of data in the worksheet
sub _find_data_bounds ($worksheet, $initial_rows, $initial_cols) {
    my $max_rows = $initial_rows;
    my $max_cols = $initial_cols;
    
    # Check if there's data beyond our initial bounds
    # We can check the last few rows/columns to see if we need to expand
    
    # Check rows beyond 1000
    my $check_rows = 100;
    my $range_spec = "A" . ($max_rows + 1) . ":Z" . ($max_rows + $check_rows);
    
    eval {
        my $range = $worksheet->range($range_spec);
        my $data = $range->values();
        
        # If we found any data, expand our bounds
        for my $i (0 .. $#$data) {
            my $row = $data->[$i];
            if ($row && grep { defined && /\S/ } @$row) {
                $max_rows = $max_rows + $i + 1;
                last;
            }
        }
    };
    
    # Check columns beyond Z (AA, AB, etc.)
    eval {
        my $col_aa = $worksheet->range('AA1:AA100');
        my $data = $col_aa->values();
        
        # If we found data in column AA, expand to include more columns
        for my $row (@$data) {
            if ($row && defined $row->[0] && $row->[0] =~ /\S/) {
                $max_cols = 52;  # Expand to AZ
                last;
            }
        }
    };
    
    return {
        rows => $max_rows,
        cols => $max_cols,
    };
}

# Convert column number to letter (1=A, 26=Z, 27=AA, etc.)
sub _col_number_to_letter ($num) {
    return 'A' if $num == 1;
    
    my $letter = '';
    while ($num > 0) {
        $num--;  # Adjust for 0-based indexing
        $letter = chr(ord('A') + ($num % 26)) . $letter;
        $num = int($num / 26);
    }
    return $letter;
}

# Detect multiple data tables within a worksheet
sub _detect_data_tables ($data) {
    return [] unless $data && @$data;
    
    my @tables;
    my $current_table = undef;
    my $row_idx = 0;
    
    for my $row (@$data) {
        $row_idx++;
        
        # Skip completely empty rows
        next unless $row && grep { defined && /\S/ } @$row;
        
        # Check if this looks like a header row (has non-empty cells)
        my $non_empty_count = grep { defined && /\S/ } @$row;
        
        if ($non_empty_count >= 2) {  # At least 2 non-empty cells suggests a table
            # Start a new table if we don't have one or if there was a gap
            if (!defined $current_table) {
                $current_table = {
                    start_row => $row_idx - 1,  # 0-based
                    end_row   => $row_idx - 1,
                    start_col => 0,
                    end_col   => $#$row,
                    data      => [$row],
                };
            } else {
                # Extend current table
                $current_table->{end_row} = $row_idx - 1;
                $current_table->{end_col} = $#$row if $#$row > $current_table->{end_col};
                push @{ $current_table->{data} }, $row;
            }
        } else {
            # End current table if we hit a row with too few non-empty cells
            if (defined $current_table) {
                push @tables, $current_table;
                $current_table = undef;
            }
        }
    }
    
    # Add the last table if we were still building it
    push @tables, $current_table if defined $current_table;
    
    return \@tables;
}

# For now, use the first table found as the primary data
sub _get_primary_table ($data) {
    my $tables = _detect_data_tables($data);
    return $tables && @$tables ? $tables->[0] : { data => $data };
}

# Trim empty rows and columns from the end of the data
sub _trim_data ($data) {
    return [] unless $data && @$data;
    
    # Find last row with any data
    my $last_row = -1;
    for my $i (0 .. $#$data) {
        my $row = $data->[$i];
        if ($row && grep { defined && /\S/ } @$row) {
            $last_row = $i;
        }
    }
    
    return [] if $last_row == -1;
    
    # Trim rows
    my $trimmed_data = [ @{$data}[0..$last_row] ];
    
    # Find last column with any data
    my $last_col = -1;
    for my $row (@$trimmed_data) {
        next unless $row;
        for my $i (0 .. $#$row) {
            if (defined $row->[$i] && $row->[$i] =~ /\S/) {
                $last_col = $i if $i > $last_col;
            }
        }
    }
    
    # Trim columns if needed
    if ($last_col >= 0) {
        for my $row (@$trimmed_data) {
            next unless $row;
            @$row = @{$row}[0..$last_col];
        }
    }
    
    return $trimmed_data;
}

sub MaxRow ($self) {
    $self->_load_data();
    return scalar @{ $self->{_data} || [] };
}

sub MaxCol ($self) {
    $self->_load_data();
    return 0 unless @{ $self->{_data} || [] };
    
    my $max_col = 0;
    foreach my $row (@{ $self->{_data} }) {
        my $col_count = scalar @{ $row || [] };
        $max_col = $col_count if $col_count > $max_col;
    }
    return $max_col;
}

sub Cell ($self, $row, $col) {
    $self->_load_data();
    
    # Convert to 0-based indices
    $row--;
    $col--;
    
    return undef if $row < 0 || $col < 0;
    return undef if $row >= @{ $self->{_data} || [] };
    return undef if $col >= @{ $self->{_data}->[$row] || [] };
    
    my $value = $self->{_data}->[$row]->[$col];
    
    return Convert::GoogleSheets::Cell->new(
        value => $value,
        row => $row,
        col => $col,
    );
}

sub Name ($self) {
    return $self->{title};
}

package Convert::GoogleSheets::Cell;

use v5.38.0;
use utf8;
use warnings;

sub new ( $class, %args ) {
    return bless {
        value => $args{value},
        row => $args{row},
        col => $args{col},
    }, $class;
}

sub value ($self) {
    return $self->{value};
}

sub Val ($self) {
    return $self->{value};
}

1;
