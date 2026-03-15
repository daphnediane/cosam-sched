package Convert::Workbook;

use v5.38.0;
use utf8;
use warnings;

use Spreadsheet::ParseXLSX qw{};

sub open_workbook ( $filename ) {
    if ( $filename =~ m{[.]xlsx\z}xmsi ) {
        my $parser = Spreadsheet::ParseXLSX->new;
        my $wb     = $parser->parse( $filename );
        die "Unable to parse: ${filename}: " . $parser->error() . "\n"
            unless defined $wb;
        return $wb;
    }

    die "Unsupported file format: ${filename}\n"
        . "Supported: .xlsx\n";
}

1;
