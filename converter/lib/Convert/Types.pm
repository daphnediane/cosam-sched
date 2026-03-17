# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
# See LICENSE file for full license text

package Convert::Types;

use v5.38.0;
use utf8;
use warnings;

use Convert::Canonical qw{ canonical_headers canonical_data };
use Convert::SheetUtil qw{ find_sheet get_rows };
use Convert::Lookup    qw{ :all };

sub read_panel_types ( $wb, $lookup_config = {} ) {
    my $source = find_data_source($wb, $lookup_config, 'prefix');
    return [] unless defined $source;

    my $rows_ref = Convert::Lookup::get_data_rows($source);
    my @rows = @$rows_ref;
    return [] if @rows < 2;

    my @header     = @{ shift @rows };
    my @san_header = canonical_headers( @header );

    my @types;

    for my $row ( @rows ) {
        my $data = canonical_data( \@header, \@san_header, $row );

        my $prefix = $data->{ Prefix } // q{};
        next if $prefix eq q{};
        my $prefix_uc = uc $prefix;
        my $uid       = 'panel-type-' . lc $prefix_uc;
        $uid =~ s{[^a-z0-9]+}{-}xmsg;
        $uid =~ s{\A -+}{}xms;
        $uid =~ s{-+ \z}{}xms;

        my $kind = $data->{ Panel_Kind } // $data->{ PanelKind }
            // $data->{ Kind } // $prefix;

        my $is_break    = ( $kind   =~ m{\A br}xmsi )         ? 1 : 0;
        my $is_cafe     = ( $kind   =~ m{\A caf[eé] \z}xmsi ) ? 1 : 0;
        my $is_workshop = ( $prefix =~ m{\A .W \z}xmsi )      ? 1 : 0;
        my $is_split    = ( $prefix_uc eq 'SPLIT' )          ? 1 : 0;

        my $is_hidden = 0;
        if ( defined $data->{ Hidden } && $data->{ Hidden } ne q{} ) {
            $is_hidden = 1;
        }

        # Check for boolean override fields
        for my $field_name ( qw{ Is_Break } ) {
            if ( defined $data->{ $field_name } ) {
                $is_break = $data->{ $field_name } ? 1 : 0;
            }
        }
        for my $field_name ( qw{ Is_Workshop } ) {
            if ( defined $data->{ $field_name } ) {
                $is_workshop = $data->{ $field_name } ? 1 : 0;
            }
        }
        for my $field_name ( 'Is_Cafe', "Is_Caf\x{e9}" ) {
            if ( defined $data->{ $field_name } ) {
                $is_cafe = $data->{ $field_name } ? 1 : 0;
            }
        }

        my $color = $data->{ Color };

        push @types, {
            uid         => $uid,
            prefix      => $prefix_uc,
            kind        => $kind,
            is_break    => $is_break,
            is_cafe     => $is_cafe,
            is_workshop => $is_workshop,
            is_hidden   => $is_hidden,
            is_split    => $is_split,
            color       => $color,
        };
    } ## end for my $row ( @rows )

    return \@types;
} ## end sub read_panel_types

1;
