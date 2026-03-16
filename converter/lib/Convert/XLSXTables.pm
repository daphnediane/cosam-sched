# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
# See LICENSE file for full license text

package Convert::XLSXTables;

use v5.38.0;
use utf8;
use warnings;

use Archive::Zip qw{ :ERROR_CODES :CONSTANTS };
use XML::Simple qw{ XMLin };
use Spreadsheet::ParseXLSX qw{};

# Adapter that adds Excel table support to Spreadsheet::ParseXLSX
sub new ( $class, $filename ) {
    my $self = bless {
        filename => $filename,
        zip => undef,
        workbook => undef,
        tables => undef,
    }, $class;
    
    # Open the XLSX as a ZIP file
    $self->{zip} = Archive::Zip->new($filename);
    die "Cannot open $filename as ZIP archive\n" unless $self->{zip};
    
    # Parse with Spreadsheet::ParseXLSX for basic functionality
    my $parser = Spreadsheet::ParseXLSX->new;
    $self->{workbook} = $parser->parse($filename);
    die "Unable to parse $filename: " . $parser->error() . "\n"
        unless defined $self->{workbook};
    
    # Load Excel tables
    $self->_load_tables();
    
    return $self;
}

sub _load_tables ($self) {
    my $tables = {};
    
    # Look for table definitions in xl/tables/
    my @table_files = grep { /^xl\/tables\/table\d+\.xml$/ } $self->{zip}->memberNames();
    
    for my $table_file (@table_files) {
        my $content = $self->{zip}->contents($table_file);
        next unless $content;
        
        # Use regex to extract table info (simpler than XML parsing for now)
        if ($content =~ /name="([^"]+)"[^>]*ref="([^"]+)"/) {
            my $table_name = $1;
            my $ref = $2;
            
            if ($table_name && $ref) {
                $tables->{$table_name} = {
                    name => $table_name,
                    ref => $ref,
                    columns => {},  # Skip column parsing for now
                };
            }
        }
    }
    
    $self->{tables} = $tables;
}

sub _parse_table_columns ($self, $table_info) {
    my $columns = {};
    
    if ($table_info->{tableColumns} && $table_info->{tableColumns}[0] && $table_info->{tableColumns}[0]{tableColumn}) {
        my $table_columns = $table_info->{tableColumns}[0]{tableColumn};
        $table_columns = [$table_columns] unless ref($table_columns) eq 'ARRAY';
        
        for my $col (@$table_columns) {
            my $id = $col->{id};
            my $name = $col->{name} || '';
            if ($id && $name) {
                $columns->{$id} = $name;
            }
        }
    }
    
    return $columns;
}

# Convert Excel range reference (e.g., "AZ1:BJ10") to row/col indices
sub _parse_range ($self, $ref) {
    return {} unless $ref;
    
    if ($ref =~ /^([A-Z]+)(\d+):([A-Z]+)(\d+)$/) {
        return {
            start_col => $self->_col_letter_to_index($1),
            start_row => $2 - 1,  # Convert to 0-based
            end_col   => $self->_col_letter_to_index($3),
            end_row   => $4 - 1,  # Convert to 0-based
        };
    }
    
    return {};
}

sub _col_letter_to_index ($self, $letter) {
    my $index = 0;
    for my $char (split //, $letter) {
        $index = $index * 26 + (ord($char) - ord('A') + 1);
    }
    return $index - 1;  # Convert to 0-based
}

sub _col_index_to_letter ($self, $index) {
    my $letter = '';
    $index++;
    while ($index > 0) {
        $index--;
        $letter = chr(ord('A') + ($index % 26)) . $letter;
        $index = int($index / 26);
    }
    return $letter;
}

# Main interface methods
sub worksheets ($self) {
    return $self->{workbook}->worksheets();
}

sub worksheet ($self, $name) {
    return $self->{workbook}->worksheet($name);
}

sub find_table ($self, $table_name) {
    my $table_info = $self->{tables}->{$table_name};
    return undef unless $table_info;
    
    my $range = $self->_parse_range($table_info->{ref});
    return undef unless $range->{start_col} >= 0;
    
    # Find the worksheet containing this table
    my $ws = $self->_find_worksheet_for_range($range);
    return undef unless $ws;
    
    # Extract table data
    my $data = $self->_extract_table_data($ws, $range, $table_info->{columns});
    
    return {
        name => $table_name,
        data => $data,
        worksheet => $ws,
        range => $range,
        columns => $table_info->{columns},
    };
}

sub _find_worksheet_for_range ($self, $range) {
    # For now, assume the table is in the first worksheet
    # In a more sophisticated implementation, we'd check which worksheet has this range
    my @worksheets = $self->{workbook}->worksheets();
    return $worksheets[0] if @worksheets;
    return undef;
}

sub _extract_table_data ($self, $ws, $range, $columns) {
    my $data = [];
    
    for my $row ($range->{start_row} .. $range->{end_row}) {
        my $row_data = [];
        for my $col ($range->{start_col} .. $range->{end_col}) {
            my $cell = $ws->get_cell($row, $col);
            if (defined $cell) {
                my $formula = $cell->{Formula};
                my $value   = $cell->value();

                # Handle HYPERLINK formulas — extract the URL (same logic as Convert::SheetUtil)
                if (defined $formula && $formula && $formula =~ m{ \A HYPERLINK \( " (?<url>[^"]+) " (?:, " (?<title>[^"]+) " )? \) \s*\z }xms) {
                    $value = $+{url};
                }

                undef $value unless defined $value && $value =~ m{\S}xms;
                push @$row_data, $value;
            } else {
                push @$row_data, undef;
            }
        }
        push @$data, $row_data;
    }
    
    return $data;
}

1;
