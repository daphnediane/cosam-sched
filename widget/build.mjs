/**
 * esbuild script for the CosAm Calendar widget.
 *
 * Produces widget/cosam-calendar.min.js and widget/cosam-calendar.min.css,
 * which cosam-convert embeds via include_str!.
 *
 * Usage:
 *   node widget/build.mjs          # one-shot build
 *   node widget/build.mjs --watch  # rebuild on change
 */

import * as esbuild from 'esbuild';
import { argv } from 'node:process';

const watch = argv.includes('--watch');

/** @type {esbuild.BuildOptions} */
const jsOptions = {
  entryPoints: ['widget/cosam-calendar.js'],
  bundle: true,       // resolves npm imports (e.g. qrcode)
  platform: 'browser',
  format: 'iife',     // keeps global window.CosAmCalendar assignment working
  minify: true,
  // Preserve non-ASCII characters (emoji, etc.) as UTF-8 rather than escaping them.
  charset: 'utf8',
  outfile: 'widget/cosam-calendar.min.js',
  logLevel: 'info',
};

/** @type {esbuild.BuildOptions} */
const cssOptions = {
  entryPoints: ['widget/cosam-calendar.css'],
  bundle: false,
  minify: true,
  charset: 'utf8',
  outfile: 'widget/cosam-calendar.min.css',
  logLevel: 'info',
};

if (watch) {
  const [jsCtx, cssCtx] = await Promise.all([
    esbuild.context(jsOptions),
    esbuild.context(cssOptions),
  ]);
  await Promise.all([jsCtx.watch(), cssCtx.watch()]);
  console.log('Watching widget/ for changes...');
} else {
  await Promise.all([
    esbuild.build(jsOptions),
    esbuild.build(cssOptions),
  ]);
}
