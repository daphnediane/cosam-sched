/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

/**
 * CosAm Calendar Print Format Plugin
 * Advanced print-format system for the CosAm Calendar widget
 * 
 * This plugin provides:
 * - Print format CRUD with localStorage persistence
 * - Time/section splits, brand header/footer, dynamic columns
 * - Typst-style descriptions, web fonts, B&W mode
 */

(function () {
  'use strict';

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

  // Convert a hex color to HSL, pin its lightness to a pastel value, return hex.
  // Fallback for `_pastelTint` on browsers without CSS relative-color syntax;
  // mirrors the same helper in cosam-calendar.js (the two plugins are
  // independent IIFEs and keep their own copies of small utilities).
  function _lightenColor(hex, targetLightness = 0.92) {
    if (!hex || typeof hex !== 'string') return hex;
    hex = hex.replace('#', '');
    if (hex.length !== 6) return '#' + hex;

    const r = parseInt(hex.substring(0, 2), 16) / 255;
    const g = parseInt(hex.substring(2, 4), 16) / 255;
    const b = parseInt(hex.substring(4, 6), 16) / 255;
    const max = Math.max(r, g, b), min = Math.min(r, g, b);
    let h, s; const l = targetLightness;
    if (max === min) {
      h = s = 0;
    } else {
      const d = max - min;
      const mid = (max + min) / 2;
      s = mid > 0.5 ? d / (2 - max - min) : d / (max + min);
      switch (max) {
        case r: h = ((g - b) / d + (g < b ? 6 : 0)) / 6; break;
        case g: h = ((b - r) / d + 2) / 6; break;
        default: h = ((r - g) / d + 4) / 6; break;
      }
    }
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
    const toHex = (x) => Math.round(x * 255).toString(16).padStart(2, '0');
    return '#' + toHex(hue2rgb(p, q, h + 1 / 3)) + toHex(hue2rgb(p, q, h)) + toHex(hue2rgb(p, q, h - 1 / 3));
  }

  let _relColorSupport;
  function _supportsRelativeColor() {
    if (_relColorSupport === undefined) {
      _relColorSupport = typeof CSS !== 'undefined' && !!CSS.supports &&
        CSS.supports('color', 'oklch(from red 0.5 0.1 h)');
    }
    return _relColorSupport;
  }

  // Soft pastel tint of an accent color, matching the Typst `pastel-tint`: keep
  // the hue but pin lightness and chroma in OKLCh. CSS relative-color syntax
  // where available, else the HSL-lightness fallback above.
  function _pastelTint(color, lightness = 0.92, chroma = 0.1) {
    if (!color) return color;
    if (_supportsRelativeColor()) {
      return `oklch(from ${color} ${lightness} ${chroma} h)`;
    }
    return _lightenColor(color, lightness);
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

  function formatTimeRange(start, end) {
    if (!start) return '';
    const s = formatTime(start);
    if (!end) return s;
    return `${s} – ${formatTime(end)}`;
  }

  function getDayKey(isoStr) {
    return isoStr ? isoStr.substring(0, 10) : '';
  }

  function makeCompactDayLabel(isoStr, allDayKeys) {
    if (!isoStr) return 'Unknown';
    const dayStr = isoStr.substring(0, 10);
    const [y, m, d] = dayStr.split('-').map(Number);
    const dt = new Date(y, m - 1, d);
    const weekday = dt.toLocaleDateString('en-US', { weekday: 'long' });
    const keys = (allDayKeys && allDayKeys.length) ? allDayKeys : [dayStr];
    const sorted = [...keys].sort();
    const minK = sorted[0];
    const maxK = sorted[sorted.length - 1];
    const sameWeek = isoWeekKey(minK) === isoWeekKey(maxK);
    const sameMonth = minK.substring(0, 7) === maxK.substring(0, 7);
    if (sameWeek) return weekday;
    if (sameMonth) return `${weekday} ${d}`;
    return `${weekday} ${dt.toLocaleDateString('en-US', { month: 'short' })} ${d}`;
  }

  function isoWeekKey(dayStr) {
    const [y, m, d] = dayStr.split('-').map(Number);
    const dt = new Date(Date.UTC(y, m - 1, d));
    const dayNum = (dt.getUTCDay() + 6) % 7;
    dt.setUTCDate(dt.getUTCDate() - dayNum + 3);
    const firstThursday = new Date(Date.UTC(dt.getUTCFullYear(), 0, 4));
    const week = 1 + Math.round(((dt - firstThursday) / 86400000 - 3 + ((firstThursday.getUTCDay() + 6) % 7)) / 7);
    return `${dt.getUTCFullYear()}-W${week}`;
  }

  function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  // ── Print Format State Management ───────────────────────────────────────────

  class PrintFormatState {
    constructor() {
      this.printFormats = {};
      this.activePrintFormatName = null;
      this._shippedPrintFormats = [];
      this._loadState();
    }

    _storageKey() { return 'cosam-print-formats'; }

    _loadState() {
      try {
        const raw = localStorage.getItem(this._storageKey());
        if (raw) {
          const saved = JSON.parse(raw);
          if (saved.printFormats && typeof saved.printFormats === 'object' && !Array.isArray(saved.printFormats)) {
            this.printFormats = {};
            for (const [name, fmt] of Object.entries(saved.printFormats)) {
              this.printFormats[name] = this._coercePrintFormat(fmt, name);
            }
            this.activePrintFormatName = saved.activePrintFormatName || null;
          }
        }
      } catch (e) { /* ignore */ }
    }

    _saveState() {
      try {
        const state = {
          printFormats: this.printFormats,
          activePrintFormatName: this.activePrintFormatName,
        };
        localStorage.setItem(this._storageKey(), JSON.stringify(state));
      } catch (e) { /* ignore */ }
    }

    _defaultPrintFormatFields() {
      return {
        name: 'Default',
        contentMode: 'gridOnly',
        colorMode: 'color',
        columns: 0,
        headerText: '',
        footerText: '',
        footerMode: 'full',
        logo: 'none',
        pageFill: '',
        cards: false,
        panelFilter: 'all',
        timeSplit: 'none',
        sectionSplit: 'none',
        fonts: { heading: '', banner: '', subheading: '', body: '' },
        fontSizes: { base: '', grid: '', banner: '' },
      };
    }

    _coercePrintFormat(fmt, name) {
      const d = this._defaultPrintFormatFields();
      fmt = fmt || {};
      const f = fmt.fonts || {};
      const fs = fmt.fontSizes || {};
      const validTimeSplits = ['none', 'day', 'half_day', 'timeline'];
      const validSectionSplits = ['none', 'room', 'presenter'];
      return {
        name: (name || fmt.name || d.name),
        contentMode: fmt.contentMode || d.contentMode,
        colorMode: fmt.colorMode === 'bw' ? 'bw' : 'color',
        columns: Number.isFinite(fmt.columns) ? Math.max(0, Math.min(6, fmt.columns)) : d.columns,
        headerText: fmt.headerText || '',
        footerText: fmt.footerText || '',
        footerMode: fmt.footerMode || d.footerMode,
        logo: fmt.logo || d.logo,
        pageFill: fmt.pageFill || '',
        cards: !!fmt.cards,
        panelFilter: fmt.panelFilter || d.panelFilter,
        timeSplit: validTimeSplits.includes(fmt.timeSplit) ? fmt.timeSplit : d.timeSplit,
        sectionSplit: validSectionSplits.includes(fmt.sectionSplit) ? fmt.sectionSplit : d.sectionSplit,
        fonts: {
          heading: f.heading || '', banner: f.banner || '',
          subheading: f.subheading || '', body: f.body || '',
        },
        fontSizes: { base: fs.base || '', grid: fs.grid || '', banner: fs.banner || '' },
      };
    }

    // Convert from SchedulePrintFormat (camelCase from JSON) to internal format
    _fromScheduleFormat(fmt) {
      return this._coercePrintFormat({
        name: fmt.name,
        contentMode: fmt.contentMode,
        colorMode: fmt.colorMode,
        columns: fmt.columns,
        headerText: fmt.headerText,
        footerText: fmt.footerText,
        footerMode: fmt.footerMode,
        logo: fmt.logo,
        pageFill: fmt.pageFill,
        cards: fmt.cards,
        panelFilter: fmt.panelFilter,
        timeSplit: fmt.timeSplit,
        sectionSplit: fmt.sectionSplit,
        fonts: fmt.fonts,
        fontSizes: fmt.fontSizes,
      }, fmt.name);
    }

    seedPrintFormats(shipped) {
      this._shippedPrintFormats = shipped || [];
      if (!this.printFormats || Object.keys(this.printFormats).length === 0) {
        this.printFormats = {};
        const defaults = shipped.length ? shipped : [this._defaultPrintFormatFields()];
        for (const fmt of defaults) {
          // Convert from SchedulePrintFormat (camelCase) to internal format
          const f = this._fromScheduleFormat(fmt);
          this.printFormats[f.name] = f;
        }
      }
      if (!this.activePrintFormatName || !this.printFormats[this.activePrintFormatName]) {
        this.activePrintFormatName = Object.keys(this.printFormats)[0] || null;
      }
    }

    getActivePrintFormat() {
      if (this.activePrintFormatName && this.printFormats[this.activePrintFormatName]) {
        return this.printFormats[this.activePrintFormatName];
      }
      const first = Object.keys(this.printFormats)[0];
      return first ? this.printFormats[first] : this._coercePrintFormat({}, 'Default');
    }

    createPrintFormat(name) {
      const n = (name || '').trim();
      if (!n || this.printFormats[n]) return null;
      const base = this.getActivePrintFormat();
      this.printFormats[n] = this._coercePrintFormat({ ...base, name: n }, n);
      this.activePrintFormatName = n;
      this._saveState();
      return n;
    }

    deletePrintFormat(name) {
      if (!this.printFormats[name] || Object.keys(this.printFormats).length <= 1) return false;
      delete this.printFormats[name];
      if (this.activePrintFormatName === name) {
        this.activePrintFormatName = Object.keys(this.printFormats)[0];
      }
      this._saveState();
      return true;
    }

    renamePrintFormat(oldName, newName) {
      const n = (newName || '').trim();
      if (!n || !this.printFormats[oldName] || (this.printFormats[n] && n !== oldName)) return false;
      const fmt = this.printFormats[oldName];
      fmt.name = n;
      this.printFormats[n] = fmt;
      if (n !== oldName) delete this.printFormats[oldName];
      if (this.activePrintFormatName === oldName) this.activePrintFormatName = n;
      this._saveState();
      return true;
    }

    switchPrintFormat(name) {
      if (!this.printFormats[name]) return;
      this.activePrintFormatName = name;
      this._saveState();
    }

    setPrintFormatColorMode(name, colorMode) {
      if (!this.printFormats[name]) return;
      this.printFormats[name].colorMode = colorMode === 'bw' ? 'bw' : 'color';
      this._saveState();
    }

    updatePrintFormat(name, partial) {
      const cur = this.printFormats[name];
      if (!cur) return false;
      this.printFormats[name] = this._coercePrintFormat({ ...cur, ...partial }, name);
      this._saveState();
      return true;
    }

    resetPrintFormats() {
      this.printFormats = {};
      this.activePrintFormatName = null;
      this.seedPrintFormats(this._shippedPrintFormats);
      this._saveState();
    }
  }

  // ── Print Format Plugin ───────────────────────────────────────────────────

  class PrintFormatPlugin {
    constructor() {
      this.state = new PrintFormatState();
      this.renderer = null;
      this.calendarState = null;
      this._modalOverlay = null;
      this._modalContent = null;
    }

    attach({ renderer, state, ICONS }) {
      this.renderer = renderer;
      this.calendarState = state;
      this.ICONS = ICONS;

      // Seed from shipped defaults if available
      if (state.data && state.data.printFormats) {
        this.state.seedPrintFormats(state.data.printFormats);
      }

      // Create modal overlay
      this._setupModal();
    }

    _setupModal() {
      this._modalOverlay = el('div', { className: 'cosam-modal-overlay' });
      this._modalContent = el('div', { className: 'cosam-modal-content' });
      this._modalOverlay.appendChild(this._modalContent);
      this._modalOverlay.addEventListener('click', (e) => {
        if (e.target === this._modalOverlay) this._modalClose();
      });
      document.body.appendChild(this._modalOverlay);
    }

    _modalClose() {
      this._modalOverlay.classList.remove('open');
    }

    extendToolbar(toolbar, ctx) {
      // Plugin doesn't need to replace the toolbar - it can customize the menu via buildPrintMenu
    }

    buildPrintMenu(originalMenu, ctx) {
      // Replace the built-in menu with our custom format-based menu
      return this._buildPrintMenu(ctx.el);
    }

    _buildPrintMenu(el) {
      const menu = el('div', { className: 'cosam-print-menu', role: 'menu' });

      // Format options
      const formatLabel = el('div', { className: 'cosam-print-menu-label' }, 'Format');
      menu.appendChild(formatLabel);

      const formats = this.state.printFormats || {};
      const activeName = this.state.activePrintFormatName;

      for (const name of Object.keys(formats)) {
        const isActive = name === activeName;
        const item = el('button', {
          type: 'button',
          className: 'cosam-print-menu-item' + (isActive ? ' active' : ''),
          role: 'menuitem',
        });
        const check = el('span', { className: 'cosam-print-menu-check', 'aria-hidden': 'true' }, isActive ? '✓' : '');
        item.append(check, el('span', {}, name));
        item.addEventListener('click', () => {
          this.state.switchPrintFormat(name);
          menu.classList.remove('open');
          // Re-render toolbar to update active state
          if (this.renderer) this.renderer.render();
        });
        menu.appendChild(item);
      }

      menu.appendChild(el('div', { className: 'cosam-print-menu-divider', role: 'separator' }));

      // Color mode options
      const colorLabel = el('div', { className: 'cosam-print-menu-label' }, 'Color Mode');
      menu.appendChild(colorLabel);

      const activeFormat = formats[activeName] || {};
      const isBw = activeFormat.colorMode === 'bw';

      const colorItem = el('button', {
        type: 'button',
        className: 'cosam-print-menu-item' + (!isBw ? ' active' : ''),
        role: 'menuitem',
      });
      const colorCheck = el('span', { className: 'cosam-print-menu-check', 'aria-hidden': 'true' }, !isBw ? '✓' : '');
      colorItem.append(colorCheck, el('span', {}, 'Color'));
      colorItem.addEventListener('click', () => {
        this.state.setPrintFormatColorMode(activeName, 'color');
        menu.classList.remove('open');
        // Re-render toolbar to update active state
        if (this.renderer) this.renderer.render();
      });
      menu.appendChild(colorItem);

      const bwItem = el('button', {
        type: 'button',
        className: 'cosam-print-menu-item' + (isBw ? ' active' : ''),
        role: 'menuitem',
      });
      const bwCheck = el('span', { className: 'cosam-print-menu-check', 'aria-hidden': 'true' }, isBw ? '✓' : '');
      bwItem.append(bwCheck, el('span', {}, 'Black & White'));
      bwItem.addEventListener('click', () => {
        this.state.setPrintFormatColorMode(activeName, 'bw');
        menu.classList.remove('open');
        // Re-render toolbar to update active state
        if (this.renderer) this.renderer.render();
      });
      menu.appendChild(bwItem);

      menu.appendChild(el('div', { className: 'cosam-print-menu-divider', role: 'separator' }));

      // Edit options
      const editItem = el('button', { type: 'button', className: 'cosam-print-menu-item', role: 'menuitem' },
        'Edit "' + activeName + '"…');
      editItem.addEventListener('click', () => { menu.classList.remove('open'); this._showEditPrintFormatModal(); });
      menu.appendChild(editItem);

      const newItem = el('button', { type: 'button', className: 'cosam-print-menu-item', role: 'menuitem' }, '+ New Format');
      newItem.addEventListener('click', () => { menu.classList.remove('open'); this._showNewPrintFormatModal(); });
      menu.appendChild(newItem);

      const renameItem = el('button', { type: 'button', className: 'cosam-print-menu-item', role: 'menuitem' },
        'Rename "' + activeName + '"');
      renameItem.addEventListener('click', () => { menu.classList.remove('open'); this._showRenamePrintFormatModal(); });
      menu.appendChild(renameItem);

      if (Array.isArray(this.state._shippedPrintFormats) && this.state._shippedPrintFormats.length > 0) {
        const resetItem = el('button', { type: 'button', className: 'cosam-print-menu-item', role: 'menuitem' }, 'Reset to defaults');
        resetItem.addEventListener('click', () => { menu.classList.remove('open'); this._showResetPrintFormatsModal(); });
        menu.appendChild(resetItem);
      }

      return menu;
    }

    _togglePrintFormatMenu(toolbar) {
      const existing = toolbar.querySelector('.cosam-print-menu');
      if (existing) {
        existing.remove();
        return;
      }
      const menu = this._buildPrintFormatMenu();
      toolbar.appendChild(menu);
      menu.classList.add('open');
    }

    _buildPrintFormatMenu() {
      const menu = el('div', { className: 'cosam-schedule-menu cosam-print-menu', role: 'menu' });
      const formats = this.state.printFormats || {};
      const activeName = this.state.activePrintFormatName;

      for (const name of Object.keys(formats)) {
        const isActive = name === activeName;
        const item = el('button', {
          type: 'button',
          className: 'cosam-schedule-menu-item' + (isActive ? ' active' : ''),
          role: 'menuitem',
        });
        const check = el('span', { className: 'cosam-schedule-menu-check', 'aria-hidden': 'true' }, isActive ? '✓' : '');
        item.append(check, el('span', {}, name));
        item.addEventListener('click', () => {
          this.state.switchPrintFormat(name);
          menu.classList.remove('open');
          menu.remove();
        });
        menu.appendChild(item);
      }

      menu.appendChild(el('div', { className: 'cosam-schedule-menu-divider', role: 'separator' }));

      const printItem = el('button', { type: 'button', className: 'cosam-schedule-menu-item', role: 'menuitem' },
        'Print "' + activeName + '"');
      printItem.addEventListener('click', () => { menu.classList.remove('open'); menu.remove(); this.print({ renderer: this.renderer, state: this.calendarState }); });
      menu.appendChild(printItem);

      const editItem = el('button', { type: 'button', className: 'cosam-schedule-menu-item', role: 'menuitem' },
        'Edit "' + activeName + '"…');
      editItem.addEventListener('click', () => { menu.classList.remove('open'); menu.remove(); this._showEditPrintFormatModal(); });
      menu.appendChild(editItem);

      const newItem = el('button', { type: 'button', className: 'cosam-schedule-menu-item', role: 'menuitem' }, '+ New Format');
      newItem.addEventListener('click', () => { menu.classList.remove('open'); menu.remove(); this._showNewPrintFormatModal(); });
      menu.appendChild(newItem);

      const renameItem = el('button', { type: 'button', className: 'cosam-schedule-menu-item', role: 'menuitem' },
        'Rename "' + activeName + '"');
      renameItem.addEventListener('click', () => { menu.classList.remove('open'); menu.remove(); this._showRenamePrintFormatModal(); });
      menu.appendChild(renameItem);

      if (Array.isArray(this.state._shippedPrintFormats) && this.state._shippedPrintFormats.length > 0) {
        const resetItem = el('button', { type: 'button', className: 'cosam-schedule-menu-item', role: 'menuitem' }, 'Reset to defaults');
        resetItem.addEventListener('click', () => { menu.classList.remove('open'); menu.remove(); this._showResetPrintFormatsModal(); });
        menu.appendChild(resetItem);
      }

      if (Object.keys(formats).length > 1) {
        menu.appendChild(el('div', { className: 'cosam-schedule-menu-divider', role: 'separator' }));
        const deleteItem = el('button', { type: 'button', className: 'cosam-schedule-menu-item cosam-schedule-menu-danger', role: 'menuitem' },
          'Delete "' + activeName + '"');
        deleteItem.addEventListener('click', () => { menu.classList.remove('open'); menu.remove(); this._showDeletePrintFormatModal(); });
        menu.appendChild(deleteItem);
      }

      return menu;
    }

    _showNewPrintFormatModal() {
      const modal = this._modalContent;
      modal.innerHTML = '';
      modal.appendChild(el('button', { type: 'button', className: 'cosam-modal-close', innerHTML: this.ICONS.x, 'aria-label': 'Close', onClick: () => this._modalClose() }));
      modal.appendChild(el('h2', {}, 'New Print Format'));
      modal.appendChild(el('p', { className: 'cosam-modal-desc' }, 'Creates a copy of the current format that you can edit.'));
      const nameInput = el('input', { type: 'text', className: 'cosam-share-url-input', placeholder: 'Format name...', 'aria-label': 'Format name' });
      modal.appendChild(nameInput);
      const errDiv = el('div', { className: 'cosam-import-status' });
      modal.appendChild(errDiv);
      const createBtn = el('button', {
        type: 'button', className: 'cosam-btn',
        onClick: () => {
          const created = this.state.createPrintFormat(nameInput.value);
          if (!created) { errDiv.textContent = 'That name is already taken or invalid.'; return; }
          this._modalClose();
          this._showEditPrintFormatModal();
        },
      }, 'Create & Edit');
      nameInput.addEventListener('keydown', (e) => { if (e.key === 'Enter') createBtn.click(); });
      modal.appendChild(el('div', { className: 'cosam-modal-actions' }, createBtn));
      this._modalOverlay.classList.add('open');
      nameInput.focus();
    }

    _showRenamePrintFormatModal() {
      const modal = this._modalContent;
      modal.innerHTML = '';
      modal.appendChild(el('button', { type: 'button', className: 'cosam-modal-close', innerHTML: this.ICONS.x, 'aria-label': 'Close', onClick: () => this._modalClose() }));
      modal.appendChild(el('h2', {}, 'Rename Print Format'));
      const nameInput = el('input', { type: 'text', className: 'cosam-share-url-input', value: this.state.activePrintFormatName, 'aria-label': 'New format name' });
      modal.appendChild(nameInput);
      const errDiv = el('div', { className: 'cosam-import-status' });
      modal.appendChild(errDiv);
      const saveBtn = el('button', {
        type: 'button', className: 'cosam-btn',
        onClick: () => {
          if (!this.state.renamePrintFormat(this.state.activePrintFormatName, nameInput.value)) {
            errDiv.textContent = 'That name is already taken or invalid.'; return;
          }
          this._modalClose();
        },
      }, 'Save');
      nameInput.addEventListener('keydown', (e) => { if (e.key === 'Enter') saveBtn.click(); });
      modal.appendChild(el('div', { className: 'cosam-modal-actions' }, saveBtn));
      this._modalOverlay.classList.add('open');
      nameInput.select();
    }

    _showDeletePrintFormatModal() {
      const name = this.state.activePrintFormatName;
      const modal = this._modalContent;
      modal.innerHTML = '';
      modal.appendChild(el('button', { type: 'button', className: 'cosam-modal-close', innerHTML: this.ICONS.x, 'aria-label': 'Close', onClick: () => this._modalClose() }));
      modal.appendChild(el('h2', {}, 'Delete Print Format'));
      modal.appendChild(el('p', { className: 'cosam-modal-desc' }, `Delete "${name}"? This cannot be undone.`));
      const deleteBtn = el('button', {
        type: 'button', className: 'cosam-btn cosam-btn-danger',
        onClick: () => { this.state.deletePrintFormat(name); this._modalClose(); },
      }, 'Delete');
      modal.appendChild(el('div', { className: 'cosam-modal-actions' }, deleteBtn));
      this._modalOverlay.classList.add('open');
    }

    _showResetPrintFormatsModal() {
      const modal = this._modalContent;
      modal.innerHTML = '';
      modal.appendChild(el('button', { type: 'button', className: 'cosam-modal-close', innerHTML: this.ICONS.x, 'aria-label': 'Close', onClick: () => this._modalClose() }));
      modal.appendChild(el('h2', {}, 'Reset Print Formats'));
      modal.appendChild(el('p', { className: 'cosam-modal-desc' }, 'Discard your custom print formats and restore the defaults? This cannot be undone.'));
      const resetBtn = el('button', {
        type: 'button', className: 'cosam-btn cosam-btn-danger',
        onClick: () => { this.state.resetPrintFormats(); this._modalClose(); },
      }, 'Reset to defaults');
      modal.appendChild(el('div', { className: 'cosam-modal-actions' }, resetBtn));
      this._modalOverlay.classList.add('open');
    }

    _showEditPrintFormatModal() {
      const modal = this._modalContent;
      const name = this.state.activePrintFormatName;
      const fmt = this.state.getActivePrintFormat();
      const brand = (this.calendarState.data && this.calendarState.data.brand) || {};
      const brandFonts = Array.isArray(brand.fonts) ? brand.fonts : [];
      const brandLogos = brand.logos || {};
      modal.innerHTML = '';
      modal.appendChild(el('button', { type: 'button', className: 'cosam-modal-close', innerHTML: this.ICONS.x, 'aria-label': 'Close', onClick: () => this._modalClose() }));
      modal.appendChild(el('h2', {}, 'Edit "' + name + '"'));

      const form = el('div', { className: 'cosam-print-form' });
      const row = (labelText, control) => {
        const r = el('label', { className: 'cosam-print-field' });
        r.append(el('span', { className: 'cosam-print-field-label' }, labelText), control);
        return r;
      };
      const select = (options, selected) => {
        const s = el('select', { className: 'cosam-select' });
        for (const [val, lab] of options) {
          const o = el('option', { value: val }, lab);
          if (val === selected) o.selected = true;
          s.appendChild(o);
        }
        return s;
      };
      const fontSelect = (selected) => {
        const opts = [['', 'System default']];
        for (const f of brandFonts) opts.push([f.role, 'Brand ' + f.role + (f.family ? ' (' + f.family + ')' : '')]);
        return select(opts, selected);
      };

      const contentSel = select([
        ['gridOnly', 'Grid only'], ['descriptionOnly', 'Descriptions only'],
        ['both', 'Grid + descriptions'], ['panelList', 'Compact list'],
      ], fmt.contentMode);
      const colorSel = select([['color', 'Color'], ['bw', 'Black & white']], fmt.colorMode);
      const filterSel = select([['all', 'All panels'], ['workshops', 'Workshops only'], ['premium', 'Premium only']], fmt.panelFilter);
      const timeSplitSel = select([
        ['none', 'No time split'], ['day', 'By day'], ['half_day', 'By half-day'], ['timeline', 'By timeline'],
      ], fmt.timeSplit);
      const sectionSplitSel = select([
        ['none', 'No section split'], ['room', 'By room'], ['presenter', 'By presenter'],
      ], fmt.sectionSplit);
      const columnsInput = el('input', { type: 'number', min: '0', max: '6', className: 'cosam-select', value: String(fmt.columns || 0) });
      const cardsInput = el('input', { type: 'checkbox' });
      cardsInput.checked = !!fmt.cards;

      const logoOpts = [['none', 'No logo']];
      for (const alias of Object.keys(brandLogos)) logoOpts.push([alias, alias]);
      const logoSel = select(logoOpts, fmt.logo);
      const headerInput = el('input', { type: 'text', className: 'cosam-select', value: fmt.headerText || '', placeholder: 'e.g. Cosplay America 2026' });
      const footerSel = select([['full', 'Full (timestamps + page #)'], ['timestamp', 'Timestamp only'], ['none', 'None']], fmt.footerMode);
      const footerInput = el('input', { type: 'text', className: 'cosam-select', value: fmt.footerText || '', placeholder: 'Optional footer text' });
      const pageFillInput = el('input', { type: 'text', className: 'cosam-select', value: fmt.pageFill || '', placeholder: 'e.g. #f7f7f7 (blank = white)' });

      const headingFont = fontSelect(fmt.fonts.heading);
      const bannerFont = fontSelect(fmt.fonts.banner);
      const bodyFont = fontSelect(fmt.fonts.body);

      form.append(
        row('Content', contentSel),
        row('Color', colorSel),
        row('Time split', timeSplitSel),
        row('Section split', sectionSplitSel),
        row('Columns (0 = auto)', columnsInput),
        row('Panels', filterSel),
        row('Cards', cardsInput),
        row('Logo', logoSel),
        row('Header text', headerInput),
        row('Footer', footerSel),
        row('Footer text', footerInput),
        row('Page fill', pageFillInput),
      );
      if (brandFonts.length) {
        form.append(
          el('div', { className: 'cosam-print-form-sep' }, 'Fonts'),
          row('Headings', headingFont),
          row('Banner', bannerFont),
          row('Body', bodyFont),
        );
      }
      modal.appendChild(form);

      const saveBtn = el('button', {
        type: 'button', className: 'cosam-btn',
        onClick: () => {
          this.state.updatePrintFormat(name, {
            contentMode: contentSel.value,
            colorMode: colorSel.value,
            timeSplit: timeSplitSel.value,
            sectionSplit: sectionSplitSel.value,
            columns: parseInt(columnsInput.value, 10) || 0,
            panelFilter: filterSel.value,
            cards: cardsInput.checked,
            logo: logoSel.value,
            headerText: headerInput.value,
            footerMode: footerSel.value,
            footerText: footerInput.value,
            pageFill: pageFillInput.value.trim(),
            fonts: {
              heading: headingFont.value,
              banner: bannerFont.value,
              subheading: fmt.fonts.subheading,
              body: bodyFont.value,
            },
          });
          this._modalClose();
        },
      }, 'Save');
      const printBtn = el('button', {
        type: 'button', className: 'cosam-btn cosam-btn-secondary',
        onClick: () => { saveBtn.click(); this.print({ renderer: this.renderer, state: this.calendarState }); },
      }, 'Save & Print');
      modal.appendChild(el('div', { className: 'cosam-modal-actions' }, saveBtn, printBtn));
      this._modalOverlay.classList.add('open');
    }

    print(ctx) {
      const fmt = this.state.getActivePrintFormat();
      const brand = (ctx.state.data && ctx.state.data.brand) || {};

      // Collect all events across days
      let allEvents = [];
      const wasDay = ctx.state.activeDay;
      for (const day of ctx.state.days) {
        ctx.state.activeDay = day.key;
        const dayEvents = ctx.state.filteredEvents.call(ctx.state);
        allEvents = allEvents.concat(dayEvents);
      }
      ctx.state.activeDay = wasDay;

      // Apply panel filter
      allEvents = this._applyPanelFilter(allEvents, fmt.panelFilter);

      // Apply time split grouping
      const timeGroups = this._getTimeSplitGroups(allEvents, fmt.timeSplit);

      const showGrid = fmt.contentMode === 'gridOnly' || fmt.contentMode === 'both';
      const showDesc = fmt.contentMode === 'descriptionOnly' || fmt.contentMode === 'both';
      const showList = fmt.contentMode === 'panelList';

      const printContainer = el('div', { className: 'cosam-calendar cosam-print-root' });
      printContainer.setAttribute('data-theme', ctx.state.theme || 'cosam');
      if (fmt.colorMode === 'bw') printContainer.classList.add('cosam-print-bw');
      if (fmt.cards) printContainer.classList.add('cosam-print-cards');

      for (const group of timeGroups) {
        if (group.events.length === 0) continue;

        const sectionClasses = ['cosam-print-section'];
        if (showGrid) sectionClasses.push('cosam-print-has-grid');
        if (showDesc || showList) sectionClasses.push('cosam-print-has-desc');
        if (showList) sectionClasses.push('cosam-print-compact');
        const section = el('div', { className: sectionClasses.join(' ') });

        const sectionHeader = this._buildPrintHeader(fmt, brand, group.label || null);
        if (sectionHeader) section.appendChild(sectionHeader);

        if (showGrid) {
          const gridEvents = this._stripTrailingBreaks(group.events);
          const gridRegion = el('div', { className: 'cosam-print-grid-region' });
          // Use core's grid renderer with print mode
          gridRegion.appendChild(ctx.renderer._buildGridView(gridEvents, true));
          section.appendChild(gridRegion);
        }

        if (showDesc || showList) {
          const descEvents = showGrid
            ? group.events.filter(e => !ctx.state._isBreakEvent(e))
            : group.events;
          const descWrap = el('div', { className: 'cosam-print-desc-cols' });
          descWrap.appendChild(this._buildPrintTimeGroupedDescriptions(descEvents, group.label));
          section.appendChild(descWrap);
        }

        printContainer.appendChild(section);
      }

      const footer = this._buildPrintFooter(fmt, brand);
      if (footer) printContainer.appendChild(footer);

      // Expand all descriptions for print
      printContainer.querySelectorAll('.cosam-event-desc').forEach(d => { d.style.display = 'block'; });

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

      const allCSS = Array.from(document.styleSheets).map(sheet => {
        try { return Array.from(sheet.cssRules).map(r => r.cssText).join('\n'); }
        catch (e) { return ''; }
      }).join('\n');

      const fontLinks = this._buildPrintFontLinks(fmt, brand);
      const dynamicCss = this._buildPrintCssVars(fmt, brand);

      printWin.document.write(`<!DOCTYPE html><html><head><meta charset="utf-8"><title>Schedule</title>${styleTag}${inlineStyleHtml}${fontLinks}<style>${allCSS}
.cosam-event-desc{display:block!important;}
${dynamicCss}</style></head><body>${printContainer.outerHTML}</body></html>`);
      printWin.document.close();
      printWin.focus();

      let printed = false;
      const fire = () => { if (printed) return; printed = true; try { printWin.focus(); } catch (e) { /* ignore */ } printWin.print(); };
      try {
        if (printWin.document.fonts && printWin.document.fonts.ready && typeof printWin.document.fonts.ready.then === 'function') {
          printWin.document.fonts.ready.then(fire);
        }
      } catch (e) { /* ignore */ }
      setTimeout(fire, 1500);
    }

    _applyPanelFilter(events, panelFilter) {
      if (!panelFilter || panelFilter === 'all') return events;
      const types = (this.calendarState.data && this.calendarState.data.panelTypes) || [];
      const typeById = new Map(types.map(pt => [pt.uid, pt]));
      return events.filter(e => {
        if (panelFilter === 'premium') return !!e.isPremium;
        if (panelFilter === 'workshops') {
          const pt = typeById.get(e.panelType);
          return pt && pt.isWorkshop && !e.isPremium;
        }
        return true;
      });
    }

    _getTimeSplitGroups(events, timeSplit) {
      if (timeSplit === 'none') {
        return [{ label: '', events }];
      }
      const allDayKeys = [...new Set(events
        .filter(e => e.startTime).map(e => getDayKey(e.startTime)))].sort();
      if (timeSplit === 'day') {
        const groups = new Map();
        for (const e of events) {
          if (!e.startTime) continue;
          const dayKey = getDayKey(e.startTime);
          if (!groups.has(dayKey)) {
            groups.set(dayKey, { sortKey: dayKey, label: makeCompactDayLabel(e.startTime, allDayKeys), events: [] });
          }
          groups.get(dayKey).events.push(e);
        }
        return Array.from(groups.values()).sort((a, b) => a.sortKey.localeCompare(b.sortKey));
      }
      if (timeSplit === 'half_day') {
        const groups = new Map();
        for (const e of events) {
          if (!e.startTime) continue;
          const dayKey = getDayKey(e.startTime);
          const hour = parseInt(e.startTime.substring(11, 13), 10) || 0;
          const period = hour < 12 ? 'AM' : 'PM';
          const sortKey = `${dayKey}-${hour < 12 ? '0' : '1'}`;
          const groupKey = `${dayKey}-${period}`;
          if (!groups.has(groupKey)) {
            groups.set(groupKey, { sortKey, label: `${makeCompactDayLabel(e.startTime, allDayKeys)} ${period}`, events: [] });
          }
          groups.get(groupKey).events.push(e);
        }
        return Array.from(groups.values()).sort((a, b) => a.sortKey.localeCompare(b.sortKey));
      }
      if (timeSplit === 'timeline') {
        const groups = new Map();
        let currentLabel = 'Schedule';
        for (const e of events) {
          if (e.isTimeline || (e.name && e.name.includes('SPLIT'))) {
            currentLabel = e.name || 'Schedule';
            continue;
          }
          if (!groups.has(currentLabel)) {
            groups.set(currentLabel, { label: currentLabel, events: [] });
          }
          groups.get(currentLabel).events.push(e);
        }
        return Array.from(groups.values());
      }
      return [{ label: '', events }];
    }

    _stripTrailingBreaks(events) {
      let lastPanelEnd = '';
      for (const e of events) {
        if (this.calendarState._isBreakEvent(e)) continue;
        const end = e.endTime || e.startTime;
        if (end && end > lastPanelEnd) lastPanelEnd = end;
      }
      if (!lastPanelEnd) return events;
      return events.filter(e => {
        if (!this.calendarState._isBreakEvent(e)) return true;
        return e.startTime && e.startTime < lastPanelEnd;
      });
    }

    _buildPrintHeader(fmt, brand, timeLabel = null) {
      const logoUrl = (fmt.logo && fmt.logo !== 'none' && brand.logos) ? brand.logos[fmt.logo] : null;
      const headerText = timeLabel ? (fmt.headerText ? `${fmt.headerText} — ${timeLabel}` : timeLabel) : fmt.headerText;
      if (!logoUrl && !headerText) return null;
      const header = el('div', { className: 'cosam-print-header' });
      if (logoUrl) header.appendChild(el('img', { className: 'cosam-print-header-logo', src: logoUrl, alt: '' }));
      if (headerText) header.appendChild(el('span', { className: 'cosam-print-header-title' }, headerText));
      return header;
    }

    _buildPrintFooter(fmt, brand) {
      if (fmt.footerMode === 'none') return null;
      const footer = el('div', { className: 'cosam-print-footer' });
      const meta = (this.calendarState.data && this.calendarState.data.meta) || {};
      const fmtDate = (s) => { if (!s) return ''; const d = new Date(s); return isNaN(d) ? '' : d.toLocaleString(); };
      const stamps = [];
      if (meta.modified) stamps.push('Updated ' + fmtDate(meta.modified));
      if (meta.generated && meta.generated !== meta.modified) stamps.push('Generated ' + fmtDate(meta.generated));
      if (stamps.length) footer.appendChild(el('span', { className: 'cosam-print-footer-stamp' }, stamps.join(' · ')));
      if (fmt.footerMode === 'full') {
        const site = (brand.meta && (brand.meta.name || brand.meta.siteUrl)) || '';
        if (site) footer.appendChild(el('span', { className: 'cosam-print-footer-site' }, site));
      }
      if (fmt.footerText) footer.appendChild(el('span', { className: 'cosam-print-footer-text' }, fmt.footerText));
      return footer.childNodes.length ? footer : null;
    }

    _buildPrintFontLinks(fmt, brand) {
      const fonts = Array.isArray(brand.fonts) ? brand.fonts : [];
      const byRole = new Map(fonts.map(f => [f.role, f]));
      const urls = new Set();
      for (const role of Object.values(fmt.fonts || {})) {
        if (!role) continue;
        const f = byRole.get(role);
        if (f && f.googleUrl) urls.add(f.googleUrl);
      }
      if (!urls.size) return '';
      let html = '<link rel="preconnect" href="https://fonts.googleapis.com"><link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>';
      for (const u of urls) html += `<link rel="stylesheet" href="${escapeHtml(u)}">`;
      return html;
    }

    _buildPrintCssVars(fmt, brand) {
      const fonts = Array.isArray(brand.fonts) ? brand.fonts : [];
      const byRole = new Map(fonts.map(f => [f.role, f]));
      const fam = (role) => {
        const f = role ? byRole.get(role) : null;
        return f ? `'${f.family.replace(/'/g, '')}'` : '';
      };
      const colors = brand.colors || {};
      const vars = [];
      vars.push(`--cosam-print-page-fill: ${fmt.pageFill || '#ffffff'};`);

      const primary = colors.primary || '#00bcdd';
      vars.push(`--cosam-print-header-bg: ${primary};`);
      vars.push(`--cosam-accent: ${primary};`);
      // Pastel accent tint (matches the grid highlight and the Typst layout).
      // NOTE: --cosam-print-time-col-bg has no consumer after the print-plugin
      // redesign — kept in sync so the time column is correct once rewired.
      const accentLight = _pastelTint(primary);
      vars.push(`--cosam-accent-light: ${accentLight};`);
      vars.push(`--cosam-print-time-col-bg: ${accentLight};`);
      vars.push(`--cosam-print-gridline-color: #d2d2d2;`);
      vars.push(`--cosam-print-empty-bg: #fafafa;`);
      vars.push(`--cosam-print-break-bg: #f0f0f0;`);

      const cols = fmt.columns && fmt.columns > 0 ? fmt.columns : 1;
      vars.push(`--cosam-print-columns: ${cols};`);

      if (fmt.contentMode === 'both') {
        const gridCols = Math.ceil(cols / 2);
        const descCols = Math.max(1, cols - gridCols);
        vars.push(`--cosam-print-grid-cols: ${gridCols};`);
        vars.push(`--cosam-print-desc-cols: ${descCols};`);
      }
      const headingFam = fam(fmt.fonts && fmt.fonts.heading);
      const bannerFam = fam(fmt.fonts && fmt.fonts.banner);
      const bodyFam = fam(fmt.fonts && fmt.fonts.body);
      if (bannerFam) vars.push(`--cosam-print-font-banner: ${bannerFam};`);
      if (headingFam) vars.push(`--cosam-print-font-heading: ${headingFam};`);
      if (bodyFam) vars.push(`--cosam-print-font-body: ${bodyFam};`);
      const fs = fmt.fontSizes || {};
      if (fs.base) vars.push(`--cosam-print-base-pt: ${fs.base};`);
      if (fs.grid) vars.push(`--cosam-print-grid-pt: ${fs.grid};`);
      if (fs.banner) vars.push(`--cosam-print-banner-pt: ${fs.banner};`);

      const landscape = fmt.contentMode !== 'panelList';
      const pageCss = `@page{size:${landscape ? 'landscape' : 'portrait'};margin:0.2in;}`;
      return `${pageCss}.cosam-print-root{${vars.join('')}}`;
    }

    _buildPrintTimeGroupedDescriptions(events, groupLabel) {
      // Group events by time slot for description blocks
      const timeGroups = new Map();
      for (const e of events) {
        if (!e.startTime) continue;
        const timeKey = e.startTime.substring(0, 16); // YYYY-MM-DDTHH:MM
        if (!timeGroups.has(timeKey)) {
          timeGroups.set(timeKey, []);
        }
        timeGroups.get(timeKey).push(e);
      }

      const container = el('div', { className: 'cosam-print-descriptions' });
      for (const [timeKey, timeEvents] of [...timeGroups.entries()].sort()) {
        const timeHeader = el('div', { className: 'cosam-print-desc-time' }, formatTime(timeKey + ':00'));
        container.appendChild(timeHeader);

        for (const evt of timeEvents) {
          const block = el('div', { className: 'cosam-print-desc-block' });

          const titleLine = el('div', { className: 'cosam-print-desc-title' });
          if (evt.credits && evt.credits.length > 0) {
            titleLine.appendChild(el('span', { className: 'cosam-print-desc-presenter' }, evt.credits.join(', ')));
          }
          titleLine.appendChild(el('span', { className: 'cosam-print-desc-name' }, evt.name));

          const metaLine = el('div', { className: 'cosam-print-desc-meta' });
          if (evt.roomIds && evt.roomIds.length > 0) {
            const room = this.calendarState.data.rooms.find(r => r.uid === evt.roomIds[0]);
            if (room) metaLine.appendChild(el('span', {}, room.longName || room.shortName));
          }
          if (evt.startTime) {
            if (metaLine.children.length > 0) metaLine.appendChild(document.createTextNode(' \\ '));
            metaLine.appendChild(el('span', {}, formatTimeRange(evt.startTime, evt.endTime)));
          }

          block.appendChild(titleLine);
          block.appendChild(metaLine);

          if (evt.description) {
            block.appendChild(el('div', { className: 'cosam-print-desc-text' }, evt.description));
          }

          container.appendChild(block);
        }
      }

      return container;
    }
  }

  // ── Export ───────────────────────────────────────────────────────────────

  window.PrintFormatPlugin = PrintFormatPlugin;

})();
