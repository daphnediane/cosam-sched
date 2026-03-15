package Convert::SheetUtil;

use base qw{Exporter};

use v5.38.0;
use utf8;
use warnings;

our @EXPORT_OK = qw{
    find_sheet
    get_rows
};

our %EXPORT_TAGS = (
    all => [ @EXPORT_OK ],
);

sub find_sheet ( $wb, @names ) {
    for my $name ( @names ) {
        for my $ws ( $wb->worksheets() ) {
            if ( lc $ws->get_name() eq lc $name ) {
                return $ws;
            }
        }
    }
    return;
}

sub get_rows ( $sheet ) {
    return unless defined $sheet;

    my ( $min_row, $max_row ) = $sheet->row_range();
    my ( $min_col, $max_col ) = $sheet->col_range();

    my @rows;
    for my $row_idx ( $min_row .. $max_row ) {
        my @row;
        for my $col_idx ( $min_col .. $max_col ) {
            my $cell = $sheet->get_cell( $row_idx, $col_idx );
            if ( defined $cell ) {
                my $formula = $cell->{ Formula };
                my $value   = $cell->value();

                # Handle HYPERLINK formulas — extract the URL
                if (   defined $formula
                    && $formula
                    =~ m{ \A HYPERLINK \( " (?<url>[^"]+) " (?:, " (?<title>[^"]+) " )? \) \s*\z }xms
                ) {
                    $value = $+{ url };
                }

                undef $value unless defined $value && $value =~ m{\S}xms;
                push @row, $value;
            }
            else {
                push @row, undef;
            }
        }
        push @rows, \@row;
    }

    return @rows;
}

1;
