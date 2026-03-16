# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
# See LICENSE file for full license text

package Convert::Events;

use v5.38.0;
use utf8;
use warnings;

use Date::Parse qw{ str2time };
use POSIX       qw{ strftime };
use Convert::Canonical
    qw{ canonical_header canonical_headers canonical_data };
use Convert::SheetUtil qw{ find_sheet get_rows };
use Convert::Lookup    qw{ :all };

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

    # Kind:Name==Group format (always show as group)
    if ( $h =~ m{\A ([GJSIP]) : (.+) == (.+) \z}xmsi ) {
        my $prefix    = lc $1;
        my $name_part = $2;
        my $group_name = $3;

        # Strip leading < (always-grouped marker from schedule-to-html)
        $name_part =~ s{\A <}{}xms;
        $name_part =~ s{\A \s+}{}xms;
        $name_part =~ s{\s+ \z}{}xms;

        return if $name_part eq q{};

        # Kind:Name==Group — always show as group
        return {
            rank           => $RANK_PREFIXES{ $prefix },
            index          => 0,
            is_other       => 0,
            is_named       => 1,
            header_name    => $name_part,
            group_name     => $group_name,
            is_group_member => 1,
            always_grouped => 1,
        };
    }
    
    # Kind:Name=Group format (group member)
    if ( $h =~ m{\A ([GJSIP]) : (.+) = (.+) \z}xmsi ) {
        my $prefix    = lc $1;
        my $name_part = $2;
        my $group_name = $3;

        # Strip leading < (always-grouped marker from schedule-to-html)
        $name_part =~ s{\A <}{}xms;
        $name_part =~ s{\A \s+}{}xms;
        $name_part =~ s{\s+ \z}{}xms;

        return if $name_part eq q{};

        # Kind:Name=Group — presenter is member of Group
        return {
            rank           => $RANK_PREFIXES{ $prefix },
            index          => 0,
            is_other       => 0,
            is_named       => 1,
            header_name    => $name_part,
            group_name     => $group_name,
            is_group_member => 1,
            always_grouped => 0,
        };
    }
    
    if ( $h =~ m{\A ([GJSIP]) : (.+) \z}xmsi ) {
        my $prefix    = lc $1;
        my $name_part = $2;

        # Strip =Group suffix (backward compatibility)
        $name_part =~ s{ = .+ \z}{}xms;

        # Strip leading < (always-grouped marker from schedule-to-html)
        $name_part =~ s{\A <}{}xms;
        $name_part =~ s{\A \s+}{}xms;
        $name_part =~ s{\s+ \z}{}xms;

        # Kind:Other — cell value contains comma-separated names
        if ( lc $name_part eq 'other' ) {
            return {
                rank     => $RANK_PREFIXES{ $prefix },
                index    => 0,
                is_other => 1,
                is_named => 0,
            };
        }

        return if $name_part eq q{};

        # Kind:Name — cell value is a flag; name comes from the header
        return {
            rank        => $RANK_PREFIXES{ $prefix },
            index       => 0,
            is_other    => 0,
            is_named    => 1,
            header_name => $name_part,
        };
    } ## end if ( $h =~ m{\A ([GJSIP]) : (.+) \z}xmsi)

    # Pattern: single letter prefix + digits (e.g. "g1", "p2", "j01")
    if ( $h =~ m{\A ([gjsip]) (\d+) \z}xmsi ) {
        my $prefix = lc $1;
        return {
            rank     => $RANK_PREFIXES{ $prefix },
            index    => int( $2 ),
            is_other => 0,
            is_named => 0,
        };
    } ## end if ( $h =~ m{\A ([gjsip]) (\d+) \z}xmsi)

    # "Guest1", "Staff2", etc.
    if ( $h
        =~ m{\A (Guest|Judge|Staff|Invited|Panelist|Fan_Panelist) [\s_]* (\d+) \z}xmsi
    ) {
        my $prefix = lc substr( $1, 0, 1 );
        return {
            rank     => $RANK_PREFIXES{ $prefix },
            index    => int( $2 ),
            is_other => 0,
            is_named => 0,
        };
    } ## end if ( $h =~ ...)

    # "Others" / "Other" / "other_panelists"
    if ( $h =~ m{\A other}xmsi ) {
        return { rank => undef, index => 0, is_other => 1, is_named => 0 };
    }

    return;
} ## end sub _parse_presenter_header

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
} ## end sub _parse_datetime

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
} ## end sub _parse_duration

# ── Cost normalization ────────────────────────────────────────────────────────

my $RE_FREE
    = qr{ \A (?: free | (?=n) (?: nothing | n /? a ) | [\$]? (?: 0+ (?: [.] 0+ )? | [.] 0+ ) ) \z }xmsi;
my $RE_TBD   = qr{ \A [\$]? T [.]? B [.]? D [.]? \z }xmsi;
my $RE_MODEL = qr{ model }xmsi;

sub _normalize_cost ( $text ) {
    return { cost => undef, is_free => 1, is_kids => 0 } unless defined $text;
    my $s = "$text";
    $s =~ s{\A \s+}{}xms;
    $s =~ s{\s+ \z}{}xms;
    return { cost => undef, is_free => 1, is_kids => 0 }
        if $s eq q{} || $s eq q{*};

    return { cost => undef,   is_free => 1, is_kids => 0 } if $s =~ $RE_FREE;
    return { cost => undef,   is_free => 1, is_kids => 1 } if lc $s eq 'kids';
    return { cost => 'TBD',   is_free => 0, is_kids => 0 } if $s =~ $RE_TBD;
    return { cost => 'model', is_free => 0, is_kids => 0 } if $s =~ $RE_MODEL;
    return { cost => $s, is_free => 0, is_kids => 0 };
} ## end sub _normalize_cost

sub _normalize_full ( $text ) {
    return 0 unless defined $text;
    my $s = "$text";
    $s =~ s{\A \s+}{}xms;
    $s =~ s{\s+ \z}{}xms;
    return 0 if $s eq q{};
    return 0 if $s =~ m{\A not? \z}xmsi;
    return 1;
} ## end sub _normalize_full

# ── Extract ID prefix ────────────────────────────────────────────────────────

sub _extract_id_prefix ( $id ) {
    return q{} unless defined $id;
    if ( $id =~ m{\A ([A-Za-z]+) }xms ) {
        return uc $1;
    }
    return q{};
} ## end sub _extract_id_prefix

sub _panel_type_uid_from_prefix ( $prefix ) {
    return undef unless defined $prefix;
    my $slug = lc "$prefix";
    $slug =~ s{[^a-z0-9]+}{-}xmsg;
    $slug =~ s{\A -+}{}xms;
    $slug =~ s{-+ \z}{}xms;
    return undef if $slug eq q{};
    return 'panel-type-' . $slug;
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
        if ( defined $room->{ hotel_room } ) {
            $map{ lc $room->{ hotel_room } } //= $room;
        }
    } ## end for my $room ( @{ $rooms...})
    return \%map;
} ## end sub _build_room_lookup

sub _find_or_create_room ( $rooms, $room_lookup, $room_name ) {
    return undef unless defined $room_name;
    
    # Try to find existing room by various names
    my $room = $room_lookup->{ lc $room_name };
    return $room if $room;
    
    # Create new room if not found
    my %existing_uids = map { $_->{ uid } => 1 } @$rooms;
    my $new_uid = 0;
    while ( exists $existing_uids{ $new_uid } ) {
        $new_uid++;
    }
    
    my $new_room = {
        uid        => $new_uid,
        short_name => $room_name,
        long_name  => $room_name,
        hotel_room => $room_name,
        sort_key   => 999,  # Put new rooms at the end
        is_hidden  => 0,
    };
    
    push @$rooms, $new_room;
    
    # Update lookup with new room
    $room_lookup->{ lc $room_name } = $new_room;
    
    return $new_room;
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
    } ## end for my $pt ( @{ $panel_types...})
    return \%map;
} ## end sub _build_type_lookup

# ── Main read function ────────────────────────────────────────────────────────

sub read_events ( $wb, $rooms, $panel_types, $lookup_config = {}, $staff_mode = 0 ) {

    # Try to find schedule data using lookup hierarchy
    my $source = find_data_source($wb, $lookup_config, 'schedule');
    return ( [], [] ) unless defined $source;

    my $rows_ref = Convert::Lookup::get_data_rows($source);
    my @rows = @$rows_ref;
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
    } ## end for my $i ( 0 .. $#header)

    my $room_lookup = _build_room_lookup( $rooms );
    my $type_lookup = _build_type_lookup( $panel_types );
    my %presenter_set;    # name -> { rank, groups, is_group }
    my %group_members;    # group_name -> [member_names]
    my @events;

    for my $row ( @rows ) {
        next unless defined $row;

        my $data = canonical_data( \@header, \@san_header, $row );

        my $uniq_id = $data->{ Uniq_ID } // $data->{ UniqID }
            // $data->{ ID } // $data->{ Id };
        my $name = $data->{ Name } // $data->{ Panel_Name }
            // $data->{ PanelName };
        next unless defined $name;

        # Time
        my $start_seconds
            = _parse_datetime( $data->{ Start_Time } // $data->{ StartTime }
                // $data->{ Start } );
        my $end_seconds
            = _parse_datetime( $data->{ End_Time } // $data->{ EndTime }
                // $data->{ End } );
        my $duration_seconds = _parse_duration( $data->{ Duration } );

        # Compute missing time values
        if (  !defined $end_seconds
            && defined $start_seconds
            && defined $duration_seconds ) {
            $end_seconds = $start_seconds + $duration_seconds;
        }
        if (  !defined $duration_seconds
            && defined $start_seconds
            && defined $end_seconds ) {
            $duration_seconds = $end_seconds - $start_seconds;
        }

        next unless defined $start_seconds;   # Skip events without start time

        # Room
        my $room_name = $data->{ Room } // $data->{ Room_Name }
            // $data->{ RoomName };
        my $room_obj;
        if ( defined $room_name ) {
            $room_obj = _find_or_create_room( $rooms, $room_lookup, $room_name );
        }

        # SPLIT events are page-break markers for print layout; skip entirely
        next if defined $room_name && lc( $room_name ) eq 'split';

        # Panel type
        my $id_prefix = _extract_id_prefix( $uniq_id );
        my $kind_raw  = $data->{ Kind } // $data->{ Panel_Kind }
            // $data->{ PanelKind };
        my $panel_type = $type_lookup->{ lc $id_prefix } if $id_prefix ne q{};

        if ( !defined $panel_type && defined $kind_raw ) {
            for my $pt ( @{ $panel_types } ) {
                if ( lc $pt->{ kind } eq lc $kind_raw ) {
                    $panel_type = $pt;
                    last;
                }
            } ## end for my $pt ( @{ $panel_types...})
        } ## end if ( !defined $panel_type...)

        # Cost
        my $cost_info = _normalize_cost( $data->{ Cost } );
        my $is_full   = _normalize_full( $data->{ Full } );

        # Presenters
        my @event_presenters;
        for my $pc ( @presenter_cols ) {
            my $val = $row->[ $pc->{ col_index } ];
            next unless defined $val;
            my $cell_str = "$val";
            $cell_str =~ s{\A \s+}{}xms;
            $cell_str =~ s{\s+ \z}{}xms;
            next if $cell_str eq q{};

            if ( $pc->{ is_named } ) {
                # Kind:Name column — cell is a flag ("Yes", "*", etc.)
                # Any non-blank value means this presenter is attending.
                my $name = $pc->{ header_name };
                push @event_presenters, $name;
                if ( !exists $presenter_set{ $name } ) {
                    my %presenter_info = (
                        rank => $pc->{ rank } // 'fan_panelist',
                        groups => [],
                        is_group => 0,
                    );
                    
                    # Track group membership
                    if ( $pc->{ is_group_member } && $pc->{ group_name } ) {
                        push @{ $presenter_info{groups} }, $pc->{ group_name };
                        push @{ $group_members{ $pc->{ group_name } } }, $name;
                        
                        # Track always_grouped flag
                        $presenter_info{always_grouped} = $pc->{ always_grouped } || 0;
                    }
                    
                    # Check if this presenter is actually a group
                    if ( _is_group_presenter($name) ) {
                        $presenter_info{is_group} = 1;
                    }
                    
                    $presenter_set{ $name } = \%presenter_info;
                }
            }
            else {
                # g1/Guest1/Other or Kind:Other — cell contains names
                my @parts
                    = split m{\s*(?:,\s*(?:and\s+)?|\band\s+)}xmsi,
                    $cell_str;
                for my $part ( @parts ) {
                    $part =~ s{\A \s+}{}xms;
                    $part =~ s{\s+ \z}{}xms;
                    next if $part eq q{};

                    push @event_presenters, $part;
                    if ( !exists $presenter_set{ $part } ) {
                        my %presenter_info = (
                            rank => $pc->{ rank } // 'fan_panelist',
                            groups => [],
                            is_group => _is_group_presenter($part) ? 1 : 0,
                        );
                        $presenter_set{ $part } = \%presenter_info;
                    }
                } ## end for my $part ( @parts )
            }
        } ## end for my $pc ( @presenter_cols)

        # Fallback: generic Presenter / Presenters column
        if ( @event_presenters == 0 ) {
            my $presenter_raw = $data->{ Presenter } // $data->{ Presenters }
                // $data->{ Presenter_s };
            if ( defined $presenter_raw && $presenter_raw =~ m{\S}xms ) {
                my @parts
                    = split m{\s*(?:,\s*(?:and\s+)?|\band\s+)}xmsi,
                    $presenter_raw;
                for my $part ( @parts ) {
                    $part =~ s{\A \s+}{}xms;
                    $part =~ s{\s+ \z}{}xms;
                    next if $part eq q{};

                    push @event_presenters, $part;
                    if ( !exists $presenter_set{ $part } ) {
                        my %presenter_info = (
                            rank => 'fan_panelist',
                            groups => [],
                            is_group => _is_group_presenter($part) ? 1 : 0,
                        );
                        $presenter_set{ $part } = \%presenter_info;
                    }
                } ## end for my $part ( @parts )
            } ## end if ( defined $presenter_raw...)
        } ## end if ( @event_presenters == 0 )

        my $is_workshop
            = $panel_type
            ? $panel_type->{ is_workshop }
            : ( $id_prefix =~ m{W\z}xmsi ? 1 : 0 );
        my $is_break
            = $panel_type ? $panel_type->{ is_break }
            : ( $id_prefix eq 'BREAK'
                || ( defined $room_name && lc( $room_name ) eq 'break' ) )
            ? 1
            : 0;
        my $is_hidden = $panel_type ? $panel_type->{ is_hidden } : 0;

        # Skip hidden events unless in staff mode
        next if $is_hidden && !$staff_mode;

        push @events, {
            id          => $uniq_id // sprintf( 'row%d', scalar @events ),
            name        => $name,
            description => $data->{ Description },
            start_time  => _seconds_to_iso( $start_seconds ),
            end_time    => _seconds_to_iso( $end_seconds ),
            duration    => defined $duration_seconds
            ? int( $duration_seconds / 60 )
            : undef,
            roomId      => $room_obj ? $room_obj->{ uid } : undef,
            panel_type => $panel_type
            ? $panel_type->{ uid }
            : _panel_type_uid_from_prefix( $id_prefix ),
            kind        => $panel_type ? $panel_type->{ kind }  : $kind_raw,
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
    } ## end for my $row ( @rows )

    # Post-process presenters to handle = suffix group members
    for my $presenter_name ( keys %presenter_set ) {
        if ( $presenter_name =~ m{ \A (.+) = \z }xms ) {
            my $base_name = $1;
            my $info = $presenter_set{ $presenter_name };
            
            # This is a group member, use the group_name from header if available
            my $group_name;
            if ( $info->{ groups } && @{ $info->{ groups } } > 0 ) {
                $group_name = $info->{ groups }[0];
            } else {
                # Fallback: try to infer group name
                $group_name = $base_name;
                if ( $base_name =~ m{ \A (Pro|Con) \z }xmsi ) {
                    $group_name = 'Pros and Cons';
                }
            }
            
            # Add group membership
            push @{ $info->{groups} }, $group_name if !grep { $_ eq $group_name } @{ $info->{groups} || [] };
            push @{ $group_members{ $group_name } }, $presenter_name;
            
            # Ensure the group exists as a presenter
            if ( !exists $presenter_set{ $group_name } ) {
                $presenter_set{ $group_name } = {
                    rank => 'guest',
                    groups => [],
                    is_group => 1,
                };
            }
        }
    }
    
    # Also ensure all referenced groups exist as presenters
    for my $presenter_name ( keys %presenter_set ) {
        my $info = $presenter_set{ $presenter_name };
        next unless $info->{ groups } && @{ $info->{ groups } } > 0;
        
        for my $group_name ( @{ $info->{ groups } } ) {
            # Ensure the group exists as a presenter
            if ( !exists $presenter_set{ $group_name } ) {
                $presenter_set{ $group_name } = {
                    rank => 'guest',
                    groups => [],
                    is_group => 1,
                };
            }
            
            # Add this presenter as a member of the group
            push @{ $group_members{ $group_name } }, $presenter_name
                unless grep { $_ eq $presenter_name } @{ $group_members{ $group_name } || [] };
        }
    }

    # Build presenter list with group information
    my @presenter_list = sort { $a->{ name } cmp $b->{ name } }
        map { 
            my $info = $presenter_set{ $_ };
            {
                name => $_,
                rank => $info->{ rank },
                groups => $info->{ groups } || [],
                is_group => $info->{ is_group } || 0,
                members => $info->{ is_group } ? ($group_members{ $_ } || []) : [],
            }
        }
        keys %presenter_set;

    return ( \@events, \@presenter_list );
} ## end sub read_events

# ── Credits generation ───────────────────────────────────────────────────────────

sub generate_credits ( $events, $presenters ) {
    # Build presenter lookup
    my %presenter_lookup;
    for my $p ( @$presenters ) {
        $presenter_lookup{ $p->{ name } } = $p;
    }
    
    # Generate credits for each event
    for my $event ( @$events ) {
        my @credits;
        my @presenters = @{ $event->{ presenters } };
        my %processed;
        
        # Handle always-grouped presenters first
        for my $name ( @presenters ) {
            next if $processed{ $name };
            
            my $info = $presenter_lookup{ $name };
            if ( !$info || !$info->{ always_grouped } ) {
                next;
            }
            
            # Always show as group name
            push @credits, $name;
            $processed{ $name } = 1;
        }
        
        # Handle regular presenters and groups
        for my $name ( @presenters ) {
            next if $processed{ $name };
            
            my $info = $presenter_lookup{ $name };
            if ( !$info ) {
                push @credits, $name;
                $processed{ $name } = 1;
                next;
            }
            
            # Check if this presenter is a member of any group
            if ( $info->{ groups } && @{ $info->{ groups } } > 0 ) {
                for my $group_name ( @{ $info->{ groups } } ) {
                    my $group_info = $presenter_lookup{ $group_name };
                    next unless $group_info && $group_info->{ is_group };
                    
                    my $members = $group_info->{ members } || [];
                    my @present_members = grep { my $m = $_; grep { $_ eq $m } @presenters } @$members;
                    
                    # Mark all members as processed to avoid duplicates
                    for my $member ( @present_members ) {
                        $processed{ $member } = 1;
                    }
                    $processed{ $group_name } = 1;
                    
                    if ( @present_members == @$members && @$members > 0 ) {
                        # All members present, show group name
                        push @credits, $group_name;
                    }
                    else {
                        # Partial attendance - show individual members with group context
                        for my $member ( @present_members ) {
                            push @credits, "$member of $group_name";
                        }
                    }
                    last; # Only handle first group for now
                }
            }
            elsif ( $info->{ is_group } ) {
                # This is a group name that's directly in the presenters list
                my $members = $info->{ members } || [];
                my @present_members = grep { my $m = $_; grep { $_ eq $m } @presenters } @$members;
                
                if ( @present_members == 0 ) {
                    # No members actually present, show group name
                    push @credits, $name;
                }
                elsif ( @present_members == @$members && @$members > 0 ) {
                    # All members present, show group name
                    push @credits, $name;
                }
                else {
                    # Partial attendance - show individual members with group context
                    for my $member ( @present_members ) {
                        push @credits, "$member of $name";
                    }
                }
                
                $processed{ $name } = 1;
                for my $member ( @present_members ) {
                    $processed{ $member } = 1;
                }
            }
            else {
                # Individual presenter
                push @credits, $name;
                $processed{ $name } = 1;
            }
        }
        
        $event->{ credits } = \@credits;
    }
    
    return $events;
} ## end sub generate_credits

# ── Conflict detection ────────────────────────────────────────────────────────

# Group presenter detection patterns
my @GROUP_PATTERNS = (
    qr{\b (?i:staff) \z}xms,
);

sub _is_group_presenter ( $presenter_name, $presenter_info = undef ) {
    # Check if presenter info indicates it's a group
    return 1 if $presenter_info && $presenter_info->{ is_group };
    
    # Check if presenter name ends with = (group member indicator)
    return 1 if $presenter_name =~ m{ = \z }xms;
    
    # Check against group patterns (e.g., names ending with "staff")
    for my $pattern (@GROUP_PATTERNS) {
        return 1 if $presenter_name =~ $pattern;
    }
    
    # Check if presenter comes from =Group header
    return 1 if $presenter_info && $presenter_info->{ is_group_member };
    
    return 0;
}

sub _events_overlap ( $event1, $event2 ) {
    return 0 unless defined $event1->{ start_time } && defined $event2->{ start_time };
    return 0 unless defined $event1->{ end_time } && defined $event2->{ end_time };
    
    my $start1 = $event1->{ start_time };
    my $end1   = $event1->{ end_time };
    my $start2 = $event2->{ start_time };
    my $end2   = $event2->{ end_time };
    
    # Events overlap if start1 < end2 AND start2 < end1
    return ( $start1 lt $end2 && $start2 lt $end1 );
}

sub detect_conflicts ( $events ) {
    my @conflicts;
    
    # Build presenter index: { presenter_name => [ event_refs ] }
    my %presenter_events;
    for my $event ( @$events ) {
        next if $event->{ is_break };
        for my $presenter ( @{ $event->{ presenters } } ) {
            push @{ $presenter_events{ $presenter } }, $event;
        }
    }
    
    # Build room index: { room_id => [ event_refs ] }
    my %room_events;
    for my $event ( @$events ) {
        next if $event->{ is_break };
        next unless defined $event->{ roomId };
        push @{ $room_events{ $event->{ roomId } } }, $event;
    }
    
    # Check presenter conflicts
    for my $presenter ( sort keys %presenter_events ) {
        my $presenter_event_list = $presenter_events{ $presenter };
        next if @$presenter_event_list < 2;
        
        # Skip group presenters - they can be in multiple places
        if ( _is_group_presenter($presenter) ) {
            # Still track conflicts for JSON data but mark as group type
            my @sorted_events = sort { $a->{ start_time } cmp $b->{ start_time } } @$presenter_event_list;
            my @overlap_groups = _find_overlap_groups( @sorted_events );
            
            for my $group ( @overlap_groups ) {
                next if @$group < 2;
                
                # Create conflicts marked as group type
                for my $i ( 0 .. $#$group - 1 ) {
                    for my $j ( $i + 1 .. $#$group ) {
                        push @conflicts, {
                            type      => 'group_presenter',
                            presenter => $presenter,
                            event1    => $group->[$i],
                            event2    => $group->[$j],
                        };
                    }
                }
            }
            next;
        }
        
        # Individual presenter conflict detection
        # Sort by start time
        my @sorted_events = sort { $a->{ start_time } cmp $b->{ start_time } } @$presenter_event_list;
        
        # Find overlapping groups
        my @overlap_groups = _find_overlap_groups( @sorted_events );
        
        for my $group ( @overlap_groups ) {
            next if @$group < 2;  # Skip single-event groups
            
            # Create conflicts for all pairs in this group
            for my $i ( 0 .. $#$group - 1 ) {
                for my $j ( $i + 1 .. $#$group ) {
                    push @conflicts, {
                        type      => 'presenter',
                        presenter => $presenter,
                        event1    => $group->[$i],
                        event2    => $group->[$j],
                    };
                }
            }
            
            # Warning for multi-way conflicts
            if ( @$group > 2 ) {
                warn sprintf(
                    "WARNING: Presenter \"%s\" has %d-way booking conflict:\n",
                    $presenter, scalar @$group
                );
                for my $event ( @$group ) {
                    warn sprintf(
                        "  %s \"%s\" (%s, %s)\n",
                        $event->{ id } // 'unknown',
                        $event->{ name },
                        _format_time_range( $event ),
                        $event->{ roomId } // 'no room',
                    );
                }
                warn "\n";
            } else {
                # Standard two-way conflict warning
                my $event1 = $group->[0];
                my $event2 = $group->[1];
                warn sprintf(
                    "WARNING: Presenter \"%s\" is double-booked:\n  %s \"%s\" (%s, %s)\n  %s \"%s\" (%s, %s)\n",
                    $presenter,
                    $event1->{ id } // 'unknown',
                    $event1->{ name },
                    _format_time_range( $event1 ),
                    $event1->{ roomId } // 'no room',
                    $event2->{ id } // 'unknown', 
                    $event2->{ name },
                    _format_time_range( $event2 ),
                    $event2->{ roomId } // 'no room',
                );
            }
        }
    }
    
    # Check room conflicts
    for my $room_id ( sort keys %room_events ) {
        my $room_event_list = $room_events{ $room_id };
        next if @$room_event_list < 2;
        
        # Sort by start time
        my @sorted_events = sort { $a->{ start_time } cmp $b->{ start_time } } @$room_event_list;
        
        # Find overlapping groups
        my @overlap_groups = _find_overlap_groups( @sorted_events );
        
        for my $group ( @overlap_groups ) {
            next if @$group < 2;  # Skip single-event groups
            
            # Create conflicts for all pairs in this group
            for my $i ( 0 .. $#$group - 1 ) {
                for my $j ( $i + 1 .. $#$group ) {
                    push @conflicts, {
                        type   => 'room',
                        room   => $room_id,
                        event1 => $group->[$i],
                        event2 => $group->[$j],
                    };
                }
            }
            
            # Warning for multi-way conflicts
            if ( @$group > 2 ) {
                warn sprintf(
                    "WARNING: Room %d has %d-way scheduling conflict:\n",
                    $room_id, scalar @$group
                );
                for my $event ( @$group ) {
                    warn sprintf(
                        "  %s \"%s\" (%s)\n",
                        $event->{ id } // 'unknown',
                        $event->{ name },
                        _format_time_range( $event ),
                    );
                }
                warn "\n";
            } else {
                # Standard two-way conflict warning
                my $event1 = $group->[0];
                my $event2 = $group->[1];
                warn sprintf(
                    "WARNING: Room conflict in room %d:\n  %s \"%s\" (%s)\n  %s \"%s\" (%s)\n",
                    $room_id,
                    $event1->{ id } // 'unknown',
                    $event1->{ name },
                    _format_time_range( $event1 ),
                    $event2->{ id } // 'unknown',
                    $event2->{ name },
                    _format_time_range( $event2 ),
                );
            }
        }
    }
    
    return \@conflicts;
}

sub _find_overlap_groups ( @sorted_events ) {
    my @groups;
    my @current_group;
    
    for my $event ( @sorted_events ) {
        if ( !@current_group ) {
            # Start first group
            push @current_group, $event;
        }
        else {
            # Check if this event overlaps with any event in current group
            my $overlaps_with_group = 0;
            for my $group_event ( @current_group ) {
                if ( _events_overlap( $event, $group_event ) ) {
                    $overlaps_with_group = 1;
                    last;
                }
            }
            
            if ( $overlaps_with_group ) {
                # Add to current group
                push @current_group, $event;
            }
            else {
                # Save current group and start new one
                push @groups, [@current_group] if @current_group > 1;
                @current_group = ($event);
            }
        }
    }
    
    # Save final group
    push @groups, [@current_group] if @current_group > 1;
    
    return @groups;
}

sub _format_time_range ( $event ) {
    my $start = $event->{ start_time };
    my $end   = $event->{ end_time };
    
    # Extract time portion from ISO format
    $start =~ s{T(\d{2}:\d{2}).*}{$1} if defined $start;
    $end   =~ s{T(\d{2}:\d{2}).*}{$1} if defined $end;
    
    return "${start}-${end}";
}

1;
