# Google Sheets Integration

The cosam_sched converter now supports reading schedule data directly from Google Sheets using the Google Sheets API v4.

## Setup

### 1. Install Dependencies

```bash
cpanm Google::RestApi::SheetsApi4
```

### 2. Google Cloud Project Setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select an existing one
3. Enable the Google Sheets API
4. Create credentials (OAuth 2.0 Client ID)
5. Download the credentials JSON file

### 3. Get Access Token

You'll need to obtain an OAuth2 access token. You can use Google's OAuth 2.0 Playground or a script to get this.

### 4. Configuration

Create a YAML configuration file (e.g., `google-sheets-config.yaml`):

```yaml
auth:
  class: OAuth2Client
  client_id: your_client_id_here
  client_secret: your_client_secret_here
  token_file: path/to/token/file.yaml
```

Use the OAuth token creator script to generate the token file:

```bash
google_restapi_oauth_token_creator google-sheets-config.yaml
```

## Usage

### Command Line

```bash
./converter/schedule_to_json \
  --input "https://docs.google.com/spreadsheets/d/1kkHPUuzwD-GBTDwKt_F4Jx5oiPQqOJ4-/edit?usp=sharing&ouid=106780714679471954888&rtpof=true&sd=true" \
  --config google-sheets-config.yaml \
  --output schedule.json \
  --title "Event Name"
```

### URL Format

The converter supports Google Sheets URLs in these formats:

- `https://docs.google.com/spreadsheets/d/SPREADSHEET_ID/edit`
- `https://docs.google.com/spreadsheets/d/SPREADSHEET_ID/edit?usp=sharing`

The spreadsheet ID is automatically extracted from the URL.

## Security Notes

- Never commit your configuration file with real credentials to version control
- Use environment variables or a secure credential store in production
- Consider using service accounts for automated access
- Regularly rotate your access tokens

## Advanced Features

### Table Detection

The converter automatically detects and handles data tables in Google Sheets:

1. **Native Tables**: If you've created formal Google Sheets tables, the converter uses the Tables API to get precise table boundaries and metadata
2. **Detected Tables**: For non-table data regions, the converter uses intelligent detection to identify data tables based on content patterns
3. **Multiple Tables**: If a worksheet contains multiple tables, the converter uses the first substantial table found

### Table Best Practices

For best results, consider these approaches:

**Option 1: Use Google Sheets Tables (Recommended)**
- Convert your data ranges to formal Google Sheets tables
- This provides precise boundaries and metadata
- Tables support column types, filters, and structured data

**Option 2: Organized Data Ranges**
- Keep data in contiguous blocks with clear headers
- Avoid empty rows within data tables
- Use consistent column structures

**Option 3: Single Table per Worksheet**
- For schedule data, use one main table per worksheet
- Place reference data or auxiliary tables in separate sheets

## Troubleshooting

### Common Errors

1. **"Google Sheets API configuration required"**
   - Ensure you're using the `--config` parameter with a valid JSON file

2. **"Unable to extract spreadsheet ID"**
   - Check that your URL format is correct
   - Ensure the spreadsheet is publicly accessible or you have proper permissions

3. **Authentication errors**
   - Verify your access token is valid and not expired
   - Check that the token has the right scopes for Google Sheets

### Debug Mode

For debugging, you can add print statements to see the API responses:

```perl
use Data::Dumper;
warn Dumper($self->{_data});
```

## Example Workflow

1. Share your Google Sheet (ensure it's accessible with your service account)
2. Create and configure your credentials
3. Run the converter with the Google Sheets URL
4. The output `schedule.json` will be compatible with the widget

The converter maintains full compatibility with existing XLSX files while adding Google Sheets support.
