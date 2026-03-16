# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
# See LICENSE file for full license text

package Convert::Workbook;

use v5.38.0;
use utf8;
use warnings;

use Spreadsheet::ParseXLSX qw{};
use Convert::GoogleSheets qw{};
use Convert::XLSXTables qw{};
use Google::RestApi::SheetsApi4 qw{};
use URI qw{};

sub open_workbook ( $filename, $config = {} ) {
    if ( $filename =~ m{[.]xlsx\z}xmsi ) {
        # Try XLSX with table support first
        my $wb;
        my $success = eval {
            $wb = Convert::XLSXTables->new( $filename );
            1;  # Return true for success
        };
        
        if ($success) {
            return $wb;  # XLSXTables worked
        }
        
        # Only reach here if XLSXTables failed
        warn "XLSXTables failed, falling back to basic parser: $@" if $@;
        my $parser = Spreadsheet::ParseXLSX->new;
        $wb     = $parser->parse( $filename );
        die "Unable to parse: ${filename}: " . $parser->error() . "\n"
            unless defined $wb;
        return $wb;
    } ## end if ( $filename =~ m{[.]xlsx\z}xmsi)

    # Handle Google Sheets URLs
    if ( $filename =~ m{docs[.]google[.]com/spreadsheets}xmsi ) {
        my $uri = URI->new($filename);
        my $path = $uri->path;
        
        # Extract spreadsheet ID from URL
        # Format: /spreadsheets/d/SPREADSHEET_ID/edit
        if ( $path =~ m{/spreadsheets/d/([^/]+)}xms ) {
            my $spreadsheet_id = $1;
            
            # Create Google Sheets API client
            die "Google Sheets API configuration required\n" 
                unless $config && $config->{config_file};
            
            my $rest_api = Google::RestApi->new(config_file => $config->{config_file});
            my $sheets_api = Google::RestApi::SheetsApi4->new(api => $rest_api);
            
            return Convert::GoogleSheets->new(
                sheets_api => $sheets_api,
                spreadsheet_id => $spreadsheet_id,
            );
        }
        
        die "Unable to extract spreadsheet ID from: ${filename}\n";
    } ## end if ( $filename =~ m{docs[.]google[.]com/spreadsheets}xmsi)

    die "Unsupported file format: ${filename}\n" . "Supported: .xlsx, Google Sheets URLs\n";
} ## end sub open_workbook

1;
