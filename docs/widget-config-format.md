# Widget Config Format

This document describes the `ScheduleConfig` presentation configuration format used by the Cosplay America calendar widget. This is a separate structure from the core schedule data (`WidgetExport`) and contains branding and print-format configuration.

## Purpose

The config format exists to separate presentation configuration from schedule data, allowing:

- The same schedule to be displayed with different branding or print formats without modifying the core data
- Independent versioning of presentation configuration
- Loading of config from a separate source (e.g., a config file or API endpoint)

## Top-Level Structure

```json
{
  "version": 1,
  "brand": Brand,                    // optional; branding bridge for print
  "printFormats": Array<PrintFormat> // optional; shipped print-format defaults
}
```

`brand` and `printFormats` are optional and emitted by `cosam-convert` when a
brand config (`config/brand.toml`) and/or widget config
(`config/widget-default.toml`, overridable by `config/widget.toml`) are present.

## Version

The `version` field allows consumers to handle structural changes to the config
format. Current version is `1`.

## Brand

Optional. The branding bridge carries house-style assets from
`config/brand.toml` to the widget so its print formats can match the printed/PDF
output. All fields are optional; the whole object is omitted when no brand
config is available.

| Field    | Type                  | Description                                                                |
| -------- | --------------------- | -------------------------------------------------------------------------- |
| `colors` | Object                | `primary`, `black`, `darkGrey`, `white` (hex strings).                     |
| `logos`  | Object<string,string> | Logo alias → URL usable in `<img src>`: a base64 `data:` URL or `http(s)`. |
| `fonts`  | Array<PrintFont>      | Web-equivalent print fonts, one per configured role.                       |
| `meta`   | Object                | `name`, `siteUrl`.                                                         |

### Brand Colors

| Field      | Type   | Description          |
| ---------- | ------ | -------------------- |
| `primary`  | string | Primary brand color. |
| `black`    | string | Black color.         |
| `darkGrey` | string | Dark grey color.     |
| `white`    | string | White color.         |

### Brand Logos

A hashmap of logo aliases to URLs. Logos can be either:

- Base64-encoded data URLs (e.g., `data:image/svg+xml;base64,…`)
- HTTP/HTTPS URLs to hosted images

Common aliases include `brand` (main logo) and `small` (compact version).

### Print Font

A web-equivalent print font for a specific role.

| Field       | Type   | Description                                                               |
| ----------- | ------ | ------------------------------------------------------------------------- |
| `role`      | string | Font role: `heading`, `banner`, `subheading`, or `body`.                  |
| `family`    | string | CSS font-family to apply.                                                 |
| `weight`    | string | Optional font weight (e.g., `600`).                                       |
| `style`     | string | Optional font style (e.g., `italic`).                                     |
| `googleUrl` | string | Optional Google Fonts stylesheet URL the print window loads via `<link>`. |

### Brand Meta

| Field     | Type   | Description                 |
| --------- | ------ | --------------------------- |
| `name`    | string | Event or organization name. |
| `siteUrl` | string | Event website URL.          |

### Brand Example

```json
"brand": {
  "colors": {
    "primary": "#00BCDD",
    "black": "#000000",
    "darkGrey": "#18191C",
    "white": "#FFFFFF"
  },
  "logos": {
    "brand": "data:image/svg+xml;base64,…",
    "small": "https://static1.squarespace.com/…/logo.png"
  },
  "fonts": [
    {
      "role": "heading",
      "family": "Montserrat",
      "weight": "600",
      "googleUrl": "https://fonts.googleapis.com/css2?family=Montserrat:wght@600&display=swap"
    }
  ],
  "meta": {
    "name": "Cosplay America",
    "siteUrl": "https://cosplayamerica.com"
  }
}
```

## Print Formats

Optional. Shipped default print formats that seed the widget's "Print format"
dropdown, authored in `config/widget-default.toml` (override:
`config/widget.toml`). Each references brand by alias (`logo`) and role
(`fonts.*`). Empty-string fields mean "use the widget default".

| Field          | Type   | Values                                                                                                              |
| -------------- | ------ | ------------------------------------------------------------------------------------------------------------------- |
| `name`         | string | Display name (unique).                                                                                              |
| `contentMode`  | string | `both` \| `gridOnly` \| `descriptionOnly` \| `panelList`.                                                           |
| `colorMode`    | string | `color` \| `bw`.                                                                                                    |
| `columns`      | number | `0` = per-mode auto; `1`–`6` override.                                                                              |
| `headerText`   | string | Print header band text.                                                                                             |
| `footerText`   | string | Extra footer text.                                                                                                  |
| `footerMode`   | string | `full` \| `timestamp` \| `none`.                                                                                    |
| `logo`         | string | Brand logo alias or `none`.                                                                                         |
| `pageFill`     | string | CSS color; empty = white.                                                                                           |
| `cards`        | bool   | Render descriptions as bordered cards.                                                                              |
| `panelFilter`  | string | `all` \| `workshops` \| `premium`.                                                                                  |
| `timeSplit`    | string | `none` \| `day` \| `half_day` \| `timeline`. Splits the grid and descriptions into per-day / per-half-day sections. |
| `sectionSplit` | string | `none` \| `room` \| `presenter`.                                                                                    |
| `fonts`        | Object | `heading`/`banner`/`subheading`/`body` → a `brand.fonts` role or "".                                                |
| `fontSizes`    | Object | `base`/`grid`/`banner` point sizes (e.g. `"9pt"`) or "".                                                            |

### Content Mode

Controls what content appears in the print output:

- `both`: Grid and description sections
- `gridOnly`: Grid section only
- `descriptionOnly`: Description/list section only
- `panelList`: Simple list of panels

### Color Mode

Controls color rendering:

- `color`: Full color output
- `bw`: Black and white output

### Footer Mode

Controls footer content:

- `full`: Full footer with timestamp
- `timestamp`: Timestamp only
- `none`: No footer

### Panel Filter

Controls which panels are included:

- `all`: All panels
- `workshops`: Workshop panels only
- `premium`: Premium panels only

### Print Fonts

Object mapping font roles to brand font role references. Each value is a role key
into `brand.fonts` (e.g., `heading`) or an empty string to use the widget default.

| Field        | Type   | Description                     |
| ------------ | ------ | ------------------------------- |
| `heading`    | string | Heading font role reference.    |
| `banner`     | string | Banner font role reference.     |
| `subheading` | string | Subheading font role reference. |
| `body`       | string | Body font role reference.       |

### Print Font Sizes

Object mapping font roles to point size overrides. Each value is a CSS point size
(e.g., `9pt`) or an empty string to use the widget default.

| Field    | Type   | Description       |
| -------- | ------ | ----------------- |
| `base`   | string | Base font size.   |
| `grid`   | string | Grid font size.   |
| `banner` | string | Banner font size. |

### Print Format Example

```json
{
  "name": "Standard",
  "contentMode": "both",
  "colorMode": "color",
  "columns": 0,
  "headerText": "Cosplay America 2026",
  "footerText": "",
  "footerMode": "timestamp",
  "logo": "brand",
  "pageFill": "",
  "cards": true,
  "panelFilter": "all",
  "fonts": {
    "heading": "heading",
    "banner": "banner",
    "subheading": "subheading",
    "body": "body"
  },
  "fontSizes": {
    "base": "9pt",
    "grid": "8pt",
    "banner": "12pt"
  }
}
```

## Complete Example

```json
{
  "version": 1,
  "brand": {
    "colors": {
      "primary": "#00BCDD",
      "black": "#000000",
      "darkGrey": "#18191C",
      "white": "#FFFFFF"
    },
    "logos": {
      "brand": "data:image/svg+xml;base64,…",
      "small": "https://static1.squarespace.com/…/logo.png"
    },
    "fonts": [
      {
        "role": "heading",
        "family": "Montserrat",
        "weight": "600",
        "googleUrl": "https://fonts.googleapis.com/css2?family=Montserrat:wght@600&display=swap"
      }
    ],
    "meta": {
      "name": "Cosplay America",
      "siteUrl": "https://cosplayamerica.com"
    }
  },
  "printFormats": [
    {
      "name": "Standard",
      "contentMode": "both",
      "colorMode": "color",
      "columns": 0,
      "headerText": "Cosplay America 2026",
      "footerText": "",
      "footerMode": "timestamp",
      "logo": "brand",
      "pageFill": "",
      "cards": true,
      "panelFilter": "all",
      "fonts": {
        "heading": "heading",
        "banner": "banner",
        "subheading": "subheading",
        "body": "body"
      },
      "fontSizes": {
        "base": "9pt",
        "grid": "8pt",
        "banner": "12pt"
      }
    }
  ]
}
```

## Related Documentation

- [Widget JSON Format](widget-json-format.md) — Core schedule data format
- [Widget HTML Format](widget-html-format.md) — HTML-embedded schedule format
- [cosam-convert CLI](cosam-convert.md) — How to generate config from TOML files
