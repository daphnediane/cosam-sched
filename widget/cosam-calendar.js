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
    search: '<svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>',
    print: '<svg viewBox="0 0 24 24"><polyline points="6 9 6 2 18 2 18 9"/><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"/><rect x="6" y="14" width="12" height="8"/></svg>',
    x: '<svg viewBox="0 0 24 24"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>',
    share: '<svg viewBox="0 0 24 24"><circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/><line x1="8.59" y1="13.51" x2="15.42" y2="17.49"/><line x1="15.41" y1="6.51" x2="8.59" y2="10.49"/></svg>',
    shareApple: '<svg viewBox="0 0 24 24"><path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"/><polyline points="16 6 12 2 8 6"/><line x1="12" y1="2" x2="12" y2="15"/></svg>',
    shareWindows: '<svg viewBox="0 0 24 24"><path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"/><path d="M16 6l-4-4-4 4"/><line x1="12" y1="2" x2="12" y2="15"/></svg>',
    people: '<svg viewBox="0 0 24 24"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>',
    chevronDown: '<svg viewBox="0 0 24 24"><polyline points="6 9 12 15 18 9"/></svg>',
    clock: '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>',
    mappin: '<svg viewBox="0 0 24 24"><path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z"/><circle cx="12" cy="10" r="3"/></svg>',
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
    const d = new Date(isoStr);
    let h = d.getHours();
    const m = d.getMinutes();
    if (h === 0 && m === 0) return 'Midnight';
    if (h === 12 && m === 0) return 'Noon';
    const ampm = h >= 12 ? 'PM' : 'AM';
    h = h % 12 || 12;
    return m === 0 ? `${h} ${ampm}` : `${h}:${String(m).padStart(2, '0')} ${ampm}`;
  }

  function formatTimeGrid(isoStr) {
    if (!isoStr) return '';
    const d = new Date(isoStr);
    let h = d.getHours();
    const m = d.getMinutes();
    if (h === 0 && m === 0) return 'Midnight';
    if (h === 12 && m === 0) return 'Noon';
    const ampm = h >= 12 ? 'PM' : 'AM';
    h = h % 12 || 12;
    return m === 0 ? `${h} ${ampm}` : `${h}:${String(m).padStart(2, '0')}`;
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
    const d = new Date(isoStr);
    let h = d.getHours();
    const m = d.getMinutes();

    // Midnight and Noon span both columns (centered)
    if (h === 0 && m === 0) {
      return { isSpecial: true, hour: 'Midnight', suffix: '', full: 'Midnight', label: 'Midnight' };
    }
    if (h === 12 && m === 0) {
      return { isSpecial: true, hour: 'Noon', suffix: '', full: 'Noon', label: 'Noon' };
    }

    const ampm = h >= 12 ? 'PM' : 'AM';
    h = h % 12 || 12;

    if (m === 0) {
      // On the hour: hour in left, AM/PM in right (with non-breaking space)
      return {
        isSpecial: false,
        hour: String(h),
        suffix: `\u00A0${ampm}`,
        full: `${h} ${ampm}`,
        label: `${h} ${ampm}`
      };
    } else {
      // With minutes: hour in left, :MM in right
      return {
        isSpecial: false,
        hour: String(h),
        suffix: `:${String(m).padStart(2, '0')}`,
        full: `${h}:${String(m).padStart(2, '0')} ${ampm}`,
        label: `${h}:${String(m).padStart(2, '0')} ${ampm}`
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
    const d = new Date(isoStr);
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

  function escapeHtml(s) {
    if (!s) return '';
    const div = document.createElement('div');
    div.textContent = s;
    return div.innerHTML;
  }

  // ── State ───────────────────────────────────────────────────────────────

  class CalendarState {
    constructor() {
      this.view = 'list'; // 'list' or 'grid'
      this.theme = 'cosam';
      this.activeDay = null;
      this.days = [];
      // Named schedules: { scheduleName: Set<eventId> }
      this.schedules = { 'My Schedule': new Set() };
      this.activeScheduleName = 'My Schedule';
      // Transient: starred events from a shared URL (never persisted)
      this.sharedStarred = new Set();
      this.sharedScheduleName = 'Shared Schedule';
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

      // Day filter
      if (this.activeDay) {
        events = events.filter(e => getDayKey(e.startTime) === this.activeDay);
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

        // V9 format: Use panelIds for efficient filtering
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
          events = []; // Presenter not found in V9 data or group has no panels
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
      this.root.classList.add('cosam-calendar');
      this.root.setAttribute('role', 'region');
      this.root.setAttribute('aria-label', 'Cosplay America schedule');
    }

    render() {
      this.root.innerHTML = '';
      if (!this.state.data) {
        this.root.appendChild(el('div', { className: 'cosam-loading' }, 'Loading schedule...'));
        return;
      }
      const theme = this.state.theme || 'cosam';
      this.root.setAttribute('data-theme', theme);
      this._applyPageStyling(theme);
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
        eventsRegion.appendChild(this._buildGridView(events));
      } else {
        eventsRegion.appendChild(this._buildListView(events));
      }
      this.root.appendChild(eventsRegion);

      this.root.appendChild(this._buildModal());

      // Apply color bar styles after rendering
      this._updateColorBarStyles();
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

    _panelTypeClass(panelTypeUid) {
      if (!panelTypeUid) return '';
      const slug = String(panelTypeUid).trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
      return slug ? 'cosam-panel-type-' + slug : '';
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
      // Version 0 is the new export format (structurally identical to V9).
      let presenters = [];
      let presenterToPanels = new Map();

      const version = data.meta && data.meta.version;
      if (Array.isArray(data.presenters) && version >= 0) {
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

      return {
        ...data,
        panelTypes,
        panels,
        presenters,
        rooms,
        presenterToPanels
      };
    }

    _ensurePanelTypeThemeStyles() {
      const panelTypes = this.state.data && this.state.data.panelTypes;
      if (!Array.isArray(panelTypes) || panelTypes.length === 0) return;

      // Store panel type colors for direct application
      this._panelTypeColors = new Map();
      for (const pt of panelTypes) {
        const cls = this._panelTypeClass(pt.uid);
        if (!cls || !pt.color) continue;
        this._panelTypeColors.set(cls, pt.color);
      }

      // Apply styles to existing color bars
      this._updateColorBarStyles();
    }

    _updateColorBarStyles() {
      if (!this._panelTypeColors) return;

      // Update list view color bars
      const colorBars = this.root.querySelectorAll('.cosam-event-color-bar');
      for (const bar of colorBars) {
        for (const [cls, color] of this._panelTypeColors) {
          if (bar.classList.contains(cls)) {
            bar.style.backgroundColor = color;
            break;
          }
        }
      }

      // Update grid view events
      const gridEvents = this.root.querySelectorAll('.cosam-grid-event');
      for (const event of gridEvents) {
        for (const [cls, color] of this._panelTypeColors) {
          if (event.classList.contains(cls)) {
            event.style.borderLeftColor = color;
            break;
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

      const themeSelect = el('select', {
        className: 'cosam-theme-select',
        'aria-label': 'Theme',
      });
      const themeOptions = [
        ['cosam', 'Default'],
        ['light', 'Light'],
        ['dark', 'Dark'],
        ['high-contrast', 'High Contrast'],
      ];
      for (const [value, label] of themeOptions) {
        const option = el('option', { value }, label);
        if (this.state.theme === value) option.selected = true;
        themeSelect.appendChild(option);
      }
      themeSelect.addEventListener('change', () => {
        this.state.setTheme(themeSelect.value);
        this.render();
      });
      right.appendChild(themeSelect);

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
      const printBtn = el('button', {
        type: 'button',
        className: 'cosam-btn cosam-btn-icon',
        title: 'Print schedule',
        'aria-label': 'Print schedule',
        innerHTML: ICONS.print,
        onClick: () => this._handlePrint(),
      });
      right.appendChild(printBtn);

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

    _buildListView(events) {
      const container = el('div', { className: 'cosam-list-view' });

      // Group by time slot
      const groups = new Map();
      for (const evt of events) {
        const key = getTimeSlotKey(evt.startTime);
        if (!groups.has(key)) groups.set(key, []);
        groups.get(key).push(evt);
      }

      const showAllDays = !this.state.activeDay;
      let lastDayKey = null;

      // Sort time keys chronologically for proper day transition detection
      const sortedTimeKeys = Array.from(groups.keys()).sort();

      for (const timeKey of sortedTimeKeys) {
        const evts = groups.get(timeKey);
        const group = el('div', { className: 'cosam-time-group' });
        const timeLabel = evts[0] ? formatTime(evts[0].startTime) : timeKey;
        // Day transition handling
        let dayLabel = null;
        if (showAllDays && evts && evts.length > 0) {
          const dayKey = getDayKey(evts[0].startTime);
          if (dayKey !== lastDayKey) {
            dayLabel = getDayLabel(evts[0].startTime);
            lastDayKey = dayKey;
          }
        }
        const timeHeader = el('div', { className: 'cosam-time-header' });
        if (dayLabel) {
          timeHeader.appendChild(el('div', { className: 'cosam-time-header-day' }, dayLabel));
        }
        // Use split time format for aligned display with accessibility
        const timeSplit = formatTimeSplit(evts[0] ? evts[0].startTime : null);
        if (timeSplit.isSpecial) {
          // Midnight/Noon - centered across both columns
          timeHeader.appendChild(el('div', {
            className: 'cosam-time-header-time cosam-time-split cosam-time-special',
            'aria-label': timeSplit.label,
          }, timeSplit.hour));
        } else {
          // Regular time - split into hour (right) and suffix (left)
          const timeContainer = el('div', {
            className: 'cosam-time-header-time cosam-time-split',
            'aria-label': timeSplit.label,
          });
          // Screen reader only full time
          timeContainer.appendChild(el('span', { className: 'cosam-sr-only' }, timeSplit.full));
          // Visible hour part (right-aligned)
          timeContainer.appendChild(el('span', {
            className: 'cosam-time-hour',
            'aria-hidden': 'true',
          }, timeSplit.hour));
          // Visible suffix part (left-aligned, AM/PM or :MM)
          timeContainer.appendChild(el('span', {
            className: 'cosam-time-suffix',
            'aria-hidden': 'true',
          }, timeSplit.suffix));
          timeHeader.appendChild(timeContainer);
        }
        group.appendChild(timeHeader);

        for (const evt of evts) {
          if (this.state._isBreakEvent(evt)) {
            group.appendChild(this._buildBreakBanner(evt));
          } else {
            group.appendChild(this._buildEventCard(evt));
          }
        }
        container.appendChild(group);
      }

      return container;
    }

    _buildBreakBanner(evt) {
      const isOvernight = evt.panelType === '%NB';
      const banner = el('div', {
        className: 'cosam-break-banner' + (isOvernight ? ' cosam-implicit-overnight-break' : ''),
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
        const nameWrapper = el('div', { className: 'cosam-break-name' });
        nameWrapper.appendChild(el('span', { className: 'cosam-implicit-overnight-moon' }, '🌙'));
        nameWrapper.appendChild(document.createTextNode(' ' + evt.name));
        banner.appendChild(nameWrapper);
      } else {
        banner.appendChild(el('div', { className: 'cosam-break-name' }, evt.name));
      }
      if (evt.description) {
        banner.appendChild(el('div', { className: 'cosam-break-desc' }, evt.description));
      }
      const timeStr = formatTimeRange(evt.startTime, evt.endTime);
      if (timeStr) {
        const meta = el('div', { className: 'cosam-break-meta' });
        meta.innerHTML = ICONS.clock + ' ' + escapeHtml(timeStr);
        banner.appendChild(meta);
      }
      return banner;
    }

    _buildEventCard(evt) {
      const isStarred = this.state.starred.has(evt.id);
      const isShared = this.state.sharedStarred.has(evt.id);
      const typeClass = this._panelTypeClass(evt.panelType);
      const card = el('div', {
        className: 'cosam-event-card' + (isStarred ? ' starred' : '') + (isShared ? ' cosam-shared' : ''),
      });

      // Color bar
      if (typeClass) {
        card.appendChild(el('div', {
          className: 'cosam-event-color-bar ' + typeClass,
          'aria-hidden': 'true',
        }));
      }

      // Body
      const body = el('div', { className: 'cosam-event-body' });

      // Title
      body.appendChild(el('div', { className: 'cosam-event-title' }, evt.name));

      // Meta
      const meta = el('div', { className: 'cosam-event-meta' });
      if (evt.startTime) {
        const timeSpan = el('span', { className: 'cosam-meta-time' });
        timeSpan.innerHTML = ICONS.clock + ' ' + escapeHtml(formatTimeRange(evt.startTime, evt.endTime));
        meta.appendChild(timeSpan);
      }
      // Rooms - V5 roomIds array
      if (evt.roomIds && evt.roomIds.length > 0) {
        const roomSpan = el('span', { className: 'cosam-meta-room' });
        roomSpan.innerHTML = ICONS.mappin;
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
          if (i > 0) roomSpan.appendChild(document.createTextNode(', '));
          roomSpan.appendChild(roomElements[i]);
        }
        if (roomElements.length > 0) meta.appendChild(roomSpan);
      }
      if (evt.kind) {
        meta.appendChild(el('span', {}, evt.kind));
      }
      body.appendChild(meta);

      // Badges
      const badges = el('div', { className: 'cosam-event-badges' });
      if (isShared) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-shared', 'aria-label': 'In shared schedule' }, 'Shared'));
      if (evt.isWorkshop) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-workshop' }, 'Workshop'));
      if (evt.cost && evt.isPremium) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-paid' }, evt.cost));
      if (evt.isFull) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-full' }, 'Full'));
      if (evt.isKids) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-kids' }, 'Kids'));
      if (badges.children.length > 0) body.appendChild(badges);

      // Presenters/Credits
      if (evt.credits && evt.credits.length > 0) {
        body.appendChild(el('div', { className: 'cosam-event-presenters' }, evt.credits.join(', ')));
      }

      // Description (hidden, shown on expand)
      if (evt.description) {
        body.appendChild(el('div', { className: 'cosam-event-desc' }, evt.description));
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
      const right = el('div', { className: 'cosam-event-right' });

      // People indicator: shown when event is in any schedule other than (or in
      // addition to) the active one. Count shows only the "other" schedules so
      // it's additive to the star, not double-counting the active schedule.
      const scheduleNames = this.state.schedulesForEvent(evt.id);
      const otherCount = isStarred ? scheduleNames.length - 1 : scheduleNames.length;
      const showPeople = otherCount > 0;
      if (showPeople) {
        const peopleEl = el('div', {
          className: 'cosam-event-people',
          'aria-label': `Also in ${otherCount} other schedule${otherCount === 1 ? '' : 's'}: ${scheduleNames.join(', ')}`,
          title: `Schedules: ${scheduleNames.join(', ')}`,
        });
        peopleEl.innerHTML = ICONS.people + `<span class="cosam-people-count" aria-hidden="true">${otherCount}</span>`;
        right.appendChild(peopleEl);
      }

      const starBtn = el('button', {
        type: 'button',
        className: 'cosam-event-star' + (isStarred ? ' starred' : ''),
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

    _buildGridView(events) {
      const container = el('div', { className: 'cosam-grid-view' });

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
        container.appendChild(el('div', { className: 'cosam-empty' }, 'No rooms to display.'));
        return container;
      }

      // Generate time slots from event start/end times
      const eventTimeKeys = [...new Set(events.flatMap(e => [getTimeSlotKey(e.startTime), getTimeSlotKey(e.endTime)]))].sort();

      const allTimeKeys = eventTimeKeys;

      // Convert to shorter names: weekday number + hour + minute (e.g., t51030 for Friday 10:30 AM)
      const timeSlotMap = {};
      const timeSlots = allTimeKeys.map(key => {
        const date = new Date(key + ':00');
        const dayNum = date.getDay(); // 0=Sunday, 1=Monday, ..., 5=Friday, 6=Saturday
        const hour = date.getHours();
        const minute = date.getMinutes();
        const shortName = `t${dayNum}${hour.toString().padStart(2, '0')}${minute.toString().padStart(2, '0')}`;
        timeSlotMap[key] = shortName;
        return shortName;
      });

      // Create grid template styles
      const gridColumns = `[time] minmax(80px, 120px) ` + roomOrder.map(roomId => `[room-${roomId}] minmax(0, 1fr)`).join(' ');
      const gridRows = `[header] auto ` + timeSlots.map(timeSlot => `[${timeSlot}] minmax(60px, auto)`).join(' ') + ` [footer] auto`;

      // Build CSS grid
      const grid = el('div', {
        className: 'cosam-grid',
        role: 'table',
        'aria-label': 'Schedule grid view',
        style: {
          gridTemplateColumns: gridColumns,
          gridTemplateRows: gridRows
        }
      });

      grid.style.gridTemplateColumns = gridColumns;
      grid.style.gridTemplateRows = gridRows;

      // Add header row
      const header = this._buildGridHeader(roomOrder);
      header.style.gridRow = 'header';
      grid.appendChild(header);

      // Add time slots and events
      const showAllDays = !this.state.activeDay;
      let lastDayKey = null;

      for (let i = 0; i < timeSlots.length; i++) {
        const timeSlot = timeSlots[i];
        const originalKey = allTimeKeys[i];
        const slotEvents = events.filter(e => getTimeSlotKey(e.startTime) === originalKey);
        const slotRegular = slotEvents.filter(e => !this.state._isBreakEvent(e));
        const slotBreaks = slotEvents.filter(e => this.state._isBreakEvent(e));

        // Determine if this is a half-hour (non-on-the-hour) slot
        const slotDate = new Date(originalKey + ':00');
        const isHalfHour = slotDate.getMinutes() !== 0;

        // Day transition handling
        let dayLabel = null;
        if (showAllDays) {
          const dayKey = slotEvents.length > 0
            ? getDayKey(slotEvents[0].startTime)
            : getDayKey(originalKey + ':00');
          if (dayKey !== lastDayKey) {
            const daySource = slotEvents.length > 0 ? slotEvents[0].startTime : originalKey + ':00';
            const eventTime = new Date(daySource);
            dayLabel = eventTime.toLocaleDateString('en-US', { weekday: 'long' });
            lastDayKey = dayKey;
          }
        }

        // Build time header with split time format for aligned display
        const timeHeader = el('div', {
          className: 'cosam-grid-time-header' + (isHalfHour ? ' cosam-grid-time-half' : ''),
          style: {
            gridColumn: 'time',
            gridRow: timeSlot,
          }
        });

        if (dayLabel) {
          timeHeader.appendChild(el('div', { className: 'cosam-grid-day-label' }, dayLabel));
        }

        // Use split time format for accessibility and aligned display
        const timeSource = slotEvents.length > 0 ? slotEvents[0].startTime : originalKey + ':00';
        const timeSplit = formatTimeSplit(timeSource);

        if (timeSplit.isSpecial) {
          // Midnight/Noon - centered
          timeHeader.appendChild(el('div', {
            className: (isHalfHour ? 'cosam-grid-time-minor' : 'cosam-grid-time-major') + ' cosam-grid-time-split cosam-grid-time-special',
            'aria-label': timeSplit.label,
          }, timeSplit.hour));
        } else {
          // Regular time - split display
          const timeContainer = el('div', {
            className: (isHalfHour ? 'cosam-grid-time-minor' : 'cosam-grid-time-major') + ' cosam-grid-time-split',
            'aria-label': timeSplit.label,
          });
          // Screen reader only full time
          timeContainer.appendChild(el('span', { className: 'cosam-sr-only' }, timeSplit.full));
          // Visible hour (right-aligned)
          timeContainer.appendChild(el('span', {
            className: 'cosam-grid-time-hour',
            'aria-hidden': 'true',
          }, timeSplit.hour));
          // Visible suffix (left-aligned: AM/PM or :MM)
          timeContainer.appendChild(el('span', {
            className: 'cosam-grid-time-suffix',
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
                const eventEl = this._buildGridEvent(evt);
                eventEl.style.gridColumn = `room-${roomId}`;

                // Calculate row span for multi-time slot events
                const endTimeSlot = getTimeSlotKey(evt.endTime);
                const endRowIndex = timeSlots.indexOf(endTimeSlot);
                const startRowIndex = timeSlots.indexOf(timeSlot);

                if (endRowIndex > startRowIndex && endRowIndex < timeSlots.length) {
                  // Multi-time slot event - span to end time
                  const endSlotName = timeSlots[endRowIndex];
                  eventEl.style.gridRow = `${timeSlot} / ${endSlotName}`;
                } else {
                  // Single time slot event
                  eventEl.style.gridRow = timeSlot;
                }

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
                const breakEl = this._buildGridBreak(breakEvt);

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

                // Calculate row span for break events
                const endTimeSlot = getTimeSlotKey(breakEvt.endTime);
                const endSlotShortName = timeSlotMap[endTimeSlot];
                const endRowIndex = timeSlots.indexOf(endSlotShortName);
                const startRowIndex = i;

                if (endRowIndex > startRowIndex && endRowIndex < timeSlots.length) {
                  // Multi-time slot break - span to end time
                  const endSlotName = timeSlots[endRowIndex];
                  breakEl.style.gridRow = `${timeSlot} / ${endSlotName}`;
                } else {
                  // Single time slot break
                  breakEl.style.gridRow = timeSlot;
                }

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
              const eventEl = this._buildGridEvent(evt);
              eventEl.style.gridColumn = `room-${roomId}`;

              // Calculate row span for multi-time slot events
              const endTimeSlot = getTimeSlotKey(evt.endTime);
              const endSlotShortName = timeSlotMap[endTimeSlot];
              const endRowIndex = timeSlots.indexOf(endSlotShortName);
              const startRowIndex = i;

              if (endRowIndex > startRowIndex) {
                // Multi-time slot event - span to end time
                const endSlotName = timeSlots[endRowIndex] || timeSlots[timeSlots.length - 1];
                eventEl.style.gridRow = `${timeSlot} / ${endSlotName}`;
              } else {
                // Calculate span based on duration if no exact end time slot found
                const durationMinutes = evt.duration || 60;
                const slotsToSpan = Math.ceil(durationMinutes / 30); // 30-minute slots

                if (slotsToSpan > 1 && startRowIndex + slotsToSpan <= timeSlots.length) {
                  const endSlotName = timeSlots[startRowIndex + slotsToSpan];
                  eventEl.style.gridRow = `${timeSlot} / ${endSlotName}`;
                } else {
                  // Single time slot event
                  eventEl.style.gridRow = timeSlot;
                }
              }

              grid.appendChild(eventEl);
            }
          }
        }
      }

      // Add subtle background gridlines
      // Horizontal row lines at each time slot
      for (const ts of timeSlots) {
        const rowLine = el('div', {
          className: 'cosam-grid-row-line',
          style: {
            gridColumn: `room-${roomOrder[0]} / -1`,
            gridRow: ts
          }
        });
        grid.appendChild(rowLine);
      }
      // Vertical column lines between rooms
      for (let r = 0; r < roomOrder.length - 1; r++) {
        const colLine = el('div', {
          className: 'cosam-grid-col-line',
          style: {
            gridColumn: `room-${roomOrder[r]}`,
            gridRow: `${timeSlots[0]} / footer`
          }
        });
        grid.appendChild(colLine);
      }

      // Add footer row
      const footer = el('div', { className: 'cosam-grid-footer' });
      footer.style.gridRow = 'footer';
      footer.style.gridColumn = '1 / -1'; // Span all columns

      // Add footer content
      const footerContent = el('div', { className: 'cosam-grid-footer-content' });
      let footerText = 'End of Schedule';

      if (this.state.data && this.state.data.meta) {
        const meta = this.state.data.meta;
        let timestamps = [];

        // Add modified time if available
        if (meta.modified) {
          const modDate = new Date(meta.modified);
          const month = modDate.toLocaleDateString('en-US', { month: 'short' });
          const day = modDate.getDate();
          let h = modDate.getHours();
          let m = modDate.getMinutes();
          const ampm = h >= 12 ? 'PM' : 'AM';
          h = h % 12 || 12;
          const timeStr = `${h}:${String(m).padStart(2, '0')} ${ampm}`;
          timestamps.push(`Modified: ${month} ${day} ${timeStr}`);
        }

        // Add generated time if available and different from modified
        if (meta.generated && (!meta.modified || meta.generated !== meta.modified)) {
          const genDate = new Date(meta.generated);
          const month = genDate.toLocaleDateString('en-US', { month: 'short' });
          const day = genDate.getDate();
          let h = genDate.getHours();
          let m = genDate.getMinutes();
          const ampm = h >= 12 ? 'PM' : 'AM';
          h = h % 12 || 12;
          const timeStr = `${h}:${String(m).padStart(2, '0')} ${ampm}`;
          timestamps.push(`Generated: ${month} ${day} ${timeStr}`);
        }

        if (timestamps.length > 0) {
          footerText = timestamps.join(' | ');
        }
      }

      footerContent.textContent = footerText;
      footer.appendChild(footerContent);

      grid.appendChild(footer);

      container.appendChild(grid);
      return container;
    }

    _buildGridHeader(roomOrder) {
      const header = el('div', { className: 'cosam-grid-header' });

      // Time header - centered across split columns
      const timeHeader = el('div', {
        className: 'cosam-grid-header-cell cosam-grid-time-header',
        style: { gridColumn: 'time' }
      });
      const timeLabel = el('div', {
        className: 'cosam-grid-time-split cosam-grid-time-special',
        'aria-label': 'Time column',
      }, 'Time');
      timeHeader.appendChild(timeLabel);
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
          className: 'cosam-grid-header-cell',
          style: { gridColumn: `room-${roomId}` }
        });
        roomHeader.innerHTML = roomDisplay;
        header.appendChild(roomHeader);
      }

      return header;
    }

    _buildGridSleepBreak(columnCount) {
      const sleepBreak = el('div', { className: 'cosam-sleep-break' });
      sleepBreak.appendChild(el('div', { className: 'cosam-sleep-break-icon' }, '🌙'));
      sleepBreak.appendChild(el('div', { className: 'cosam-sleep-break-text' }, 'Overnight Break'));
      return sleepBreak;
    }

    _buildGridBreak(evt) {
      const isOvernight = evt.panelType === '%NB';
      const div = el('div', {
        className: 'cosam-grid-break' + (isOvernight ? ' cosam-implicit-overnight-break' : ''),
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
        const nameWrapper = el('div', { className: 'cosam-grid-break-name' });
        nameWrapper.appendChild(el('span', { className: 'cosam-implicit-overnight-moon' }, '🌙'));
        nameWrapper.appendChild(document.createTextNode(' ' + evt.name));
        div.appendChild(nameWrapper);
      } else {
        div.appendChild(el('div', { className: 'cosam-grid-break-name' }, evt.name));
      }
      if (evt.duration) {
        div.appendChild(el('div', { className: 'cosam-grid-event-time' }, formatDuration(evt.duration)));
      }
      return div;
    }

    _buildGridEvent(evt) {
      const isStarred = this.state.starred.has(evt.id);
      const isShared = this.state.sharedStarred.has(evt.id);
      const typeClass = this._panelTypeClass(evt.panelType);
      const div = el('div', {
        className: 'cosam-grid-event' + (isStarred ? ' starred' : '') + (isShared ? ' cosam-shared' : '') + (typeClass ? (' ' + typeClass) : ''),
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
      const actionsEl = el('div', { className: 'cosam-grid-event-actions' });

      const starEl = el('span', {
        role: 'button',
        tabindex: '0',
        className: 'cosam-grid-event-star' + (isStarred ? ' starred' : ''),
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
          className: 'cosam-grid-event-people',
          'aria-label': `Also in ${otherCount} other schedule${otherCount === 1 ? '' : 's'}: ${scheduleNames.join(', ')}`,
          title: `Schedules: ${scheduleNames.join(', ')}`,
        });
        peopleEl.innerHTML = ICONS.people + `<span class="cosam-people-count" aria-hidden="true">${otherCount}</span>`;
        actionsEl.appendChild(peopleEl);
      }

      div.appendChild(actionsEl);
      div.appendChild(el('div', { className: 'cosam-grid-event-name' }, evt.name));

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
          div.appendChild(el('div', { className: 'cosam-grid-event-room' }, roomNames.join(', ')));
        }
      }

      if (evt.credits && evt.credits.length > 0) {
        div.appendChild(el('div', { className: 'cosam-grid-event-credits' }, evt.credits.join(', ')));
      }

      if (evt.duration) {
        div.appendChild(el('div', { className: 'cosam-grid-event-time' }, formatDuration(evt.duration)));
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

      // Store refs
      this._shareUrlInput = urlInput;
      this._shareFiltersCheckbox = inclFiltersCb;
      this._shareScheduleSelect = scheduleSelect;
      this._shareIncludeScheduleCb = inclSchedCb;
      this._shareScheduleRow = scheduleRow;
      this._shareQrImg = qrImg;
      this._shareQrPlaceholder = qrPlaceholder;

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
      if (evt.cost && evt.isPremium) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-paid' }, evt.cost));
      if (evt.isFull) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-full' }, 'Full'));
      if (evt.isKids) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-kids' }, 'Kids'));
      if (badges.children.length > 0) modal.appendChild(badges);

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

      // Ticket link
      if (evt.ticketUrl) {
        const link = el('a', { href: evt.ticketUrl, target: '_blank', className: 'cosam-btn', style: { textDecoration: 'none', display: 'inline-flex' } }, 'Get Tickets');
        modal.appendChild(el('div', { className: 'cosam-modal-actions' }, link));
      }

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

    // ── Print grid table (repeating headers) ──

    _buildPrintGridTable(events) {
      const regularEvents = events.filter(e => !this.state._isBreakEvent(e));
      const roomIds = [...new Set(regularEvents.flatMap(e => e.roomIds || []).filter(id => id !== null && id !== undefined))];
      const roomOrder = this.state.data.rooms
        .filter(r => roomIds.includes(r.uid || r.id))
        .sort((a, b) => a.sortKey - b.sortKey)
        .map(r => r.uid);
      for (const rid of roomIds) {
        if (!roomOrder.includes(rid)) roomOrder.push(rid);
      }

      if (roomOrder.length === 0) return el('div', {}, 'No rooms to display.');

      // Collect all unique time slot keys, sorted
      const allTimeKeys = [...new Set(events.flatMap(e => [
        getTimeSlotKey(e.startTime), getTimeSlotKey(e.endTime)
      ]).filter(Boolean))].sort();

      const table = el('table', { className: 'cosam-print-grid-table' });
      table.style.cssText = 'width:100%;border-collapse:collapse;table-layout:fixed;font-size:9px;';

      // <colgroup>: fixed time col, equal percentage cols for rooms.
      const colgroup = document.createElement('colgroup');
      const timeCol = document.createElement('col');
      timeCol.style.width = '64px';
      colgroup.appendChild(timeCol);
      for (let i = 0; i < roomOrder.length; i++) {
        const col = document.createElement('col');
        col.style.width = Math.floor(100 / roomOrder.length) + '%';
        colgroup.appendChild(col);
      }
      table.appendChild(colgroup);

      // <thead> repeats on every print page
      const thead = document.createElement('thead');
      const headerRow = document.createElement('tr');
      const thTime = document.createElement('th');
      thTime.textContent = 'Time';
      thTime.style.cssText = 'background:#2a9ec7;color:#fff;padding:3px 6px;text-align:right;font-size:8px;border:1px solid #999;';
      headerRow.appendChild(thTime);
      for (const roomId of roomOrder) {
        const room = this.state.data.rooms.find(r => r.uid === roomId);
        const roomName = room ? (room.longName || room.shortName || 'Room') : 'Room';
        const hotelRoom = room && room.hotelRoom && room.hotelRoom !== roomName ? room.hotelRoom : null;
        const th = document.createElement('th');
        th.style.cssText = 'background:#2a9ec7;color:#fff;padding:3px 4px;text-align:center;font-size:8px;border:1px solid #999;word-break:break-word;overflow:hidden;';
        th.textContent = roomName;
        if (hotelRoom) {
          const sub = el('div', {}, '(' + hotelRoom + ')');
          sub.style.cssText = 'font-size:7px;opacity:0.85;font-weight:400;';
          th.appendChild(sub);
        }
        headerRow.appendChild(th);
      }
      thead.appendChild(headerRow);
      table.appendChild(thead);

      // <tbody>
      const tbody = document.createElement('tbody');

      // Track which cells are already covered by rowspan
      const occupied = {}; // `${rowIdx},${colIdx}` -> true

      for (let rowIdx = 0; rowIdx < allTimeKeys.length; rowIdx++) {
        const key = allTimeKeys[rowIdx];
        const slotEvents = events.filter(e => getTimeSlotKey(e.startTime) === key);
        const slotDate = new Date(key + ':00');
        const isHalfHour = slotDate.getMinutes() !== 0;

        const tr = document.createElement('tr');

        // Time cell
        const tdTime = document.createElement('td');
        tdTime.style.cssText = 'background:#e8f4fa;padding:2px 4px;text-align:right;font-size:' + (isHalfHour ? '7' : '8') + 'px;font-weight:' + (isHalfHour ? '400' : '700') + ';border:1px solid #ccc;vertical-align:top;white-space:nowrap;color:#000;';
        tdTime.textContent = formatTimeGrid(key + ':00');
        tr.appendChild(tdTime);

        // Room cells
        for (let colIdx = 0; colIdx < roomOrder.length; colIdx++) {
          if (occupied[rowIdx + ',' + colIdx]) continue;

          const roomId = roomOrder[colIdx];
          const roomEvts = slotEvents.filter(e =>
            !this.state._isBreakEvent(e) && e.roomIds && e.roomIds.includes(roomId)
          );
          const breakEvts = slotEvents.filter(e => this.state._isBreakEvent(e));

          const td = document.createElement('td');
          td.style.cssText = 'padding:2px 3px;border:1px solid #ccc;vertical-align:top;overflow:hidden;';

          if (roomEvts.length > 0) {
            const evt = roomEvts[0];
            // Calculate rowspan based on event end time
            const endKey = getTimeSlotKey(evt.endTime);
            const endRowIdx = allTimeKeys.indexOf(endKey);
            const rowspan = endRowIdx > rowIdx ? endRowIdx - rowIdx : 1;

            if (rowspan > 1) {
              td.rowSpan = rowspan;
              for (let r = rowIdx; r < rowIdx + rowspan; r++) {
                occupied[r + ',' + colIdx] = true;
              }
            }

            // Apply panel type color as left border
            const typeClass = this._panelTypeClass(evt.panelType);
            const color = typeClass && this._panelTypeColors ? this._panelTypeColors.get(typeClass) : null;
            td.style.borderLeft = '3px solid ' + (color || '#ccc');

            const nameDiv = document.createElement('div');
            nameDiv.style.cssText = 'font-weight:600;font-size:8px;word-break:break-word;color:#000;';
            nameDiv.textContent = evt.name;
            td.appendChild(nameDiv);

            if (evt.credits && evt.credits.length > 0) {
              const credDiv = document.createElement('div');
              credDiv.style.cssText = 'font-size:7px;color:#555;font-style:italic;word-break:break-word;';
              credDiv.textContent = evt.credits.join(', ');
              td.appendChild(credDiv);
            }

            if (evt.duration) {
              const durDiv = document.createElement('div');
              durDiv.style.cssText = 'font-size:7px;color:#555;';
              durDiv.textContent = formatDuration(evt.duration);
              td.appendChild(durDiv);
            }

          } else if (breakEvts.length > 0) {
            const breakEvt = breakEvts[0];
            const endKey = getTimeSlotKey(breakEvt.endTime);
            const endRowIdx = allTimeKeys.indexOf(endKey);
            const rowspan = endRowIdx > rowIdx ? endRowIdx - rowIdx : 1;

            // Break spans from current col to end of row,
            // but only counts columns not already occupied by rowspan events
            let colSpan = 0;
            for (let c = colIdx; c < roomOrder.length; c++) {
              if (!occupied[rowIdx + ',' + c]) colSpan++;
            }

            if (rowspan > 1) {
              td.rowSpan = rowspan;
              for (let r = rowIdx; r < rowIdx + rowspan; r++) {
                for (let c = colIdx; c < roomOrder.length; c++) {
                  occupied[r + ',' + c] = true;
                }
              }
            } else {
              for (let c = colIdx; c < roomOrder.length; c++) {
                occupied[rowIdx + ',' + c] = true;
              }
            }
            if (colSpan === 0) break;
            if (colSpan > 1) td.colSpan = colSpan;

            td.style.cssText = 'background:#f0f0f0;padding:2px 3px;border:1px solid #ccc;vertical-align:middle;text-align:center;overflow:hidden;';
            const nameDiv = document.createElement('div');
            nameDiv.style.cssText = 'font-size:7px;color:#666;';
            nameDiv.textContent = breakEvt.name || '';
            td.appendChild(nameDiv);
            tr.appendChild(td);
            break; // no more cells in this row after the break span

          } else {
            td.style.background = '#fafafa';
          }

          tr.appendChild(td);
        }

        tbody.appendChild(tr);
      }

      table.appendChild(tbody);
      return table;
    }

    // ── Print ──

    _handlePrint() {
      const BREAKPOINT = 750;

      // If window is narrow, show print options modal
      if (window.innerWidth < BREAKPOINT) {
        this._showPrintOptionsModal();
        return;
      }

      // Normal print behavior for wide windows
      this._doPrint();
    }

    _showPrintOptionsModal() {
      const modal = this._modalContent;
      modal.innerHTML = '';

      // Close button
      const closeBtn = el('button', {
        type: 'button',
        className: 'cosam-modal-close',
        'aria-label': 'Close',
        innerHTML: '&times;',
        onClick: () => {
          this._modalOverlay.classList.remove('open');
        },
      });
      modal.appendChild(closeBtn);

      modal.appendChild(el('h2', {}, 'Print Schedule'));

      const optionsContainer = el('div', { className: 'cosam-print-options' });

      const printListBtn = el('button', {
        type: 'button',
        className: 'cosam-btn',
        innerHTML: ICONS.list + ' Print List View',
        onClick: () => {
          this._modalOverlay.classList.remove('open');
          this._doPrint('list');
        },
      });
      optionsContainer.appendChild(printListBtn);

      const printGridBtn = el('button', {
        type: 'button',
        className: 'cosam-btn',
        innerHTML: ICONS.grid + ' Print Grid View',
        onClick: () => {
          this._modalOverlay.classList.remove('open');
          this._doPrint('grid');
        },
      });
      optionsContainer.appendChild(printGridBtn);

      modal.appendChild(optionsContainer);

      this._modalOverlay.classList.add('open');
    }

    _doPrint(viewOverride = null) {
      const wasDay = this.state.activeDay;
      const printContainer = el('div', { className: 'cosam-calendar' });
      printContainer.setAttribute('data-theme', this.state.theme || 'cosam');

      const viewToPrint = viewOverride || this.state.view;

      if (viewToPrint === 'grid') {
        // Grid print: render each day as a table so <thead> repeats on page breaks
        for (const day of this.state.days) {
          this.state.activeDay = day.key;
          const events = this.state.filteredEvents.call(this.state);
          if (events.length === 0) continue;

          const dayLabel = el('div', { className: 'cosam-print-day-label' }, day.label);
          printContainer.appendChild(dayLabel);
          printContainer.appendChild(this._buildPrintGridTable(events));
        }
      } else {
        // List print: all days as list sections
        for (const day of this.state.days) {
          this.state.activeDay = day.key;
          const events = this.state.filteredEvents.call(this.state);
          if (events.length === 0) continue;

          printContainer.appendChild(el('div', { className: 'cosam-print-day-label' }, day.label));
          printContainer.appendChild(this._buildListView(events));
        }
      }

      // Expand all descriptions for print
      printContainer.querySelectorAll('.cosam-event-desc').forEach(d => { d.style.display = 'block'; });

      // Apply panel type color styles to print container (grid event left borders)
      if (this._panelTypeColors) {
        const gridEvents = printContainer.querySelectorAll('.cosam-grid-event');
        for (const event of gridEvents) {
          for (const [cls, color] of this._panelTypeColors) {
            if (event.classList.contains(cls)) {
              event.style.borderLeftColor = color;
              break;
            }
          }
        }
        const colorBars = printContainer.querySelectorAll('.cosam-event-color-bar');
        for (const bar of colorBars) {
          for (const [cls, color] of this._panelTypeColors) {
            if (bar.classList.contains(cls)) {
              bar.style.backgroundColor = color;
              break;
            }
          }
        }
      }

      // Restore state
      this.state.activeDay = wasDay;

      // Open print window
      const printWin = window.open('', '_blank');
      if (!printWin) { window.print(); return; }

      const styles = document.querySelector('link[href*="cosam-calendar"]');
      const styleTag = styles ? `<link rel="stylesheet" href="${styles.href}">` : '';
      const inlineStyles = document.querySelectorAll('style');
      let inlineStyleHtml = '';
      inlineStyles.forEach(s => {
        if (s.textContent.includes('cosam-')) inlineStyleHtml += s.outerHTML;
      });

      // Also grab the CSS from the current page
      const allCSS = Array.from(document.styleSheets).map(sheet => {
        try { return Array.from(sheet.cssRules).map(r => r.cssText).join('\n'); }
        catch (e) { return ''; }
      }).join('\n');

      printWin.document.write(`<!DOCTYPE html><html><head><meta charset="utf-8"><title>Schedule</title>${styleTag}${inlineStyleHtml}<style>${allCSS}
.cosam-event-desc{display:block!important;}</style></head><body>${printContainer.outerHTML}</body></html>`);
      printWin.document.close();
      printWin.focus();
      setTimeout(() => { printWin.print(); }, 500);
    }

  }

  // ── Public API ──────────────────────────────────────────────────────────

  function applyData(rawData, state, renderer, rootEl) {
    const data = renderer._normalizeDataModel(rawData);
    state.data = data;

    // Extract days (skip SPLIT events which are print-layout markers)
    const daySet = new Map();
    const events = data.panels;
    for (const evt of events) {
      if (!evt.startTime) continue;
      // Check for SPLIT events directly here since state._isSplitEvent needs this.state.data
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

  function startFileWatcher() {
    var meta = document.querySelector('meta[name="cosam-generation"]');
    if (!meta) return;
    var currentGeneration = meta.getAttribute('content');
    setInterval(function () {
      fetch(location.href, { cache: 'no-store' })
        .then(function (r) { return r.text(); })
        .then(function (html) {
          var match = html.match(/name="cosam-generation"\s+content="([^"]+)"/);
          if (match && match[1] !== currentGeneration) {
            location.reload();
          }
        })
        .catch(function () { });
    }, 2000);
  }

  window.CosAmCalendar = {
    init: function (opts) {
      const rootEl = typeof opts.el === 'string' ? document.querySelector(opts.el) : opts.el;
      if (!rootEl) { console.error('CosAmCalendar: element not found:', opts.el); return; }

      const state = new CalendarState();
      if (opts.stylePageBody !== undefined) {
        state.stylePageBody = !!opts.stylePageBody;
      }
      const renderer = new CalendarRenderer(rootEl, state);

      // Set up render callback for responsive view changes
      state._renderCallback = () => renderer.render();

      // Show loading
      renderer.render();

      if (opts.watchForChanges) {
        startFileWatcher();
      }

      if (opts.data) {
        applyData(opts.data, state, renderer, rootEl);
        return;
      }

      // Fetch data
      const dataUrl = opts.dataUrl || 'schedule.json';
      fetch(dataUrl)
        .then(r => { if (!r.ok) throw new Error('HTTP ' + r.status); return r.json(); })
        .then(rawData => {
          applyData(rawData, state, renderer, rootEl);
        })
        .catch(err => {
          rootEl.innerHTML = '<div class="cosam-calendar"><div class="cosam-empty">Failed to load schedule: ' + escapeHtml(err.message) + '</div></div>';
        });
    }
  };
})();
