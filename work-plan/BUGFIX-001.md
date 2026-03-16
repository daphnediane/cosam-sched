# Fix missing presenters

## Summary

Converting the existing spreadsheets loses presenter information during the conversion process.

## Status

Open

## Priority

High

## Description

The converter is not properly extracting presenter data from the spreadsheet columns. This results in events without presenter information in the generated JSON, which is critical for attendees to know who is running each event.

## Steps to Fix

1. Identify the presenter column in the spreadsheet format

2. Update Events.pm to properly parse presenter data

3. Handle multiple presenters if applicable

4. Test with existing spreadsheets to ensure data is preserved
