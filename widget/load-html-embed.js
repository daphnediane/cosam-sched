// CosAm Calendar — HTML embed loader
// Reads schedule data from the widget-html format:
//   - Structural JSON from <script id="cosam-schedule-data" data-cosam="schedule">
//   - Panel list from <article class="cosam-panel"> inside .cosam-static-schedule
// Exposes CosAmCalendar.HtmlEmbedLoader.
//
// Copyright (c) 2026 Daphne Pfister | BSD-2-Clause

window.CosAmCalendar = window.CosAmCalendar || {};

function _parsePanelElement(el) {
  var d = el.dataset;
  var nameEl = el.querySelector('.cosam-panel-name');
  var panel = {
    id: d.id || '',
    baseId: d.baseId || '',
    name: nameEl ? nameEl.textContent.trim() : '',
    duration: parseInt(d.duration || '0', 10) || 0,
    isPremium: d.isPremium === 'true',
    isFull: d.isFull === 'true',
    isKids: d.isKids === 'true',
    roomIds: d.roomIds
      ? d.roomIds.split(' ').map(function (s) { return parseInt(s, 10); }).filter(function (n) { return !isNaN(n); })
      : [],
  };

  if (d.panelType) panel.panelType = d.panelType;
  if (d.startTime) panel.startTime = d.startTime;
  if (d.endTime) panel.endTime = d.endTime;
  // FEATURE-154: canonical epoch-seconds times (widget format v2). The
  // normalizer derives wall-clock display times from these when present.
  if (d.startEpoch !== undefined) panel.startEpoch = parseInt(d.startEpoch, 10);
  if (d.endEpoch !== undefined) panel.endEpoch = parseInt(d.endEpoch, 10);
  if (d.partNum !== undefined) panel.partNum = parseInt(d.partNum, 10);
  if (d.sessionNum !== undefined) panel.sessionNum = parseInt(d.sessionNum, 10);
  if (d.totalParts !== undefined) panel.totalParts = parseInt(d.totalParts, 10);
  if (d.isSeriesLead === 'true') panel.isSeriesLead = true;
  if (d.cost !== undefined) panel.cost = d.cost;
  if (d.capacity !== undefined) panel.capacity = d.capacity;
  if (d.difficulty !== undefined) panel.difficulty = d.difficulty;
  if (d.ticketUrl !== undefined) panel.ticketUrl = d.ticketUrl;

  var descEl = el.querySelector('.cosam-panel-desc');
  if (descEl) panel.description = descEl.textContent.trim();

  var noteEl = el.querySelector('.cosam-panel-note');
  if (noteEl) panel.note = noteEl.textContent.trim();

  var prereqEl = el.querySelector('.cosam-panel-prereq');
  if (prereqEl) panel.prereq = prereqEl.textContent.trim();

  var creditEls = el.querySelectorAll('.cosam-panel-credits li');
  panel.credits = Array.prototype.map.call(creditEls, function (li) {
    return li.textContent.trim();
  }).filter(Boolean);

  return panel;
}

/**
 * Factory for the widget-html embed loader.
 * @param {object} [opts]
 * @param {string} [opts.dataId='cosam-schedule-data'] - ID of the structural JSON script element.
 * @param {string} [opts.panelSelector='.cosam-static-schedule article.cosam-panel'] - Selector for panel articles.
 */
window.CosAmCalendar.HtmlEmbedLoader = function (opts) {
  var dataId = (opts && opts.dataId) || 'cosam-schedule-data';
  var panelSelector = (opts && opts.panelSelector) || '.cosam-static-schedule article.cosam-panel';

  return {
    load: function (rootEl) {
      var scriptEl = document.getElementById(dataId);
      if (!scriptEl || scriptEl.getAttribute('data-cosam') !== 'schedule') {
        return Promise.reject(new Error('No widget-html schedule data found at #' + dataId));
      }
      var structural;
      try {
        structural = JSON.parse(scriptEl.textContent.trim());
      } catch (e) {
        return Promise.reject(new Error('Failed to parse schedule JSON: ' + e.message));
      }
      var panelEls = document.querySelectorAll(panelSelector);
      var panels = Array.prototype.map.call(panelEls, _parsePanelElement);
      return Promise.resolve({
        meta: structural.meta,
        rooms: structural.rooms,
        panelTypes: structural.panelTypes,
        timeline: structural.timeline,
        presenters: structural.presenters,
        panels: panels,
      });
    },
  };
};
