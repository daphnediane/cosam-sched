package Convert::Rooms;

use v5.38.0;
use utf8;
use warnings;

use Convert::Canonical qw{ canonical_headers canonical_data };
use Convert::SheetUtil qw{ find_sheet get_rows };

sub read_rooms ( $wb ) {
    my $sheet = find_sheet( $wb, 'Rooms' );
    return [] unless defined $sheet;

    my @rows = get_rows( $sheet );
    return [] if @rows < 2;

    my @header     = @{ shift @rows };
    my @san_header = canonical_headers( @header );

    my %col;
    for my $i ( 0 .. $#san_header ) {
        $col{ lc( $san_header[ $i ] // q{} ) } = $i;
    }

    my @rooms;
    my $next_id = 0;

    for my $row ( @rows ) {
        my $data = canonical_data( \@header, \@san_header, $row );

        my $short_name = $data->{ Room_Name } // $data->{ Room }
            // $data->{ Name };
        my $long_name = $data->{ Long_Name } // $short_name;
        $short_name //= $long_name;

        next unless defined $short_name;

        my $sort_key_raw = $data->{ Sort_Key } // $data->{ SortKey };
        my $sort_key     = defined $sort_key_raw ? int( $sort_key_raw ) : 999;
        my $hotel_room   = $data->{ Hotel_Room } // $data->{ HotelRoom };

        push @rooms, {
            id         => $next_id++,
            short_name => $short_name,
            long_name  => $long_name,
            hotel_room => $hotel_room,
            sort_key   => $sort_key,
            is_hidden  => ( $sort_key >= 100 ) ? 1 : 0,
        };
    } ## end for my $row ( @rows )

    @rooms = sort { $a->{ sort_key } <=> $b->{ sort_key } } @rooms;

    # Re-assign ids after sort
    my $idx = 0;
    $_->{ id } = $idx++ for @rooms;

    return \@rooms;
} ## end sub read_rooms

1;
