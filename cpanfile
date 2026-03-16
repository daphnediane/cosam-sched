# cpanfile for cosam_sched
requires 'perl', '>= 5.038';
requires 'Date::Parse';
requires 'Feature::Compat::Class';
requires 'File::Slurp';
requires 'Getopt::Long';
requires 'JSON';
requires 'List::MoreUtils';
requires 'Readonly';
requires 'Spreadsheet::ParseXLSX';
requires 'Google::RestApi::SheetsApi4';
requires 'Archive::Zip';
requires 'XML::Simple';

on 'develop' => sub {
    requires 'App::cpanminus';
    requires 'Perl::Critic';
    requires 'Perl::Tidy';
};
