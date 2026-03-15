package Convert::Events;

use v5.38.0;
use utf8;
use warnings;

use Date::Parse       qw{ str2time };
use POSIX             qw{ strftime };
use Convert::Canonical qw{ canonical_header canonical_headers canonical_data };
use Convert::SheetUtil qw{ find_sheet get_rows };

# ── Presenter column detection ────────────────────────────────────────────────

my %RANK_PREFIXES = (
    g => 'guest',
    j => 'judge',
    s => 'staff',
    i => 'invited_guest',
    p => 'fan_panelist',
);

sub _parse_presenter_header ( $hdr ) {
    return unless defined $hdr;
    my $h = $hdr;
    $h =~ s{\A \s+}{}xms;
    $h =~ s{\s+ \z}{}xms;
    return if $h eq q{};

    # Pattern: single letter prefix + digits (e.g. "g1", "p2", "j01")
    if ( $h =~ m{\A ([gjsip]) (\d+) \z}xmsi ) {
        my $prefix = lc $1;
        return {
            rank     => $RANK_PREFIXES{ $prefix },
            index    => int( $2 ),
            is_other => 0,
        };
    }

    # "Guest1", "Staff2", etc.
    if ( $h =~ m{\A (Guest|Judge|Staff|Invited|Panelist|Fan_Panelist) [\s_]* (\d+) \z}xmsi ) {
        my $prefix = lc substr( $1, 0, 1 );
        return {
            rank     => $RANK_PREFIXES{ $prefix },
            index    => int( $2 ),
            is_other => 0,
        };
    }

    # "Others" / "Other" / "other_panelists"
    if ( $h =~ m{\A other}xmsi ) {
        return { rank => undef, index => 0, is_other => 1 };
    }

    return;
}

# ── Time parsing ──────────────────────────────────────────────────────────────

sub _parse_datetime ( $text ) {
    return unless defined $text;
    my $s = "$text";
    $s =~ s{\A \s+}{}xms;
    $s =~ s{\s+ \z}{}xms;
    return if $s eq q{};

    # Already epoch seconds
    return int( $s ) if $s =~ m{\A \d{9,} \z}xms;

    my $time = str2time( $s );
    return $time if defined $time;

    warn "Unable to parse time: ${s}\n";
    return;
}

sub _seconds_to_iso ( $seconds ) {
    return unless defined $seconds;
    return strftime( '%Y-%m-%dT%H:%M:%S', localtime $seconds );
}

sub _parse_duration ( $text ) {
    return unless defined $text;
    my $s = "$text";
    $s =~ s{\A \s+}{}xms;
    $s =~ s{\s+ \z}{}xms;
    return if $s eq q{};

    # "H:MM" or "HH:MM"
    if ( $s =~ m{\A (\d+) : (\d{1,2}) \z}xms ) {
        return ( int( $1 ) * 60 + int( $2 ) ) * 60;
    }

    # Plain number = minutes
    if ( $s =~ m{\A (\d+(?:\.\d+)?) \z}xms ) {
        return int( $1 * 60 );
    }

    return;
}

# ── Cost normalization ────────────────────────────────────────────────────────

my $RE_FREE  = qr{ \A (?: free | (?=n) (?: nothing | n /? a ) | [\$]? (?: 0+ (?: [.] 0+ )? | [.] 0+ ) ) \z }xmsi;
my $RE_TBD   = qr{ \A [\$]? T [.]? B [.]? D [.]? \z }xmsi;
my $RE_MODEL = qr{ model }xmsi;

sub _normalize_cost ( $text ) {
    return { cost => undef, is_free => 1, is_kids => 0 } unless defined $text;
    my $s = "$text";
    $s =~ s{\A \s+}{}xms;
    $s =~ s{\s+ \z}{}xms;
    return { cost => undef, is_free => 1, is_kids => 0 } if $s eq q{} || $s eq q{*};

    return { cost => undef, is_free => 1, is_kids => 0 } if $s =~ $RE_FREE;
    return { cost => undef, is_free => 1, is_kids => 1 } if lc $s eq 'kids';
    return { cost => 'TBD', is_free => 0, is_kids => 0 } if $s =~ $RE_TBD;
    return { cost => 'model', is_free => 0, is_kids => 0 } if $s =~ $RE_MODEL;
    return { cost => $s, is_free => 0, is_kids => 0 };
}

sub _normalize_full ( $text ) {
    return 0 unless defined $text;
    my $s = "$text";
    $s =~ s{\A \s+}{}xms;
    $s =~ s{\s+ \z}{}xms;
    return 0 if $s eq q{};
    return 0 if $s =~ m{\A not? \z}xmsi;
    return 1;
}

# ── Extract ID prefix ────────────────────────────────────────────────────────

sub _extract_id_prefix ( $id ) {
    return q{} unless defined $id;
    if ( $id =~ m{\A ([A-Za-z]+) }xms ) {
        return uc $1;
    }
    return q{};
}

# ── Room lookup helpers ───────────────────────────────────────────────────────

sub _build_room_lookup ( $rooms ) {
    my %map;
    for my $room ( @{ $rooms } ) {
        if ( defined $room->{ short_name } ) {
            $map{ lc $room->{ short_name } } //= $room;
        }
        if ( defined $room->{ long_name } ) {
            $map{ lc $room->{ long_name } } //= $room;
        }
    }
    return \%map;
}

# ── PanelType lookup helpers ─────────────────────────────────────────────────

sub _build_type_lookup ( $panel_types ) {
    my %map;
    for my $pt ( @{ $panel_types } ) {
        next unless defined $pt->{ prefix } && $pt->{ prefix } ne q{};
        my $key = lc $pt->{ prefix };
        $map{ $key } //= $pt;
        if ( length( $key ) > 2 ) {
            $map{ substr( $key, 0, 2 ) } //= $pt;
        }
    }
    return \%map;
}

# ── Main read function ────────────────────────────────────────────────────────

sub read_events ( $wb, $rooms, $panel_types ) {
    # Try "Schedule" sheet, fall back to first sheet
    my $sheet = find_sheet( $wb, 'Schedule' );
    if ( !defined $sheet ) {
        my @all_sheets = $wb->worksheets();
        $sheet = $all_sheets[ 0 ] if @all_sheets;
    }
    return ( [], [] ) unless defined $sheet;

    my @rows = get_rows( $sheet );
    return ( [], [] ) if @rows < 2;

    my @header     = @{ shift @rows };
    my @san_header = canonical_headers( @header );

    # Build column index by sanitized header name
    my %col;
    for my $i ( 0 .. $#san_header ) {
        my $h = $san_header[ $i ];
        next unless defined $h;
        $col{ $h } //= $i;
    }

    # Detect presenter columns
    my @presenter_cols;
    for my $i ( 0 .. $#header ) {
        my $info = _parse_presenter_header( $header[ $i ] );
        next unless defined $info;
        $info->{ col_index } = $i;
        push @presenter_cols, $info;
    }

    my $room_lookup = _build_room_lookup( $rooms );
    my $type_lookup = _build_type_lookup( $panel_types );
    my %presenter_set;  # name -> rank
    my @events;

    for my $row ( @rows ) {
        next unless defined $row;

        my $data = canonical_data( \@header, \@san_header, $row );

        my $uniq_id = $data->{ Uniq_ID } // $data->{ UniqID } // $data->{ ID } // $data->{ Id };
        my $name    = $data->{ Name } // $data->{ Panel_Name } // $data->{ PanelName };
        next unless defined $name;

        # Time
        my $start_seconds = _parse_datetime(
            $data->{ Start_Time } // $data->{ StartTime } // $data->{ Start }
        );
        my $end_seconds = _parse_datetime(
            $data->{ End_Time } // $data->{ EndTime } // $data->{ End }
        );
        my $duration_seconds = _parse_duration(
            $data->{ Duration }
        );

        # Compute missing time values
        if ( !defined $end_seconds && defined $start_seconds && defined $duration_seconds ) {
            $end_seconds = $start_seconds + $duration_seconds;
        }
        if ( !defined $duration_seconds && defined $start_seconds && defined $end_seconds ) {
            $duration_seconds = $end_seconds - $start_seconds;
        }

        next unless defined $start_seconds; # Skip events without start time

        # Room
        my $room_name = $data->{ Room } // $data->{ Room_Name } // $data->{ RoomName };
        my $room_obj;
        if ( defined $room_name ) {
            $room_obj = $room_lookup->{ lc $room_name };
        }

        # Panel type
        my $id_prefix = _extract_id_prefix( $uniq_id );
        my $kind_raw  = $data->{ Kind } // $data->{ Panel_Kind } // $data->{ PanelKind };
        my $panel_type = $type_lookup->{ lc $id_prefix } if $id_prefix ne q{};

        if ( !defined $panel_type && defined $kind_raw ) {
            for my $pt ( @{ $panel_types } ) {
                if ( lc $pt->{ kind } eq lc $kind_raw ) {
                    $panel_type = $pt;
                    last;
                }
            }
        }

        # Cost
        my $cost_info = _normalize_cost( $data->{ Cost } );
        my $is_full   = _normalize_full( $data->{ Full } );

        # Presenters
        my @event_presenters;
        for my $pc ( @presenter_cols ) {
            my $val = $row->[ $pc->{ col_index } ];
            next unless defined $val;
            my $names_str = "$val";
            $names_str =~ s{\A \s+}{}xms;
            $names_str =~ s{\s+ \z}{}xms;
            next if $names_str eq q{};

            # Split on comma/and separators
            my @parts = split m{\s*(?:,\s*(?:and\s+)?|\band\s+)}xmsi, $names_str;
            for my $part ( @parts ) {
                $part =~ s{\A \s+}{}xms;
                $part =~ s{\s+ \z}{}xms;
                next if $part eq q{};

                push @event_presenters, $part;
                if ( !exists $presenter_set{ $part } ) {
                    $presenter_set{ $part } = $pc->{ rank } // 'fan_panelist';
                }
            }
        }

        my $is_workshop = $panel_type ? $panel_type->{ is_workshop } : ( $id_prefix =~ m{W\z}xmsi ? 1 : 0 );
        my $is_break    = $panel_type ? $panel_type->{ is_break }    : 0;
        my $is_hidden   = $panel_type ? $panel_type->{ is_hidden }   : 0;

        next if $is_hidden;

        push @events, {
            id          => $uniq_id // sprintf( 'row%d', scalar @events ),
            name        => $name,
            description => $data->{ Description },
            start_time  => _seconds_to_iso( $start_seconds ),
            end_time    => _seconds_to_iso( $end_seconds ),
            duration    => defined $duration_seconds ? int( $duration_seconds / 60 ) : undef,
            room        => $room_obj ? ( $room_obj->{ long_name } // $room_obj->{ short_name } ) : $room_name,
            room_id     => $room_obj ? $room_obj->{ id } : undef,
            panel_type  => $panel_type ? $panel_type->{ prefix } : ( $id_prefix ne q{} ? $id_prefix : undef ),
            kind        => $panel_type ? $panel_type->{ kind } : $kind_raw,
            color       => $panel_type ? $panel_type->{ color } : undef,
            cost        => $cost_info->{ cost },
            is_free     => $cost_info->{ is_free },
            is_kids     => $cost_info->{ is_kids },
            is_workshop => $is_workshop,
            is_break    => $is_break,
            difficulty  => $data->{ Difficulty },
            capacity    => $data->{ Capacity },
            presenters  => \@event_presenters,
            note        => $data->{ Note },
            prereq      => $data->{ Prereq },
            ticket_url  => $data->{ Ticket_Sale } // $data->{ TicketSale },
            is_full     => $is_full,
        };
    }

    # Build presenter list
    my @presenter_list =
        sort { $a->{ name } cmp $b->{ name } }
        map  { { name => $_, rank => $presenter_set{ $_ } } }
        keys %presenter_set;

    return ( \@events, \@presenter_list );
}

1;
