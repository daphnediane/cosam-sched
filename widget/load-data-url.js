// CosAm Calendar — data URL loader
// Factory that returns a loader fetching JSON from a URL.
// Usage: CosAmCalendar.init({ el: ..., loader: CosAmCalendar.DataUrlLoader({ url: 'schedule.json' }) })
//
// Copyright (c) 2026 Daphne Pfister | BSD-2-Clause

window.CosAmCalendar = window.CosAmCalendar || {};

/**
 * Factory for the data URL loader.
 * @param {object} [opts]
 * @param {string} [opts.url='schedule.json'] - URL to fetch schedule JSON from.
 * @param {boolean} [opts.watch=false] - Enable periodic polling for data changes.
 * @param {number} [opts.intervalMs=5000] - Polling interval in milliseconds.
 */
window.CosAmCalendar.DataUrlLoader = function (opts) {
  var dataUrl = (opts && opts.url) || 'schedule.json';
  var enableWatch = !!(opts && opts.watch);
  // var intervalMs = (opts && opts.intervalMs) || 5000;

  var loader = {
    load: function (rootEl) {
      return fetch(dataUrl)
        .then(function (r) {
          if (!r.ok) throw new Error('HTTP ' + r.status);
          return r.json();
        });
    },
  };

  if (enableWatch) {
    loader.watch = function (rootEl, reload) {
      // TODO: poll dataUrl for changes (compare ETag / Last-Modified or content hash)
    };
  }

  return loader;
};
