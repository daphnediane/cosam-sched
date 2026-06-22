/**
 * CosAm Calendar Widget
 * Embeddable interactive event calendar for Cosplay America
 * Vanilla JS — no framework dependencies
 * 
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */
import QRCode from 'qrcode';

(function () {
  'use strict';

  // ── SVG Icons ────────────────────────────────────────────────────────────
  // @TODO(dpfister): Double check if Windsurf borrowed this icons from somewhere and if so replace with properly attributed SVG assets.

  const ICONS = {
    star: '<svg viewBox="0 0 24 24"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>',
    filter: '<svg viewBox="0 0 24 24"><polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/></svg>',
    list: '<svg viewBox="0 0 24 24"><line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/></svg>',
    grid: '<svg viewBox="0 0 24 24"><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/></svg>',
    gridLines: '<svg viewBox="0 0 24 24"><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="18" x2="21" y2="18"/><line x1="6" y1="3" x2="6" y2="21"/><line x1="12" y1="3" x2="12" y2="21"/><line x1="18" y1="3" x2="18" y2="21"/></svg>',
    search: '<svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>',
    print: '<svg viewBox="0 0 24 24"><polyline points="6 9 6 2 18 2 18 9"/><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"/><rect x="6" y="14" width="12" height="8"/></svg>',
    x: '<svg viewBox="0 0 24 24"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>',
    share: '<svg viewBox="0 0 24 24"><circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/><line x1="8.59" y1="13.51" x2="15.42" y2="17.49"/><line x1="15.41" y1="6.51" x2="8.59" y2="10.49"/></svg>',
    shareApple: '<svg viewBox="0 0 24 24"><path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"/><polyline points="16 6 12 2 8 6"/><line x1="12" y1="2" x2="12" y2="15"/></svg>',
    shareWindows: '<svg viewBox="0 0 24 24"><path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"/><path d="M16 6l-4-4-4 4"/><line x1="12" y1="2" x2="12" y2="15"/></svg>',
    people: '<svg viewBox="0 0 24 24"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>',
    chevronDown: '<svg viewBox="0 0 24 24"><polyline points="6 9 12 15 18 9"/></svg>',
    clock: '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>',
    theme: '<svg viewBox="0 0 24 24"><g transform="translate(0.8, 1.9) scale(0.92)"><path d="M19.93,2.94C16.93.1,12.26-.71,8.43.64,4.91,1.77,1.7,4.75.59,8.32c-1.77,4.94.53,11.3,6.16,11.83,1.96.31,4.91.07,6.26-.89,2.39-1.32-.54-4.59,1.82-5.66l.12-.04c1.71-.53,4.05.18,5.56-1.07,2.94-2.36,2.02-7.25-.58-9.55ZM4,8.79c0-.94.76-1.7,1.7-1.7s1.71.76,1.71,1.7-.76,1.71-1.71,1.71-1.7-.77-1.7-1.71ZM7.64,17.01c-.94,0-1.7-.77-1.7-1.71s.76-1.7,1.7-1.7,1.7.76,1.7,1.7-.76,1.71-1.7,1.71ZM11.16,5.64c-.94,0-1.7-.76-1.7-1.71s.76-1.7,1.7-1.7,1.71.76,1.71,1.7-.76,1.71-1.71,1.71ZM17.31,9.14c-.95,0-1.71-.76-1.71-1.7s.76-1.71,1.71-1.71,1.7.77,1.7,1.71-.76,1.7-1.7,1.7Z"/></g></svg>',
    mappin: '<svg viewBox="0 0 24 24"><path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z"/><circle cx="12" cy="10" r="3"/></svg>',
    calendar: '<svg viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/></svg>',
    question: '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>',
  };

  // ── Helpers ──────────────────────────────────────────────────────────────

  function getShareIcon() {
    const ua = navigator.userAgent;
    const platform = navigator.platform;

    // Check for Windows platform
    if (/Win/.test(platform) ||
      /Win/.test(ua) ||
      (navigator.userAgentData && navigator.userAgentData.platform === 'Windows')) {
      return ICONS.shareWindows;
    }

    // Check for Android/Chrome/ChromeOS
    if (/Android|ChromeOS|CrOS/.test(ua) ||
      (navigator.userAgentData && (navigator.userAgentData.platform === 'Android' || navigator.userAgentData.platform === 'ChromeOS'))) {
      return ICONS.share;
    }

    // Default to Apple-style share icon
    return ICONS.shareApple;
  }

  function el(tag, attrs, ...children) {
    const e = document.createElement(tag);
    if (attrs) {
      for (const [k, v] of Object.entries(attrs)) {
        if (k === 'className') e.className = v;
        else if (k === 'innerHTML') e.innerHTML = v;
        else if (k.startsWith('on')) e.addEventListener(k.slice(2).toLowerCase(), v);
        else if (k === 'style' && typeof v === 'object') Object.assign(e.style, v);
        else e.setAttribute(k, v);
      }
    }
    for (const c of children) {
      if (typeof c === 'string') e.appendChild(document.createTextNode(c));
      else if (c) e.appendChild(c);
    }
    return e;
  }


  function formatTime(isoStr) {
    if (!isoStr) return '';
    const h = parseInt(isoStr.substring(11, 13), 10);
    const m = parseInt(isoStr.substring(14, 16), 10);
    if (isNaN(h) || isNaN(m)) return '';
    if (h === 0 && m === 0) return 'Midnight';
    if (h === 12 && m === 0) return 'Noon';
    const ampm = h >= 12 ? 'PM' : 'AM';
    const h12 = h % 12 || 12;
    return m === 0 ? `${h12} ${ampm}` : `${h12}:${String(m).padStart(2, '0')} ${ampm}`;
  }

  function formatTimeGrid(isoStr) {
    if (!isoStr) return '';
    const h = parseInt(isoStr.substring(11, 13), 10);
    const m = parseInt(isoStr.substring(14, 16), 10);
    if (isNaN(h) || isNaN(m)) return '';
    if (h === 0 && m === 0) return 'Midnight';
    if (h === 12 && m === 0) return 'Noon';
    const ampm = h >= 12 ? 'PM' : 'AM';
    const h12 = h % 12 || 12;
    return m === 0 ? `${h12} ${ampm}` : `${h12}:${String(m).padStart(2, '0')}`;
  }

  /**
   * Split time format for aligned time display.
   * Returns an object with:
   *   - isSpecial: true for Midnight/Noon (display centered across both columns)
   *   - hour: the hour part (right-aligned in left half)
   *   - suffix: AM/PM or :MM (left-aligned in right half)
   *   - full: complete time string for accessibility
   *   - label: user-friendly label for aria-label
   */
  function formatTimeSplit(isoStr) {
    if (!isoStr) return { isSpecial: true, hour: '', suffix: '', full: '', label: '' };
    // Parse hours and minutes directly from the schedule-timezone ISO wall-clock
    // string. Avoids browser-local timezone drift: new Date(naiveIso) is
    // implementation-defined (local or UTC depending on runtime).
    const h = parseInt(isoStr.substring(11, 13), 10);
    const m = parseInt(isoStr.substring(14, 16), 10);
    if (isNaN(h) || isNaN(m)) return { isSpecial: true, hour: '', suffix: '', full: '', label: '' };

    // Midnight and Noon span both columns (centered)
    if (h === 0 && m === 0) {
      return { isSpecial: true, hour: 'Midnight', suffix: '', full: 'Midnight', label: 'Midnight' };
    }
    if (h === 12 && m === 0) {
      return { isSpecial: true, hour: 'Noon', suffix: '', full: 'Noon', label: 'Noon' };
    }

    const ampm = h >= 12 ? 'PM' : 'AM';
    const h12 = h % 12 || 12;
    const ms = String(m).padStart(2, '0');

    if (m === 0) {
      // On the hour: hour in left, AM/PM in right (with non-breaking space)
      return {
        isSpecial: false,
        hour: String(h12),
        suffix: `\u00A0${ampm}`,
        full: `${h12} ${ampm}`,
        label: `${h12} ${ampm}`
      };
    } else {
      // With minutes: hour in left, :MM in right
      return {
        isSpecial: false,
        hour: String(h12),
        suffix: `:${ms}`,
        full: `${h12}:${ms} ${ampm}`,
        label: `${h12}:${ms} ${ampm}`
      };
    }
  }

  function formatDuration(minutes) {
    if (!minutes || minutes <= 0) return '';
    const h = Math.floor(minutes / 60);
    const m = minutes % 60;
    if (h === 0) return `${m} min`;
    if (m === 0) return `${h} hr`;
    return `${h} hr ${m} min`;
  }

  function formatTimeRange(start, end) {
    if (!start) return '';
    const s = formatTime(start);
    if (!end) return s;
    return `${s} – ${formatTime(end)}`;
  }

  function getDayLabel(isoStr) {
    if (!isoStr) return 'Unknown';
    // Parse only the date portion to avoid timezone drift: `new Date(isoStr)` on a
    // naive ISO string (no Z / offset) is implementation-defined and some runtimes
    // treat it as UTC, shifting the weekday in negative-UTC-offset timezones.
    // Using `new Date(year, month-1, day)` always creates a local-time date.
    const datePart = isoStr.length >= 10 ? isoStr.substring(0, 10) : isoStr;
    const parts = datePart.split('-').map(Number);
    if (parts.length !== 3 || parts.some(isNaN)) return 'Unknown';
    const d = new Date(parts[0], parts[1] - 1, parts[2]);
    return d.toLocaleDateString('en-US', { weekday: 'long', month: 'short', day: 'numeric' });
  }

  function getDayKey(isoStr) {
    if (!isoStr) return 'unknown';
    return isoStr.substring(0, 10);
  }

  function getTimeSlotKey(isoStr) {
    if (!isoStr) return '';
    return isoStr.substring(0, 16); // YYYY-MM-DDTHH:MM
  }

  // Round epoch seconds down to the nearest minute boundary for time-slot
  // grouping. Pure integer arithmetic — no Date object, no timezone lookup.
  function epochToSlotEpoch(epochSec) {
    return Math.floor(epochSec / 60) * 60;
  }

  // CSS-safe grid-line name from a slot epoch (epoch-minutes, prefixed 't').
  // Using absolute epoch-minutes avoids the weekday-collision in the old
  // t${dayNum}${HH}${MM} scheme (same weekday+time on different weeks collide)
  // and requires no Date parsing.
  function slotEpochToName(slotEpoch) {
    return 't' + Math.floor(slotEpoch / 60);
  }

  // FEATURE-154: Convert Unix epoch seconds to a naive wall-clock ISO string
  // (`YYYY-MM-DDTHH:MM:SS`) expressed in the given IANA timezone. The rest of the
  // widget parses these with `new Date(iso)` (i.e. as browser-local), which keeps
  // the wall-clock digits intact for display and time-slot bucketing. Returns ''
  // for non-numeric input. An unrecognized timezone falls back to UTC.
  function epochToLocalIso(epochSec, tzName) {
    if (typeof epochSec !== 'number' || !isFinite(epochSec)) return '';
    const format = (tz) => {
      const fmt = new Intl.DateTimeFormat('en-US', {
        timeZone: tz,
        year: 'numeric', month: '2-digit', day: '2-digit',
        hour: '2-digit', minute: '2-digit', second: '2-digit',
        hourCycle: 'h23',
      });
      const parts = {};
      for (const p of fmt.formatToParts(new Date(epochSec * 1000))) parts[p.type] = p.value;
      return `${parts.year}-${parts.month}-${parts.day}T${parts.hour}:${parts.minute}:${parts.second}`;
    };
    try {
      return format(tzName || 'UTC');
    } catch (e) {
      return format('UTC');
    }
  }

  // Get the local minute-of-hour (0–59) for a slot epoch using precomputed TZ
  // offsets from meta. Corrects for non-integer-hour timezone offsets (e.g. IST
  // UTC+5:30) where pure epoch modulo gives the wrong local minute. When
  // `tzOffsetMinutes` is null/undefined, falls back to plain epoch modulo.
  // `dstTransitionEpoch` / `dstOffsetMinutes` may be null for zones with no DST
  // transition in the schedule window.
  function localMinuteOfHour(slotEpoch, tzOffsetMinutes, dstTransitionEpoch, dstOffsetMinutes) {
    const offsetSecs = ((dstTransitionEpoch != null && slotEpoch >= dstTransitionEpoch)
      ? dstOffsetMinutes
      : (tzOffsetMinutes ?? 0)) * 60;
    return ((slotEpoch + offsetSecs) % 3600 + 3600) % 3600 / 60;
  }

  // Fill intermediate slot epochs from startEpoch to endEpoch at unitSecs
  // intervals (both boundaries inclusive). Pure epoch arithmetic — no ISO
  // parsing, no Date objects, no hardcoded reference dates.
  function getIntermediateSlotEpochs(startEpoch, endEpoch, unitSecs) {
    const slots = [];
    const start = epochToSlotEpoch(startEpoch);
    const end = epochToSlotEpoch(endEpoch);
    for (let s = start; s <= end; s += unitSecs) {
      slots.push(s);
    }
    return slots;
  }

  // Build the full set of slot epochs for fillPage (print) mode: all event
  // start/end boundaries plus intermediate unit-interval epochs so the grid
  // has an even time axis. Unit is the GCD of local minute-of-hour components
  // across regular events (using precomputed TZ offsets for correctness with
  // non-integer-hour zones like IST UTC+5:30), clamped to 15–60 min.
  function evenSlotEpochs(events, regularEvents, tzOffsetMinutes, dstTransitionEpoch, dstOffsetMinutes) {
    const out = new Set();
    for (const e of events) {
      if (typeof e.startEpoch === 'number') out.add(epochToSlotEpoch(e.startEpoch));
      if (typeof e.endEpoch === 'number') out.add(epochToSlotEpoch(e.endEpoch));
    }
    if (out.size < 2) return [...out].sort((a, b) => a - b);

    const gcd = (a, b) => b === 0 ? a : gcd(b, a % b);
    let unitMinutes = 60;
    for (const e of regularEvents) {
      for (const ep of [e.startEpoch, e.endEpoch]) {
        if (typeof ep === 'number') {
          const m = localMinuteOfHour(epochToSlotEpoch(ep), tzOffsetMinutes, dstTransitionEpoch, dstOffsetMinutes);
          if (m > 0) unitMinutes = gcd(unitMinutes, m);
        }
      }
    }
    unitMinutes = Math.max(15, Math.min(60, unitMinutes));
    const unitSecs = unitMinutes * 60;

    for (const evt of regularEvents) {
      if (typeof evt.startEpoch === 'number' && typeof evt.endEpoch === 'number') {
        for (const s of getIntermediateSlotEpochs(evt.startEpoch, evt.endEpoch, unitSecs)) {
          out.add(s);
        }
      }
    }
    return [...out].sort((a, b) => a - b);
  }

  function escapeHtml(s) {
    if (!s) return '';
    const div = document.createElement('div');
    div.textContent = s;
    return div.innerHTML;
  }

  // Convert hex to HSL and set lightness to a fixed pastel value (default 92%).
  // Returns a hex string.
  function _lightenColor(hex, targetLightness = 0.92) {
    if (!hex || typeof hex !== 'string') return hex;
    // Remove # if present
    hex = hex.replace('#', '');
    if (hex.length !== 6) return hex;

    const r = parseInt(hex.substring(0, 2), 16) / 255;
    const g = parseInt(hex.substring(2, 4), 16) / 255;
    const b = parseInt(hex.substring(4, 6), 16) / 255;

    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    let h, s, l = (max + min) / 2;

    if (max === min) {
      h = s = 0; // achromatic
    } else {
      const d = max - min;
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
      switch (max) {
        case r: h = ((g - b) / d + (g < b ? 6 : 0)) / 6; break;
        case g: h = ((b - r) / d + 2) / 6; break;
        case b: h = ((r - g) / d + 4) / 6; break;
      }
    }

    // Set lightness to target pastel value
    l = targetLightness;

    // Convert HSL back to RGB
    const hue2rgb = (p, q, t) => {
      if (t < 0) t += 1;
      if (t > 1) t -= 1;
      if (t < 1 / 6) return p + (q - p) * 6 * t;
      if (t < 1 / 2) return q;
      if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
      return p;
    };

    const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
    const p = 2 * l - q;
    const newR = hue2rgb(p, q, h + 1 / 3);
    const newG = hue2rgb(p, q, h);
    const newB = hue2rgb(p, q, h - 1 / 3);

    const toHex = (x) => Math.round(x * 255).toString(16).padStart(2, '0');
    return '#' + toHex(newR) + toHex(newG) + toHex(newB);
  }

  // Normalized, non-empty capacity string for an event, or '' when absent.
  function capacityText(evt) {
    if (evt.capacity === undefined || evt.capacity === null) return '';
    const c = String(evt.capacity).trim();
    return c;
  }

  // Explanatory sentence for a premium multi-part series, or '' when not
  // applicable. Every part states the same shared price so it is clear one
  // purchase covers the whole series. Mirrors the Typst premium-workshop notice.
  function seriesCostNote(evt) {
    if (!evt.totalParts || !evt.isPremium) return '';
    return evt.cost
      ? evt.cost + ' for the full series (Parts 1–' + evt.totalParts + ').'
      : 'One price covers all ' + evt.totalParts + ' parts.';
  }

  // ── iCalendar (.ics) helpers ─────────────────────────────────────────────
  // Schedule times are wall-clock in the schedule's timezone (meta.timezone).
  // When a timezone is known we emit DTSTART/DTEND with a TZID parameter and
  // embed the matching VTIMEZONE (meta.vtimezone) so calendar apps anchor the
  // event to the correct instant regardless of the viewer's local zone. When no
  // timezone is available we fall back to floating local DATE-TIME values.

  function _pad2(n) { return String(n).padStart(2, '0'); }

  // Floating local DATE-TIME (YYYYMMDDTHHMMSS, no trailing Z) from an ISO string.
  function icsDateLocal(iso) {
    if (!iso) return '';
    const d = new Date(iso);
    if (isNaN(d.getTime())) return '';
    return `${d.getFullYear()}${_pad2(d.getMonth() + 1)}${_pad2(d.getDate())}T` +
      `${_pad2(d.getHours())}${_pad2(d.getMinutes())}${_pad2(d.getSeconds())}`;
  }

  // UTC DATE-TIME with trailing Z, for DTSTAMP.
  function icsStampUtc(d) {
    return `${d.getUTCFullYear()}${_pad2(d.getUTCMonth() + 1)}${_pad2(d.getUTCDate())}T` +
      `${_pad2(d.getUTCHours())}${_pad2(d.getUTCMinutes())}${_pad2(d.getUTCSeconds())}Z`;
  }

  // Escape TEXT values per RFC 5545 (backslash, semicolon, comma, newlines).
  function icsEscape(s) {
    return String(s == null ? '' : s)
      .replace(/\\/g, '\\\\')
      .replace(/;/g, '\\;')
      .replace(/,/g, '\\,')
      .replace(/\r?\n/g, '\\n');
  }

  // Fold a content line to <=75 octets with CRLF + single-space continuation.
  // Approximated by character count, which is safe for the common ASCII case.
  function icsFold(line) {
    if (line.length <= 75) return line;
    let out = line.slice(0, 75);
    let idx = 75;
    while (idx < line.length) {
      out += '\r\n ' + line.slice(idx, idx + 74);
      idx += 74;
    }
    return out;
  }

  // Build the VEVENT component lines for a single event. `tzParam` is the
  // pre-computed `;TZID=<tzid>` suffix (empty for floating local times).
  function _icsVeventLines(evt, { rooms = [], descriptionLines = [], url = '', tzParam = '' } = {}) {
    const startIso = evt.startTime;
    let endIso = evt.endTime;
    if (!endIso && startIso && evt.duration) {
      // Derive an end from start + duration, kept as a local floating ISO value.
      const d = new Date(startIso);
      if (!isNaN(d.getTime())) {
        d.setMinutes(d.getMinutes() + evt.duration);
        endIso = `${d.getFullYear()}-${_pad2(d.getMonth() + 1)}-${_pad2(d.getDate())}T${_pad2(d.getHours())}:${_pad2(d.getMinutes())}:${_pad2(d.getSeconds())}`;
      }
    }

    const lines = [
      'BEGIN:VEVENT',
      'UID:' + icsEscape((evt.id || 'event') + '@cosam-calendar'),
      'DTSTAMP:' + icsStampUtc(new Date()),
    ];
    const dtStart = icsDateLocal(startIso);
    if (dtStart) lines.push('DTSTART' + tzParam + ':' + dtStart);
    const dtEnd = icsDateLocal(endIso);
    if (dtEnd) lines.push('DTEND' + tzParam + ':' + dtEnd);
    lines.push('SUMMARY:' + icsEscape(evt.name || 'Event'));
    if (rooms.length > 0) lines.push('LOCATION:' + icsEscape(rooms.join(', ')));
    const desc = descriptionLines.filter(Boolean).join('\n');
    if (desc) lines.push('DESCRIPTION:' + icsEscape(desc));
    if (url) lines.push('URL:' + icsEscape(url));
    lines.push('END:VEVENT');
    return lines;
  }

  // Build an iCalendar document string from one or more event entries. Each
  // entry is `{ evt, rooms, descriptionLines, url }`.
  //
  // When `tzid` is supplied the event times are emitted as DATE-TIME values
  // qualified with `;TZID=<tzid>` and the matching `vtimezone` block (if any) is
  // embedded; otherwise they are emitted as floating local DATE-TIME values.
  function buildIcsDoc(entries, { tzid = '', vtimezone = '' } = {}) {
    const lines = [
      'BEGIN:VCALENDAR',
      'VERSION:2.0',
      'PRODID:-//Cosplay America//Schedule Widget//EN',
      'CALSCALE:GREGORIAN',
      'METHOD:PUBLISH',
    ];
    // Embed the timezone definition before any component that references it.
    if (tzid && vtimezone) {
      for (const vtzLine of vtimezone.split(/\r?\n/)) {
        if (vtzLine) lines.push(vtzLine);
      }
    }
    // A TZID parameter is only valid if a matching VTIMEZONE is present.
    const tzParam = (tzid && vtimezone) ? ';TZID=' + tzid : '';
    for (const entry of entries) {
      const opts = Object.assign({ tzParam }, entry);
      lines.push(..._icsVeventLines(entry.evt, opts));
    }
    lines.push('END:VCALENDAR');

    return lines.map(icsFold).join('\r\n') + '\r\n';
  }

  // Trigger a client-side download of text content as a named file.
  function downloadFile(filename, content, mime) {
    const blob = new Blob([content], { type: mime || 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.style.display = 'none';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  // iOS/iPadOS Safari ignores the <a download> attribute for blob URLs and will
  // not hand a generated .ics to Calendar — those devices need a data: URL.
  function isAppleMobile() {
    const nav = window.navigator || {};
    const ua = nav.userAgent || '';
    return /iP(hone|ad|od)/.test(ua) ||
      // iPadOS 13+ reports as desktop Safari but exposes touch points.
      (nav.platform === 'MacIntel' && (nav.maxTouchPoints || 0) > 1);
  }

  // Filesystem-safe slug for filenames.
  function slugify(s) {
    return String(s || 'event').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 60) || 'event';
  }

  // ── State ───────────────────────────────────────────────────────────────

  class CalendarState {
    constructor() {
      this.view = 'list'; // 'list' or 'grid'
      this.theme = 'cosam';
      this.largeType = false; // large text mode
      this.printLayout = 'default'; // 'default', 'grid', 'list'
      this.printColor = 'color'; // 'color', 'bw'
      this.activeDay = null;
      this.days = [];
      // Named schedules: { scheduleName: Set<eventId> }
      this.schedules = { 'My Schedule': new Set() };
      this.activeScheduleName = 'My Schedule';
      // Transient: starred events from a shared URL (never persisted)
      this.sharedStarred = new Set();
      this.sharedScheduleName = 'Shared Schedule';
      // Transient: a single panel id from a share URL whose detail sheet should
      // auto-open on first render. Consumed (cleared) when opened.
      this.pendingPanelId = null;
      this.filters = {
        search: '',
        rooms: new Set(),
        types: new Set(),
        cost: 'all', // 'all', 'free', 'paid', 'workshop'
        presenter: '',
        starredOnly: false,
        sharedOnly: false,
      };
      this.filtersOpen = false;
      this.modalEvent = null;
      this.stylePageBody = false;
      // Grid view: show even time grid lines (regular time-unit rows between events)
      this.evenTimeGrid = false;
      // Show the even grid toggle button in the toolbar (for testing print layouts)
      this.showEvenGridSwitch = false;
      // Sticky-header top offset config (see CosAmCalendar.init).
      this.stickyOffset = 0;
      this.stickyOffsetSelector = null;
      this._hasRestoredState = false;
      this._savedView = null; // Saved view state before forced list mode
      this._renderCallback = null;
      this._loadState();
      this._syncStarred();
      this._loadFromHash();
      this._setupResponsiveView();
    }

    // starred is a direct reference to the active schedule's Set.
    // It must be kept in sync whenever activeScheduleName or schedules changes.
    // Use _syncStarred() after any such change.
    _syncStarred() {
      if (!(this.schedules[this.activeScheduleName] instanceof Set)) {
        this.schedules[this.activeScheduleName] = new Set();
      }
      this.starred = this.schedules[this.activeScheduleName];
    }

    _storageKey() { return 'cosam-calendar-starred'; }
    _themeStorageKey() { return 'cosam-calendar-theme'; }
    _stateStorageKey() { return 'cosam-calendar-state'; }

    _loadState() {
      try {
        const raw = localStorage.getItem(this._stateStorageKey());
        if (raw) {
          const saved = JSON.parse(raw);
          if (saved.theme) this.theme = saved.theme;
          if (saved.view) this.view = saved.view;
          if (saved.activeDay !== undefined) this.activeDay = saved.activeDay;
          if (saved.filtersOpen !== undefined) this.filtersOpen = saved.filtersOpen;
          if (saved.evenTimeGrid !== undefined) this.evenTimeGrid = saved.evenTimeGrid;
          if (saved.largeType !== undefined) this.largeType = saved.largeType;
          if (saved.printLayout) this.printLayout = saved.printLayout;
          if (saved.printColor) this.printColor = saved.printColor;

          // Load named schedules (new format), or migrate from single starred array
          if (saved.schedules && typeof saved.schedules === 'object' && !Array.isArray(saved.schedules)) {
            this.schedules = {};
            for (const [name, ids] of Object.entries(saved.schedules)) {
              this.schedules[name] = new Set(ids);
            }
            this.activeScheduleName = saved.activeScheduleName || 'My Schedule';
            if (!this.schedules[this.activeScheduleName]) {
              this.activeScheduleName = Object.keys(this.schedules)[0] || 'My Schedule';
            }
          } else if (saved.starred) {
            // Migrate single starred array → 'My Schedule'
            this.schedules = { 'My Schedule': new Set(saved.starred) };
            this.activeScheduleName = 'My Schedule';
          } else {
            this.schedules = { 'My Schedule': new Set() };
            this.activeScheduleName = 'My Schedule';
          }
          // Fallback: ensure at least one schedule exists
          if (Object.keys(this.schedules).length === 0) this.schedules['My Schedule'] = new Set();

          if (saved.filters) {
            if (saved.filters.search) this.filters.search = saved.filters.search;
            if (saved.filters.rooms) this.filters.rooms = new Set(saved.filters.rooms.map(Number));
            if (saved.filters.types) this.filters.types = new Set(saved.filters.types);
            if (saved.filters.cost) this.filters.cost = saved.filters.cost;
            if (saved.filters.presenter) this.filters.presenter = saved.filters.presenter;
            if (saved.filters.starredOnly) this.filters.starredOnly = saved.filters.starredOnly;
          }
          this._hasRestoredState = true;
          this._syncStarred();
          return;
        }
      } catch (e) { /* ignore */ }

      this.schedules = { 'My Schedule': new Set() };
      this.activeScheduleName = 'My Schedule';
      try {
        const themeRaw = localStorage.getItem(this._themeStorageKey());
        if (themeRaw) this.theme = themeRaw;
      } catch (e) { /* ignore */ }
      try {
        const starredRaw = localStorage.getItem(this._storageKey());
        if (starredRaw) this.schedules['My Schedule'] = new Set(JSON.parse(starredRaw));
      } catch (e) { /* ignore */ }
      this._syncStarred();
    }

    _saveState() {
      try {
        const schedulesObj = {};
        for (const [name, ids] of Object.entries(this.schedules)) {
          schedulesObj[name] = [...ids];
        }
        const state = {
          theme: this.theme,
          view: this.view,
          activeDay: this.activeDay,
          filtersOpen: this.filtersOpen,
          evenTimeGrid: this.evenTimeGrid,
          largeType: this.largeType,
          printLayout: this.printLayout,
          printColor: this.printColor,
          activeScheduleName: this.activeScheduleName,
          schedules: schedulesObj,
          filters: {
            search: this.filters.search,
            rooms: [...this.filters.rooms],
            types: [...this.filters.types],
            cost: this.filters.cost,
            presenter: this.filters.presenter,
            starredOnly: this.filters.starredOnly,
          },
        };
        localStorage.setItem(this._stateStorageKey(), JSON.stringify(state));
      } catch (e) { /* ignore */ }
    }

    setTheme(theme) {
      this.theme = theme || 'cosam';
      this._saveState();
    }

    _loadFromHash() {
      const hash = window.location.hash;
      if (!hash || hash.length < 2) return;
      const params = new URLSearchParams(hash.substring(1));

      if (params.has('panel')) {
        const pid = decodeURIComponent(params.get('panel')).trim();
        if (pid) this.pendingPanelId = pid;
      }
      if (params.has('starred')) {
        const ids = decodeURIComponent(params.get('starred')).split(',').filter(Boolean);
        if (ids.length > 0) {
          // Treat starred IDs from the URL as a shared schedule, not the user's own.
          // The user's own starred items are always loaded from localStorage only.
          for (const id of ids) this.sharedStarred.add(id);
        }
      }
      if (params.has('scheduleName')) {
        this.sharedScheduleName = decodeURIComponent(params.get('scheduleName'));
      }
      if (params.has('view')) {
        const view = params.get('view');
        if (view === 'list' || view === 'grid') this.view = view;
      }
      if (params.has('day')) {
        this.activeDay = params.get('day') || null;
      }
      if (params.has('search')) {
        this.filters.search = params.get('search');
      }
      if (params.has('rooms')) {
        const rooms = decodeURIComponent(params.get('rooms')).split(',').filter(Boolean).map(Number);
        this.filters.rooms = new Set(rooms);
      }
      if (params.has('types')) {
        const types = decodeURIComponent(params.get('types')).split(',').filter(Boolean);
        this.filters.types = new Set(types);
      }
      if (params.has('cost')) {
        this.filters.cost = params.get('cost');
      }
      if (params.has('presenter')) {
        this.filters.presenter = decodeURIComponent(params.get('presenter'));
      }
      if (params.has('starredOnly')) {
        this.filters.starredOnly = params.get('starredOnly') === '1';
      }

      this._saveState();
    }

    _setupResponsiveView() {
      const BREAKPOINT = 750;

      const checkWidth = () => {
        const isNarrow = window.innerWidth < BREAKPOINT;
        if (isNarrow && this.view === 'grid') {
          this._savedView = 'grid';
          this.view = 'list';
          if (this._renderCallback) this._renderCallback();
          return true;
        } else if (!isNarrow && this._savedView === 'grid' && this.view === 'list') {
          this.view = 'grid';
          this._savedView = null;
          if (this._renderCallback) this._renderCallback();
          return true;
        }
        return false;
      };

      // Check on load
      checkWidth();

      // Listen for resize
      window.addEventListener('resize', () => {
        checkWidth();
      });
    }

    toggleStar(eventId) {
      if (this.starred.has(eventId)) this.starred.delete(eventId);
      else this.starred.add(eventId);
      this._saveState();
    }

    // ── Schedule management ──

    schedulesForEvent(eventId) {
      const names = [];
      for (const [name, ids] of Object.entries(this.schedules)) {
        if (ids.has(eventId)) names.push(name);
      }
      if (this.sharedStarred.has(eventId)) names.push(this.sharedScheduleName);
      return names;
    }

    createSchedule(name) {
      const n = name && name.trim();
      if (!n || this.schedules[n]) return null;
      this.schedules[n] = new Set();
      this._saveState();
      return n;
    }

    deleteSchedule(name) {
      if (!this.schedules[name] || Object.keys(this.schedules).length <= 1) return false;
      delete this.schedules[name];
      if (this.activeScheduleName === name) {
        this.activeScheduleName = Object.keys(this.schedules)[0];
      }
      this._syncStarred();
      this._saveState();
      return true;
    }

    renameSchedule(oldName, newName) {
      const n = newName && newName.trim();
      if (!n || !this.schedules[oldName] || (this.schedules[n] && n !== oldName)) return false;
      this.schedules[n] = this.schedules[oldName];
      if (n !== oldName) delete this.schedules[oldName];
      if (this.activeScheduleName === oldName) this.activeScheduleName = n;
      this._saveState();
      return true;
    }

    switchSchedule(name) {
      if (!this.schedules[name]) return;
      this.activeScheduleName = name;
      this._syncStarred();
      this._saveState();
    }

    mergeIntoSchedule(sourceIds, targetName) {
      if (!this.schedules[targetName]) return false;
      for (const id of sourceIds) this.schedules[targetName].add(id);
      this._saveState();
      return true;
    }

    replaceSchedule(sourceIds, targetName) {
      this.schedules[targetName] = new Set(sourceIds);
      if (targetName === this.activeScheduleName) this._syncStarred();
      this._saveState();
    }

    importAsNewSchedule(name, ids) {
      let n = (name && name.trim()) ? name.trim() : 'Imported Schedule';
      if (this.schedules[n]) {
        let i = 2;
        while (this.schedules[n + ' ' + i]) i++;
        n = n + ' ' + i;
      }
      this.schedules[n] = new Set(ids);
      this._saveState();
      return n;
    }

    // Share URL for a single panel. Opening it auto-pops the detail sheet for
    // this panel (see _loadFromHash / render auto-open).
    getPanelShareUrl(eventId) {
      const base = window.location.href.split('#')[0];
      return base + '#panel=' + encodeURIComponent(eventId);
    }

    getShareUrl(options = {}) {
      const { includeFilters = false, scheduleName, includeSchedule = true } = options;
      const shareScheduleName = scheduleName !== undefined ? scheduleName : this.activeScheduleName;
      const schedule = includeSchedule ? this.schedules[shareScheduleName] : null;
      const parts = [];

      if (schedule && schedule.size > 0) {
        parts.push('starred=' + encodeURIComponent([...schedule].join(',')));
        parts.push('scheduleName=' + encodeURIComponent(shareScheduleName));
      }
      if (includeFilters) {
        if (this.view && this.view !== 'list') {
          parts.push('view=' + this.view);
        }
        if (this.activeDay) {
          parts.push('day=' + encodeURIComponent(this.activeDay));
        }
        if (this.filters.search) {
          parts.push('search=' + encodeURIComponent(this.filters.search));
        }
        if (this.filters.rooms.size > 0) {
          parts.push('rooms=' + encodeURIComponent([...this.filters.rooms].join(',')));
        }
        if (this.filters.types.size > 0) {
          parts.push('types=' + encodeURIComponent([...this.filters.types].join(',')));
        }
        if (this.filters.cost && this.filters.cost !== 'all') {
          parts.push('cost=' + this.filters.cost);
        }
        if (this.filters.presenter) {
          parts.push('presenter=' + encodeURIComponent(this.filters.presenter));
        }
        if (this.filters.starredOnly) {
          parts.push('starredOnly=1');
        }
      }

      const base = window.location.href.split('#')[0];
      return parts.length > 0 ? base + '#' + parts.join('&') : base;
    }

    _isBreakEvent(e) {
      if (!e.panelType || !this.data.panelTypes) return false;
      const pt = this.data.panelTypes.find(p => p.uid === e.panelType);
      return pt && pt.isBreak;
    }

    _isSplitEvent(e) {
      if (!e.panelType || !this.data.panelTypes) return false;
      const pt = this.data.panelTypes.find(p => p.uid === e.panelType);
      return pt && pt.isTimeline;
    }

    filteredEvents() {
      if (!this.data) return [];
      let events = this.data.panels;

      // Remove SPLIT events (page-break markers for print layout)
      events = events.filter(e => !this._isSplitEvent(e));

      // Day filter — prefer precomputed dayKey (FEATURE-154); fall back to
      // substring extraction from the naive wall-clock startTime ISO string for
      // pre-v2 data (no epoch seconds, no dayKey). getDayKey is pure string
      // parsing (isoStr.substring(0, 10)), so no Date conversion is needed.
      if (this.activeDay) {
        events = events.filter(e => (e.dayKey || getDayKey(e.startTime)) === this.activeDay);
      }

      // Search — breaks excluded when searching
      if (this.filters.search) {
        const q = this.filters.search.toLowerCase();
        events = events.filter(e =>
          (e.name && e.name.toLowerCase().includes(q)) ||
          (e.description && e.description.toLowerCase().includes(q)) ||
          (e.presenters && e.presenters.some(p => p.toLowerCase().includes(q)))
        );
      }

      // Rooms — breaks pass through
      if (this.filters.rooms.size > 0) {
        events = events.filter(e => this._isBreakEvent(e) || (e.roomIds && e.roomIds.some(id => this.filters.rooms.has(id))));
      }

      // Types — breaks excluded when filtering by type
      if (this.filters.types.size > 0) {
        events = events.filter(e => e.panelType && this.filters.types.has(e.panelType));
      }

      // Cost — breaks excluded when filtering by cost
      if (this.filters.cost === 'included') {
        events = events.filter(e => !e.isPremium);
      } else if (this.filters.cost === 'premium') {
        events = events.filter(e => e.isPremium);
      }

      // Presenter — breaks excluded when filtering by presenter
      if (this.filters.presenter) {
        const selectedPresenter = this.filters.presenter;

        // Use precomputed panelIds for efficient filtering
        let presenterPanelIds = this.data.presenterToPanels.get(selectedPresenter);

        if (!presenterPanelIds) {
          // If no direct panelIds, check if this is a group and collect from members
          const selectedPresenterData = this.data.presenters.find(p => p.name === selectedPresenter);
          if (selectedPresenterData && selectedPresenterData.isGroup && selectedPresenterData.members) {
            const allGroupPanelIds = new Set();
            for (const memberName of selectedPresenterData.members) {
              const memberPanelIds = this.data.presenterToPanels.get(memberName);
              if (memberPanelIds) {
                for (const panelId of memberPanelIds) {
                  allGroupPanelIds.add(panelId);
                }
              }
            }
            presenterPanelIds = allGroupPanelIds;
          }
        }

        if (presenterPanelIds && presenterPanelIds.size > 0) {
          events = events.filter(e => presenterPanelIds.has(e.id));
        } else {
          events = []; // Presenter not found, or group has no panels
        }
      }

      // Starred only
      if (this.filters.starredOnly) {
        events = events.filter(e => this.starred.has(e.id));
      }

      // Shared only
      if (this.filters.sharedOnly) {
        events = events.filter(e => this.sharedStarred.has(e.id));
      }

      return events;
    }
  }

  // ── Renderer ────────────────────────────────────────────────────────────

  class CalendarRenderer {
    constructor(rootEl, state) {
      this.root = rootEl;
      this.state = state;
      this._filtersId = 'cosam-filters-panel';
      this._eventsRegionId = 'cosam-events-region';
      // .cosam-screen marks the live widget so screen-only rules (responsive
      // collapse, hover, sticky) can target it positively, mirroring the print
      // container's .cosam-print marker.
      this.root.classList.add('cosam-calendar', 'cosam-screen');
      this.root.setAttribute('role', 'region');
      this.root.setAttribute('aria-label', 'Cosplay America schedule');

      // Intercept Ctrl/Cmd-P so the browser prints our purpose-built print DOM
      // (day pages, page-fill grid) via _doPrint instead of the on-screen
      // widget. Only when data is loaded; otherwise let the browser print
      // normally. The print DOM is built in a separate window, so no @media
      // print rules are needed on the widget itself.
      this._onPrintShortcut = (e) => {
        if ((e.ctrlKey || e.metaKey) && !e.altKey && (e.key === 'p' || e.key === 'P')) {
          if (!this.state.data) return;
          e.preventDefault();
          this._doPrint();
        }
      };
      window.addEventListener('keydown', this._onPrintShortcut);
    }

    render() {
      this.root.innerHTML = '';
      if (!this.state.data) {
        const loadStatus = this.state._loadStatus || 'loading';
        const isError = loadStatus === 'error';
        const loadingDiv = el('div', { className: 'cosam-loading' + (isError ? ' cosam-loading--error' : '') });
        loadingDiv.appendChild(document.createTextNode(
          isError ? 'Failed to load schedule: ' + (this.state._loadError || 'Unknown error') : 'Loading schedule...'
        ));
        if ((loadStatus === 'slow' || isError) && this.state._reloadCallback) {
          const reloadBtn = el('button', { type: 'button', className: 'cosam-reload-btn' }, 'Reload');
          reloadBtn.addEventListener('click', this.state._reloadCallback);
          loadingDiv.appendChild(reloadBtn);
        }
        this.root.appendChild(loadingDiv);
        return;
      }
      const theme = this.state.theme || 'cosam';
      this.root.setAttribute('data-theme', theme);
      if (this.state.largeType) {
        this.root.classList.add('cosam-large-type');
      } else {
        this.root.classList.remove('cosam-large-type');
      }
      this._applyPageStyling(theme);
      this._applyHeaderPageColor(theme);
      this._applyStickyOffset();
      this.state._saveState();
      this._ensurePanelTypeThemeStyles();
      this.root.appendChild(el('a', { className: 'cosam-skip-link', href: '#' + this._eventsRegionId }, 'Skip to events'));
      if (this.state.sharedStarred.size > 0) {
        this.root.appendChild(this._buildSharedScheduleBanner());
      }
      this.root.appendChild(this._buildToolbar());
      this.root.appendChild(this._buildFilters());
      this.root.appendChild(this._buildDayTabs());

      const events = this.state.filteredEvents.call(this.state);
      const eventsRegion = el('section', {
        id: this._eventsRegionId,
        className: 'cosam-events-region',
        'aria-live': 'polite',
        'aria-label': 'Filtered schedule results',
      });
      if (events.length === 0) {
        eventsRegion.appendChild(el('div', { className: 'cosam-empty' }, 'No events match your filters.'));
      } else if (this.state.view === 'grid') {
        eventsRegion.appendChild(this._buildGridView(events, false, this.state.evenTimeGrid));
      } else {
        eventsRegion.appendChild(this._buildListView(events));
      }
      this.root.appendChild(eventsRegion);

      this.root.appendChild(this._buildModal());

      // Apply color bar styles after rendering
      this._updateColorBarStyles();

      // Auto-open the detail sheet for a shared single panel (one-shot).
      if (this.state.pendingPanelId) {
        const pid = this.state.pendingPanelId;
        this.state.pendingPanelId = null;
        const evt = this.state.data.panels.find(p => p.id === pid);
        if (evt) {
          this.state.modalEvent = evt;
          this._showModal(evt);
        }
      }
    }

    // Set --cosam-sticky-offset so sticky headers pin below a host-page fixed
    // top bar (e.g. a Squarespace mobile nav). Uses the configured fixed pixel
    // offset and/or the measured height of a fixed bar matched by selector.
    _applyStickyOffset() {
      let px = this.state.stickyOffset || 0;
      const sel = this.state.stickyOffsetSelector;
      if (sel) {
        try {
          const bar = document.querySelector(sel);
          if (bar) {
            const cs = window.getComputedStyle(bar);
            const pinnedTop = (cs.position === 'fixed' || cs.position === 'sticky');
            if (pinnedTop && cs.display !== 'none' && cs.visibility !== 'hidden') {
              const r = bar.getBoundingClientRect();
              // Only count a bar actually pinned at/near the viewport top.
              if (r.height > 0 && r.top <= 1) px = Math.max(px, Math.round(r.bottom));
            }
          }
        } catch (e) { /* invalid selector — ignore */ }
      }
      this.root.style.setProperty('--cosam-sticky-offset', px + 'px');
    }

    _applyPageStyling(theme) {
      if (this.state.stylePageBody) {
        document.body.classList.add('cosam-page-styled');
        document.body.setAttribute('data-cosam-theme', theme);
      } else {
        document.body.classList.remove('cosam-page-styled');
        document.body.removeAttribute('data-cosam-theme');
      }
    }

    // Sticky list-view day/time headers need an opaque background to occlude
    // panels scrolling under them. The default (transparent) theme sits on the
    // host page, so the headers should take the host's page color rather than a
    // fixed surface color that reads as white. Detect the effective page
    // background and expose it as --cosam-page-bg; the CSS falls back to the
    // surface color when nothing opaque is found. Named themes ship their own
    // opaque --cosam-time-header-bg, so we clear any detected value for them.
    _applyHeaderPageColor(theme) {
      if (theme && theme !== 'cosam') {
        this.root.style.removeProperty('--cosam-page-bg');
        return;
      }
      const color = this._resolveOpaquePageColor(this.root);
      if (color) {
        this.root.style.setProperty('--cosam-page-bg', color);
      } else {
        this.root.style.removeProperty('--cosam-page-bg');
      }
    }

    // Walk ancestors from the widget root outward, returning the first fully
    // opaque computed background color (skipping the transparent widget itself).
    // Returns null if none is found, so the CSS fallback applies.
    _resolveOpaquePageColor(startEl) {
      let node = startEl ? startEl.parentElement : null;
      while (node) {
        const bg = window.getComputedStyle(node).backgroundColor;
        if (this._isOpaqueColor(bg)) return bg;
        node = node.parentElement;
      }
      return null;
    }

    _isOpaqueColor(color) {
      if (!color || color === 'transparent') return false;
      const m = color.match(/^rgba?\(([^)]+)\)$/i);
      if (!m) return true; // named/hex value: assume opaque
      const parts = m[1].split(',').map((s) => s.trim());
      if (parts.length < 4) return true; // rgb() with no alpha is opaque
      return parseFloat(parts[3]) >= 0.999;
    }

    _panelTypeClass(panelTypeUid, prefix = 'cosam-') {
      if (!panelTypeUid) return '';
      const slug = String(panelTypeUid).trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
      return slug ? prefix + 'panel-type-' + slug : '';
    }

    _normalizeDataModel(data) {
      if (!data || typeof data !== 'object') return data;

      // v7 hashmap panelTypes: convert to array with uid and flattened color
      let panelTypes;
      if (data.panelTypes && typeof data.panelTypes === 'object' && !Array.isArray(data.panelTypes)) {
        panelTypes = Object.entries(data.panelTypes).map(([prefix, pt]) => ({
          ...pt,
          uid: prefix,
          prefix: prefix,
          color: (pt.colors && pt.colors.color) || null,
        }));
      } else {
        panelTypes = [];
      }

      // Panels are used as-is; panelType is the raw prefix matching panelTypes keys
      const panels = Array.isArray(data.panels) ? data.panels : [];

      // Build presenter-to-panel mapping for efficient lookups.
      // Presenters are a top-level array in the export format.
      let presenters = [];
      let presenterToPanels = new Map();

      if (Array.isArray(data.presenters)) {
        presenters = data.presenters;

        for (const presenter of data.presenters.filter(p => p.panelIds && p.panelIds.length > 0)) {
          presenterToPanels.set(presenter.name, new Set(presenter.panelIds));
        }
      } else {
        console.error('Unsupported format - no presenter data found');
        presenters = [];
      }

      // Filter rooms to those used by real (non-break, non-timeline) panels,
      // then normalize all field names to camelCase.
      // The new export format uses camelCase (shortName, longName, hotelRoom,
      // sortKey); accept snake_case from older data too.
      let rooms = [];
      if (Array.isArray(data.rooms)) {
        const usedRoomIds = new Set();
        for (const panel of panels) {
          if (!panel.roomIds) continue;
          const pt = panelTypes.find(p => p.uid === panel.panelType);
          if (pt && (pt.isBreak || pt.isTimeline)) continue;
          for (const roomId of panel.roomIds) {
            usedRoomIds.add(roomId);
          }
        }

        rooms = data.rooms
          .filter(room => {
            const roomId = room.uid !== undefined ? room.uid : room.id;
            if (room.isBreak || room.is_break) return false;
            return usedRoomIds.has(roomId);
          })
          .map(room => ({
            uid: room.uid !== undefined ? room.uid : room.id,
            shortName: room.shortName || room.short_name || '',
            longName: room.longName || room.long_name || '',
            hotelRoom: room.hotelRoom || room.hotel_room || '',
            sortKey: room.sortKey !== undefined ? room.sortKey
              : room.sort_key !== undefined ? room.sort_key : 0,
          }));
      }

      // FEATURE-154: derive wall-clock ISO times from the canonical epoch-seconds
      // fields (interpreted in meta.timezone) so display and time-slot bucketing
      // are timezone-correct and no longer depend on the emitted ISO strings.
      // Pre-v2 data without epoch keeps its existing naive ISO strings.
      const tzName = (data.meta && data.meta.timezone) || 'UTC';
      const tzOffsetMinutes = (data.meta && data.meta.tzOffsetMinutes) ?? null;
      const tzDstTransitionEpoch = (data.meta && data.meta.tzDstTransitionEpoch) ?? null;
      const tzDstOffsetMinutes = (data.meta && data.meta.tzDstOffsetMinutes) ?? null;
      const localizedPanels = panels.map(p => {
        if (typeof p.startEpoch !== 'number' && typeof p.endEpoch !== 'number') return p;
        const out = { ...p };
        if (typeof p.startEpoch === 'number') out.startTime = epochToLocalIso(p.startEpoch, tzName);
        if (typeof p.endEpoch === 'number') out.endTime = epochToLocalIso(p.endEpoch, tzName);
        return out;
      });
      const localizedTimeline = Array.isArray(data.timeline)
        ? data.timeline.map(t => (typeof t.startEpoch === 'number'
          ? { ...t, startTime: epochToLocalIso(t.startEpoch, tzName) }
          : t))
        : data.timeline;
      let meta = data.meta;
      if (meta && (meta.startEpoch || meta.endEpoch)) {
        meta = { ...meta };
        if (meta.startEpoch) meta.startTime = epochToLocalIso(meta.startEpoch, tzName);
        if (meta.endEpoch) meta.endTime = epochToLocalIso(meta.endEpoch, tzName);
      }

      return {
        ...data,
        meta,
        panelTypes,
        panels: localizedPanels,
        timeline: localizedTimeline,
        presenters,
        rooms,
        presenterToPanels,
        tzOffsetMinutes,
        tzDstTransitionEpoch,
        tzDstOffsetMinutes,
      };
    }

    _ensurePanelTypeThemeStyles() {
      const panelTypes = this.state.data && this.state.data.panelTypes;
      if (!Array.isArray(panelTypes) || panelTypes.length === 0) return;

      // Store panel type colors for direct application (key by slug, not full class)
      this._panelTypeColors = new Map();
      for (const pt of panelTypes) {
        const slug = String(pt.uid).trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
        if (!slug || !pt.color) continue;
        this._panelTypeColors.set(slug, pt.color);
      }

      // Apply styles to existing color bars
      this._updateColorBarStyles();
    }

    _updateColorBarStyles() {
      if (!this._panelTypeColors) return;

      // Helper to extract slug from panel-type class
      const getSlugFromClass = (el, prefix) => {
        for (const cls of el.classList) {
          const match = cls.match(new RegExp(`^${prefix}panel-type-(.+)$`));
          if (match) return match[1];
        }
        return null;
      };

      // Live widget only ever holds screen (.cosam-) elements; print containers
      // are built detached and written to a separate window.
      const prefixes = ['cosam-'];
      for (const prefix of prefixes) {
        const colorBars = this.root.querySelectorAll('.' + prefix + 'event-color-bar');
        for (const bar of colorBars) {
          const slug = getSlugFromClass(bar, prefix);
          if (slug && this._panelTypeColors.has(slug)) {
            bar.style.backgroundColor = this._panelTypeColors.get(slug);
          }
        }

        // Update grid view events
        const gridEvents = this.root.querySelectorAll('.' + prefix + 'grid-event');
        for (const event of gridEvents) {
          const slug = getSlugFromClass(event, prefix);
          if (slug && this._panelTypeColors.has(slug)) {
            event.style.borderLeftColor = this._panelTypeColors.get(slug);
          }
        }
      }
    }

    // ── Schedule Menu ──

    _buildScheduleMenu() {
      const menu = el('div', { className: 'cosam-schedule-menu', role: 'menu' });

      // One item per named schedule
      for (const [name, ids] of Object.entries(this.state.schedules)) {
        const isActive = name === this.state.activeScheduleName;
        const item = el('button', {
          type: 'button',
          className: 'cosam-schedule-menu-item' + (isActive ? ' active' : ''),
          role: 'menuitem',
        });
        const check = el('span', { className: 'cosam-schedule-menu-check', 'aria-hidden': 'true' }, isActive ? '✓' : '');
        const label = el('span', {}, name);
        const count = el('span', { className: 'cosam-schedule-menu-count' }, '(' + ids.size + ')');
        item.append(check, label, count);
        item.addEventListener('click', () => {
          this.state.switchSchedule(name);
          this.state.filters.starredOnly = false;
          this.state.filters.sharedOnly = false;
          menu.classList.remove('open');
          this.render();
        });
        menu.appendChild(item);
      }

      menu.appendChild(el('div', { className: 'cosam-schedule-menu-divider', role: 'separator' }));

      // New schedule
      const newItem = el('button', { type: 'button', className: 'cosam-schedule-menu-item', role: 'menuitem' }, '+ New Schedule');
      newItem.addEventListener('click', () => { menu.classList.remove('open'); this._showNewScheduleModal(); });
      menu.appendChild(newItem);

      // Rename active
      const renameItem = el('button', { type: 'button', className: 'cosam-schedule-menu-item', role: 'menuitem' },
        'Rename "' + this.state.activeScheduleName + '"');
      renameItem.addEventListener('click', () => { menu.classList.remove('open'); this._showRenameScheduleModal(); });
      menu.appendChild(renameItem);

      // Merge (only when >1 schedule)
      if (Object.keys(this.state.schedules).length > 1) {
        const mergeItem = el('button', { type: 'button', className: 'cosam-schedule-menu-item', role: 'menuitem' }, 'Merge schedules...');
        mergeItem.addEventListener('click', () => { menu.classList.remove('open'); this._showMergeScheduleModal(); });
        menu.appendChild(mergeItem);
      }

      // Import from URL
      const importItem = el('button', { type: 'button', className: 'cosam-schedule-menu-item', role: 'menuitem' }, 'Import from URL...');
      importItem.addEventListener('click', () => { menu.classList.remove('open'); this._showImportFromUrlModal(); });
      menu.appendChild(importItem);

      // Delete (only when >1 schedule)
      if (Object.keys(this.state.schedules).length > 1) {
        menu.appendChild(el('div', { className: 'cosam-schedule-menu-divider', role: 'separator' }));
        const deleteItem = el('button', { type: 'button', className: 'cosam-schedule-menu-item cosam-schedule-menu-danger', role: 'menuitem' },
          'Delete "' + this.state.activeScheduleName + '"');
        deleteItem.addEventListener('click', () => { menu.classList.remove('open'); this._showDeleteScheduleModal(); });
        menu.appendChild(deleteItem);
      }

      return menu;
    }

    _getThemeLabel(theme) {
      const labels = {
        'cosam': 'Default',
        'light': 'Light',
        'dark': 'Dark',
        'high-contrast': 'High Contrast',
      };
      return labels[theme] || 'Default';
    }

    _buildThemeMenu() {
      const menu = el('div', { className: 'cosam-theme-menu', role: 'menu' });
      const themes = [
        ['cosam', 'Default'],
        ['light', 'Light'],
        ['dark', 'Dark'],
        ['high-contrast', 'High Contrast'],
      ];
      for (const [value, label] of themes) {
        const isActive = this.state.theme === value;
        const item = el('button', {
          type: 'button',
          className: 'cosam-theme-menu-item' + (isActive ? ' active' : ''),
          role: 'menuitem',
        });
        const check = el('span', { className: 'cosam-theme-menu-check', 'aria-hidden': 'true' }, isActive ? '✓' : '');
        const text = el('span', {}, label);
        item.append(check, text);
        item.addEventListener('click', () => {
          this.state.setTheme(value);
          menu.classList.remove('open');
          this.render();
        });
        menu.appendChild(item);
      }

      menu.appendChild(el('div', { className: 'cosam-theme-menu-divider', role: 'separator' }));

      // Large type toggle
      const largeTypeItem = el('button', {
        type: 'button',
        className: 'cosam-theme-menu-item' + (this.state.largeType ? ' active' : ''),
        role: 'menuitem',
      });
      const largeTypeCheck = el('span', { className: 'cosam-theme-menu-check', 'aria-hidden': 'true' }, this.state.largeType ? '✓' : '');
      const largeTypeText = el('span', {}, 'Large Type');
      largeTypeItem.append(largeTypeCheck, largeTypeText);
      largeTypeItem.addEventListener('click', () => {
        this.state.largeType = !this.state.largeType;
        menu.classList.remove('open');
        this.render();
      });
      menu.appendChild(largeTypeItem);

      return menu;
    }

    _buildPrintMenu() {
      const menu = el('div', { className: 'cosam-print-menu', role: 'menu' });

      // Layout options
      const layoutLabel = el('div', { className: 'cosam-print-menu-label' }, 'Layout');
      menu.appendChild(layoutLabel);

      const layouts = [
        ['default', 'Default (Smart)'],
        ['grid', 'Grid'],
        ['list', 'List'],
      ];
      for (const [value, label] of layouts) {
        const isActive = this.state.printLayout === value;
        const item = el('button', {
          type: 'button',
          className: 'cosam-print-menu-item' + (isActive ? ' active' : ''),
          role: 'menuitem',
        });
        const check = el('span', { className: 'cosam-print-menu-check', 'aria-hidden': 'true' }, isActive ? '✓' : '');
        const text = el('span', {}, label);
        item.append(check, text);
        item.addEventListener('click', () => {
          this.state.printLayout = value;
          menu.classList.remove('open');
          this.render();
        });
        menu.appendChild(item);
      }

      menu.appendChild(el('div', { className: 'cosam-print-menu-divider', role: 'separator' }));

      // Color options
      const colorLabel = el('div', { className: 'cosam-print-menu-label' }, 'Color');
      menu.appendChild(colorLabel);

      const colors = [
        ['color', 'Color'],
        ['bw', 'Black & White'],
      ];
      for (const [value, label] of colors) {
        const isActive = this.state.printColor === value;
        const item = el('button', {
          type: 'button',
          className: 'cosam-print-menu-item' + (isActive ? ' active' : ''),
          role: 'menuitem',
        });
        const check = el('span', { className: 'cosam-print-menu-check', 'aria-hidden': 'true' }, isActive ? '✓' : '');
        const text = el('span', {}, label);
        item.append(check, text);
        item.addEventListener('click', () => {
          this.state.printColor = value;
          menu.classList.remove('open');
          this.render();
        });
        menu.appendChild(item);
      }

      return menu;
    }

    // ── Shared Schedule Banner ──

    _buildSharedScheduleBanner() {
      const count = this.state.sharedStarred.size;
      const name = this.state.sharedScheduleName;
      const banner = el('div', {
        className: 'cosam-shared-banner',
        role: 'region',
        'aria-label': 'Shared schedule notification',
      });

      const textSpan = el('span', { className: 'cosam-shared-banner-text' });
      textSpan.innerHTML = ICONS.people;
      textSpan.appendChild(document.createTextNode(
        ` "${name}": ${count} event${count === 1 ? '' : 's'}`
      ));
      banner.appendChild(textSpan);

      const actions = el('div', { className: 'cosam-shared-banner-actions' });

      // Import as new schedule
      const importNewBtn = el('button', {
        type: 'button',
        className: 'cosam-btn',
        onClick: () => {
          const newName = this.state.importAsNewSchedule(name, this.state.sharedStarred);
          this.state.sharedStarred.clear();
          this.state.filters.sharedOnly = false;
          this.state.switchSchedule(newName);
          this.render();
        },
      }, 'Import as New Schedule');
      actions.appendChild(importNewBtn);

      // Merge into selector
      const mergeWrap = el('span', { className: 'cosam-shared-banner-merge' });
      const mergeSelect = el('select', { className: 'cosam-select', 'aria-label': 'Target schedule for merge' });
      for (const schedName of Object.keys(this.state.schedules)) {
        mergeSelect.appendChild(el('option', { value: schedName }, schedName));
      }
      // Default to matching name if it exists, else active schedule
      mergeSelect.value = this.state.schedules[name] ? name : this.state.activeScheduleName;
      const mergeBtn = el('button', {
        type: 'button',
        className: 'cosam-btn',
        onClick: () => {
          this.state.mergeIntoSchedule(this.state.sharedStarred, mergeSelect.value);
          this.state.sharedStarred.clear();
          this.state.filters.sharedOnly = false;
          this.render();
        },
      }, 'Merge into:');
      mergeWrap.append(mergeBtn, mergeSelect);
      actions.appendChild(mergeWrap);

      const dismissBtn = el('button', {
        type: 'button',
        className: 'cosam-btn cosam-btn-icon',
        innerHTML: ICONS.x,
        'aria-label': 'Dismiss shared schedule',
        title: 'Dismiss shared schedule',
        onClick: () => {
          this.state.sharedStarred.clear();
          this.state.filters.sharedOnly = false;
          this.render();
        },
      });
      actions.appendChild(dismissBtn);

      banner.appendChild(actions);
      return banner;
    }

    // ── Toolbar ──

    _buildToolbar() {
      const left = el('div', { className: 'cosam-toolbar-left' });

      // View toggles
      const listBtn = el('button', {
        type: 'button',
        className: 'cosam-btn cosam-btn-icon cosam-view-toggle' + (this.state.view === 'list' ? ' active' : ''),
        title: 'List View',
        'aria-label': 'List view',
        'aria-pressed': this.state.view === 'list' ? 'true' : 'false',
        innerHTML: ICONS.list,
        onClick: () => { this.state.view = 'list'; this.render(); },
      });
      const gridBtn = el('button', {
        type: 'button',
        className: 'cosam-btn cosam-btn-icon cosam-view-toggle' + (this.state.view === 'grid' ? ' active' : ''),
        title: 'Grid View',
        'aria-label': 'Grid view',
        'aria-pressed': this.state.view === 'grid' ? 'true' : 'false',
        innerHTML: ICONS.grid,
        onClick: () => { this.state.view = 'grid'; this.render(); },
      });
      left.append(listBtn, gridBtn);

      // Even time grid toggle (only in grid view and when enabled via init flag)
      if (this.state.view === 'grid' && this.state.showEvenGridSwitch) {
        const evenGridBtn = el('button', {
          type: 'button',
          className: 'cosam-btn cosam-btn-icon' + (this.state.evenTimeGrid ? ' active' : ''),
          title: 'Even Time Grid',
          'aria-label': 'Even time grid',
          'aria-pressed': this.state.evenTimeGrid ? 'true' : 'false',
          innerHTML: ICONS.gridLines,
          onClick: () => { this.state.evenTimeGrid = !this.state.evenTimeGrid; this.render(); },
        });
        left.appendChild(evenGridBtn);
      }

      // Filter toggle
      const filterBtn = el('button', {
        type: 'button',
        className: 'cosam-btn' + (this.state.filtersOpen ? ' active' : ''),
        innerHTML: ICONS.filter + ' Filters',
        'aria-expanded': this.state.filtersOpen ? 'true' : 'false',
        'aria-controls': this._filtersId,
        onClick: () => { this.state.filtersOpen = !this.state.filtersOpen; this.render(); },
      });
      left.appendChild(filterBtn);

      // Schedule group: filter button + dropdown menu
      const scheduleGroup = el('div', { className: 'cosam-schedule-group' });

      const schedFilterBtn = el('button', {
        type: 'button',
        className: 'cosam-btn cosam-schedule-filter-btn' + (this.state.filters.starredOnly ? ' active' : ''),
        innerHTML: ICONS.star + ' ' + escapeHtml(this.state.activeScheduleName),
        'aria-pressed': this.state.filters.starredOnly ? 'true' : 'false',
        title: 'Filter to ' + this.state.activeScheduleName,
        onClick: () => { this.state.filters.starredOnly = !this.state.filters.starredOnly; this.render(); },
      });

      const schedMenuBtn = el('button', {
        type: 'button',
        className: 'cosam-btn cosam-btn-icon cosam-schedule-menu-btn',
        innerHTML: ICONS.chevronDown,
        'aria-label': 'Schedule options',
        'aria-haspopup': 'menu',
        'aria-expanded': 'false',
        title: 'Schedule options',
      });

      const schedMenu = this._buildScheduleMenu();
      schedMenuBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        const isOpen = schedMenu.classList.toggle('open');
        schedMenuBtn.setAttribute('aria-expanded', isOpen ? 'true' : 'false');
        if (isOpen) {
          const close = (ev) => {
            if (!scheduleGroup.contains(ev.target)) {
              schedMenu.classList.remove('open');
              schedMenuBtn.setAttribute('aria-expanded', 'false');
              document.removeEventListener('click', close);
            }
          };
          document.addEventListener('click', close);
        }
      });

      scheduleGroup.append(schedFilterBtn, schedMenuBtn, schedMenu);
      left.appendChild(scheduleGroup);

      // Shared schedule toggle — only shown when a shared schedule is loaded
      if (this.state.sharedStarred.size > 0) {
        const sharedBtn = el('button', {
          type: 'button',
          className: 'cosam-btn cosam-btn-shared' + (this.state.filters.sharedOnly ? ' active' : ''),
          innerHTML: ICONS.people + ' ' + escapeHtml(this.state.sharedScheduleName),
          'aria-pressed': this.state.filters.sharedOnly ? 'true' : 'false',
          title: 'Show only events from the shared schedule',
          onClick: () => { this.state.filters.sharedOnly = !this.state.filters.sharedOnly; this.render(); },
        });
        left.appendChild(sharedBtn);
      }

      const right = el('div', { className: 'cosam-toolbar-right' });

      // Search
      const searchWrap = el('div', { className: 'cosam-search-wrap' });
      searchWrap.innerHTML = ICONS.search;
      const searchInput = el('input', {
        type: 'text',
        placeholder: 'Search events...',
        value: this.state.filters.search,
        'aria-label': 'Search events',
      });
      let searchTimer = null;
      searchInput.addEventListener('input', () => {
        clearTimeout(searchTimer);
        searchTimer = setTimeout(() => {
          this.state.filters.search = searchInput.value;
          this.render();
          // Refocus search after render
          const newInput = this.root.querySelector('.cosam-search-wrap input');
          if (newInput) { newInput.focus(); newInput.selectionStart = newInput.selectionEnd = newInput.value.length; }
        }, 250);
      });
      searchWrap.appendChild(searchInput);
      right.appendChild(searchWrap);

      // Theme dropdown
      const themeGroup = el('div', { className: 'cosam-theme-group' });
      const themeBtn = el('button', {
        type: 'button',
        className: 'cosam-btn cosam-btn-icon cosam-theme-btn',
        'aria-label': 'Theme',
        'aria-haspopup': 'menu',
        'aria-expanded': 'false',
        title: 'Theme',
      });
      const themeLabel = el('span', { className: 'cosam-theme-label' });
      themeLabel.innerHTML = ICONS.theme + ' ' + escapeHtml(this._getThemeLabel(this.state.theme));
      const themeChevron = el('span', { className: 'cosam-theme-chevron' });
      themeChevron.innerHTML = ICONS.chevronDown;
      themeBtn.append(themeLabel, themeChevron);

      const themeMenu = this._buildThemeMenu();
      themeBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        const isOpen = themeMenu.classList.toggle('open');
        themeBtn.setAttribute('aria-expanded', isOpen ? 'true' : 'false');
        if (isOpen) {
          const close = (ev) => {
            if (!themeGroup.contains(ev.target)) {
              themeMenu.classList.remove('open');
              themeBtn.setAttribute('aria-expanded', 'false');
              document.removeEventListener('click', close);
            }
          };
          document.addEventListener('click', close);
        }
      });

      themeGroup.append(themeBtn, themeMenu);
      right.appendChild(themeGroup);

      // Share
      const shareBtn = el('button', {
        type: 'button',
        className: 'cosam-btn',
        title: 'Share schedule',
        'aria-label': 'Share schedule',
        innerHTML: getShareIcon() + ' Share',
        onClick: () => { this._showShareModal(); },
      });
      right.appendChild(shareBtn);

      // Print
      if (this.state._printPlugin && typeof this.state._printPlugin.extendToolbar === 'function') {
        // Print plugin registered: show simple print button and let plugin add its UI
        const printBtn = el('button', {
          type: 'button',
          className: 'cosam-btn cosam-btn-icon',
          title: 'Print schedule',
          'aria-label': 'Print schedule',
          innerHTML: ICONS.print,
          onClick: () => this._handlePrint(),
        });
        right.appendChild(printBtn);
        this.state._printPlugin.extendToolbar(right, this._printPluginCtx());
      } else {
        // No print plugin: show built-in print dropdown
        const printGroup = el('div', { className: 'cosam-print-group' });

        // Print action button
        const printBtn = el('button', {
          type: 'button',
          className: 'cosam-btn cosam-btn-icon cosam-print-action-btn',
          title: 'Print schedule',
          'aria-label': 'Print schedule',
          innerHTML: ICONS.print,
          onClick: () => this._handlePrint(),
        });
        printGroup.appendChild(printBtn);

        // Options dropdown button
        const printOptionsBtn = el('button', {
          type: 'button',
          className: 'cosam-btn cosam-btn-icon cosam-print-options-btn',
          'aria-label': 'Print options',
          'aria-haspopup': 'menu',
          'aria-expanded': 'false',
          title: 'Print options',
        });
        const printChevron = el('span', { className: 'cosam-print-chevron' });
        printChevron.innerHTML = ICONS.chevronDown;
        printOptionsBtn.appendChild(printChevron);

        const printMenu = this._buildPrintMenu();
        printOptionsBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          const isOpen = printMenu.classList.toggle('open');
          printOptionsBtn.setAttribute('aria-expanded', isOpen ? 'true' : 'false');
          if (isOpen) {
            const close = (ev) => {
              if (!printGroup.contains(ev.target)) {
                printMenu.classList.remove('open');
                printOptionsBtn.setAttribute('aria-expanded', 'false');
                document.removeEventListener('click', close);
              }
            };
            document.addEventListener('click', close);
          }
        });

        printGroup.append(printOptionsBtn, printMenu);
        right.appendChild(printGroup);
      }

      // Help
      const helpBtn = el('button', {
        type: 'button',
        className: 'cosam-btn cosam-btn-icon',
        title: 'Help / How to use',
        'aria-label': 'Help',
        innerHTML: ICONS.question,
        onClick: () => { this._showHelpModal(); },
      });
      right.appendChild(helpBtn);

      const toolbar = el('div', { className: 'cosam-toolbar' }, left, right);
      return toolbar;
    }

    // ── Filters ──

    _buildFilters() {
      const panel = el('div', {
        id: this._filtersId,
        className: 'cosam-filters' + (this.state.filtersOpen ? ' open' : ''),
        role: 'region',
        'aria-label': 'Schedule filters',
      });

      // Row 1: Room + Type
      const row1 = el('div', { className: 'cosam-filter-row' });

      // Room filter
      const roomGroup = el('div', { className: 'cosam-filter-group' });
      roomGroup.appendChild(el('label', {}, 'Room'));
      const roomChips = el('div', { className: 'cosam-filter-checkboxes' });
      for (const room of this.state.data.rooms) {
        const name = room.longName || room.shortName;
        const displayName = (room.hotelRoom && room.hotelRoom !== name)
          ? `${name} (${room.hotelRoom})` : name;
        const selected = this.state.filters.rooms.has(room.uid);
        const chip = el('button', {
          type: 'button',
          className: 'cosam-filter-chip' + (selected ? ' selected' : ''),
          'aria-pressed': selected ? 'true' : 'false',
          onClick: () => {
            if (this.state.filters.rooms.has(room.uid)) this.state.filters.rooms.delete(room.uid);
            else this.state.filters.rooms.add(room.uid);
            this.render();
          },
        }, displayName);
        roomChips.appendChild(chip);
      }
      roomGroup.appendChild(roomChips);
      row1.appendChild(roomGroup);

      // Type filter
      const typeGroup = el('div', { className: 'cosam-filter-group' });
      typeGroup.appendChild(el('label', {}, 'Event Type'));
      const typeChips = el('div', { className: 'cosam-filter-checkboxes' });
      for (const pt of this.state.data.panelTypes) {
        if (pt.isBreak || pt.isHidden || pt.isTimeline || pt.isPrivate) continue;
        const typeValue = pt.uid;
        const selected = this.state.filters.types.has(typeValue);
        const chip = el('button', {
          type: 'button',
          className: 'cosam-filter-chip' + (selected ? ' selected' : ''),
          'aria-pressed': selected ? 'true' : 'false',
          onClick: () => {
            if (this.state.filters.types.has(typeValue)) this.state.filters.types.delete(typeValue);
            else this.state.filters.types.add(typeValue);
            this.render();
          },
        }, pt.kind || pt.uid);
        typeChips.appendChild(chip);
      }
      typeGroup.appendChild(typeChips);
      row1.appendChild(typeGroup);
      panel.appendChild(row1);

      // Row 2: Cost + Presenter
      const row2 = el('div', { className: 'cosam-filter-row' });

      // Pricing filter
      const costGroup = el('div', { className: 'cosam-filter-group' });
      costGroup.appendChild(el('label', {}, 'Pricing'));
      const costChips = el('div', { className: 'cosam-filter-checkboxes' });
      const activeCost = ['all', 'included', 'premium'].includes(this.state.filters.cost)
        ? this.state.filters.cost : 'all';
      for (const [value, label] of [['all', 'All'], ['included', 'Included'], ['premium', 'Premium']]) {
        const selected = activeCost === value;
        const chip = el('button', {
          type: 'button',
          className: 'cosam-filter-chip' + (selected ? ' selected' : ''),
          'aria-pressed': selected ? 'true' : 'false',
          onClick: () => { this.state.filters.cost = value; this.render(); },
        }, label);
        costChips.appendChild(chip);
      }
      costGroup.appendChild(costChips);
      row2.appendChild(costGroup);

      // Presenter filter — guests, guest-ranked groups, other panelists, non-guest-ranked groups
      const presGroup = el('div', { className: 'cosam-filter-group' });
      presGroup.appendChild(el('label', {}, 'Presenter'));

      const guestPresenters = [];
      const panelistPresenters = [];
      const guestGroups = [];
      const otherGroups = [];

      for (const p of this.state.data.presenters) {
        // Treat groups with empty member lists as individuals
        if (p.isGroup && p.members && p.members.length > 0) {
          if (p.rank === 'guest') guestGroups.push(p);
          else otherGroups.push(p);
        } else if (p.rank === 'guest') {
          guestPresenters.push(p);
        } else {
          panelistPresenters.push(p);
        }
      }

      // Sort presenters alphabetically within each category
      guestPresenters.sort((a, b) => a.name.localeCompare(b.name));
      panelistPresenters.sort((a, b) => a.name.localeCompare(b.name));
      guestGroups.sort((a, b) => a.name.localeCompare(b.name));
      otherGroups.sort((a, b) => a.name.localeCompare(b.name));

      const presSelect = el('select', {
        className: 'cosam-select cosam-presenter-select',
        onChange: (e) => {
          this.state.filters.presenter = e.target.value;
          this.render();
        }
      });

      presSelect.appendChild(el('option', { value: '' }, '— All Presenters —'));

      if (guestPresenters.length > 0) {
        const guestGroup = el('optgroup', { label: 'Guest Presenters' });
        for (const p of guestPresenters) {
          const opt = el('option', { value: p.name }, p.name);
          if (this.state.filters.presenter === p.name) opt.selected = true;
          guestGroup.appendChild(opt);
        }
        presSelect.appendChild(guestGroup);
      }

      if (guestGroups.length > 0) {
        const guestGroupGroup = el('optgroup', { label: 'Guest Groups' });
        for (const p of guestGroups) {
          const opt = el('option', { value: p.name }, p.name);
          if (this.state.filters.presenter === p.name) opt.selected = true;
          guestGroupGroup.appendChild(opt);
        }
        presSelect.appendChild(guestGroupGroup);
      }

      if (panelistPresenters.length > 0) {
        const panelistGroup = el('optgroup', { label: 'Panelists' });
        for (const p of panelistPresenters) {
          let displayText = p.name;
          // Only append group name if the group has subsumes_members: true (defined with ==)
          if (p.groups && p.groups.length > 0) {
            const subsumingGroups = p.groups.filter(groupName => {
              const group = this.state.data.presenters.find(p => p.name === groupName);
              return group && group.subsumesMembers;
            });
            if (subsumingGroups.length > 0) {
              displayText += ' (' + subsumingGroups.join(', ') + ')';
            }
          }
          const opt = el('option', { value: p.name }, displayText);
          if (this.state.filters.presenter === p.name) opt.selected = true;
          panelistGroup.appendChild(opt);
        }
        presSelect.appendChild(panelistGroup);
      }

      if (otherGroups.length > 0) {
        const otherGroupGroup = el('optgroup', { label: 'Other Groups' });
        for (const p of otherGroups) {
          const opt = el('option', { value: p.name }, p.name);
          if (this.state.filters.presenter === p.name) opt.selected = true;
          otherGroupGroup.appendChild(opt);
        }
        presSelect.appendChild(otherGroupGroup);
      }


      presGroup.appendChild(presSelect);
      row2.appendChild(presGroup);

      panel.appendChild(row2);

      // Clear filters button
      const actions = el('div', { className: 'cosam-filter-actions' });
      actions.appendChild(el('button', {
        type: 'button',
        className: 'cosam-btn',
        onClick: () => {
          this.state.filters.search = '';
          this.state.filters.rooms.clear();
          this.state.filters.types.clear();
          this.state.filters.cost = 'all';
          this.state.filters.presenter = '';
          this.state.filters.starredOnly = false;
          this.render();
        },
      }, 'Clear All Filters'));
      panel.appendChild(actions);

      return panel;
    }

    // ── Day Tabs ──

    _buildDayTabs() {
      const container = el('div', { className: 'cosam-day-tabs-container' });

      // Tab buttons (shown on larger screens)
      const tabs = el('div', { className: 'cosam-day-tabs' });

      // "All" tab
      const allTab = el('button', {
        className: 'cosam-day-tab' + (!this.state.activeDay ? ' active' : ''),
        onClick: () => { this.state.activeDay = null; this.render(); },
      }, 'All Days');
      tabs.appendChild(allTab);

      for (const day of this.state.days) {
        const tab = el('button', {
          className: 'cosam-day-tab' + (this.state.activeDay === day.key ? ' active' : ''),
          onClick: () => { this.state.activeDay = day.key; this.render(); },
        }, day.label);
        tabs.appendChild(tab);
      }
      container.appendChild(tabs);

      // Select dropdown (shown on smaller screens)
      const select = el('select', {
        className: 'cosam-day-select',
        'aria-label': 'Select day',
      });

      const allOption = el('option', { value: '' }, 'All Days');
      if (!this.state.activeDay) allOption.selected = true;
      select.appendChild(allOption);

      for (const day of this.state.days) {
        const option = el('option', { value: day.key }, day.label);
        if (this.state.activeDay === day.key) option.selected = true;
        select.appendChild(option);
      }

      select.addEventListener('change', () => {
        this.state.activeDay = select.value || null;
        this.render();
      });

      container.appendChild(select);

      return container;
    }

    // ── List View ──

    _buildListView(events, isPrintLayout = false, printPrefix = 'cosam-') {
      const container = el('div', { className: printPrefix + 'list-view' });

      // Group by epoch-minute slot key (sort order = chronological).
      const listDayTimeline = (this.state.data && this.state.data.dayTimeline) || [];
      const groups = new Map();
      for (const evt of events) {
        const key = typeof evt.startEpoch === 'number' ? epochToSlotEpoch(evt.startEpoch) : getTimeSlotKey(evt.startTime);
        if (!groups.has(key)) groups.set(key, []);
        groups.get(key).push(evt);
      }

      // Sort slot keys chronologically (numbers sort correctly with numeric compare).
      const sortedTimeKeys = Array.from(groups.keys()).sort((a, b) => a - b);

      // Each day becomes a sticky section: the day header pins to the top of the
      // viewport for the whole day, with the time header pinning just beneath it.
      // Use evt.dayKey (precomputed by Rust, respects overnight borrow) for day
      // boundary detection; derive the label from the dayTimeline entry date string.
      let currentDayKey = null;
      let daySection = container;

      for (const timeKey of sortedTimeKeys) {
        const evts = groups.get(timeKey);
        const firstEvt = evts && evts.length > 0 ? evts[0] : null;

        // dayKey: precomputed on panel (correct for borrowed overnight sessions).
        const dayKey = firstEvt
          ? (firstEvt.dayKey || getDayKey(firstEvt.startTime))
          : null;
        if (dayKey && dayKey !== currentDayKey) {
          currentDayKey = dayKey;
          // Label: prefer dayTimeline entry (no Date parsing of epoch needed);
          // fall back to getDayLabel which parses from the YYYY-MM-DD string.
          const dtEntry = listDayTimeline.find(d => d.date === dayKey);
          const dayLabel = dtEntry ? getDayLabel(dtEntry.date) : getDayLabel(dayKey);
          daySection = el('div', { className: printPrefix + 'day-section' });
          daySection.appendChild(el('div', {
            className: printPrefix + 'day-header',
            role: 'heading',
            'aria-level': '2',
          }, dayLabel));
          container.appendChild(daySection);
        }

        const group = el('div', { className: printPrefix + 'time-group' });
        const timeHeader = el('div', { className: printPrefix + 'time-header' });
        // Use split time format for aligned display with accessibility
        const timeSplit = formatTimeSplit(evts[0] ? evts[0].startTime : null);
        if (timeSplit.isSpecial) {
          // Midnight/Noon - centered across both columns
          timeHeader.appendChild(el('div', {
            className: printPrefix + 'time-header-time ' + printPrefix + 'time-split ' + printPrefix + 'time-special',
            'aria-label': timeSplit.label,
          }, timeSplit.hour));
        } else {
          // Regular time - split into hour (right) and suffix (left)
          const timeContainer = el('div', {
            className: printPrefix + 'time-header-time ' + printPrefix + 'time-split',
            'aria-label': timeSplit.label,
          });
          // Screen reader only full time
          timeContainer.appendChild(el('span', { className: printPrefix + 'sr-only' }, timeSplit.full));
          // Visible hour part (right-aligned)
          timeContainer.appendChild(el('span', {
            className: printPrefix + 'time-hour',
            'aria-hidden': 'true',
          }, timeSplit.hour));
          // Visible suffix part (left-aligned, AM/PM or :MM)
          timeContainer.appendChild(el('span', {
            className: printPrefix + 'time-suffix',
            'aria-hidden': 'true',
          }, timeSplit.suffix));
          timeHeader.appendChild(timeContainer);
        }
        group.appendChild(timeHeader);

        for (const evt of evts) {
          if (this.state._isBreakEvent(evt)) {
            group.appendChild(this._buildBreakBanner(evt, printPrefix));
          } else {
            group.appendChild(this._buildEventCard(evt, isPrintLayout, printPrefix));
          }
        }
        daySection.appendChild(group);
      }

      return container;
    }

    _buildBreakBanner(evt, printPrefix = 'cosam-') {
      const isOvernight = evt.panelType === '%NB';
      const banner = el('div', {
        className: printPrefix + 'break-banner' + (isOvernight ? ' ' + printPrefix + 'implicit-overnight-break' : ''),
        role: 'button',
        tabindex: '0',
        onClick: () => { this.state.modalEvent = evt; this._showModal(evt); },
      });
      banner.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          this.state.modalEvent = evt;
          this._showModal(evt);
        }
      });

      // Add moon for overnight breaks
      if (isOvernight) {
        const nameWrapper = el('div', { className: printPrefix + 'break-name' });
        nameWrapper.appendChild(el('span', { className: printPrefix + 'implicit-overnight-moon' }, '🌙'));
        nameWrapper.appendChild(document.createTextNode(' ' + evt.name));
        banner.appendChild(nameWrapper);
      } else {
        banner.appendChild(el('div', { className: printPrefix + 'break-name' }, evt.name));
      }
      if (evt.description) {
        banner.appendChild(el('div', { className: printPrefix + 'break-desc' }, evt.description));
      }
      const timeStr = formatTimeRange(evt.startTime, evt.endTime);
      if (timeStr) {
        const meta = el('div', { className: printPrefix + 'break-meta' });
        meta.innerHTML = ICONS.clock + ' ' + escapeHtml(timeStr);
        banner.appendChild(meta);
      }
      return banner;
    }

    _buildEventCard(evt, isPrintLayout = false, printPrefix = 'cosam-') {
      const isStarred = this.state.starred.has(evt.id);
      const isShared = this.state.sharedStarred.has(evt.id);
      const typeClass = this._panelTypeClass(evt.panelType, printPrefix);
      const card = el('div', {
        className: printPrefix + 'event-card' + (isStarred ? ' starred' : '') + (isShared ? ' ' + printPrefix + 'shared' : ''),
      });

      // Color bar
      if (typeClass) {
        card.appendChild(el('div', {
          className: printPrefix + 'event-color-bar ' + typeClass,
          'aria-hidden': 'true',
        }));
      }

      // Body
      const body = el('div', { className: printPrefix + 'event-body' });

      // Title
      body.appendChild(el('div', { className: printPrefix + 'event-title' }, evt.name));

      // Meta
      const meta = el('div', { className: printPrefix + 'event-meta' });
      const metaLeft = el('div', { className: printPrefix + 'event-meta-left' });
      const metaRight = el('div', { className: printPrefix + 'event-meta-right' });

      // For print layout (Typst style): title+presenter left, room+time right
      // For screen display: time+room left, cost+capacity right
      if (isPrintLayout) {
        // Print layout: presenter on left, room+time on right
        if (evt.credits && evt.credits.length > 0) {
          const presenterSpan = el('span', { className: printPrefix + 'meta-presenter' });
          presenterSpan.textContent = evt.credits.join(', ');
          metaLeft.appendChild(presenterSpan);
        }

        // Right side: room and time
        if (evt.roomIds && evt.roomIds.length > 0) {
          const roomElements = [];
          for (const roomId of evt.roomIds) {
            const room = this.state.data.rooms.find(r => r.uid === roomId);
            if (!room) continue;
            const roomName = room.longName || room.shortName;
            roomElements.push(roomName);
          }
          if (roomElements.length > 0) {
            const roomSpan = el('span', { className: printPrefix + 'meta-room' });
            roomSpan.textContent = roomElements.join(', ');
            metaRight.appendChild(roomSpan);
          }
        }
        if (evt.startTime) {
          const timeSpan = el('span', { className: printPrefix + 'meta-time' });
          timeSpan.textContent = formatTimeRange(evt.startTime, evt.endTime);
          if (metaRight.children.length > 0) {
            metaRight.appendChild(document.createTextNode(' \\ '));
          }
          metaRight.appendChild(timeSpan);
        }
      } else {
        // Screen layout: time and room on left
        if (evt.startTime) {
          const timeSpan = el('span', { className: printPrefix + 'meta-time' });
          timeSpan.innerHTML = ICONS.clock + ' ' + escapeHtml(formatTimeRange(evt.startTime, evt.endTime));
          metaLeft.appendChild(timeSpan);
        }
        // Rooms - V5 roomIds array
        if (evt.roomIds && evt.roomIds.length > 0) {
          const roomSpan = el('span', { className: printPrefix + 'meta-room' });
          roomSpan.innerHTML = ICONS.mappin;
          const roomElements = [];
          for (const roomId of evt.roomIds) {
            const room = this.state.data.rooms.find(r => r.uid === roomId);
            if (!room) continue;
            const roomName = room.longName || room.shortName;
            const textWrap = el('span', { className: printPrefix + 'meta-room-text' });
            textWrap.appendChild(el('span', {}, roomName));
            if (room.hotelRoom && room.hotelRoom !== roomName) {
              textWrap.appendChild(el('span', { className: printPrefix + 'meta-room-sub' }, `(${room.hotelRoom})`));
            }
            roomElements.push(textWrap);
          }
          for (let i = 0; i < roomElements.length; i++) {
            if (i > 0) roomSpan.appendChild(document.createTextNode(', '));
            roomSpan.appendChild(roomElements[i]);
          }
          if (roomElements.length > 0) metaLeft.appendChild(roomSpan);
        }
        if (evt.kind) {
          metaLeft.appendChild(el('span', {}, evt.kind));
        }

        // Right side: cost, capacity, etc.
        if (evt.cost && evt.isPremium) {
          metaRight.appendChild(el('span', { className: printPrefix + 'meta-cost' }, evt.cost));
        }
        const cardCap = capacityText(evt);
        if (cardCap) {
          metaRight.appendChild(el('span', { className: printPrefix + 'meta-capacity' }, 'Capacity: ' + cardCap));
        }
      }

      meta.appendChild(metaLeft);
      if (metaRight.children.length > 0) {
        meta.appendChild(metaRight);
      }
      body.appendChild(meta);

      // Badges
      const badges = el('div', { className: printPrefix + 'event-badges' });
      if (isShared) badges.appendChild(el('span', { className: printPrefix + 'badge ' + printPrefix + 'badge-shared', 'aria-label': 'In shared schedule' }, 'Shared'));
      if (evt.isWorkshop) badges.appendChild(el('span', { className: printPrefix + 'badge ' + printPrefix + 'badge-workshop' }, 'Workshop'));
      // Multi-part continuation parts show the price as a faded, italic pill;
      // the series note (below) explains one purchase covers every part.
      if (evt.cost && evt.isPremium) {
        if (evt.totalParts && !evt.isSeriesLead) {
          badges.appendChild(el('span', { className: printPrefix + 'badge ' + printPrefix + 'badge-paid ' + printPrefix + 'badge-series' }, evt.cost));
        } else {
          badges.appendChild(el('span', { className: printPrefix + 'badge ' + printPrefix + 'badge-paid' }, evt.cost));
        }
      }
      const cardCap = capacityText(evt);
      if (cardCap) badges.appendChild(el('span', {
        className: printPrefix + 'badge ' + printPrefix + 'badge-capacity',
        'aria-label': 'Capacity ' + cardCap,
      }, 'Capacity: ' + cardCap));
      if (evt.isFull) badges.appendChild(el('span', { className: printPrefix + 'badge ' + printPrefix + 'badge-full' }, 'Full'));
      if (evt.isKids) badges.appendChild(el('span', { className: printPrefix + 'badge ' + printPrefix + 'badge-kids' }, 'Kids'));
      if (badges.children.length > 0) body.appendChild(badges);

      // Multi-part series cost note
      const cardSeriesNote = seriesCostNote(evt);
      if (cardSeriesNote) body.appendChild(el('div', { className: printPrefix + 'event-series-note' }, cardSeriesNote));

      // Presenters/Credits (only in screen layout, not in print layout where they're in meta)
      if (!isPrintLayout && evt.credits && evt.credits.length > 0) {
        body.appendChild(el('div', { className: printPrefix + 'event-presenters' }, evt.credits.join(', ')));
      }

      // Description (hidden, shown on expand)
      if (evt.description) {
        body.appendChild(el('div', { className: printPrefix + 'event-desc' }, evt.description));
      }

      // Click to expand / open modal
      body.setAttribute('role', 'button');
      body.setAttribute('tabindex', '0');
      body.addEventListener('click', () => {
        this.state.modalEvent = evt;
        this._showModal(evt);
      });
      body.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          this.state.modalEvent = evt;
          this._showModal(evt);
        }
      });

      card.appendChild(body);

      // Right side: people indicator + star, grouped so they align together.
      const right = el('div', { className: printPrefix + 'event-right' });

      // People indicator: shown when event is in any schedule other than (or in
      // addition to) the active one. Count shows only the "other" schedules so
      // it's additive to the star, not double-counting the active schedule.
      const scheduleNames = this.state.schedulesForEvent(evt.id);
      const otherCount = isStarred ? scheduleNames.length - 1 : scheduleNames.length;
      const showPeople = otherCount > 0;
      if (showPeople) {
        const peopleEl = el('div', {
          className: printPrefix + 'event-people',
          'aria-label': `Also in ${otherCount} other schedule${otherCount === 1 ? '' : 's'}: ${scheduleNames.join(', ')}`,
          title: `Schedules: ${scheduleNames.join(', ')}`,
        });
        peopleEl.innerHTML = ICONS.people + `<span class="${printPrefix}people-count" aria-hidden="true">${otherCount}</span>`;
        right.appendChild(peopleEl);
      }

      const starBtn = el('button', {
        type: 'button',
        className: printPrefix + 'event-star' + (isStarred ? ' starred' : ''),
        innerHTML: ICONS.star,
        title: isStarred ? 'Remove from ' + this.state.activeScheduleName : 'Add to ' + this.state.activeScheduleName,
        'aria-label': isStarred ? 'Remove from ' + this.state.activeScheduleName : 'Add to ' + this.state.activeScheduleName,
        onClick: (e) => {
          e.stopPropagation();
          this.state.toggleStar(evt.id);
          this.render();
        },
      });
      right.appendChild(starBtn);

      card.appendChild(right);

      return card;
    }

    // ── Grid View ──

    // printMode renders a print-friendly variant of the same CSS Grid: time
    // rows become equal `1fr` tracks (even height, filling the column), the
    // container is tagged with print prefix, and the interactive on-screen
    // footer is omitted (print emits its own footer band).
    // printMode renders a print-friendly variant of the same CSS Grid (no
    // interactive chrome, compact spacing). fillPage uses even `1fr` rows that
    // stretch to fill a sized container (the advanced print's page-fill layout);
    // without it, rows take a natural minimum height so a sparse day isn't
    // stretched (the simple print).
    _buildGridView(events, printMode = false, fillPage = false, printPrefix = 'cosam-') {
      const container = el('div', { className: printPrefix + 'grid-view' + (printMode ? ' ' + printPrefix + 'print-grid' : '') });

      // Separate break events from regular events
      const regularEvents = events.filter(e => !this.state._isBreakEvent(e));
      const breakEvents = events.filter(e => this.state._isBreakEvent(e));

      // Get visible rooms from regular events only (BREAK/pseudo rooms excluded)
      const roomIds = [...new Set(regularEvents.flatMap(e => e.roomIds || []).filter(id => id !== null && id !== undefined))];
      const roomOrder = this.state.data.rooms
        .filter(r => roomIds.includes(r.uid || r.id))
        .sort((a, b) => a.sortKey - b.sortKey)
        .map(r => r.uid);

      // Add any rooms not in the rooms list
      for (const rid of roomIds) {
        if (!roomOrder.includes(rid)) roomOrder.push(rid);
      }

      if (roomOrder.length === 0) {
        container.appendChild(el('div', { className: printPrefix + 'empty' }, 'No rooms to display.'));
        return container;
      }

      // Build slot epochs: all event start/end epoch boundaries, plus
      // intermediate unit-interval epochs in fillPage mode so the grid has an
      // even time axis. Epoch-based keys need no Date/ISO parsing and correctly
      // handle the schedule timezone via the precomputed startEpoch values.
      const tz = this.state.data && this.state.data.meta && this.state.data.meta.timezone || '';
      const dayTimeline = (this.state.data && this.state.data.dayTimeline) || [];
      const tzOffsetMinutes = this.state.data?.tzOffsetMinutes ?? null;
      const tzDstTransitionEpoch = this.state.data?.tzDstTransitionEpoch ?? null;
      const tzDstOffsetMinutes = this.state.data?.tzDstOffsetMinutes ?? null;

      const eventSlotEpochs = new Set();
      for (const e of events) {
        if (typeof e.startEpoch === 'number') eventSlotEpochs.add(epochToSlotEpoch(e.startEpoch));
        if (typeof e.endEpoch === 'number') eventSlotEpochs.add(epochToSlotEpoch(e.endEpoch));
      }
      const allSlotEpochs = fillPage
        ? evenSlotEpochs(events, regularEvents, tzOffsetMinutes, tzDstTransitionEpoch, tzDstOffsetMinutes)
        : [...eventSlotEpochs].sort((a, b) => a - b);

      // CSS grid-line names: epoch-minutes prefixed with 't' (unique across all
      // dates, no weekday-collision, no Date object needed).
      const timeSlotMap = {};
      const timeSlots = allSlotEpochs.map(slotEpoch => {
        const name = slotEpochToName(slotEpoch);
        timeSlotMap[slotEpoch] = name;
        return name;
      });

      // Create grid template styles
      const timeCol = printMode ? 'minmax(60px, 80px)' : 'minmax(80px, 120px)';
      const gridColumns = `[time] ${timeCol} ` + roomOrder.map(roomId => `[room-${roomId}] minmax(0, 1fr)`).join(' ');

      // A sticky header row repeats at each new day: its time-column corner
      // shows the day (weekday over date) and the room columns repeat. Because
      // every day's header row is identical in shape, they swap cleanly as you
      // scroll across a day boundary — no awkward partial overlap of the rooms.
      // The global-max time key is an event END that nothing starts on (e.g. the
      // overnight break's next-day end), so a track for it would render as an
      // empty trailing slot — and at a day boundary, a spurious day header too.
      // Don't emit a track or header for such trailing end-only keys; instead
      // fold their grid-line names onto the [footer] line so event/break spans
      // ending there still resolve.
      const startSlotEpochs = new Set(
        events.filter(e => typeof e.startEpoch === 'number').map(e => epochToSlotEpoch(e.startEpoch))
      );
      let lastStartIdx = -1;
      for (let i = 0; i < allSlotEpochs.length; i++) {
        if (startSlotEpochs.has(allSlotEpochs[i])) lastStartIdx = i;
      }
      // In fillPage mode every even unit is a real row, so only the final
      // closing line (the last event end) folds onto the footer; otherwise fold
      // every end-only key after the last event start.
      const lastTrackIdx = fillPage ? timeSlots.length - 2 : lastStartIdx;

      const dayHeaders = [];
      const rowParts = [];
      const trailingLineNames = [];
      let hdrLastDayKey = null;
      for (let i = 0; i < timeSlots.length; i++) {
        if (i > lastTrackIdx) {
          trailingLineNames.push(timeSlots[i]);
          continue;
        }
        // Day key from dayTimeline (epoch-range lookup, respects borrow convention).
        const slotEpoch = allSlotEpochs[i];
        const dayEntry = dayTimeline.find(d =>
          slotEpoch >= d.startEpoch && slotEpoch <= (d.borrowedEndEpoch || d.endEpoch)
        );
        const dayKey = dayEntry ? dayEntry.date : null;
        if (dayKey && dayKey !== hdrLastDayKey) {
          // Only add day header if there's a non-break event starting at this slot
          const hasNonBreakStart = events.some(e =>
            !this.state._isBreakEvent(e) &&
            typeof e.startEpoch === 'number' &&
            epochToSlotEpoch(e.startEpoch) === slotEpoch
          );
          if (hasNonBreakStart) {
            hdrLastDayKey = dayKey;
            const rowName = 'dayhdr-' + dayKey.replace(/[^0-9a-z]/gi, '');
            dayHeaders.push({ rowName, dayEntry });
            rowParts.push(`[${rowName}] auto`);
          }
        }
        // fillPage: even `minmax(0, 1fr)` slots that divide the page evenly. The
        // `0` minimum (vs plain `1fr` = `minmax(auto, 1fr)`) lets a row shrink
        // below its content on an over-full day, so every hour stays on the page
        // and the panel content clips inside its cell instead of the hours.
        // Simple print: a natural minimum so a sparse day isn't stretched tall.
        // Screen: a taller scrollable minimum so dense slots stay legible.
        const slotRow = fillPage ? 'minmax(0, 1fr)' : (printMode ? 'minmax(32px, auto)' : 'minmax(60px, auto)');
        rowParts.push(`[${timeSlots[i]}] ${slotRow}`);
      }
      const footerLine = `[${trailingLineNames.concat(['footer']).join(' ')}] auto`;
      const gridRows = (dayHeaders.length > 0 ? rowParts.join(' ') : '[header] auto') + ` ${footerLine}`;

      // Build CSS grid
      const grid = el('div', {
        className: printPrefix + 'grid',
        role: 'table',
        'aria-label': 'Schedule grid view',
        style: {
          gridTemplateColumns: gridColumns,
          gridTemplateRows: gridRows
        }
      });

      grid.style.gridTemplateColumns = gridColumns;
      grid.style.gridTemplateRows = gridRows;

      // Add a sticky header row per day (corner = day/date, room columns repeat).
      const headerSpecs = dayHeaders.length > 0 ? dayHeaders : [{ rowName: 'header', dayEntry: null }];
      for (const dh of headerSpecs) {
        grid.appendChild(this._buildGridHeader(roomOrder, dh.dayEntry, dh.rowName, printPrefix));
      }

      // Add time slots and events
      for (let i = 0; i < timeSlots.length; i++) {
        const timeSlot = timeSlots[i];
        const slotEpoch = allSlotEpochs[i];
        const slotEvents = events.filter(e =>
          typeof e.startEpoch === 'number' && epochToSlotEpoch(e.startEpoch) === slotEpoch
        );
        const slotRegular = slotEvents.filter(e => !this.state._isBreakEvent(e));
        const slotBreaks = slotEvents.filter(e => this.state._isBreakEvent(e));

        // Half-hour slot: local minute-of-hour is non-zero. Uses precomputed TZ
        // offsets so non-integer-hour zones (e.g. IST UTC+5:30) are handled correctly.
        const isHalfHour = localMinuteOfHour(slotEpoch, tzOffsetMinutes, tzDstTransitionEpoch, tzDstOffsetMinutes) !== 0;

        // Build time header with split time format for aligned display
        const timeHeader = el('div', {
          className: printPrefix + 'grid-time-header' + (isHalfHour ? ' ' + printPrefix + 'grid-time-half' : ''),
          style: {
            gridColumn: 'time',
            gridRow: timeSlot,
          }
        });

        // Use split time format for accessibility and aligned display.
        // startTime is the schedule-timezone wall-clock ISO string (set by
        // _normalizeDataModel from epoch). Fall back to epochToLocalIso for
        // filler slots (evenSlotEpochs rows with no event).
        const timeSource = slotEvents.length > 0
          ? slotEvents[0].startTime
          : epochToLocalIso(slotEpoch, tz);
        const timeSplit = formatTimeSplit(timeSource);

        if (timeSplit.isSpecial) {
          // Midnight/Noon - centered
          timeHeader.appendChild(el('div', {
            className: (isHalfHour ? printPrefix + 'grid-time-minor' : printPrefix + 'grid-time-major') + ' ' + printPrefix + 'grid-time-split ' + printPrefix + 'grid-time-special',
            'aria-label': timeSplit.label,
          }, timeSplit.hour));
        } else {
          // Regular time - split display
          const timeContainer = el('div', {
            className: (isHalfHour ? printPrefix + 'grid-time-minor' : printPrefix + 'grid-time-major') + ' ' + printPrefix + 'grid-time-split',
            'aria-label': timeSplit.label,
          });
          // Screen reader only full time
          timeContainer.appendChild(el('span', { className: printPrefix + 'sr-only' }, timeSplit.full));
          // Visible hour (right-aligned)
          timeContainer.appendChild(el('span', {
            className: printPrefix + 'grid-time-hour',
            'aria-hidden': 'true',
          }, timeSplit.hour));
          // Visible suffix (left-aligned: AM/PM or :MM)
          timeContainer.appendChild(el('span', {
            className: printPrefix + 'grid-time-suffix',
            'aria-hidden': 'true',
          }, timeSplit.suffix));
          timeHeader.appendChild(timeContainer);
        }

        grid.appendChild(timeHeader);

        // Add events for each room
        if (slotBreaks.length > 0) {
          // Determine which rooms have real events at this time
          const occupiedRoomIds = new Set(slotRegular.flatMap(e => e.roomIds || []).filter(id => id !== null && id !== undefined));

          // Build cells: span across unoccupied rooms, show real events in occupied rooms
          let i = 0;
          while (i < roomOrder.length) {
            const roomId = roomOrder[i];
            if (occupiedRoomIds.has(roomId)) {
              // Room has a real event — render it normally
              const roomEvents = slotRegular.filter(e => e.roomIds && e.roomIds.includes(roomId));
              for (const evt of roomEvents) {
                const eventEl = this._buildGridEvent(evt, printPrefix);
                eventEl.style.gridColumn = `room-${roomId}`;

                // Row span: O(1) lookup via timeSlotMap[endEpoch].
                const endSlotName = typeof evt.endEpoch === 'number'
                  ? timeSlotMap[epochToSlotEpoch(evt.endEpoch)]
                  : null;
                eventEl.style.gridRow = endSlotName && endSlotName !== timeSlot
                  ? `${timeSlot} / ${endSlotName}`
                  : timeSlot;

                grid.appendChild(eventEl);
              }
              i++;
            } else {
              // Start a span across consecutive unoccupied rooms
              let spanEnd = i + 1;
              while (spanEnd < roomOrder.length && !occupiedRoomIds.has(roomOrder[spanEnd])) {
                spanEnd++;
              }
              const startRoom = roomOrder[i];
              const endRoom = roomOrder[spanEnd - 1];
              for (const breakEvt of slotBreaks) {
                const breakEl = this._buildGridBreak(breakEvt, printPrefix);

                // Calculate grid column span
                if (spanEnd === i + 1) {
                  // Single room
                  breakEl.style.gridColumn = `room-${startRoom}`;
                } else {
                  // Multiple rooms - span to the room after the last unoccupied room
                  const nextRoomIndex = spanEnd < roomOrder.length ? spanEnd : roomOrder.length;
                  const endRoomName = nextRoomIndex < roomOrder.length ? `room-${roomOrder[nextRoomIndex]}` : -1;
                  breakEl.style.gridColumn = `room-${startRoom} / ${endRoomName}`;
                }

                // Row span: O(1) lookup via timeSlotMap[endEpoch].
                const breakEndSlotName = typeof breakEvt.endEpoch === 'number'
                  ? timeSlotMap[epochToSlotEpoch(breakEvt.endEpoch)]
                  : null;
                breakEl.style.gridRow = breakEndSlotName && breakEndSlotName !== timeSlot
                  ? `${timeSlot} / ${breakEndSlotName}`
                  : timeSlot;

                grid.appendChild(breakEl);
              }
              i = spanEnd;
            }
          }
        } else {
          // Normal row — no breaks
          for (const roomId of roomOrder) {
            const roomEvents = slotRegular.filter(e => e.roomIds && e.roomIds.includes(roomId));
            for (const evt of roomEvents) {
              const eventEl = this._buildGridEvent(evt, printPrefix);
              eventEl.style.gridColumn = `room-${roomId}`;

              // Row span: O(1) lookup via timeSlotMap[endEpoch].
              const endSlotName = typeof evt.endEpoch === 'number'
                ? timeSlotMap[epochToSlotEpoch(evt.endEpoch)]
                : null;
              eventEl.style.gridRow = endSlotName && endSlotName !== timeSlot
                ? `${timeSlot} / ${endSlotName}`
                : timeSlot;

              grid.appendChild(eventEl);
            }
          }
        }
      }

      // Add subtle background gridlines
      // Horizontal row lines at each visible time slot. Use lastTrackIdx (not
      // lastStartIdx) so fillPage's even rows after the last event start still
      // get lines; trailing end-only keys (folded onto [footer]) are skipped.
      for (let i = 0; i <= lastTrackIdx; i++) {
        const rowLine = el('div', {
          className: printPrefix + 'grid-row-line',
          style: {
            gridColumn: `room-${roomOrder[0]} / -1`,
            gridRow: timeSlots[i]
          }
        });
        grid.appendChild(rowLine);
      }
      // Vertical column lines between rooms
      for (let r = 0; r < roomOrder.length - 1; r++) {
        const colLine = el('div', {
          className: printPrefix + 'grid-col-line',
          style: {
            gridColumn: `room-${roomOrder[r]}`,
            gridRow: `${timeSlots[0]} / footer`
          }
        });
        grid.appendChild(colLine);
      }

      // Add the footer row (generated/modified stamp). The advanced page-fill
      // print emits its own footer band, so skip it there; the simple print and
      // the on-screen view keep this one.
      const footer = el('div', { className: printPrefix + 'grid-footer' });
      footer.style.gridRow = 'footer';
      footer.style.gridColumn = '1 / -1'; // Span all columns

      // Add footer content
      const footerContent = el('div', { className: printPrefix + 'grid-footer-content' });
      let footerText = 'End of Schedule';

      const tsText = this._scheduleTimestampText();
      if (tsText) footerText = tsText;

      footerContent.textContent = footerText;
      footer.appendChild(footerContent);

      grid.appendChild(footer);

      container.appendChild(grid);
      return container;
    }

    _buildGridHeader(roomOrder, dayEntry, rowName, printPrefix = 'cosam-') {
      const header = el('div', { className: printPrefix + 'grid-header' });
      const cellRow = rowName || 'header';

      // Time-column corner: shows this day's weekday over its date (like the
      // room name / hotel room split) so the day rides along with the sticky
      // header row. Falls back to a plain "Time" label when no day is given.
      const timeHeader = el('div', {
        className: printPrefix + 'grid-header-cell ' + printPrefix + 'grid-time-header',
        style: { gridColumn: 'time', gridRow: cellRow }
      });
      if (dayEntry) {
        // dayEntry.date is 'YYYY-MM-DD' in schedule timezone. Parse via
        // local-midnight constructor to avoid UTC-shift on naive date strings.
        const parts = dayEntry.date.split('-').map(Number);
        const d = new Date(parts[0], parts[1] - 1, parts[2]);
        const dayCell = el('div', {
          className: printPrefix + 'grid-time-header-day',
          role: 'heading',
          'aria-level': '2',
        });
        dayCell.appendChild(el('span', { className: printPrefix + 'grid-time-header-weekday' },
          d.toLocaleDateString('en-US', { weekday: 'long' })));
        dayCell.appendChild(el('span', { className: printPrefix + 'grid-time-header-date' },
          d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })));
        timeHeader.appendChild(dayCell);
      } else {
        timeHeader.appendChild(el('div', {
          className: printPrefix + 'grid-time-split ' + printPrefix + 'grid-time-special',
          'aria-label': 'Time column',
        }, 'Time'));
      }
      header.appendChild(timeHeader);

      // Room headers
      for (const roomId of roomOrder) {
        const room = this.state.data.rooms.find(r => r.uid === roomId);
        const roomName = room ? (room.longName || room.shortName) : 'Unknown';
        let roomDisplay = roomName;
        if (room && room.hotelRoom && room.hotelRoom !== roomName) {
          roomDisplay = `${roomName}<br><small style="opacity: 0.8">(${room.hotelRoom})</small>`;
        }
        const roomHeader = el('div', {
          className: printPrefix + 'grid-header-cell',
          style: { gridColumn: `room-${roomId}`, gridRow: cellRow }
        });
        roomHeader.innerHTML = roomDisplay;
        header.appendChild(roomHeader);
      }

      return header;
    }

    _buildGridSleepBreak(columnCount, printPrefix = 'cosam-') {
      const sleepBreak = el('div', { className: printPrefix + 'sleep-break' });
      sleepBreak.appendChild(el('div', { className: printPrefix + 'sleep-break-icon' }, '🌙'));
      sleepBreak.appendChild(el('div', { className: printPrefix + 'sleep-break-text' }, 'Overnight Break'));
      return sleepBreak;
    }

    _buildGridBreak(evt, printPrefix = 'cosam-') {
      const isOvernight = evt.panelType === '%NB';
      const div = el('div', {
        className: printPrefix + 'grid-break' + (isOvernight ? ' ' + printPrefix + 'implicit-overnight-break' : ''),
        role: 'button',
        tabindex: '0',
        onClick: () => { this.state.modalEvent = evt; this._showModal(evt); },
      });
      div.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          this.state.modalEvent = evt;
          this._showModal(evt);
        }
      });

      // Add moon for overnight breaks
      if (isOvernight) {
        const nameWrapper = el('div', { className: printPrefix + 'grid-break-name' });
        nameWrapper.appendChild(el('span', { className: printPrefix + 'implicit-overnight-moon' }, '🌙'));
        nameWrapper.appendChild(document.createTextNode(' ' + evt.name));
        div.appendChild(nameWrapper);
      } else {
        div.appendChild(el('div', { className: printPrefix + 'grid-break-name' }, evt.name));
      }
      if (evt.duration) {
        div.appendChild(el('div', { className: printPrefix + 'grid-event-time' }, formatDuration(evt.duration)));
      }
      return div;
    }

    _buildGridEvent(evt, printPrefix = 'cosam-') {
      const isStarred = this.state.starred.has(evt.id);
      const isShared = this.state.sharedStarred.has(evt.id);
      const typeClass = this._panelTypeClass(evt.panelType, printPrefix);
      const div = el('div', {
        className: printPrefix + 'grid-event' + (isStarred ? ' starred' : '') + (isShared ? ' ' + printPrefix + 'shared' : '') + (typeClass ? (' ' + typeClass) : ''),
        role: 'button',
        tabindex: '0',
        onClick: () => { this.state.modalEvent = evt; this._showModal(evt); },
      });
      div.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          this.state.modalEvent = evt;
          this._showModal(evt);
        }
      });

      // Actions float — must be inserted BEFORE the name so text wraps around it.
      // Star + optional people indicator are stacked in a float:right container.
      const actionsEl = el('div', { className: printPrefix + 'grid-event-actions' });

      const starEl = el('span', {
        role: 'button',
        tabindex: '0',
        className: printPrefix + 'grid-event-star' + (isStarred ? ' starred' : ''),
        innerHTML: ICONS.star,
        onClick: (e) => {
          e.stopPropagation();
          this.state.toggleStar(evt.id);
          this.render();
        },
      });
      starEl.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          e.stopPropagation();
          this.state.toggleStar(evt.id);
          this.render();
        }
      });
      actionsEl.appendChild(starEl);

      // People indicator — stacked below the star inside the same float.
      // Count shows only "other" schedules (not the active one, already shown by the star).
      const scheduleNames = this.state.schedulesForEvent(evt.id);
      const otherCount = isStarred ? scheduleNames.length - 1 : scheduleNames.length;
      if (otherCount > 0) {
        const peopleEl = el('span', {
          className: printPrefix + 'grid-event-people',
          'aria-label': `Also in ${otherCount} other schedule${otherCount === 1 ? '' : 's'}: ${scheduleNames.join(', ')}`,
          title: `Schedules: ${scheduleNames.join(', ')}`,
        });
        peopleEl.innerHTML = ICONS.people + `<span class="${printPrefix}people-count" aria-hidden="true">${otherCount}</span>`;
        actionsEl.appendChild(peopleEl);
      }

      div.appendChild(actionsEl);
      div.appendChild(el('div', { className: printPrefix + 'grid-event-name' }, evt.name));

      // Add room information for mobile view
      if (evt.roomIds && evt.roomIds.length > 0) {
        const roomNames = evt.roomIds.map(roomId => {
          const room = this.state.data.rooms.find(r => r.uid === roomId);
          if (!room) return null;
          const rn = room.longName || room.shortName;
          const roomDisplay = (room.hotelRoom && room.hotelRoom !== rn)
            ? `${rn} (${room.hotelRoom})` : rn;
          return roomDisplay;
        }).filter(Boolean);

        if (roomNames.length > 0) {
          div.appendChild(el('div', { className: printPrefix + 'grid-event-room' }, roomNames.join(', ')));
        }
      }

      if (evt.credits && evt.credits.length > 0) {
        div.appendChild(el('div', { className: printPrefix + 'grid-event-credits' }, evt.credits.join(', ')));
      }

      if (evt.duration) {
        div.appendChild(el('div', { className: printPrefix + 'grid-event-time' }, formatDuration(evt.duration)));
      }

      // Premium cost — shown as a compact pill, mirroring the list/detail badge.
      // For a multi-part series the shared price is shown only on the lead part;
      // continuation parts show "Part N of M" so the price never reads as a
      // separate per-part charge.
      if (evt.cost && evt.isPremium) {
        if (evt.totalParts && !evt.isSeriesLead) {
          // Continuation part — faded, italic price pill; the series note (in
          // the detail view) explains one purchase covers every part.
          div.appendChild(el('span', { className: printPrefix + 'grid-event-cost ' + printPrefix + 'grid-event-cost-series' }, evt.cost));
        } else {
          const costText = evt.totalParts ? evt.cost + ' (full series)' : evt.cost;
          div.appendChild(el('span', { className: printPrefix + 'grid-event-cost' }, costText));
        }
      }

      return div;
    }

    // ── Modal ──

    _buildModal() {
      const overlay = el('div', { className: 'cosam-modal-overlay' });
      overlay.addEventListener('click', (e) => {
        if (e.target === overlay) { overlay.classList.remove('open'); }
      });
      this._modalOverlay = overlay;

      const modal = el('div', { className: 'cosam-modal' });
      this._modalContent = modal;
      overlay.appendChild(modal);

      return overlay;
    }

    // ── Schedule Management Modals ──

    _modalClose() {
      this._modalOverlay.classList.remove('open');
    }

    _showNewScheduleModal() {
      const modal = this._modalContent;
      modal.innerHTML = '';
      modal.appendChild(el('button', { type: 'button', className: 'cosam-modal-close', innerHTML: ICONS.x, 'aria-label': 'Close', onClick: () => this._modalClose() }));
      modal.appendChild(el('h2', {}, 'New Schedule'));
      const nameInput = el('input', { type: 'text', className: 'cosam-share-url-input', placeholder: 'Schedule name...', 'aria-label': 'Schedule name' });
      modal.appendChild(nameInput);
      const errDiv = el('div', { className: 'cosam-import-status' });
      modal.appendChild(errDiv);
      const createBtn = el('button', {
        type: 'button', className: 'cosam-btn',
        onClick: () => {
          const created = this.state.createSchedule(nameInput.value);
          if (!created) { errDiv.textContent = 'That name is already taken or invalid.'; return; }
          this.state.switchSchedule(created);
          this._modalClose();
          this.render();
        },
      }, 'Create');
      nameInput.addEventListener('keydown', (e) => { if (e.key === 'Enter') createBtn.click(); });
      modal.appendChild(el('div', { className: 'cosam-modal-actions' }, createBtn));
      this._modalOverlay.classList.add('open');
      nameInput.focus();
    }

    _showRenameScheduleModal() {
      const modal = this._modalContent;
      modal.innerHTML = '';
      modal.appendChild(el('button', { type: 'button', className: 'cosam-modal-close', innerHTML: ICONS.x, 'aria-label': 'Close', onClick: () => this._modalClose() }));
      modal.appendChild(el('h2', {}, 'Rename Schedule'));
      const nameInput = el('input', { type: 'text', className: 'cosam-share-url-input', value: this.state.activeScheduleName, 'aria-label': 'New schedule name' });
      modal.appendChild(nameInput);
      const errDiv = el('div', { className: 'cosam-import-status' });
      modal.appendChild(errDiv);
      const saveBtn = el('button', {
        type: 'button', className: 'cosam-btn',
        onClick: () => {
          if (!this.state.renameSchedule(this.state.activeScheduleName, nameInput.value)) {
            errDiv.textContent = 'That name is already taken or invalid.'; return;
          }
          this._modalClose();
          this.render();
        },
      }, 'Save');
      nameInput.addEventListener('keydown', (e) => { if (e.key === 'Enter') saveBtn.click(); });
      modal.appendChild(el('div', { className: 'cosam-modal-actions' }, saveBtn));
      this._modalOverlay.classList.add('open');
      nameInput.select();
    }

    _showDeleteScheduleModal() {
      const name = this.state.activeScheduleName;
      const modal = this._modalContent;
      modal.innerHTML = '';
      modal.appendChild(el('button', { type: 'button', className: 'cosam-modal-close', innerHTML: ICONS.x, 'aria-label': 'Close', onClick: () => this._modalClose() }));
      modal.appendChild(el('h2', {}, 'Delete Schedule'));
      modal.appendChild(el('p', { className: 'cosam-modal-desc' }, `Delete "${name}"? This cannot be undone.`));
      const deleteBtn = el('button', {
        type: 'button', className: 'cosam-btn cosam-btn-danger',
        onClick: () => {
          this.state.deleteSchedule(name);
          this._modalClose();
          this.render();
        },
      }, 'Delete');
      modal.appendChild(el('div', { className: 'cosam-modal-actions' }, deleteBtn));
      this._modalOverlay.classList.add('open');
    }

    _showMergeScheduleModal() {
      const modal = this._modalContent;
      modal.innerHTML = '';
      modal.appendChild(el('button', { type: 'button', className: 'cosam-modal-close', innerHTML: ICONS.x, 'aria-label': 'Close', onClick: () => this._modalClose() }));
      modal.appendChild(el('h2', {}, 'Merge Schedules'));
      modal.appendChild(el('p', { className: 'cosam-modal-desc' }, 'Copy all events from one schedule into another.'));

      const fromLabel = el('label', { className: 'cosam-share-option' });
      fromLabel.appendChild(document.createTextNode('From: '));
      const fromSelect = el('select', { className: 'cosam-select', 'aria-label': 'Source schedule' });
      for (const name of Object.keys(this.state.schedules)) {
        if (name !== this.state.activeScheduleName) {
          fromSelect.appendChild(el('option', { value: name }, name));
        }
      }
      fromLabel.appendChild(fromSelect);
      modal.appendChild(fromLabel);

      modal.appendChild(el('p', {}, `Into: "${this.state.activeScheduleName}"`));

      const mergeBtn = el('button', {
        type: 'button', className: 'cosam-btn',
        onClick: () => {
          const src = fromSelect.value;
          if (!src) return;
          this.state.mergeIntoSchedule(this.state.schedules[src], this.state.activeScheduleName);
          this._modalClose();
          this.render();
        },
      }, 'Merge');
      modal.appendChild(el('div', { className: 'cosam-modal-actions' }, mergeBtn));
      this._modalOverlay.classList.add('open');
    }

    _showImportFromUrlModal() {
      const modal = this._modalContent;
      modal.innerHTML = '';
      modal.appendChild(el('button', { type: 'button', className: 'cosam-modal-close', innerHTML: ICONS.x, 'aria-label': 'Close', onClick: () => this._modalClose() }));
      modal.appendChild(el('h2', {}, 'Import from URL'));
      modal.appendChild(el('p', { className: 'cosam-modal-desc' }, 'Paste a shared schedule URL to import it.'));

      const urlInput = el('input', { type: 'text', className: 'cosam-share-url-input', placeholder: 'Paste URL...', 'aria-label': 'Shared schedule URL' });
      modal.appendChild(urlInput);

      const resultDiv = el('div', { className: 'cosam-import-status' });
      modal.appendChild(resultDiv);

      const parseBtn = el('button', {
        type: 'button', className: 'cosam-btn',
        onClick: () => {
          const url = urlInput.value.trim();
          const hashIdx = url.indexOf('#');
          if (hashIdx === -1) { resultDiv.textContent = 'No schedule data found in URL.'; return; }
          const params = new URLSearchParams(url.substring(hashIdx + 1));
          const ids = params.has('starred') ? decodeURIComponent(params.get('starred')).split(',').filter(Boolean) : [];
          const schedName = params.has('scheduleName') ? decodeURIComponent(params.get('scheduleName')) : 'Imported Schedule';
          if (ids.length === 0) { resultDiv.textContent = 'No starred events found in URL.'; return; }

          resultDiv.innerHTML = '';
          resultDiv.appendChild(el('p', {}, `Found "${schedName}" with ${ids.length} event${ids.length === 1 ? '' : 's'}.`));

          const actionsDiv = el('div', { className: 'cosam-import-actions' });

          const asNewBtn = el('button', {
            type: 'button', className: 'cosam-btn',
            onClick: () => {
              const n = this.state.importAsNewSchedule(schedName, ids);
              this.state.switchSchedule(n);
              this._modalClose();
              this.render();
            },
          }, 'Import as new schedule');
          actionsDiv.appendChild(asNewBtn);

          const mergeSelect = el('select', { className: 'cosam-select', 'aria-label': 'Schedule to merge into' });
          for (const n of Object.keys(this.state.schedules)) mergeSelect.appendChild(el('option', { value: n }, n));
          mergeSelect.value = this.state.activeScheduleName;

          const mergeBtn = el('button', {
            type: 'button', className: 'cosam-btn',
            onClick: () => {
              this.state.mergeIntoSchedule(new Set(ids), mergeSelect.value);
              this._modalClose();
              this.render();
            },
          }, 'Merge into:');
          const mergeRow = el('div', { className: 'cosam-import-row' }, mergeBtn, mergeSelect);
          actionsDiv.appendChild(mergeRow);

          const replaceBtn = el('button', {
            type: 'button', className: 'cosam-btn',
            onClick: () => {
              const target = mergeSelect.value;
              if (!confirm(`Replace "${target}" with "${schedName}"? This cannot be undone.`)) return;
              this.state.replaceSchedule(ids, target);
              this._modalClose();
              this.render();
            },
          }, 'Replace:');
          const replaceRow = el('div', { className: 'cosam-import-row' });
          const replaceSelect = el('select', { className: 'cosam-select', 'aria-label': 'Schedule to replace' });
          for (const n of Object.keys(this.state.schedules)) replaceSelect.appendChild(el('option', { value: n }, n));
          replaceSelect.value = this.state.activeScheduleName;
          replaceRow.append(replaceBtn, replaceSelect);
          actionsDiv.appendChild(replaceRow);

          resultDiv.appendChild(actionsDiv);
        },
      }, 'Parse URL');
      urlInput.addEventListener('keydown', (e) => { if (e.key === 'Enter') parseBtn.click(); });
      modal.appendChild(el('div', { className: 'cosam-modal-actions' }, parseBtn));
      this._modalOverlay.classList.add('open');
      urlInput.focus();
    }

    _showShareModal() {
      const modal = this._modalContent;
      modal.innerHTML = '';

      // Close button
      modal.appendChild(el('button', {
        type: 'button',
        className: 'cosam-modal-close',
        innerHTML: ICONS.x,
        'aria-label': 'Close dialog',
        onClick: () => this._modalOverlay.classList.remove('open'),
      }));

      modal.appendChild(el('h2', {}, 'Share Schedule'));

      // ── Upper section: options (left) + QR code (right) ──
      const upper = el('div', { className: 'cosam-share-upper' });

      // Options column
      const optionsDiv = el('div', { className: 'cosam-share-options' });

      // "Include schedule" checkbox (default: on)
      const inclSchedLabel = el('label', { className: 'cosam-share-option' });
      const inclSchedCb = el('input', { type: 'checkbox' });
      inclSchedCb.checked = true;
      inclSchedLabel.append(inclSchedCb, ' Include schedule');
      optionsDiv.appendChild(inclSchedLabel);

      // Schedule selector row (indented, hidden when include-schedule is off)
      const scheduleRow = el('div', { className: 'cosam-share-schedule-row' });
      scheduleRow.appendChild(document.createTextNode('Schedule: '));
      const scheduleSelect = el('select', { className: 'cosam-select', 'aria-label': 'Schedule to share' });
      for (const [name, ids] of Object.entries(this.state.schedules)) {
        const opt = el('option', { value: name }, `${name} (${ids.size} events)`);
        if (name === this.state.activeScheduleName) opt.selected = true;
        scheduleSelect.appendChild(opt);
      }
      scheduleRow.appendChild(scheduleSelect);
      optionsDiv.appendChild(scheduleRow);

      // "Include filters" checkbox (default: off)
      const inclFiltersLabel = el('label', { className: 'cosam-share-option' });
      const inclFiltersCb = el('input', { type: 'checkbox' });
      inclFiltersCb.checked = false;
      inclFiltersLabel.append(inclFiltersCb, ' Include current filters');
      optionsDiv.appendChild(inclFiltersLabel);

      upper.appendChild(optionsDiv);

      // QR code column
      const qrDiv = el('div', {
        className: 'cosam-share-qr',
        role: 'img',
        'aria-label': 'QR code for the share URL',
      });
      const qrImg = el('img', { className: 'cosam-share-qr-img', alt: 'QR code' });
      const qrPlaceholder = el('div', { className: 'cosam-share-qr-placeholder' }, 'Nothing to share');
      qrDiv.append(qrImg, qrPlaceholder);
      upper.appendChild(qrDiv);

      modal.appendChild(upper);

      // ── URL row ──
      const urlWrapper = el('div', { className: 'cosam-share-url-wrapper' });
      const urlInput = el('input', {
        type: 'text',
        className: 'cosam-share-url-input',
        readOnly: true,
        'aria-label': 'Share URL',
      });
      const copyBtn = el('button', {
        type: 'button',
        className: 'cosam-btn',
        onClick: () => {
          if (!urlInput.value) return;
          if (navigator.clipboard) {
            navigator.clipboard.writeText(urlInput.value).then(() => {
              copyBtn.textContent = 'Copied!';
              setTimeout(() => { copyBtn.textContent = 'Copy URL'; }, 1500);
            });
          } else {
            prompt('Copy this URL:', urlInput.value);
          }
        },
      });
      copyBtn.textContent = 'Copy URL';
      urlWrapper.append(urlInput, copyBtn);
      modal.appendChild(urlWrapper);

      // ── Add-to-calendar row ──
      // Downloads a multi-event .ics for the selected schedule so every starred
      // event lands in the viewer's own calendar app.
      const calRow = el('div', { className: 'cosam-share-calendar-row' });
      const calBtn = el('button', {
        type: 'button',
        className: 'cosam-btn',
        title: 'Add every event in this schedule to your calendar',
        'aria-label': 'Add schedule to calendar',
        innerHTML: ICONS.calendar + ' Add Schedule to Calendar',
        onClick: () => this._addScheduleToCalendar(this._shareScheduleSelect.value),
      });
      calRow.appendChild(calBtn);
      modal.appendChild(calRow);

      // Store refs
      this._shareUrlInput = urlInput;
      this._shareFiltersCheckbox = inclFiltersCb;
      this._shareScheduleSelect = scheduleSelect;
      this._shareIncludeScheduleCb = inclSchedCb;
      this._shareScheduleRow = scheduleRow;
      this._shareQrImg = qrImg;
      this._shareQrPlaceholder = qrPlaceholder;
      this._shareCalendarBtn = calBtn;

      // Wire up change handlers
      inclSchedCb.addEventListener('change', () => {
        scheduleRow.hidden = !inclSchedCb.checked;
        this._updateShareUrl();
      });
      scheduleSelect.addEventListener('change', () => this._updateShareUrl());
      inclFiltersCb.addEventListener('change', () => this._updateShareUrl());

      this._updateShareUrl();
      this._modalOverlay.classList.add('open');
    }

    _updateShareUrl() {
      if (!this._shareUrlInput) return;
      const includeSchedule = this._shareIncludeScheduleCb ? this._shareIncludeScheduleCb.checked : true;
      const scheduleName = this._shareScheduleSelect ? this._shareScheduleSelect.value : this.state.activeScheduleName;
      const includeFilters = this._shareFiltersCheckbox ? this._shareFiltersCheckbox.checked : false;
      const url = this.state.getShareUrl({ includeSchedule, scheduleName, includeFilters });
      this._shareUrlInput.value = url;

      // Add-to-calendar only makes sense when the selected schedule has events.
      if (this._shareCalendarBtn) {
        const ids = this.state.schedules[scheduleName];
        this._shareCalendarBtn.disabled = !ids || ids.size === 0;
      }

      // QR code
      if (!this._shareQrImg) return;
      const hasContent = url !== window.location.href.split('#')[0];
      if (hasContent) {
        this._shareQrPlaceholder.hidden = true;
        QRCode.toDataURL(url, { width: 200, margin: 2 }).then(dataUrl => {
          this._shareQrImg.src = dataUrl;
          this._shareQrImg.hidden = false;
        }).catch(() => {
          this._shareQrImg.hidden = true;
          this._shareQrPlaceholder.hidden = false;
        });
      } else {
        this._shareQrImg.hidden = true;
        this._shareQrImg.src = '';
        this._shareQrPlaceholder.hidden = false;
      }
    }

    _showModal(evt) {
      const modal = this._modalContent;
      modal.innerHTML = '';

      // Close button
      modal.appendChild(el('button', {
        type: 'button',
        className: 'cosam-modal-close',
        innerHTML: ICONS.x,
        'aria-label': 'Close dialog',
        onClick: () => this._modalOverlay.classList.remove('open'),
      }));

      // Title
      modal.appendChild(el('h2', {}, evt.name));

      // Meta
      const meta = el('div', { className: 'cosam-modal-meta' });
      if (evt.startTime) {
        // Day first, so screenshots of the detail view always carry the date.
        const ds = el('span', { className: 'cosam-meta-day' });
        ds.innerHTML = ICONS.calendar + ' ' + escapeHtml(getDayLabel(evt.startTime));
        meta.appendChild(ds);
      }
      if (evt.startTime) {
        const ts = el('span', { className: 'cosam-meta-time' });
        ts.innerHTML = ICONS.clock + ' ' + escapeHtml(formatTimeRange(evt.startTime, evt.endTime));
        meta.appendChild(ts);
      }
      if (evt.duration) {
        meta.appendChild(el('span', {}, formatDuration(evt.duration)));
      }
      // Rooms - V5 roomIds array
      if (evt.roomIds && evt.roomIds.length > 0) {
        const rs = el('span', { className: 'cosam-meta-room' });
        rs.innerHTML = ICONS.mappin;
        const roomElements = [];
        for (const roomId of evt.roomIds) {
          const room = this.state.data.rooms.find(r => r.uid === roomId);
          if (!room) continue;
          const roomName = room.longName || room.shortName;
          const textWrap = el('span', { className: 'cosam-meta-room-text' });
          textWrap.appendChild(el('span', {}, roomName));
          if (room.hotelRoom && room.hotelRoom !== roomName) {
            textWrap.appendChild(el('span', { className: 'cosam-meta-room-sub' }, `(${room.hotelRoom})`));
          }
          roomElements.push(textWrap);
        }
        for (let i = 0; i < roomElements.length; i++) {
          if (i > 0) rs.appendChild(document.createTextNode(', '));
          rs.appendChild(roomElements[i]);
        }
        if (roomElements.length > 0) meta.appendChild(rs);
      }
      if (evt.kind) {
        meta.appendChild(el('span', {}, evt.kind));
      }
      modal.appendChild(meta);

      // Badges
      const badges = el('div', { className: 'cosam-event-badges' });
      if (evt.isWorkshop) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-workshop' }, 'Workshop'));
      // Multi-part continuation parts show the price as a faded, italic pill;
      // the series note (below) explains one purchase covers every part.
      if (evt.cost && evt.isPremium) {
        if (evt.totalParts && !evt.isSeriesLead) {
          badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-paid cosam-badge-series' }, evt.cost));
        } else {
          badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-paid' }, evt.cost));
        }
      }
      const modalCap = capacityText(evt);
      if (modalCap) badges.appendChild(el('span', {
        className: 'cosam-badge cosam-badge-capacity',
        'aria-label': 'Capacity ' + modalCap,
      }, 'Capacity: ' + modalCap));
      if (evt.isFull) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-full' }, 'Full'));
      if (evt.isKids) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-kids' }, 'Kids'));
      if (badges.children.length > 0) modal.appendChild(badges);

      // Multi-part series cost note
      const modalSeriesNote = seriesCostNote(evt);
      if (modalSeriesNote) modal.appendChild(el('div', { className: 'cosam-event-series-note' }, modalSeriesNote));

      // Description
      if (evt.description) {
        modal.appendChild(el('div', { className: 'cosam-modal-desc' }, evt.description));
      }

      // Presenters/Credits
      if (evt.credits && evt.credits.length > 0) {
        modal.appendChild(el('div', { className: 'cosam-modal-presenters' }, 'Presenters: ' + evt.credits.join(', ')));
      }

      // Note
      if (evt.note) {
        modal.appendChild(el('div', { className: 'cosam-modal-note' }, evt.note));
      }

      // Prereq
      if (evt.prereq) {
        modal.appendChild(el('div', { className: 'cosam-modal-note' }, 'Prerequisite: ' + evt.prereq));
      }

      // Action buttons: tickets (when present), add-to-calendar, share panel.
      const actions = el('div', { className: 'cosam-modal-actions' });
      if (evt.ticketUrl) {
        actions.appendChild(el('a', {
          href: evt.ticketUrl, target: '_blank', rel: 'noopener',
          className: 'cosam-btn', style: { textDecoration: 'none', display: 'inline-flex' },
        }, 'Get Tickets'));
      }
      if (evt.startTime) {
        const calBtn = el('button', {
          type: 'button', className: 'cosam-btn',
          title: 'Add this event to your calendar',
          'aria-label': 'Add to calendar',
          innerHTML: ICONS.calendar + ' Add to Calendar',
          onClick: () => this._addEventToCalendar(evt),
        });
        actions.appendChild(calBtn);
      }
      const sharePanelBtn = el('button', {
        type: 'button', className: 'cosam-btn',
        title: 'Share a link to this event',
        'aria-label': 'Share this event',
        innerHTML: getShareIcon() + ' Share',
        onClick: () => this._showSharePanelModal(evt),
      });
      actions.appendChild(sharePanelBtn);
      modal.appendChild(actions);

      // Schedule membership section
      const schedSection = el('div', { className: 'cosam-modal-schedules' });
      const schedHeading = el('div', { className: 'cosam-modal-schedules-heading' });
      schedHeading.innerHTML = ICONS.people + ' Schedules';
      schedSection.appendChild(schedHeading);

      const checkList = el('div', { className: 'cosam-modal-schedule-list' });

      // One checkbox per named schedule
      for (const [name, ids] of Object.entries(this.state.schedules)) {
        const isIn = ids.has(evt.id);
        const itemLabel = el('label', { className: 'cosam-modal-schedule-item' });
        const cb = el('input', { type: 'checkbox' });
        if (isIn) cb.checked = true;
        cb.addEventListener('change', () => {
          if (cb.checked) this.state.schedules[name].add(evt.id);
          else this.state.schedules[name].delete(evt.id);
          this.state._saveState();
          // Re-render the whole view so the underlying grid/list stars reflect
          // the change immediately, then re-open the modal (render() rebuilds a
          // fresh, closed modal overlay).
          this.render();
          this._showModal(evt);
        });
        itemLabel.appendChild(cb);
        itemLabel.appendChild(document.createTextNode(' ' + name));
        checkList.appendChild(itemLabel);
      }

      // Shared URL schedule as read-only indicator
      if (this.state.sharedStarred.has(evt.id)) {
        const sharedRow = el('div', { className: 'cosam-modal-schedule-item cosam-modal-schedule-shared' });
        sharedRow.innerHTML = ICONS.people;
        sharedRow.appendChild(document.createTextNode(' ' + this.state.sharedScheduleName + ' (shared, read-only)'));
        checkList.appendChild(sharedRow);
      }

      schedSection.appendChild(checkList);
      modal.appendChild(schedSection);

      this._modalOverlay.classList.add('open');
    }

    // Display room names (with hotel room in parens) for an event.
    _eventRoomNames(evt) {
      if (!evt.roomIds || !this.state.data) return [];
      const names = [];
      for (const roomId of evt.roomIds) {
        const room = this.state.data.rooms.find(r => r.uid === roomId);
        if (!room) continue;
        const rn = room.longName || room.shortName;
        names.push(room.hotelRoom && room.hotelRoom !== rn ? `${rn} (${room.hotelRoom})` : rn);
      }
      return names;
    }

    // Per-event entry ({ evt, rooms, descriptionLines, url }) for buildIcsDoc.
    _eventIcsEntry(evt) {
      const rooms = this._eventRoomNames(evt);
      const descLines = [];
      if (evt.description) descLines.push(evt.description);
      if (evt.credits && evt.credits.length > 0) descLines.push('Presenters: ' + evt.credits.join(', '));
      if (evt.isPremium) {
        const seriesNote = seriesCostNote(evt);
        if (seriesNote) {
          descLines.push('Premium — ' + seriesNote);
        } else {
          descLines.push('Premium — requires a separate purchase' + (evt.cost ? ' (' + evt.cost + ')' : ''));
        }
      }
      const cap = capacityText(evt);
      if (cap) descLines.push('Capacity: ' + cap);
      if (evt.prereq) descLines.push('Prerequisite: ' + evt.prereq);
      const shareUrl = this.state.getPanelShareUrl(evt.id);
      descLines.push(shareUrl);
      return { evt, rooms, descriptionLines: descLines, url: shareUrl };
    }

    // Hand an .ics document to the user's calendar app.
    // iOS/iPadOS won't open a downloaded blob in Calendar; navigating to a
    // data: URL lets the OS detect the calendar payload and prompt to add the
    // event(s). Desktop browsers block top-level data: navigation, so there we
    // fall back to a normal file download (opened by the default app).
    _deliverIcs(filename, ics) {
      if (isAppleMobile()) {
        window.location.href = 'data:text/calendar;charset=utf-8,' + encodeURIComponent(ics);
        return;
      }
      downloadFile(filename, ics, 'text/calendar;charset=utf-8');
    }

    // Build a single-event .ics and hand it to the user's calendar app.
    _addEventToCalendar(evt) {
      const meta = (this.state.data && this.state.data.meta) || {};
      const ics = buildIcsDoc([this._eventIcsEntry(evt)], {
        tzid: meta.timezone || '',
        vtimezone: meta.vtimezone || '',
      });
      this._deliverIcs(slugify(evt.name) + '.ics', ics);
    }

    // Build a multi-event .ics for every event in a named schedule (in time
    // order) and hand it to the user's calendar app.
    _addScheduleToCalendar(scheduleName) {
      const ids = this.state.schedules[scheduleName];
      if (!ids || ids.size === 0) return;
      const events = ((this.state.data && this.state.data.panels) || [])
        .filter(e => ids.has(e.id) && e.startTime)
        .sort((a, b) => String(a.startTime).localeCompare(String(b.startTime)));
      if (events.length === 0) return;
      const meta = (this.state.data && this.state.data.meta) || {};
      const ics = buildIcsDoc(events.map(e => this._eventIcsEntry(e)), {
        tzid: meta.timezone || '',
        vtimezone: meta.vtimezone || '',
      });
      this._deliverIcs(slugify(scheduleName) + '-schedule.ics', ics);
    }

    // Share modal for a single event: link + QR. Closing returns to the detail
    // sheet it was opened from.
    _showSharePanelModal(evt) {
      const modal = this._modalContent;
      modal.innerHTML = '';

      modal.appendChild(el('button', {
        type: 'button',
        className: 'cosam-modal-close',
        innerHTML: ICONS.x,
        'aria-label': 'Back to event details',
        onClick: () => this._showModal(evt),
      }));

      modal.appendChild(el('h2', {}, 'Share Event'));
      modal.appendChild(el('p', { className: 'cosam-modal-desc' }, evt.name));

      const url = this.state.getPanelShareUrl(evt.id);

      const upper = el('div', { className: 'cosam-share-upper' });
      const optionsDiv = el('div', { className: 'cosam-share-options' });
      optionsDiv.appendChild(el('p', {}, 'Anyone who opens this link will see this event’s details pop up.'));
      upper.appendChild(optionsDiv);

      const qrDiv = el('div', { className: 'cosam-share-qr', role: 'img', 'aria-label': 'QR code for the event link' });
      const qrImg = el('img', { className: 'cosam-share-qr-img', alt: 'QR code' });
      qrDiv.appendChild(qrImg);
      upper.appendChild(qrDiv);
      modal.appendChild(upper);

      const urlWrapper = el('div', { className: 'cosam-share-url-wrapper' });
      const urlInput = el('input', { type: 'text', className: 'cosam-share-url-input', readOnly: true, value: url, 'aria-label': 'Event share URL' });
      const copyBtn = el('button', {
        type: 'button', className: 'cosam-btn',
        onClick: () => {
          if (navigator.clipboard) {
            navigator.clipboard.writeText(url).then(() => {
              copyBtn.textContent = 'Copied!';
              setTimeout(() => { copyBtn.textContent = 'Copy URL'; }, 1500);
            });
          } else {
            prompt('Copy this URL:', url);
          }
        },
      }, 'Copy URL');
      urlWrapper.append(urlInput, copyBtn);
      modal.appendChild(urlWrapper);

      QRCode.toDataURL(url, { width: 200, margin: 2 })
        .then(dataUrl => { qrImg.src = dataUrl; })
        .catch(() => { qrImg.hidden = true; });

      this._modalOverlay.classList.add('open');
    }

    // ── Print ──

    /**
     * Context handed to a print plugin's hooks (print / extendToolbar / attach).
     * Exposes the data and renderer helpers a plugin needs to take over print
     * rendering and contribute toolbar UI.
     */
    _printPluginCtx() {
      return {
        renderer: this,
        state: this.state,
        data: this.state.data,
        brand: (this.state.data && this.state.data.brand) || {},
        view: this.state.view,
      };
    }

    _handlePrint() {
      // Delegate the print action to a registered plugin (advanced format
      // system, Typst PDF, …) when present; otherwise run the built-in simple
      // print below.
      const plugin = this.state._printPlugin;
      if (plugin && typeof plugin.print === 'function') {
        plugin.print(this._printPluginCtx());
        return;
      }

      // Print with the current layout/color (set via the print dropdown, which
      // is available at every width — so no separate narrow-window modal).
      this._doPrint();
    }

    /**
     * "Modified: … | Generated: …" line from schedule metadata, or '' when no
     * timestamps are present. Shared by the on-screen grid footer and the simple
     * print's per-page footer.
     */
    _scheduleTimestampText() {
      const meta = this.state.data && this.state.data.meta;
      if (!meta) return '';
      const fmt = (iso) => {
        const d = new Date(iso);
        const month = d.toLocaleDateString('en-US', { month: 'short' });
        const day = d.getDate();
        let h = d.getHours();
        const m = d.getMinutes();
        const ampm = h >= 12 ? 'PM' : 'AM';
        h = h % 12 || 12;
        return `${month} ${day} ${h}:${String(m).padStart(2, '0')} ${ampm}`;
      };
      const ts = [];
      if (meta.modified) ts.push(`Modified: ${fmt(meta.modified)}`);
      if (meta.generated && (!meta.modified || meta.generated !== meta.modified)) {
        ts.push(`Generated: ${fmt(meta.generated)}`);
      }
      return ts.join(' | ');
    }

    /**
     * Drop breaks that bracket the schedule: keep a break only if it overlaps
     * the panel window — its end is after the earliest panel start and its start
     * is before the latest panel end. Avoids empty leading/trailing break rows in
     * the printed grid (e.g. overnight/simulated breaks at a day's edges).
     */
    _stripBoundaryBreaks(events) {
      let firstStart = '', lastEnd = '';
      for (const e of events) {
        if (this.state._isBreakEvent(e)) continue;
        const start = e.startTime;
        const end = e.endTime || e.startTime;
        if (start && (firstStart === '' || start < firstStart)) firstStart = start;
        if (end && end > lastEnd) lastEnd = end;
      }
      if (firstStart === '' || lastEnd === '') return events;
      return events.filter(e => {
        if (!this.state._isBreakEvent(e)) return true;
        const bStart = e.startTime || '';
        const bEnd = e.endTime || e.startTime || '';
        return bEnd > firstStart && bStart < lastEnd;
      });
    }

    _doPrint(viewOverride = null) {
      const wasDay = this.state.activeDay;

      // Ensure panel type colors are loaded
      this._ensurePanelTypeThemeStyles();

      // Print reuses the widget's .cosam- classes; the print container is marked
      // with .cosam-print so print-only rules apply positively (.cosam-print
      // .cosam-x) and inherit the shared base rules. B&W is a further .cosam-bw
      // modifier that recolors to grey. No parallel class sheet, no @media/!important.
      const isBw = this.state.printColor === 'bw';
      const printPrefix = 'cosam-';
      const printContainer = el('div', {
        className: 'cosam-print' + (isBw ? ' cosam-bw' : ''),
      });

      // Determine view to print based on printLayout setting
      let viewToPrint;
      if (viewOverride) {
        viewToPrint = viewOverride;
      } else if (this.state.printLayout === 'default') {
        // Smart selection: use current view
        viewToPrint = this.state.view;
      } else {
        viewToPrint = this.state.printLayout;
      }

      if (viewToPrint === 'grid') {
        // Grid print: reuse the on-screen CSS Grid engine in print + fillPage
        // mode (even 1fr time-unit rows that fill the page), one day per page.
        // The grid's own footer row carries the generated/modified stamp (same
        // as the web view). Mark the user's starred picks, but only when the
        // schedule isn't already filtered to starred-only (where every panel
        // would be starred and the marker is redundant). Also skip when viewing
        // a specific named schedule (not "My Schedule") since all panels are
        // already from that schedule.
        printContainer.classList.add(printPrefix + 'grid-pages');
        if (!this.state.filters.starredOnly && this.state.activeScheduleName === 'My Schedule') {
          printContainer.classList.add(printPrefix + 'show-stars');
        }
        for (const day of this.state.days) {
          this.state.activeDay = day.key;
          let events = this.state.filteredEvents.call(this.state);
          if (events.length === 0) continue;
          // Drop breaks that bracket the day so the grid spans only the real
          // panel window (no empty leading/trailing break rows).
          events = this._stripBoundaryBreaks(events);

          const dayPage = el('div', { className: printPrefix + 'day-page' });
          dayPage.appendChild(el('div', { className: printPrefix + 'day-label' }, day.label));
          dayPage.appendChild(this._buildGridView(events, true, true, printPrefix));
          printContainer.appendChild(dayPage);
        }
      } else {
        // List print: all days as list sections
        for (const day of this.state.days) {
          this.state.activeDay = day.key;
          const events = this.state.filteredEvents.call(this.state);
          if (events.length === 0) continue;

          printContainer.appendChild(el('div', { className: printPrefix + 'day-label' }, day.label));
          printContainer.appendChild(this._buildListView(events, false, printPrefix));
        }
      }

      // Restore state
      this.state.activeDay = wasDay;

      // Apply panel type colors to print container elements. Skipped for B&W:
      // leaving these inline colors off lets the grayscale .cosam-print.cosam-bw rules
      // take effect (inline styles would otherwise win over the stylesheet).
      if (this._panelTypeColors && !isBw) {
        const prefixes = [printPrefix];
        for (const prefix of prefixes) {
          // Update list view color bars
          const colorBars = printContainer.querySelectorAll('.' + prefix + 'event-color-bar');
          for (const bar of colorBars) {
            for (const [slug, color] of this._panelTypeColors) {
              if (bar.classList.contains(prefix + 'panel-type-' + slug)) {
                bar.style.backgroundColor = color;
                break;
              }
            }
          }

          // Update grid view events
          const gridEvents = printContainer.querySelectorAll('.' + prefix + 'grid-event');
          for (const event of gridEvents) {
            for (const [slug, color] of this._panelTypeColors) {
              if (event.classList.contains(prefix + 'panel-type-' + slug)) {
                event.style.borderLeftColor = color;
                // For starred events, also set a lighter background version of the accent color
                if (event.classList.contains('starred')) {
                  event.style.backgroundColor = _lightenColor(color);
                }
                break;
              }
            }
          }
        }
      }

      // Open print window
      const printWin = window.open('', '_blank');
      if (!printWin) { window.print(); return; }

      // Collect all CSS from the current document
      const allCSS = Array.from(document.styleSheets)
        .map(styleSheet => {
          try {
            return Array.from(styleSheet.cssRules)
              .map(rule => rule.cssText)
              .join('');
          } catch (e) {
            console.warn('Could not read CSS rules from stylesheet:', styleSheet.href, e);
            return '';
          }
        })
        .join('');

      printWin.document.write(`<!DOCTYPE html><html><head><meta charset="utf-8"><title>Schedule</title><style>
html,body{background:#fff!important;margin:0;height:100%;}
${allCSS}
</style></head><body>${printContainer.outerHTML}</body></html>`);
      printWin.document.close();
      printWin.focus();
      setTimeout(() => { printWin.print(); }, 500);
    }

    // ── Help ──

    _showHelpModal() {
      const modal = this._modalContent;
      modal.innerHTML = '';

      modal.appendChild(el('button', {
        type: 'button',
        className: 'cosam-modal-close',
        innerHTML: ICONS.x,
        'aria-label': 'Close',
        onClick: () => this._modalClose(),
      }));
      modal.appendChild(el('h2', {}, 'How to Use the Schedule'));

      const helpData = [
        {
          heading: 'Browsing',
          items: [
            { icon: ICONS.list, text: '<strong>List / Grid view</strong> — Switch views with the list or grid buttons. Grid view shows events in a room-by-time layout.' },
            { icon: ICONS.clock, text: '<strong>Day tabs</strong> — Click a day to see only that day\'s events, or <em>All Days</em> to see the full schedule.' },
          ],
        },
        {
          heading: 'Searching & Filtering',
          items: [
            { icon: ICONS.search, text: '<strong>Search</strong> — Type in the search box to filter events by title, description, or presenter name.' },
            { icon: ICONS.filter, text: '<strong>Filters</strong> — Click <em>Filters</em> to narrow events by room, event type, pricing (Included / Premium), or presenter.' },
          ],
        },
        {
          heading: 'My Schedule',
          items: [
            { icon: ICONS.star, text: '<strong>Star an event</strong> — Click ★ on any event card to save it to your schedule. Stars are stored in your browser.' },
            { icon: ICONS.star, text: '<strong>View your schedule</strong> — Click the <em>My Schedule</em> button in the toolbar to show only your starred events.' },
            { icon: ICONS.chevronDown, text: '<strong>Multiple schedules</strong> — Use the ▾ dropdown next to My Schedule to create, rename, merge, or delete named schedules.' },
          ],
        },
        {
          heading: 'Event Details',
          items: [
            { icon: ICONS.list, text: '<strong>Open an event</strong> — Click any event to open its details, including description, room, presenters, and pricing.' },
          ],
        },
        {
          heading: 'Add to Calendar',
          items: [
            { icon: ICONS.calendar, text: '<strong>Add one event</strong> — Open an event and click <em>Add to Calendar</em> to send it to your phone or computer\'s calendar app.' },
            { icon: ICONS.calendar, text: '<strong>Add your whole schedule</strong> — In the <em>Share</em> dialog, click <em>Add Schedule to Calendar</em> to add every event in the selected schedule at once.' },
          ],
        },
        {
          heading: 'Sharing',
          items: [
            { icon: getShareIcon(), text: '<strong>Share your schedule</strong> — Click <em>Share</em> to generate a link and QR code with your starred schedule. Anyone with the link can view your picks.' },
            { icon: getShareIcon(), text: '<strong>Share a single event</strong> — Open an event and click <em>Share</em> to get a link and QR code for just that event. Opening it pops up the event\'s details.' },
            { icon: ICONS.people, text: '<strong>Received a shared link?</strong> — A banner appears when you open one. Import it as a new schedule or merge it into an existing one.' },
          ],
        },
        {
          heading: 'Printing',
          items: [
            { icon: ICONS.print, text: '<strong>Print</strong> — Click the print icon to print the schedule. Current filters apply — star your events and filter to My Schedule first to print a personal copy.' },
            { icon: ICONS.chevronDown, text: '<strong>Print options</strong> — Use the ▾ dropdown next to the print icon to choose layout (Default/Smart, Grid, or List) and color (Color or Black & White).' },
          ],
        },
      ];

      const sections = el('div', { className: 'cosam-help-sections' });

      for (const section of helpData) {
        const sec = el('div', { className: 'cosam-help-section' });
        sec.appendChild(el('h3', {}, section.heading));
        for (const item of section.items) {
          const row = el('div', { className: 'cosam-help-item' });
          const iconEl = el('span', {
            className: 'cosam-help-icon',
            'aria-hidden': 'true',
            innerHTML: item.icon,
          });
          const textEl = el('span', {
            className: 'cosam-help-text',
            innerHTML: item.text,
          });
          row.append(iconEl, textEl);
          sec.appendChild(row);
        }
        sections.appendChild(sec);
      }

      modal.appendChild(sections);
      this._modalOverlay.classList.add('open');
    }

  }

  // ── Public API ──────────────────────────────────────────────────────────

  function applyData(rawData, state, renderer, rootEl) {
    const data = renderer._normalizeDataModel(rawData);
    state.data = data;

    // Build state.days from precomputed dayTimeline when available (FEATURE-154).
    // dayTimeline entries carry a stable YYYY-MM-DD `date` key and a localized
    // `label` — no wall-clock parsing needed. Fall back to scanning panels for
    // pre-v2 data that lacks dayTimeline; pre-v2 panels have a naive startTime
    // ISO string (no epoch seconds) so getDayKey uses substring extraction and
    // getDayLabel parses the date part without relying on new Date(isoStr).
    const events = data.panels;
    if (Array.isArray(data.dayTimeline) && data.dayTimeline.length > 0 &&
      data.dayTimeline.every(d => d.date)) {
      state.days = data.dayTimeline.map(d => ({ key: d.date, label: d.label }));
    } else {
      const daySet = new Map();
      for (const evt of events) {
        if (!evt.startTime) continue;
        if (evt.panelType && data.panelTypes) {
          const pt = data.panelTypes.find(p => p.uid === evt.panelType);
          if (pt && pt.isTimeline) continue;
        }
        const key = getDayKey(evt.startTime);
        if (!daySet.has(key)) {
          daySet.set(key, getDayLabel(evt.startTime));
        }
      }
      state.days = [...daySet.entries()].sort((a, b) => a[0].localeCompare(b[0])).map(([key, label]) => ({ key, label }));
    }

    // Only set defaults if no saved/hash state was loaded
    if (!state._hasRestoredState) {
      state.activeDay = null;
    }

    // Validate restored activeDay against available days
    if (state.activeDay && !state.days.some(d => d.key === state.activeDay)) {
      state.activeDay = null;
    }

    renderer._ensurePanelTypeThemeStyles();
    renderer.render();
  }

  window.CosAmCalendar = {
    /**
     * Built-in config loader: reads presentation config (branding +
     * print-format defaults) from an embedded
     * `<script id="cosam-config-data" data-cosam="config">` element holding a
     * ScheduleConfig. This is the default `configLoader` for {@link init}, so
     * any schedule loader picks up an embedded config element automatically; it
     * resolves to `null` when the element is absent or malformed, leaving
     * non-branded embeds unaffected.
     * @param {object} [opts]
     * @param {string} [opts.configId='cosam-config-data'] - ID of the config script element.
     */
    EmbeddedConfigLoader: function (opts) {
      var configId = (opts && opts.configId) || 'cosam-config-data';
      return {
        load: function () {
          var el = document.getElementById(configId);
          if (!el || el.getAttribute('data-cosam') !== 'config') {
            return Promise.resolve(null);
          }
          try {
            var cfg = JSON.parse(el.textContent.trim());
            return Promise.resolve({ brand: cfg.brand, printFormats: cfg.printFormats });
          } catch (e) {
            return Promise.resolve(null);
          }
        },
      };
    },

    init: function (opts) {
      const rootEl = typeof opts.el === 'string' ? document.querySelector(opts.el) : opts.el;
      if (!rootEl) { console.error('CosAmCalendar: element not found:', opts.el); return; }

      const state = new CalendarState();
      if (opts.stylePageBody !== undefined) {
        state.stylePageBody = !!opts.stylePageBody;
      }
      if (opts.showEvenGridSwitch !== undefined) {
        state.showEvenGridSwitch = !!opts.showEvenGridSwitch;
      }
      // Sticky-header top offset, so headers pin below a host-page fixed bar
      // (e.g. a Squarespace mobile nav). `stickyOffset` is a fixed pixel value;
      // `stickyOffsetSelector` auto-measures a fixed top bar by selector and
      // tracks it across resizes/orientation changes.
      if (typeof opts.stickyOffset === 'number' && isFinite(opts.stickyOffset)) {
        state.stickyOffset = Math.max(0, opts.stickyOffset);
      }
      if (typeof opts.stickyOffsetSelector === 'string') {
        state.stickyOffsetSelector = opts.stickyOffsetSelector;
      }
      const renderer = new CalendarRenderer(rootEl, state);

      // Optional print plugin. When registered, it owns the print action (and may
      // contribute toolbar UI); without one, core runs its built-in simple print.
      // This generalizes the Typst branch's `pdfExportHook`. Pass `null`/`false`
      // to force the simple print explicitly.
      if (opts.printPlugin) {
        state._printPlugin = opts.printPlugin;
        if (typeof opts.printPlugin.attach === 'function') {
          opts.printPlugin.attach({ renderer, state });
        }
      }

      // Keep the sticky offset in sync with viewport changes.
      if (state.stickyOffset || state.stickyOffsetSelector) {
        const applyOffset = () => renderer._applyStickyOffset();
        window.addEventListener('resize', applyOffset);
        window.addEventListener('orientationchange', applyOffset);
        // Recompute shortly after load: host fixed bars may mount late.
        setTimeout(applyOffset, 250);
        setTimeout(applyOffset, 1000);
      }

      // Set up render callback for responsive view changes
      state._renderCallback = () => renderer.render();

      // Show loading
      renderer.render();

      if (opts.data) {
        applyData(opts.data, state, renderer, rootEl);
        return;
      }

      if (opts.loader) {
        // Presentation config (branding + print-format defaults) loads through a
        // separate, optional hook so config delivery is independent of schedule
        // delivery (e.g. embedded config + a data-url schedule). Defaults to the
        // built-in EmbeddedConfigLoader; pass `null`/`false` to disable. Config
        // loading is non-fatal — branding is additive, so a failure resolves to
        // null and the schedule still renders.
        const configLoader = opts.configLoader === undefined
          ? window.CosAmCalendar.EmbeddedConfigLoader()
          : opts.configLoader;
        const loadConfig = () => {
          if (!configLoader || typeof configLoader.load !== 'function') {
            return Promise.resolve(null);
          }
          return Promise.resolve()
            .then(() => configLoader.load(rootEl))
            .catch(() => null);
        };
        const doLoad = () => {
          Promise.all([opts.loader.load(rootEl), loadConfig()])
            .then(([data, config]) => {
              if (config) {
                if (config.brand) data.brand = config.brand;
                if (config.printFormats) data.printFormats = config.printFormats;
              }
              applyData(data, state, renderer, rootEl);
            })
            .catch(err => {
              state._loadStatus = 'error';
              state._loadError = err.message;
              renderer.render();
            });
        };
        state._reloadCallback = doLoad;
        if (opts.loader.watch) {
          opts.loader.watch(rootEl, doLoad);
        }
        doLoad();
        return;
      }

      console.error('CosAmCalendar: no data source configured (opts.data or opts.loader required)');
      state._loadStatus = 'error';
      state._loadError = 'No data source configured';
      renderer.render();
    }
  };
})();
