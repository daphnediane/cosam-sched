# OneDrive/Office 365 Integration

## Summary

Support reading from and writing to Excel files stored in OneDrive.

## Status

Open

## Priority

Low

## Description

Enable the editor to work with XLSX files shared via OneDrive/Office 365. This supports workflows where the schedule spreadsheet lives in a shared OneDrive folder.

## Implementation Details

- Microsoft Graph API integration for OneDrive file access
- OAuth 2.0 authentication with Microsoft identity
- Download XLSX from OneDrive, edit locally, upload changes
- File locking or conflict detection for shared files
- Credential storage (system keychain)
- Offline editing with sync on reconnect
