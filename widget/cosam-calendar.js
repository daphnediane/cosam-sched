/**
 * CosAm Calendar Widget
 * Embeddable interactive event calendar for Cosplay America
 * Vanilla JS — no framework dependencies
 * 
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */
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
    clock: '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>',
    mappin: '<svg viewBox="0 0 24 24"><path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z"/><circle cx="12" cy="10" r="3"/></svg>',
  };

  // ── Helpers ──────────────────────────────────────────────────────────────

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
    const ampm = h >= 12 ? 'PM' : 'AM';
    h = h % 12 || 12;
    return m === 0 ? `${h} ${ampm}` : `${h}:${String(m).padStart(2, '0')} ${ampm}`;
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
      this.data = null;
      this.view = 'list'; // 'list' or 'grid'
      this.activeDay = null;
      this.days = [];
      this.starred = new Set();
      this.filters = {
        search: '',
        rooms: new Set(),
        types: new Set(),
        cost: 'all', // 'all', 'free', 'paid', 'workshop'
        presenter: '',
        starredOnly: false,
      };
      this.filtersOpen = false;
      this.modalEvent = null;
      this._loadStarred();
    }

    _storageKey() { return 'cosam-calendar-starred'; }

    _loadStarred() {
      try {
        const raw = localStorage.getItem(this._storageKey());
        if (raw) this.starred = new Set(JSON.parse(raw));
      } catch (e) { /* ignore */ }
      // Also check URL hash
      this._loadFromHash();
    }

    _saveStarred() {
      try {
        localStorage.setItem(this._storageKey(), JSON.stringify([...this.starred]));
      } catch (e) { /* ignore */ }
    }

    _loadFromHash() {
      const hash = window.location.hash;
      if (!hash) return;
      const m = hash.match(/starred=([^&]+)/);
      if (m) {
        const ids = decodeURIComponent(m[1]).split(',').filter(Boolean);
        if (ids.length > 0) {
          for (const id of ids) this.starred.add(id);
          this._saveStarred();
        }
      }
    }

    toggleStar(eventId) {
      if (this.starred.has(eventId)) this.starred.delete(eventId);
      else this.starred.add(eventId);
      this._saveStarred();
    }

    getShareUrl() {
      if (this.starred.size === 0) return window.location.href.split('#')[0];
      const ids = [...this.starred].join(',');
      return window.location.href.split('#')[0] + '#starred=' + encodeURIComponent(ids);
    }

    _isBreakEvent(e) {
      return e.isBreak || e.panelType === 'BREAK';
    }

    _isSplitEvent(e) {
      return e.panelType === 'SPLIT' || e.room === 'SPLIT';
    }

    filteredEvents() {
      if (!this.data) return [];
      let events = this.data.events;

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
        events = events.filter(e => this._isBreakEvent(e) || (e.roomId && this.filters.rooms.has(e.roomId)));
      }

      // Types — breaks excluded when filtering by type
      if (this.filters.types.size > 0) {
        events = events.filter(e => e.panelType && this.filters.types.has(e.panelType));
      }

      // Cost — breaks excluded when filtering by cost
      if (this.filters.cost === 'included') {
        events = events.filter(e => e.isFree);
      } else if (this.filters.cost === 'paid') {
        events = events.filter(e => !e.isFree && !e.isWorkshop);
      } else if (this.filters.cost === 'workshop') {
        events = events.filter(e => e.isWorkshop);
      }

      // Presenter — breaks excluded when filtering by presenter
      if (this.filters.presenter) {
        const p = this.filters.presenter.toLowerCase();
        events = events.filter(e =>
          e.presenters && e.presenters.some(pr => pr.toLowerCase().includes(p))
        );
      }

      // Starred only
      if (this.filters.starredOnly) {
        events = events.filter(e => this.starred.has(e.id));
      }

      return events;
    }
  }

  // ── Renderer ────────────────────────────────────────────────────────────

  class CalendarRenderer {
    constructor(rootEl, state) {
      this.root = rootEl;
      this.state = state;
      this.root.classList.add('cosam-calendar');
    }

    render() {
      this.root.innerHTML = '';
      if (!this.state.data) {
        this.root.appendChild(el('div', { className: 'cosam-loading' }, 'Loading schedule...'));
        return;
      }
      this.root.appendChild(this._buildToolbar());
      this.root.appendChild(this._buildFilters());
      this.root.appendChild(this._buildDayTabs());

      const events = this.state.filteredEvents();
      if (events.length === 0) {
        this.root.appendChild(el('div', { className: 'cosam-empty' }, 'No events match your filters.'));
      } else if (this.state.view === 'grid') {
        this.root.appendChild(this._buildGridView(events));
      } else {
        this.root.appendChild(this._buildListView(events));
      }

      this.root.appendChild(this._buildModal());
    }

    // ── Toolbar ──

    _buildToolbar() {
      const left = el('div', { className: 'cosam-toolbar-left' });

      // View toggles
      const listBtn = el('button', {
        className: 'cosam-btn cosam-btn-icon' + (this.state.view === 'list' ? ' active' : ''),
        title: 'List View',
        innerHTML: ICONS.list,
        onClick: () => { this.state.view = 'list'; this.render(); },
      });
      const gridBtn = el('button', {
        className: 'cosam-btn cosam-btn-icon' + (this.state.view === 'grid' ? ' active' : ''),
        title: 'Grid View',
        innerHTML: ICONS.grid,
        onClick: () => { this.state.view = 'grid'; this.render(); },
      });
      left.append(listBtn, gridBtn);

      // Filter toggle
      const filterBtn = el('button', {
        className: 'cosam-btn' + (this.state.filtersOpen ? ' active' : ''),
        innerHTML: ICONS.filter + ' Filters',
        onClick: () => { this.state.filtersOpen = !this.state.filtersOpen; this.render(); },
      });
      left.appendChild(filterBtn);

      // Starred only toggle
      const starBtn = el('button', {
        className: 'cosam-btn' + (this.state.filters.starredOnly ? ' active' : ''),
        innerHTML: ICONS.star + ' My Schedule',
        onClick: () => { this.state.filters.starredOnly = !this.state.filters.starredOnly; this.render(); },
      });
      left.appendChild(starBtn);

      const right = el('div', { className: 'cosam-toolbar-right' });

      // Search
      const searchWrap = el('div', { className: 'cosam-search-wrap' });
      searchWrap.innerHTML = ICONS.search;
      const searchInput = el('input', {
        type: 'text',
        placeholder: 'Search events...',
        value: this.state.filters.search,
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

      // Share
      const shareBtn = el('button', {
        className: 'cosam-btn cosam-btn-icon',
        title: 'Share starred events',
        innerHTML: ICONS.share,
        onClick: () => {
          const url = this.state.getShareUrl();
          if (navigator.clipboard) {
            navigator.clipboard.writeText(url).then(() => {
              shareBtn.textContent = 'Copied!';
              setTimeout(() => { shareBtn.innerHTML = ICONS.share; }, 1500);
            });
          } else {
            prompt('Copy this URL:', url);
          }
        },
      });
      right.appendChild(shareBtn);

      // Print
      const printBtn = el('button', {
        className: 'cosam-btn cosam-btn-icon',
        title: 'Print schedule',
        innerHTML: ICONS.print,
        onClick: () => this._handlePrint(),
      });
      right.appendChild(printBtn);

      const toolbar = el('div', { className: 'cosam-toolbar' }, left, right);
      return toolbar;
    }

    // ── Filters ──

    _buildFilters() {
      const panel = el('div', { className: 'cosam-filters' + (this.state.filtersOpen ? ' open' : '') });

      // Row 1: Room + Type
      const row1 = el('div', { className: 'cosam-filter-row' });

      // Room filter
      const roomGroup = el('div', { className: 'cosam-filter-group' });
      roomGroup.appendChild(el('label', {}, 'Room'));
      const roomChips = el('div', { className: 'cosam-filter-checkboxes' });
      for (const room of this.state.data.rooms) {
        const name = room.long_name || room.short_name;
        const selected = this.state.filters.rooms.has(room.uid);
        const chip = el('span', {
          className: 'cosam-filter-chip' + (selected ? ' selected' : ''),
          onClick: () => {
            if (this.state.filters.rooms.has(room.uid)) this.state.filters.rooms.delete(room.uid);
            else this.state.filters.rooms.add(room.uid);
            this.render();
          },
        }, name);
        roomChips.appendChild(chip);
      }
      roomGroup.appendChild(roomChips);
      row1.appendChild(roomGroup);

      // Type filter
      const typeGroup = el('div', { className: 'cosam-filter-group' });
      typeGroup.appendChild(el('label', {}, 'Event Type'));
      const typeChips = el('div', { className: 'cosam-filter-checkboxes' });
      for (const pt of this.state.data.panelTypes) {
        if (pt.isBreak || pt.isHidden) continue;
        const selected = this.state.filters.types.has(pt.prefix);
        const chip = el('span', {
          className: 'cosam-filter-chip' + (selected ? ' selected' : ''),
          onClick: () => {
            if (this.state.filters.types.has(pt.prefix)) this.state.filters.types.delete(pt.prefix);
            else this.state.filters.types.add(pt.prefix);
            this.render();
          },
        }, pt.kind || pt.prefix);
        typeChips.appendChild(chip);
      }
      typeGroup.appendChild(typeChips);
      row1.appendChild(typeGroup);
      panel.appendChild(row1);

      // Row 2: Cost + Presenter
      const row2 = el('div', { className: 'cosam-filter-row' });

      // Cost filter
      const costGroup = el('div', { className: 'cosam-filter-group' });
      costGroup.appendChild(el('label', {}, 'Cost'));
      const costChips = el('div', { className: 'cosam-filter-checkboxes' });
      for (const [value, label] of [['all', 'All'], ['included', 'Included'], ['paid', 'Additional Cost'], ['workshop', 'Workshops']]) {
        const selected = this.state.filters.cost === value;
        const chip = el('span', {
          className: 'cosam-filter-chip' + (selected ? ' selected' : ''),
          onClick: () => { this.state.filters.cost = value; this.render(); },
        }, label);
        costChips.appendChild(chip);
      }
      costGroup.appendChild(costChips);
      row2.appendChild(costGroup);

      // Presenter filter
      const presGroup = el('div', { className: 'cosam-filter-group' });
      presGroup.appendChild(el('label', {}, 'Presenter'));
      const presSelect = el('select');
      presSelect.appendChild(el('option', { value: '' }, '— All Presenters —'));
      for (const p of this.state.data.presenters) {
        const opt = el('option', { value: p.name }, p.name);
        if (this.state.filters.presenter === p.name) opt.selected = true;
        presSelect.appendChild(opt);
      }
      presSelect.addEventListener('change', () => {
        this.state.filters.presenter = presSelect.value;
        this.render();
      });
      presGroup.appendChild(presSelect);
      row2.appendChild(presGroup);

      panel.appendChild(row2);

      // Clear filters button
      const actions = el('div', { className: 'cosam-filter-actions' });
      actions.appendChild(el('button', {
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
      return tabs;
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
        let timeLabel = evts[0] ? formatTime(evts[0].startTime) : timeKey;
        if (showAllDays && evts[0]) {
          const dayKey = getDayKey(evts[0].startTime);
          if (dayKey !== lastDayKey) {
            // Add sleep break between days (except for first day)
            if (lastDayKey !== null) {
              const sleepBreak = el('div', { className: 'cosam-sleep-break' });
              sleepBreak.appendChild(el('div', { className: 'cosam-sleep-break-icon' }, '🌙'));
              sleepBreak.appendChild(el('div', { className: 'cosam-sleep-break-text' }, 'Overnight Break'));
              container.appendChild(sleepBreak);
            }
            timeLabel = getDayLabel(evts[0].startTime) + '\n' + timeLabel;
            lastDayKey = dayKey;
          }
        }
        const timeHeader = el('div', { className: 'cosam-time-header' });
        timeHeader.style.whiteSpace = 'pre-line';
        timeHeader.textContent = timeLabel;
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
      const banner = el('div', {
        className: 'cosam-break-banner',
        onClick: () => { this.state.modalEvent = evt; this._showModal(evt); },
      });
      banner.appendChild(el('div', { className: 'cosam-break-name' }, evt.name));
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
      const card = el('div', {
        className: 'cosam-event-card' + (isStarred ? ' starred' : ''),
      });

      // Color bar
      const color = evt.color || this._typeColor(evt.panelType);
      if (color) {
        card.appendChild(el('div', {
          className: 'cosam-event-color-bar',
          style: { backgroundColor: color },
        }));
      }

      // Body
      const body = el('div', { className: 'cosam-event-body' });

      // Title
      body.appendChild(el('div', { className: 'cosam-event-title' }, evt.name));

      // Meta
      const meta = el('div', { className: 'cosam-event-meta' });
      if (evt.startTime) {
        const timeSpan = el('span');
        timeSpan.innerHTML = ICONS.clock + ' ' + escapeHtml(formatTimeRange(evt.startTime, evt.endTime));
        meta.appendChild(timeSpan);
      }
      if (evt.roomId !== null && evt.roomId !== undefined) {
        const room = this.state.data.rooms.find(r => r.uid === evt.roomId);
        if (room) {
          let roomDisplay = room.long_name || room.short_name;
          if (room.hotel_room && room.hotel_room !== (room.long_name || room.short_name)) {
            roomDisplay = `${room.long_name || room.short_name}<br><small style="opacity: 0.8">(${room.hotel_room})</small>`;
          }
          const roomSpan = el('span');
          roomSpan.innerHTML = ICONS.mappin + ' ' + roomDisplay;
          meta.appendChild(roomSpan);
        }
      }
      if (evt.kind) {
        meta.appendChild(el('span', {}, evt.kind));
      }
      body.appendChild(meta);

      // Badges
      const badges = el('div', { className: 'cosam-event-badges' });
      if (evt.isWorkshop) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-workshop' }, 'Workshop'));
      if (evt.cost && evt.cost !== 'TBD' && !evt.isFree) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-paid' }, evt.cost));
      if (evt.isFull) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-full' }, 'Full'));
      if (evt.isKids) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-kids' }, 'Kids'));
      if (badges.children.length > 0) body.appendChild(badges);

      // Presenters
      if (evt.presenters && evt.presenters.length > 0) {
        body.appendChild(el('div', { className: 'cosam-event-presenters' }, evt.presenters.join(', ')));
      }

      // Description (hidden, shown on expand)
      if (evt.description) {
        body.appendChild(el('div', { className: 'cosam-event-desc' }, evt.description));
      }

      // Click to expand / open modal
      body.addEventListener('click', () => {
        this.state.modalEvent = evt;
        this._showModal(evt);
      });

      card.appendChild(body);

      // Star button
      const starBtn = el('button', {
        className: 'cosam-event-star' + (isStarred ? ' starred' : ''),
        innerHTML: ICONS.star,
        title: isStarred ? 'Remove from My Schedule' : 'Add to My Schedule',
        onClick: (e) => {
          e.stopPropagation();
          this.state.toggleStar(evt.id);
          this.render();
        },
      });
      card.appendChild(starBtn);

      return card;
    }

    // ── Grid View ──

    _buildGridView(events) {
      console.log('=== BUILD GRID VIEW STARTED ===');
      console.log('Total events:', events.length);

      const container = el('div', { className: 'cosam-grid-view' });

      // Separate break events from regular events
      const regularEvents = events.filter(e => !this.state._isBreakEvent(e));
      const breakEvents = events.filter(e => this.state._isBreakEvent(e));

      // Get visible rooms from regular events only (BREAK room excluded)
      const roomIds = [...new Set(regularEvents.map(e => e.roomId).filter(id => id !== null && id !== undefined))];
      const roomOrder = this.state.data.rooms
        .filter(r => roomIds.includes(r.uid || r.id))
        .sort((a, b) => (a.sort_key || a.sortKey) - (b.sort_key || b.sortKey))
        .map(r => r.uid || r.id);

      // Add any rooms not in the rooms list
      for (const rid of roomIds) {
        if (!roomOrder.includes(rid)) roomOrder.push(rid);
      }

      if (roomOrder.length === 0) {
        container.appendChild(el('div', { className: 'cosam-empty' }, 'No rooms to display.'));
        return container;
      }

      // Generate time slots - include all event start/end times plus key transition points
      const eventTimeKeys = [...new Set(events.flatMap(e => [getTimeSlotKey(e.startTime), getTimeSlotKey(e.endTime)]))].sort();

      // Add important transition times (end of days) to ensure overnight breaks work
      const transitionKeys = new Set();
      const dayGroups = {};

      // Group events by day
      for (const key of eventTimeKeys) {
        const dayKey = getDayKey(key + ':00');
        if (!dayGroups[dayKey]) dayGroups[dayKey] = [];
        dayGroups[dayKey].push(key);
      }

      // For each day, add the last event time as a transition point
      for (const dayKey in dayGroups) {
        const dayTimes = dayGroups[dayKey].sort();
        if (dayTimes.length > 0) {
          const lastTime = dayTimes[dayTimes.length - 1];
          transitionKeys.add(lastTime);
        }
      }

      // Combine all time keys
      const allTimeKeys = [...new Set([...eventTimeKeys, ...transitionKeys])].sort();

      console.log('Event time keys:', eventTimeKeys);
      console.log('Transition keys:', Array.from(transitionKeys));
      console.log('All time keys:', allTimeKeys);

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
      const gridColumns = `[time] minmax(80px, 120px) ` + roomOrder.map(roomId => `[room-${roomId}] 1fr`).join(' ');
      const gridRows = `[header] auto ` + timeSlots.map(timeSlot => `[${timeSlot}] minmax(60px, auto)`).join(' ') + ` [footer] auto`;

      console.log('Final gridRows string:', gridRows);
      console.log('gridRows length:', gridRows.length);
      console.log('timeSlots length:', timeSlots.length);
      console.log('First few time slots:', timeSlots.slice(0, 10));
      console.log('Last few time slots:', timeSlots.slice(-10));

      console.log('Generated grid columns:', gridColumns);
      console.log('Generated grid rows:', gridRows);
      console.log('Time slots for rows:', timeSlots);

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

      console.log('Applied grid styles - columns:', grid.style.gridTemplateColumns);
      console.log('Applied grid styles - rows:', grid.style.gridTemplateRows);

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

        console.log('Time slot:', timeSlot);
        console.log('Regular events:', slotRegular.map(e => e.name));
        console.log('Break events:', slotBreaks.map(e => e.name));

        let timeLabel = slotEvents.length > 0 ? formatTime(slotEvents[0].startTime) : '';
        if (showAllDays && slotEvents.length > 0) {
          const dayKey = getDayKey(slotEvents[0].startTime);
          console.log('Processing time slot:', timeSlot, 'with day:', dayKey, 'lastDayKey:', lastDayKey);
          if (dayKey !== lastDayKey) {
            console.log('Day transition detected from', lastDayKey, 'to', dayKey);
            // Add sleep break row between days (except for first day)
            if (lastDayKey !== null) {
              // Find the last time slot of the previous day
              let lastSlotIndex = i - 1;
              console.log('Looking for last slot of previous day:', lastDayKey, 'starting from index:', lastSlotIndex);
              while (lastSlotIndex >= 0) {
                const prevKey = allTimeKeys[lastSlotIndex];
                const prevDayKey = getDayKey(prevKey + ':00');
                console.log('Checking slot', lastSlotIndex, 'key:', prevKey, 'day:', prevDayKey);
                if (prevDayKey === lastDayKey) {
                  console.log('Found last slot of previous day at index:', lastSlotIndex, 'slot:', timeSlots[lastSlotIndex]);
                  break;
                }
                lastSlotIndex--;
              }

              if (lastSlotIndex >= 0) {
                // Create sleep break that spans from last slot of previous day to current slot
                const sleepBreak = this._buildGridSleepBreak(roomOrder.length + 1);
                sleepBreak.style.gridColumn = `room-${roomOrder[0]} / -1`; // Span only room columns, not time column
                sleepBreak.style.gridRow = `${timeSlots[lastSlotIndex]} / ${timeSlot}`;
                console.log('Creating sleep break with gridRow:', `${timeSlots[lastSlotIndex]} / ${timeSlot}`);
                grid.appendChild(sleepBreak);
              } else {
                console.log('Could not find last slot for previous day:', lastDayKey);
              }
            }
            timeLabel = getDayLabel(slotEvents[0].startTime) + '\n' + timeLabel;
            lastDayKey = dayKey;
          }
        }

        // Add time slot header
        const timeHeader = el('div', {
          className: 'cosam-grid-time-header',
          style: {
            gridColumn: 'time',
            gridRow: timeSlot,
            whiteSpace: 'pre-line'
          }
        });

        // Always show time label, even if no events start at this time
        if (slotEvents.length > 0) {
          timeLabel = formatTime(slotEvents[0].startTime);
        } else {
          // For transition slots without events, format the time from the original key
          const originalKey = allTimeKeys[i];
          const date = new Date(originalKey + ':00');
          timeLabel = formatTime(date.toISOString());
        }

        if (showAllDays && slotEvents.length > 0) {
          const dayKey = getDayKey(slotEvents[0].startTime);
          if (dayKey !== lastDayKey) {
            console.log('Day transition detected from', lastDayKey, 'to', dayKey);
            // Add sleep break row between days (except for first day)
            if (lastDayKey !== null) {
              // Find the last time slot of the previous day
              let lastSlotIndex = i - 1;
              console.log('Looking for last slot of previous day:', lastDayKey, 'starting from index:', lastSlotIndex);
              while (lastSlotIndex >= 0) {
                const prevKey = allTimeKeys[lastSlotIndex];
                const prevDayKey = getDayKey(prevKey + ':00');
                console.log('Checking slot', lastSlotIndex, 'key:', prevKey, 'day:', prevDayKey);
                if (prevDayKey === lastDayKey) {
                  console.log('Found last slot of previous day at index:', lastSlotIndex, 'slot:', timeSlots[lastSlotIndex]);
                  break;
                }
                lastSlotIndex--;
              }

              if (lastSlotIndex >= 0) {
                // Create sleep break that spans from last slot of previous day to current slot
                const sleepBreak = this._buildGridSleepBreak(roomOrder.length + 1);
                sleepBreak.style.gridColumn = `room-${roomOrder[0]} / -1`; // Span only room columns, not time column
                sleepBreak.style.gridRow = `${timeSlots[lastSlotIndex]} / ${timeSlot}`;
                console.log('Creating sleep break with gridRow:', `${timeSlots[lastSlotIndex]} / ${timeSlot}`);
                grid.appendChild(sleepBreak);
              } else {
                console.log('Could not find last slot for previous day:', lastDayKey);
              }
            }
            timeLabel = getDayLabel(slotEvents[0].startTime) + '\n' + timeLabel;
            lastDayKey = dayKey;
          }
        }

        timeHeader.textContent = timeLabel;
        grid.appendChild(timeHeader);

        // Add events for each room
        if (slotBreaks.length > 0) {
          // Determine which rooms have real events at this time
          const occupiedRoomIds = new Set(slotRegular.map(e => e.roomId).filter(id => id !== null && id !== undefined));

          // Build cells: span across unoccupied rooms, show real events in occupied rooms
          let i = 0;
          while (i < roomOrder.length) {
            const roomId = roomOrder[i];
            if (occupiedRoomIds.has(roomId)) {
              // Room has a real event — render it normally
              const roomEvents = slotRegular.filter(e => e.roomId === roomId);
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
          console.log('Processing normal row for time slot:', timeSlot);
          for (const roomId of roomOrder) {
            const roomEvents = slotRegular.filter(e => e.roomId === roomId);
            console.log('Room', roomId, 'events:', roomEvents.map(e => e.name));
            for (const evt of roomEvents) {
              const eventEl = this._buildGridEvent(evt);
              eventEl.style.gridColumn = `room-${roomId}`;

              // Calculate row span for multi-time slot events
              const endTimeSlot = getTimeSlotKey(evt.endTime);
              const endSlotShortName = timeSlotMap[endTimeSlot];
              const endRowIndex = timeSlots.indexOf(endSlotShortName);
              const startRowIndex = i;

              console.log('Event:', evt.name);
              console.log('Start time:', timeSlot, 'Start index:', startRowIndex);
              console.log('End time:', endTimeSlot, 'End index:', endRowIndex);
              console.log('Duration:', evt.duration, 'minutes');
              console.log('Available time slots:', timeSlots);

              if (endRowIndex > startRowIndex) {
                // Multi-time slot event - span to end time
                const endSlotName = timeSlots[endRowIndex] || timeSlots[timeSlots.length - 1];
                console.log('End slot name:', endSlotName);
                eventEl.style.gridRow = `${timeSlot} / ${endSlotName}`;
                console.log('Applied gridRow (exact):', eventEl.style.gridRow);
              } else {
                // Calculate span based on duration if no exact end time slot found
                const durationMinutes = evt.duration || 60;
                const slotsToSpan = Math.ceil(durationMinutes / 30); // 30-minute slots
                console.log('Calculated slots to span:', slotsToSpan);

                if (slotsToSpan > 1 && startRowIndex + slotsToSpan <= timeSlots.length) {
                  const endSlotName = timeSlots[startRowIndex + slotsToSpan];
                  eventEl.style.gridRow = `${timeSlot} / ${endSlotName}`;
                  console.log('Applied gridRow (duration):', eventEl.style.gridRow);
                } else {
                  // Single time slot event
                  eventEl.style.gridRow = timeSlot;
                  console.log('Applied gridRow (single):', eventEl.style.gridRow);
                }
              }

              console.log('Final event element styles:', eventEl.style.cssText);

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
      footerContent.textContent = 'End of Schedule';
      footer.appendChild(footerContent);

      grid.appendChild(footer);

      container.appendChild(grid);
      return container;
    }

    _buildGridHeader(roomOrder) {
      const header = el('div', { className: 'cosam-grid-header' });

      // Time header
      const timeHeader = el('div', {
        className: 'cosam-grid-header-cell cosam-grid-time-header',
        style: { gridColumn: 'time' }
      });
      timeHeader.textContent = 'Time';
      header.appendChild(timeHeader);

      // Room headers
      for (const roomId of roomOrder) {
        const room = this.state.data.rooms.find(r => (r.uid || r.id) === roomId);
        let roomDisplay = room ? (room.long_name || room.longName || room.short_name || room.shortName) : 'Unknown';
        if (room && room.hotel_room && room.hotel_room !== (room.long_name || room.longName || room.short_name || room.shortName)) {
          roomDisplay = `${room.long_name || room.longName || room.short_name || room.shortName}<br><small style="opacity: 0.8">(${room.hotel_room})</small>`;
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
      const div = el('div', {
        className: 'cosam-grid-break',
        onClick: () => { this.state.modalEvent = evt; this._showModal(evt); },
      });
      div.appendChild(el('div', { className: 'cosam-grid-break-name' }, evt.name));
      if (evt.duration) {
        div.appendChild(el('div', { className: 'cosam-grid-event-time' }, evt.duration + ' min'));
      }
      return div;
    }

    _buildGridEvent(evt) {
      const isStarred = this.state.starred.has(evt.id);
      const color = evt.color || this._typeColor(evt.panelType);
      const div = el('div', {
        className: 'cosam-grid-event' + (isStarred ? ' starred' : ''),
        style: { backgroundColor: '#ffffff', borderLeftColor: color || 'transparent' },
        onClick: () => { this.state.modalEvent = evt; this._showModal(evt); },
      });

      div.appendChild(el('div', { className: 'cosam-grid-event-name' }, evt.name));

      // Add room information for mobile view
      if (evt.roomId !== null && evt.roomId !== undefined) {
        const room = this.state.data.rooms.find(r => (r.uid || r.id) === evt.roomId);
        if (room) {
          let roomDisplay = room.long_name || room.longName || room.short_name || room.shortName;
          if (room.hotel_room && room.hotel_room !== (room.long_name || room.longName || room.short_name || room.shortName)) {
            roomDisplay = `${room.long_name || room.longName || room.short_name || room.shortName} (${room.hotel_room})`;
          }
          div.appendChild(el('div', { className: 'cosam-grid-event-room' }, roomDisplay));
        }
      }

      if (evt.duration) {
        div.appendChild(el('div', { className: 'cosam-grid-event-time' }, evt.duration + ' min'));
      }

      // Star indicator
      const starEl = el('span', {
        className: 'cosam-grid-event-star' + (isStarred ? ' starred' : ''),
        innerHTML: ICONS.star,
        onClick: (e) => {
          e.stopPropagation();
          this.state.toggleStar(evt.id);
          this.render();
        },
      });
      div.appendChild(starEl);

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

    _showModal(evt) {
      const modal = this._modalContent;
      modal.innerHTML = '';

      // Close button
      modal.appendChild(el('button', {
        className: 'cosam-modal-close',
        innerHTML: ICONS.x,
        onClick: () => this._modalOverlay.classList.remove('open'),
      }));

      // Title
      modal.appendChild(el('h2', {}, evt.name));

      // Meta
      const meta = el('div', { className: 'cosam-modal-meta' });
      if (evt.startTime) {
        const ts = el('span');
        ts.innerHTML = ICONS.clock + ' ' + escapeHtml(formatTimeRange(evt.startTime, evt.endTime));
        meta.appendChild(ts);
      }
      if (evt.duration) {
        meta.appendChild(el('span', {}, evt.duration + ' min'));
      }
      if (evt.roomId !== null && evt.roomId !== undefined) {
        const room = this.state.data.rooms.find(r => r.uid === evt.roomId);
        if (room) {
          let roomDisplay = room.long_name || room.short_name;
          if (room.hotel_room && room.hotel_room !== (room.long_name || room.short_name)) {
            roomDisplay = `${room.long_name || room.short_name}<br><small style="opacity: 0.8">(${room.hotel_room})</small>`;
          }
          const rs = el('span');
          rs.innerHTML = ICONS.mappin + ' ' + roomDisplay;
          meta.appendChild(rs);
        }
      }
      if (evt.kind) {
        meta.appendChild(el('span', {}, evt.kind));
      }
      modal.appendChild(meta);

      // Badges
      const badges = el('div', { className: 'cosam-event-badges' });
      if (evt.isWorkshop) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-workshop' }, 'Workshop'));
      if (evt.cost && evt.cost !== 'TBD' && !evt.isFree) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-paid' }, evt.cost));
      if (evt.isFull) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-full' }, 'Full'));
      if (evt.isKids) badges.appendChild(el('span', { className: 'cosam-badge cosam-badge-kids' }, 'Kids'));
      if (badges.children.length > 0) modal.appendChild(badges);

      // Description
      if (evt.description) {
        modal.appendChild(el('div', { className: 'cosam-modal-desc' }, evt.description));
      }

      // Presenters
      if (evt.presenters && evt.presenters.length > 0) {
        modal.appendChild(el('div', { className: 'cosam-modal-presenters' }, 'Presenters: ' + evt.presenters.join(', ')));
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

      // Star action
      const isStarred = this.state.starred.has(evt.id);
      const starBtn = el('button', {
        className: 'cosam-btn' + (isStarred ? ' active' : ''),
        innerHTML: ICONS.star + (isStarred ? ' Remove from My Schedule' : ' Add to My Schedule'),
        onClick: () => {
          this.state.toggleStar(evt.id);
          this._showModal(evt); // re-render modal
        },
      });
      const existingActions = modal.querySelector('.cosam-modal-actions');
      if (existingActions) {
        existingActions.appendChild(starBtn);
      } else {
        modal.appendChild(el('div', { className: 'cosam-modal-actions' }, starBtn));
      }

      this._modalOverlay.classList.add('open');
    }

    // ── Print ──

    _handlePrint() {
      // If starred-only is on, print only starred events
      const wasStarredOnly = this.state.filters.starredOnly;
      const wasDay = this.state.activeDay;

      // Show all days for print
      this.state.activeDay = null;

      // Build print content
      const printContainer = el('div', { className: 'cosam-calendar' });

      for (const day of this.state.days) {
        this.state.activeDay = day.key;
        const events = this.state.filteredEvents();
        if (events.length === 0) continue;

        printContainer.appendChild(el('div', { className: 'cosam-print-day-label' }, day.label));
        printContainer.appendChild(this._buildListView(events));
      }

      // Expand all descriptions for print
      printContainer.querySelectorAll('.cosam-event-desc').forEach(d => { d.style.display = 'block'; });

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

      printWin.document.write(`<!DOCTYPE html><html><head><meta charset="utf-8"><title>Schedule</title>${styleTag}${inlineStyleHtml}<style>${allCSS}\n.cosam-event-desc{display:block!important;}</style></head><body>${printContainer.outerHTML}</body></html>`);
      printWin.document.close();
      printWin.focus();
      setTimeout(() => { printWin.print(); }, 500);
    }

    // ── Color helpers ──

    _typeColor(prefix) {
      if (!prefix || !this.state.data) return null;
      const pt = this.state.data.panelTypes.find(t => t.prefix === prefix);
      return pt ? pt.color : null;
    }
  }

  // ── Public API ──────────────────────────────────────────────────────────

  window.CosAmCalendar = {
    init: function (opts) {
      const rootEl = typeof opts.el === 'string' ? document.querySelector(opts.el) : opts.el;
      if (!rootEl) { console.error('CosAmCalendar: element not found:', opts.el); return; }

      const state = new CalendarState();
      const renderer = new CalendarRenderer(rootEl, state);

      // Show loading
      renderer.render();

      // Fetch data
      const dataUrl = opts.dataUrl || 'schedule.json';
      fetch(dataUrl)
        .then(r => { if (!r.ok) throw new Error('HTTP ' + r.status); return r.json(); })
        .then(data => {
          console.log('=== RAW DATA LOADED ===');
          console.log('Total events in data:', data.events?.length || 0);
          console.log('Event names:', data.events?.map(e => e.name) || []);

          state.data = data;

          // Extract days (skip SPLIT events which are print-layout markers)
          const daySet = new Map();
          for (const evt of data.events) {
            if (!evt.startTime) continue;
            if (state._isSplitEvent(evt)) continue;
            const key = getDayKey(evt.startTime);
            if (!daySet.has(key)) {
              daySet.set(key, getDayLabel(evt.startTime));
            }
          }
          state.days = [...daySet.entries()].sort((a, b) => a[0].localeCompare(b[0])).map(([key, label]) => ({ key, label }));

          // Default to first day
          if (state.days.length > 0) {
            state.activeDay = state.days[0].key;
          }

          // Default view: grid on desktop, list on mobile
          state.view = window.innerWidth >= 768 ? 'grid' : 'list';

          renderer.render();
        })
        .catch(err => {
          rootEl.innerHTML = '<div class="cosam-calendar"><div class="cosam-empty">Failed to load schedule: ' + escapeHtml(err.message) + '</div></div>';
        });
    }
  };
})();
