# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
# See LICENSE file for full license text

package Convert::Rooms;

use v5.38.0;
use utf8;
use warnings;

use Convert::Canonical qw{ canonical_headers canonical_data };
use Convert::SheetUtil qw{ find_sheet get_rows };
use Convert::Lookup    qw{ :all };

sub read_rooms ( $wb, $lookup_config = {} ) {
    my $source = find_data_source($wb, $lookup_config, 'roommap');
    return [] unless defined $source;

    my $rows_ref = Convert::Lookup::get_data_rows($source);
    my @rows = @$rows_ref;
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
        my $long_name_raw = $data->{ Long_Name };
        my $long_name = (defined $long_name_raw && $long_name_raw ne '#ERROR!') 
            ? $long_name_raw 
            : ($data->{ Hotel_Room } // $short_name);
        $short_name //= $long_name;

        next unless defined $short_name;

        my $sort_key_raw = $data->{ Sort_Key } // $data->{ SortKey };
        my $sort_key     = defined $sort_key_raw ? int( $sort_key_raw ) : 999;
        my $hotel_room   = $data->{ Hotel_Room } // $data->{ HotelRoom };

        push @rooms, {
            id         => $next_id++,
            uid        => $next_id - 1,  # Use id as uid
            short_name => $short_name,
            long_name  => $long_name,
            hotel_room => $hotel_room,
            sort_key   => $sort_key,
            is_hidden  => ( $sort_key >= 100 ) ? 1 : 0,
        };
    } ## end for my $row ( @rows )

    @rooms = sort { $a->{ sort_key } <=> $b->{ sort_key } } @rooms;

    # Re-assign ids after sort but preserve original UIDs
    my $idx = 0;
    for my $room (@rooms) {
        $room->{ id } = $idx++;
        # UID was already assigned during creation
    }

    return \@rooms;
} ## end sub read_rooms

1;
