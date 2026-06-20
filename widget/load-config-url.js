// CosAm Calendar — config URL loader
// Factory that returns a config loader fetching a ScheduleConfig (branding +
// print-format defaults) from a URL, for fully URL-driven hosts. Pair with a
// schedule loader: CosAmCalendar.init({ el, loader: ..., configLoader: ... }).
// Usage: CosAmCalendar.init({ el, loader, configLoader: CosAmCalendar.ConfigUrlLoader({ url: 'config.json' }) })
//
// Copyright (c) 2026 Daphne Pfister | BSD-2-Clause

window.CosAmCalendar = window.CosAmCalendar || {};

/**
 * Factory for the config URL loader.
 * @param {object} [opts]
 * @param {string} [opts.url='config.json'] - URL to fetch the ScheduleConfig JSON from.
 */
window.CosAmCalendar.ConfigUrlLoader = function (opts) {
  var configUrl = (opts && opts.url) || 'config.json';

  return {
    load: function (rootEl) {
      return fetch(configUrl)
        .then(function (r) {
          if (!r.ok) throw new Error('HTTP ' + r.status);
          return r.json();
        })
        .then(function (cfg) {
          // Forward only the presentation fields; init merges these into data.
          return { brand: cfg.brand, printFormats: cfg.printFormats };
        });
    },
  };
};
