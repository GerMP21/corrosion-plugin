#!/usr/bin/env node
// The guide's checks (`add-user-guide` Decision 6). Run by `npm run check`,
// in CI and before the build. Fails, naming each problem, when:
//   1. a page has no twin in the other language, or the twins' heading
//      sequences (levels, in order) differ;
//   2. a model, mode, factory folder, feature or reference panel has no page
//      declaring it in `covers:`, in either language, or `covers:` names
//      something that doesn't exist;
//   3. the IDs of parameters.json and parameters.es.json differ, or a
//      Spanish entry lacks a text the English one has;
//   4. a page references a clip that isn't in clips.json or on disk;
//   5. with --complete, a page still has a <Stub /> placeholder;
//   6. an exciter or object page lacks the seven model sections in order,
//      its Listen section lacks the model's clips, or, once written, its
//      Controls section doesn't link every control the model shows.
// It also warns (7) about a bare number next to a parameter's name.
// Internal links are checked at build by starlight-links-validator.
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import yaml from 'js-yaml';
import { modelControls } from '../src/lib/model-controls.mjs';

const root = fileURLToPath(new URL('..', import.meta.url));
const args = process.argv.slice(2);
const complete = args.includes('--complete');
const docsDir = args.find((a) => a.startsWith('--docs='))?.slice(7) ?? join(root, 'src/content/docs');
const dataDir = join(root, 'src/data');
const clipsDir = join(root, 'public/clips');

/** Pages that exist in one language only. */
const SINGLE_LANGUAGE = new Set(['404']);
/** Features that need a page besides the models and modes (Decision 5). */
export const FEATURES = [
  'kit-editor',
  'user-presets',
  'midi',
  'expression',
  'ui-scale',
  'web-app',
  'randomizer',
  'macros',
];
/** The reference pages, one per editor panel plus the kit. */
export const PANELS = ['header', 'macros', 'exciter', 'resonator', 'processing', 'kit'];

const errors = [];
const fail = (message) => errors.push(message);
const readJson = (path) => JSON.parse(readFileSync(path, 'utf8'));

// --- Pages -----------------------------------------------------------------

function walk(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return walk(path);
    return /\.mdx?$/.test(name) ? [path] : [];
  });
}

function parsePage(path) {
  const text = readFileSync(path, 'utf8');
  const match = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/.exec(text);
  let data = {};
  let body = text;
  if (match) {
    try {
      data = yaml.load(match[1]) ?? {};
    } catch (e) {
      fail(`${rel(path)}: front matter isn't valid YAML (${e.reason ?? e.message})`);
    }
    body = match[2];
  } else {
    fail(`${rel(path)}: no front matter`);
  }
  const headings = [];
  let fence = false;
  for (const line of body.split(/\r?\n/)) {
    if (/^\s*(```|~~~)/.test(line)) fence = !fence;
    const h = !fence && /^(#{2,6})\s+(.+?)\s*#*\s*$/.exec(line);
    if (h) headings.push({ level: h[1].length, text: h[2] });
  }
  return { path, data, body, headings };
}

const rel = (path) => relative(root, path).split(sep).join('/');
const pages = { en: new Map(), es: new Map() };
for (const path of walk(docsDir)) {
  const id = relative(docsDir, path).split(sep).join('/').replace(/\.mdx?$/, '');
  const [lang, key] = id.startsWith('es/') ? ['es', id.slice(3)] : ['en', id];
  pages[lang].set(key, parsePage(path));
}
if (pages.en.size === 0) fail(`no English pages in ${rel(docsDir)}`);

// 1. Twins and heading sequences.
for (const [lang, other] of [
  ['en', 'es'],
  ['es', 'en'],
]) {
  for (const [key, page] of pages[lang]) {
    if (SINGLE_LANGUAGE.has(key)) continue;
    const twin = pages[other].get(key);
    if (!twin) {
      const where = other === 'es' ? `es/${key}` : key;
      fail(`${rel(page.path)} has no ${other.toUpperCase()} twin (expected src/content/docs/${where}.md or .mdx)`);
      continue;
    }
    if (lang === 'en') {
      const a = page.headings.map((h) => h.level).join(',');
      const b = twin.headings.map((h) => h.level).join(',');
      if (a !== b) {
        const show = (p) => p.headings.map((h) => `${'#'.repeat(h.level)} ${h.text}`).join(' | ') || '(none)';
        fail(
          `${rel(page.path)} and ${rel(twin.path)} have different headings:\n` +
            `    EN: ${show(page)}\n    ES: ${show(twin)}`,
        );
      }
    }
  }
}

// 2. Coverage.
const models = readJson(join(dataDir, 'models.json'));
const names = (list) => list.map((item) => item.name);
const known = {
  exciters: names(models.exciters),
  objects: names(models.objects),
  modes: {
    space: names(models.space_modes),
    quality: names(models.quality_modes),
    note: names(models.note_modes),
    limiter: names(models.limiter_modes),
    randomizer: names(models.randomizer_modes),
  },
  folders: [...new Set(models.factory.folders.map((f) => f.folder.split('/')[0]))],
  features: FEATURES,
  panels: PANELS,
};
const covered = { en: new Set(), es: new Set() };
for (const lang of ['en', 'es']) {
  for (const page of pages[lang].values()) {
    const covers = page.data.covers;
    if (covers === undefined) continue;
    if (typeof covers !== 'object' || covers === null) {
      fail(`${rel(page.path)}: \`covers\` must be a mapping`);
      continue;
    }
    for (const [kind, value] of Object.entries(covers)) {
      if (kind === 'modes') {
        for (const [mode, items] of Object.entries(value ?? {})) {
          if (!known.modes[mode]) {
            fail(`${rel(page.path)}: unknown mode list \`${mode}\` (one of ${Object.keys(known.modes).join(', ')})`);
            continue;
          }
          for (const item of items ?? []) {
            if (!known.modes[mode].includes(item)) fail(`${rel(page.path)}: unknown ${mode} mode \`${item}\``);
            covered[lang].add(`modes.${mode}:${item}`);
          }
        }
      } else if (known[kind]) {
        for (const item of value ?? []) {
          if (!known[kind].includes(item)) fail(`${rel(page.path)}: unknown ${kind} entry \`${item}\``);
          covered[lang].add(`${kind}:${item}`);
        }
      } else {
        fail(`${rel(page.path)}: unknown \`covers\` key \`${kind}\``);
      }
    }
  }
}
const required = [
  ...known.exciters.map((n) => ['exciter', `exciters:${n}`, n]),
  ...known.objects.map((n) => ['object', `objects:${n}`, n]),
  ...Object.entries(known.modes).flatMap(([mode, list]) =>
    list.map((n) => [`${mode} mode`, `modes.${mode}:${n}`, n]),
  ),
  ...known.folders.map((n) => ['factory folder', `folders:${n}`, n]),
  ...known.features.map((n) => ['feature', `features:${n}`, n]),
  ...known.panels.map((n) => ['reference panel', `panels:${n}`, n]),
];
for (const [what, key, name] of required) {
  const missing = ['en', 'es'].filter((lang) => !covered[lang].has(key));
  if (missing.length) fail(`${what} \`${name}\` has no page in: ${missing.map((l) => l.toUpperCase()).join(', ')}`);
}

// 3. Spanish parameter texts.
const parameters = readJson(join(dataDir, 'parameters.json')).parameters;
const esPath = join(dataDir, 'parameters.es.json');
if (!existsSync(esPath)) {
  fail('src/data/parameters.es.json is missing');
} else {
  const es = readJson(esPath);
  const ids = new Set(parameters.map((p) => p.id));
  for (const id of Object.keys(es)) if (!ids.has(id)) fail(`parameters.es.json: \`${id}\` is not a parameter`);
  for (const p of parameters) {
    const entry = es[p.id];
    if (!entry) {
      fail(`parameters.es.json: no entry for \`${p.id}\``);
      continue;
    }
    const enHelp = (p.controls ?? []).some((c) => c.help);
    if (enHelp && !entry.help) fail(`parameters.es.json: \`${p.id}\` has no \`help\``);
    for (const slot of p.slots ?? []) {
      if (slot.help && !entry.slots?.[slot.model]) {
        fail(`parameters.es.json: \`${p.id}\` has no help for its ${slot.model} slot (\`slots.${slot.model}\`)`);
      }
    }
    for (const option of p.options ?? []) {
      if (option.help && !entry.options?.[option.name]) {
        fail(`parameters.es.json: \`${p.id}\` has no help for option \`${option.name}\` (\`options.${option.name}\`)`);
      }
    }
  }
}

// 4. Clip references: <Clips param="id" /> or <Clips set="name" item="x" />.
const manifestPath = join(clipsDir, 'clips.json');
const manifest = existsSync(manifestPath) ? readJson(manifestPath) : null;
for (const lang of ['en', 'es']) {
  for (const page of pages[lang].values()) {
    for (const tag of page.body.matchAll(/<Clips\b([^>]*)\/?>/g)) {
      const attr = (name) => new RegExp(`${name}="([^"]+)"`).exec(tag[1])?.[1];
      const group = attr('param') ? `param:${attr('param')}` : `set:${attr('set')}${attr('item') ? `:${attr('item')}` : ''}`;
      if (!manifest) {
        fail(`${rel(page.path)} references clips (${group}) but public/clips/clips.json doesn't exist`);
        continue;
      }
      const clips = manifest.clips.filter((c) => c.ref === group || c.ref.startsWith(`${group}:`));
      if (clips.length === 0) fail(`${rel(page.path)}: no clips for ${group} in clips.json`);
      for (const clip of clips) {
        if (!existsSync(join(clipsDir, clip.file))) fail(`${rel(page.path)}: clip file ${clip.file} is missing`);
      }
    }
  }
}

// 6. Model pages (`write-user-guide-content` Decision 2): the seven
// sections in order; once written (no <Stub />), a Controls section that
// links every control the model shows, or renders them with
// <ModelControls>; a Listen section with the model's two clips.
const MODEL_SECTIONS = {
  en: ['What it is', 'How it sounds', 'Controls', 'Pairings', 'Tips', 'Presets to try', 'Listen'],
  es: ['Qué es', 'Cómo suena', 'Controles', 'Combinaciones', 'Consejos', 'Presets para probar', 'Escúchalo'],
};
/** The body of each `##` section, by heading. */
function sections(body) {
  const out = new Map();
  let current = null;
  let fence = false;
  for (const line of body.split(/\r?\n/)) {
    if (/^\s*(```|~~~)/.test(line)) fence = !fence;
    const h = !fence && /^##\s+(.+?)\s*#*\s*$/.exec(line);
    if (h) {
      current = h[1];
      out.set(current, '');
    } else if (current) {
      out.set(current, `${out.get(current)}${line}\n`);
    }
  }
  return out;
}
for (const lang of ['en', 'es']) {
  const names = MODEL_SECTIONS[lang];
  for (const [key, page] of pages[lang]) {
    const [dir, file] = key.split('/');
    const kind = { exciters: 'exciters', objects: 'objects' }[dir];
    const model = kind && file !== 'index' && page.data.covers?.[kind]?.length === 1 && page.data.covers[kind][0];
    if (!model) continue;
    const got = page.headings.filter((h) => h.level === 2).map((h) => h.text);
    if (got.join('|') !== names.join('|')) {
      fail(`${rel(page.path)}: a model page needs the sections ${names.join(', ')}, in order (has ${got.join(', ') || 'none'})`);
      continue;
    }
    const body = sections(page.body);
    const listen = body.get(names[6]);
    const clipSets = kind === 'exciters' ? ['exciters', 'exciter-presets'] : ['objects', 'object-presets'];
    for (const set of clipSets) {
      if (!new RegExp(`<Clips\\s+set="${set}"\\s+item="${model}"`).test(listen)) {
        fail(`${rel(page.path)}: the ${names[6]} section has no clip for <Clips set="${set}" item="${model}" />`);
      }
    }
    if (/<Stub\b/.test(page.body)) continue;
    const controls = body.get(names[2]);
    const attr = kind === 'exciters' ? 'exciter' : 'object';
    if (new RegExp(`<ModelControls\\s+${attr}="${model}"`).test(controls)) continue;
    const missing = modelControls(kind, model, parameters, models).filter(
      (p) => !new RegExp(`reference/${p.panel}/#${p.id}(?![\\w-])`).test(controls),
    );
    if (missing.length) {
      fail(
        `${rel(page.path)}: the ${names[2]} section does not link ${missing.map((p) => `${p.name} (${p.id})`).join(', ')}`,
      );
    }
  }
}

// 7. Bare numbers (`write-user-guide-content` Decision 3): a number in the
// same sentence as a parameter's name, outside code and <Data>, is probably
// one the data holds. A heuristic, so it warns and doesn't fail.
const warnings = [];
const named = parameters
  .filter((p) => p.name.length >= 4)
  .map((p) => ({ p, re: new RegExp(`\\b${p.name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`) }));
for (const lang of ['en', 'es']) {
  for (const page of pages[lang].values()) {
    let fence = false;
    for (const raw of page.body.split(/\r?\n/)) {
      if (/^\s*(```|~~~)/.test(raw)) fence = !fence;
      if (fence || /^\s*(import|export)\s/.test(raw)) continue;
      const line = raw.replace(/<[A-Z][^>]*\/>/g, ' ').replace(/`[^`]*`/g, ' ').replace(/\]\([^)]*\)/g, ']');
      for (const sentence of line.split(/(?<=[.!?;:])\s+/)) {
        const number = /(?<![\w#-])[-−+]?\d[\d.,]*/.exec(sentence);
        if (!number) continue;
        for (const { p, re } of named) {
          if (re.test(sentence)) {
            warnings.push(
              `${rel(page.path)}: "${p.name}" next to ${number[0]}; if the data holds it, use <Data param="${p.id}" field="…" />`,
            );
          }
        }
      }
    }
  }
}
for (const w of warnings) console.log(`warning: ${w}`);

// 5. Placeholders.
if (complete) {
  for (const lang of ['en', 'es']) {
    for (const page of pages[lang].values()) {
      if (/<Stub\b/.test(page.body)) fail(`${rel(page.path)} is still a placeholder (<Stub />)`);
    }
  }
}

if (errors.length) {
  console.error(`Guide check failed (${errors.length} problem${errors.length === 1 ? '' : 's'}):`);
  for (const e of errors) console.error(`  - ${e}`);
  process.exit(1);
}
const count = pages.en.size + pages.es.size;
console.log(`Guide check passed: ${count} pages, ${parameters.length} parameters${complete ? ', complete' : ''}.`);
