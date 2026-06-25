/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

/**
 * CosAm Calendar Kiosk Plugin
 *
 * A full-screen, auto-scrolling schedule display for unattended kiosks, modeled
 * on the legacy schedule-to-html kiosk. Adds a "Kiosk" toolbar button that opens:
 *
 *   - a banner with the venue logo/title and a live clock (schedule timezone),
 *   - a top pane with the schedule grid that auto-scrolls to "now" and tints the
 *     currently-running panels with the accent color, and
 *   - a bottom pane listing the current + upcoming panel for every room.
 *
 * Manual scroll/click in the grid pauses auto-scroll for 2 minutes; clicking the
 * clock resumes immediately. Clicking a panel opens its detail modal.
 *
 * Registers a default instance into `window.CosAmCalendarPlugins` (so a host page
 * generator can include it without knowing the constructor name) and exposes the
 * constructor as `window.KioskPlugin`.
 */

(function () {
  'use strict';

  // Auto-scroll resumes this long (ms, real wall-clock) after a manual gesture.
  const MANUAL_PAUSE_MS = 2 * 60 * 1000;
  const WEEKDAYS = [
    'SUNDAY', 'MONDAY', 'TUESDAY', 'WEDNESDAY', 'THURSDAY', 'FRIDAY', 'SATURDAY',
  ];

  // ── Local DOM helper (mirrors the core/print-plugin `el`) ────────────────────

  function el(tag, attrs, ...children) {
    const e = document.createElement(tag);
    if (attrs) {
      for (const [k, v] of Object.entries(attrs)) {
        if (v === null || v === undefined) continue;
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

  // Minutes → compact "45m" / "1h 23m" / "2h" duration string.
  function formatCountdown(mins) {
    if (mins < 60) return mins + 'm';
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    return m ? `${h}h ${m}m` : `${h}h`;
  }

  // 24h-ISO (YYYY-MM-DDTHH:MM:SS, schedule-tz wall-clock) → "WEEKDAY h:mm:ss AM".
  function formatClock(iso) {
    if (!iso) return '';
    const d = new Date(iso); // digits preserved under browser-local parse
    const weekday = WEEKDAYS[d.getDay()] || '';
    const [, time = ''] = iso.split('T');
    const [hStr = '0', mStr = '00', sStr = '00'] = time.split(':');
    let h = parseInt(hStr, 10);
    const ampm = h < 12 ? 'AM' : 'PM';
    h = h % 12;
    if (h === 0) h = 12;
    return `${weekday} ${h}:${mStr}:${sStr} ${ampm}`;
  }

  class KioskPlugin {
    constructor() {
      this.host = null;
      this.shell = null;
      this._clockEl = null;
      this._gridScroll = null;
      this._detailPane = null;
      this._timer = null;
      this._keyHandler = null;
      // Real-clock timestamp until which auto-scroll stays paused (manual mode).
      this._manualUntilMs = 0;
      this._lastMinuteKey = null;
      // Future-preview: when set (epoch seconds), the kiosk overrides the shared
      // "now" (via state.nowProvider) so a curator can rehearse any point in the
      // event — the grid highlight, detail pane, and clock all follow it. Toggled
      // by the crystal-ball button; clearing restores the real clock.
      this._previewEpoch = null;
      this._savedNowProvider = undefined;
      this._previewBtn = null;
      this._previewInput = null;
    }

    // ── Plugin contract ───────────────────────────────────────────────────────

    attach(host) {
      this.host = host;
      this._buildShell();
      // Auto-enter when the page is opened at #kiosk (handy for dedicated kiosk
      // displays and for testing — pair with ?cosamNow=<epoch|ISO>).
      // Defer so the host's first render() (which wipes/re-appends the overlay
      // layer, resetting scroll) has already run before we enter and auto-scroll.
      try {
        if (window.location.hash === '#kiosk') setTimeout(() => this._enter(), 0);
      } catch (e) { /* ignore */ }
    }

    extendToolbar(toolbar, host) {
      this.host = host;
      const btn = host.el('button', {
        type: 'button',
        className: 'cosam-btn cosam-btn-icon cosam-kiosk-btn',
        title: 'Kiosk display',
        'aria-label': 'Open kiosk display',
        innerHTML: host.ICONS.clock,
        onClick: () => this._enter(),
      });
      toolbar.appendChild(btn);
    }

    // Contribute a section to the core help dialog.
    helpSections(host) {
      const I = host.ICONS;
      return [{
        heading: 'Kiosk Display',
        items: [
          { icon: I.clock, text: '<strong>Open the kiosk</strong> — Click the clock button in the toolbar for a full-screen, auto-scrolling display with a live clock and a per-room "now / next" panel list.' },
          { icon: I.x, text: '<strong>Exit</strong> — Press <em>Esc</em> or click the × in the top-right to leave the kiosk.' },
          { icon: I.clock, text: '<strong>Auto-scroll</strong> — The grid follows the current time. Scroll or tap to browse; it resumes after 2 minutes, or immediately if you click the clock.' },
          { icon: I.calendar, text: '<strong>Preview another time</strong> — Click 🔮 to rehearse any moment: pick a time or tap a slot in the time column; click 🔮 again to return to now.' },
          { icon: I.list, text: '<strong>Details</strong> — Click any panel (in the grid or the bottom list) to open its full details. On phones the grid is replaced by a condensed per-room list, with ★ marking panels in your schedule.' },
        ],
      }];
    }

    // ── Shell (built once, lives in the host overlay layer) ───────────────────

    _buildShell() {
      const host = this.host;
      this.shell = el('div', {
        className: 'cosam-kiosk',
        role: 'region',
        'aria-label': 'Kiosk schedule display',
      });

      // Banner: exit far left (so it is never pushed off-screen at any width),
      // then logo/title, preview controls, and the live clock on the right.
      const banner = el('div', { className: 'cosam-kiosk-banner' });
      const exitBtn = el('button', {
        type: 'button',
        className: 'cosam-kiosk-exit',
        title: 'Exit kiosk',
        'aria-label': 'Exit kiosk',
        innerHTML: host.ICONS.x,
        onClick: () => this._exit(),
      });
      banner.appendChild(exitBtn);
      banner.appendChild(this._buildBrand());

      // Future-preview: a datetime-local input (hidden until preview is on) and a
      // crystal-ball toggle. While previewing, the banner shows the input and any
      // time-column cell can be clicked to jump "now" to that slot.
      this._previewInput = el('input', {
        type: 'datetime-local',
        className: 'cosam-kiosk-preview-input',
        'aria-label': 'Preview time',
        onChange: (e) => this._setPreviewFromInput(e.target.value),
      });
      banner.appendChild(this._previewInput);
      this._previewBtn = el('button', {
        type: 'button',
        className: 'cosam-kiosk-preview-btn',
        title: 'Preview a future/past time',
        'aria-label': 'Toggle time preview',
        'aria-pressed': 'false',
        innerHTML: '🔮', // crystal ball
        onClick: () => this._togglePreview(),
      });
      banner.appendChild(this._previewBtn);

      this._clockEl = el('button', {
        type: 'button',
        className: 'cosam-kiosk-clock',
        title: 'Resume auto-scroll',
        'aria-label': 'Current time — click to resume auto-scroll',
        // Clicking the clock cancels any manual pause and snaps back to now.
        onClick: () => { this._manualUntilMs = 0; this._tick(true); },
      });
      banner.appendChild(this._clockEl);
      this.shell.appendChild(banner);

      // Top pane: scrollable schedule grid (filled on enter).
      this._gridScroll = el('div', {
        className: 'cosam-kiosk-grid-scroll',
        role: 'group',
        'aria-label': 'Schedule grid',
      });
      // Genuine user gestures (not programmatic scrolls) pause auto-scroll.
      const pause = () => { this._manualUntilMs = Date.now() + MANUAL_PAUSE_MS; };
      for (const ev of ['wheel', 'touchstart', 'pointerdown', 'keydown']) {
        this._gridScroll.addEventListener(ev, pause, { passive: true });
      }
      // While previewing, clicking a time-column cell sets "now" to that slot.
      this._gridScroll.addEventListener('click', (e) => {
        if (this._previewEpoch == null) return;
        const cell = e.target.closest('.cosam-grid-time-header[data-slot-epoch]');
        if (!cell) return;
        const se = Number(cell.dataset.slotEpoch);
        if (isFinite(se)) this._setPreview(se);
      });
      this.shell.appendChild(this._gridScroll);

      // Bottom pane: per-room current/upcoming (filled on enter).
      this._detailPane = el('div', {
        className: 'cosam-kiosk-detail',
        role: 'group',
        'aria-label': 'Current and upcoming panels by room',
      });
      this.shell.appendChild(this._detailPane);

      // Mobile pane: condensed per-room current/upcoming titles. Shown instead
      // of the grid + detail panes at phone widths (CSS-toggled via the shell's
      // .cosam-kiosk-narrow class), where the grid would reflow to a broken
      // single column.
      this._mobilePane = el('div', {
        className: 'cosam-kiosk-mobile',
        role: 'group',
        'aria-label': 'Current and upcoming panels by room',
      });
      this.shell.appendChild(this._mobilePane);

      if (host.mountOverlay) host.mountOverlay(this.shell);
    }

    _buildBrand() {
      const host = this.host;
      const brand = host.brand || {};
      const logos = brand.logos || {};
      const logoUrl = logos.brand || logos.small || brand.logo || null;
      const title = (host.data && host.data.meta && host.data.meta.title) || 'Schedule';
      const wrap = el('div', { className: 'cosam-kiosk-brand' });
      if (logoUrl) {
        wrap.appendChild(el('img', {
          className: 'cosam-kiosk-logo',
          src: logoUrl,
          alt: title,
        }));
      } else {
        wrap.appendChild(el('div', { className: 'cosam-kiosk-title' }, title));
      }
      return wrap;
    }

    // ── Enter / exit ──────────────────────────────────────────────────────────

    _enter() {
      if (!this.shell) this._buildShell();
      this.shell.classList.toggle('cosam-kiosk-narrow', this._isNarrow());
      this._renderGrid();
      this.shell.classList.add('open');
      document.documentElement.classList.add('cosam-kiosk-active');
      this._tick(true);
      if (!this._timer) this._timer = setInterval(() => this._tick(false), 1000);
      this._keyHandler = (e) => { if (e.key === 'Escape') this._exit(); };
      document.addEventListener('keydown', this._keyHandler);
      // Swap between desktop (grid + detail) and the condensed mobile list when
      // the viewport crosses the phone breakpoint.
      this._resizeHandler = () => this._applyResponsive();
      window.addEventListener('resize', this._resizeHandler);
    }

    _exit() {
      // Drop any preview override so the live widget keeps the real clock.
      if (this._previewEpoch != null) this._clearPreview();
      if (this.shell) this.shell.classList.remove('open');
      document.documentElement.classList.remove('cosam-kiosk-active');
      if (this._timer) { clearInterval(this._timer); this._timer = null; }
      if (this._keyHandler) {
        document.removeEventListener('keydown', this._keyHandler);
        this._keyHandler = null;
      }
      if (this._resizeHandler) {
        window.removeEventListener('resize', this._resizeHandler);
        this._resizeHandler = null;
      }
    }

    // React to a viewport crossing the phone breakpoint: toggle the layout and
    // refresh whichever pane is now visible.
    _applyResponsive() {
      if (!this.shell || !this.shell.classList.contains('open')) return;
      const narrow = this._isNarrow();
      const was = this.shell.classList.contains('cosam-kiosk-narrow');
      this.shell.classList.toggle('cosam-kiosk-narrow', narrow);
      if (narrow) {
        this._renderMobile();
      } else {
        this._renderDetails();
        if (was) this._autoScroll(this.host.nowEpoch(), true); // re-center grid
      }
    }

    // ── Top pane: schedule grid ───────────────────────────────────────────────

    _renderGrid() {
      const host = this.host;
      const state = host.state;
      // Build the full schedule (all days), independent of the widget's active
      // day tab, so the kiosk can scroll across the whole event.
      const prevDay = state.activeDay;
      state.activeDay = null;
      let events;
      try {
        events = state.filteredEvents.call(state);
      } finally {
        state.activeDay = prevDay;
      }
      const grid = host.renderer._buildGridView(events);
      this._grid = grid.querySelector('.cosam-grid') || grid;
      // Full-width band that tints the active hour's row (placed by grid line
      // names in _markCurrentRow). Lives inside the grid so it tracks its rows.
      this._nowBand = el('div', { className: 'cosam-kiosk-now-band', 'aria-hidden': 'true' });
      this._nowBand.style.display = 'none';
      this._grid.appendChild(this._nowBand);
      this._gridScroll.innerHTML = '';
      this._gridScroll.appendChild(grid);
      // Apply the live current-panel tint to the freshly built grid.
      host.refreshCurrent();
    }

    // ── Per-room current / upcoming ───────────────────────────────────────────

    // Sorted rooms each paired with their current + next (upcoming) panel at
    // `now`. Shared by the desktop detail pane and the condensed mobile list.
    _roomsCurrentNext(now) {
      const host = this.host;
      const data = host.data || {};
      const rooms = (data.rooms || []).slice().sort((a, b) => (a.sortKey || 0) - (b.sortKey || 0));

      // Bucket non-break panels by room.
      const byRoom = new Map();
      for (const p of data.panels || []) {
        if (host.state._isBreakEvent(p)) continue;
        if (typeof p.startEpoch !== 'number') continue;
        for (const rid of p.roomIds || []) {
          if (!byRoom.has(rid)) byRoom.set(rid, []);
          byRoom.get(rid).push(p);
        }
      }

      return rooms.map(room => {
        const panels = (byRoom.get(room.uid) || []).slice().sort((a, b) => a.startEpoch - b.startEpoch);
        let current = null;
        let next = null;
        for (const p of panels) {
          if (p.startEpoch <= now && (typeof p.endEpoch !== 'number' || now < p.endEpoch)) {
            current = p;
          } else if (p.startEpoch > now && !next) {
            next = p;
          }
        }
        return { room, current, next };
      });
    }

    // Narrow (phone) widths use the same breakpoint as the widget's grid→list
    // switch; the kiosk grid would reflow to a broken single column there.
    _isNarrow() {
      return window.innerWidth < 750;
    }

    _openPanel(panel) {
      this.host.state.modalEvent = panel;
      this.host.renderer._showModal(panel);
    }

    // ── Bottom pane: per-room current / upcoming (desktop) ────────────────────

    _renderDetails() {
      const now = this.host.nowEpoch();
      const pane = this._detailPane;
      pane.innerHTML = '';
      const grid = el('div', { className: 'cosam-kiosk-detail-grid' });
      grid.appendChild(el('div', { className: 'cosam-kiosk-detail-corner' }, 'Room'));
      grid.appendChild(el('div', { className: 'cosam-kiosk-detail-head' }, 'Current Panel'));
      grid.appendChild(el('div', { className: 'cosam-kiosk-detail-head' }, 'Upcoming Panel'));

      for (const { room, current, next } of this._roomsCurrentNext(now)) {
        const label = el('div', { className: 'cosam-kiosk-room' });
        if (room.hotelRoom) {
          label.appendChild(el('div', { className: 'cosam-kiosk-room-hotel' }, room.hotelRoom));
        }
        label.appendChild(el('div', { className: 'cosam-kiosk-room-name' }, room.longName || room.shortName || ''));

        grid.appendChild(label);
        grid.appendChild(this._buildDetailCell(current, 'current', now));
        grid.appendChild(this._buildDetailCell(next, 'next', now));
      }

      pane.appendChild(grid);
    }

    // ── Mobile: condensed per-room current / upcoming titles ──────────────────

    _renderMobile() {
      const host = this.host;
      const now = host.nowEpoch();
      const pane = this._mobilePane;
      pane.innerHTML = '';

      for (const { room, current, next } of this._roomsCurrentNext(now)) {
        const card = el('div', { className: 'cosam-kiosk-m-room' });
        card.appendChild(el('div', { className: 'cosam-kiosk-m-name' },
          room.longName || room.shortName || ''));
        card.appendChild(this._buildMobileSlot('Now', current, 'now', now));
        card.appendChild(this._buildMobileSlot('Next', next, 'next', now));
        pane.appendChild(card);
      }
    }

    _buildMobileSlot(label, panel, kind, now) {
      const row = el('div', { className: 'cosam-kiosk-m-slot cosam-kiosk-m-' + kind });
      row.appendChild(el('span', { className: 'cosam-kiosk-m-label' }, label));
      if (!panel) {
        row.appendChild(el('span', { className: 'cosam-kiosk-m-title cosam-kiosk-m-empty' }, '—'));
        return row;
      }
      // Flag panels in the viewer's own schedule (their stars) — this is their
      // phone, so it's their localStorage, unlike a shared wall kiosk.
      const state = this.host.state;
      const starred = state.starred.has(panel.id) || state.sharedStarred.has(panel.id);
      if (starred) {
        row.classList.add('cosam-kiosk-m-starred');
        const star = el('span', {
          className: 'cosam-kiosk-m-star',
          'aria-label': 'In your schedule',
          innerHTML: this.host.ICONS.star,
        });
        row.appendChild(star);
      }
      row.appendChild(el('span', { className: 'cosam-kiosk-m-title' }, panel.name || ''));
      if (kind === 'next' && typeof panel.startEpoch === 'number') {
        const mins = Math.round((panel.startEpoch - now) / 60);
        if (mins > 0) {
          row.appendChild(el('span', { className: 'cosam-kiosk-m-cd' }, 'in ' + formatCountdown(mins)));
        }
      }
      row.setAttribute('role', 'button');
      row.setAttribute('tabindex', '0');
      row.addEventListener('click', () => this._openPanel(panel));
      row.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); this._openPanel(panel); }
      });
      return row;
    }

    _buildDetailCell(panel, kind, now) {
      const cell = el('div', {
        className: 'cosam-kiosk-detail-cell cosam-kiosk-detail-' + kind,
      });
      if (!panel) {
        cell.classList.add('cosam-kiosk-detail-empty');
        return cell;
      }
      const host = this.host;
      // Clicking the cell opens the shared detail modal (mounted in the overlay
      // layer, so it renders above the kiosk), matching a grid-panel click.
      const openDetail = () => { host.state.modalEvent = panel; host.renderer._showModal(panel); };
      cell.classList.add('cosam-kiosk-detail-clickable');
      cell.setAttribute('role', 'button');
      cell.setAttribute('tabindex', '0');
      cell.addEventListener('click', openDetail);
      cell.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); openDetail(); }
      });
      const head = el('div', { className: 'cosam-kiosk-cell-head' });
      head.appendChild(el('div', { className: 'cosam-kiosk-cell-name' }, panel.name || ''));
      // Time block: start time, plus a "starts in" countdown for upcoming panels.
      const timeWrap = el('div', { className: 'cosam-kiosk-cell-time' });
      const split = host.helpers.formatTimeSplit(panel.startTime);
      if (split && split.full) {
        timeWrap.appendChild(el('span', {}, split.full));
      }
      if (kind === 'next' && typeof now === 'number' && typeof panel.startEpoch === 'number') {
        const mins = Math.round((panel.startEpoch - now) / 60);
        if (mins > 0) {
          timeWrap.appendChild(el('span', { className: 'cosam-kiosk-cell-countdown' }, 'in ' + formatCountdown(mins)));
        }
      }
      head.appendChild(timeWrap);
      cell.appendChild(head);
      if (panel.credits && panel.credits.length > 0) {
        cell.appendChild(el('div', { className: 'cosam-kiosk-cell-credits' }, panel.credits.join(', ')));
      }
      if (panel.description) {
        cell.appendChild(el('div', { className: 'cosam-kiosk-cell-desc' }, panel.description));
      }
      return cell;
    }

    // ── Per-second tick: clock, highlight, auto-scroll ────────────────────────

    _tick(force) {
      const host = this.host;
      if (!host) return;
      const now = host.nowEpoch();
      const tz = (host.data && host.data.meta && host.data.meta.timezone) || 'UTC';
      const iso = host.helpers.epochToLocalIso(now, tz);
      if (this._clockEl) this._clockEl.textContent = formatClock(iso);

      const narrow = this._isNarrow();
      // Heavier work only when the minute changes (or on a forced refresh).
      const minuteKey = iso.slice(0, 16); // YYYY-MM-DDTHH:MM
      if (force || minuteKey !== this._lastMinuteKey) {
        this._lastMinuteKey = minuteKey;
        host.refreshCurrent();
        if (narrow) {
          // Phone layout: condensed per-room list, no grid.
          this._renderMobile();
        } else {
          this._renderDetails();
          this._markCurrentRow(now);
        }
      }

      // Auto-scroll the grid (desktop only) unless the user is browsing.
      if (!narrow && (force || Date.now() >= this._manualUntilMs)) {
        this._autoScroll(now, force);
      }
    }

    // Highlight the active hour: the time-header for the active slot (largest
    // slot <= now) plus a tint band spanning that row across all columns.
    _markCurrentRow(now) {
      // Headers in slot order so we can find both the active slot and the next
      // one (the band spans active → next).
      const headers = Array.from(this._gridScroll.querySelectorAll('[data-slot-epoch]'))
        .sort((a, b) => Number(a.dataset.slotEpoch) - Number(b.dataset.slotEpoch));
      let activeIdx = -1;
      for (let i = 0; i < headers.length; i++) {
        headers[i].classList.remove('cosam-kiosk-now');
        if (Number(headers[i].dataset.slotEpoch) <= now) activeIdx = i;
      }
      const active = activeIdx >= 0 ? headers[activeIdx] : null;
      this._activeHeader = active;
      if (active) active.classList.add('cosam-kiosk-now');

      // Position (or hide) the active-hour band.
      const band = this._nowBand;
      if (band) {
        if (active) {
          const next = headers[activeIdx + 1];
          band.style.display = '';
          band.style.gridColumn = '1 / -1';
          band.style.gridRow = next
            ? `${active.dataset.slot} / ${next.dataset.slot}`
            : active.dataset.slot;
        } else {
          band.style.display = 'none';
        }
      }
    }

    _autoScroll(now, force) {
      if (!this._activeHeader) this._markCurrentRow(now);
      const active = this._activeHeader;
      if (!active) return;
      const scroll = this._gridScroll;
      const cRect = scroll.getBoundingClientRect();
      const eRect = active.getBoundingClientRect();
      // Place the active row ~64px below the top of the scroll viewport so the
      // prior slot stays visible for context. Computing the delta each tick is
      // self-correcting: a host re-render resets scrollTop, and the next tick
      // simply re-centers. No-op once positioned (delta ≈ 0). Forced jumps are
      // instant; periodic catch-ups glide.
      const delta = (eRect.top - cRect.top) - 64;
      if (!force && Math.abs(delta) < 4) return;
      scroll.scrollTo({
        top: scroll.scrollTop + delta,
        behavior: force ? 'auto' : 'smooth',
      });
    }

    // ── Future preview (crystal ball) ─────────────────────────────────────────

    _togglePreview() {
      if (this._previewEpoch == null) {
        this._setPreview(this.host.nowEpoch());
      } else {
        this._clearPreview();
      }
    }

    // Override the shared "now" so every view (grid tint, detail pane, clock)
    // reflects the previewed instant.
    _setPreview(epochSec) {
      const host = this.host;
      if (this._savedNowProvider === undefined) {
        this._savedNowProvider = host.state.nowProvider; // real or ?cosamNow= source
      }
      this._previewEpoch = epochSec;
      host.state.nowProvider = () => epochSec * 1000;
      this.shell.classList.add('cosam-kiosk-previewing');
      if (this._previewBtn) this._previewBtn.setAttribute('aria-pressed', 'true');
      if (this._previewInput) {
        const tz = (host.data && host.data.meta && host.data.meta.timezone) || 'UTC';
        this._previewInput.value = host.helpers.epochToLocalIso(epochSec, tz).slice(0, 16);
      }
      this._manualUntilMs = 0; // re-center on the previewed time
      this._tick(true);
    }

    _setPreviewFromInput(value) {
      if (!value) return;
      const ms = Date.parse(value);
      if (!isNaN(ms)) this._setPreview(Math.floor(ms / 1000));
    }

    _clearPreview() {
      const host = this.host;
      this._previewEpoch = null;
      if (this._savedNowProvider !== undefined) {
        host.state.nowProvider = this._savedNowProvider;
        this._savedNowProvider = undefined;
      }
      this.shell.classList.remove('cosam-kiosk-previewing');
      if (this._previewBtn) this._previewBtn.setAttribute('aria-pressed', 'false');
      this._tick(true);
    }
  }

  window.KioskPlugin = KioskPlugin;
  // Self-register a default instance so host-page generators can include the
  // plugin script without knowing its constructor name (see cosam-convert
  // --plugin). CosAmCalendar.init() merges this array into its plugin registry.
  (window.CosAmCalendarPlugins = window.CosAmCalendarPlugins || []).push(new KioskPlugin());
})();
