// CosAm Calendar — JSON embed loader
// Reads schedule data from a gzip+base64-encoded JSON script element
// (id="cosam-schedule-data") and exposes CosAmCalendar.JsonEmbedLoader.
//
// Copyright (c) 2026 Daphne Pfister | BSD-2-Clause

window.CosAmCalendar = window.CosAmCalendar || {};

/**
 * Factory for the gzip+base64 JSON embed loader.
 * @param {object} [opts]
 * @param {string} [opts.dataId='cosam-schedule-data'] - ID of the script element holding encoded data.
 */
window.CosAmCalendar.JsonEmbedLoader = function (opts) {
  var dataId = (opts && opts.dataId) || 'cosam-schedule-data';

  return {
    load: function (rootEl) {
      var dataEl = document.getElementById(dataId);
      if (!dataEl) {
        return Promise.reject(new Error('Element #' + dataId + ' not found'));
      }
      var raw = dataEl.textContent.trim();
      if (raw.substring(0, 4) === 'H4sI') {
        try {
          var bytes = Uint8Array.from(atob(raw), function (c) { return c.charCodeAt(0); });
          var ds = new DecompressionStream('gzip');
          var writer = ds.writable.getWriter();
          writer.write(bytes);
          writer.close();
          return new Response(ds.readable).arrayBuffer()
            .then(function (buf) { return JSON.parse(new TextDecoder().decode(buf)); });
        } catch (err) {
          return Promise.reject(err);
        }
      }
      try {
        return Promise.resolve(JSON.parse(raw));
      } catch (err) {
        return Promise.reject(err);
      }
    },
  };
};
