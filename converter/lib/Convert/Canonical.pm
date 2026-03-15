package Convert::Canonical;

use base qw{Exporter};

use v5.38.0;
use utf8;
use warnings;

use Carp qw{ croak };

our @EXPORT_OK = qw{
    canonical_header
    canonical_headers
    canonical_data
};

our %EXPORT_TAGS = (
    all => [ @EXPORT_OK ],
);

sub canonical_header ( $hdr ) {
    return unless defined $hdr;
    $hdr =~ s{\A \s+}{}xms;
    $hdr =~ s{\s+ \z}{}xms;
    return if $hdr eq q{};
    $hdr =~ s{\s+}{_}xmsg;
    $hdr =~ s{[/:().,]}{_}xmsg;
    $hdr =~ s{_+}{_}xmsg;
    $hdr =~ s{\A_}{}xmsg;
    $hdr =~ s{_\z}{}xmsg;
    return $hdr;
}

sub canonical_headers ( @hdrs ) {
    return map { defined $_ ? canonical_header( $_ ) : undef } @hdrs;
}

sub canonical_data ( $header, $san_header, $raw ) {
    my %data;
    foreach my $column ( keys @{ $raw } ) {
        my $header_text = $header->[ $column ];
        my $header_alt  = $san_header->[ $column ];

        my $raw_text = $raw->[ $column ];
        if ( defined $raw_text ) {
            $raw_text =~ s{\A \s++}{}xms;
            $raw_text =~ s{\s++ \z}{}xms;
            undef $raw_text if $raw_text eq q{};
        }

        if ( !defined $header_text ) {
            next;
        }

        $data{ $header_text } = $raw_text;
        $data{ $header_alt }  = $raw_text if defined $header_alt;
    }

    return \%data;
}

1;
