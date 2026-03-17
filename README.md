# cosam_sched

Interactive event calendar for Cosplay America. This is a complete rewrite of the [schedule-to-html](https://github.com/daphnediane/schedule-to-html) project, adapted for modern web embedding with enhanced interactivity.

## License

Copyright (c) 2026 Daphne Pfister. Licensed under the [BSD-2-Clause License](LICENSE).

## Attribution

This project is a rewrite of and based on the original [schedule-to-html](https://github.com/daphnediane/schedule-to-html) project. Development assisted by [Windsurf](https://windsurf.com/) AI.

## Spreadsheet Format

Same format as [schedule-to-html](https://github.com/daphnediane/schedule-to-html):

- **Schedule sheet** (main): Uniq_ID, Name, Description, Start_Time, End_Time, Duration, Room, Cost, Difficulty, Capacity, Kind, Note, Prereq, Ticket_Sale, Full, plus presenter columns (g1, g2, j1, s1, p1, etc.)
- **Rooms sheet**: Sort_Key, Room_Name, Hotel_Room, Long_Name
- **PanelTypes sheet**: Prefix, Panel_Kind, Hidden, Is_Break, Is_Café, Is_Workshop, Color

## Widget

See [widget/README.md](widget/README.md) for the embeddable calendar widget documentation.
