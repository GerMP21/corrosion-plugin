// Tests for the clip row captions and button values
// (`refine-wiki-naming-and-clips` 1.1, 1.2): every row says what it plays,
// rows on a page don't share a caption unless they play the same clips, and
// a parameter button states the value it plays, choices by option name.
// Usage: node --test scripts/clips.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { clipsFor, rowCaption, clipValue, duplicateCaptions, translator } from '../src/lib/clip-labels.mjs';

const root = fileURLToPath(new URL('..', import.meta.url));
const read = (path) => JSON.parse(readFileSync(join(root, path), 'utf8'));
const parameters = read('src/data/parameters.json').parameters;
const manifest = read('public/clips/clips.json');
const en = translator(read('src/content/i18n/en.json'));
const es = translator(read('src/content/i18n/es.json'));

const caption = (props, t = en) => rowCaption(props, clipsFor(props, manifest), parameters, t);

test('parameter rows carry the control label the editor shows', () => {
  assert.equal(caption({ param: 'mseg_onset' }), 'Onset');
  assert.equal(caption({ param: 'hand_mass' }), 'Hand Mass');
  assert.equal(caption({ param: 'env_attack' }), 'Attack');
  // Control names stay in English on Spanish pages.
  assert.equal(caption({ param: 'mseg_onset' }, es), 'Onset');
});

test('a label two parameters share falls back to the unique host name', () => {
  // The struck/specialty envelope and the MSEG both label a fader Attack.
  assert.equal(caption({ param: 'mseg_attack' }), 'MSEG Attack');
  assert.equal(caption({ param: 'mseg_release' }), 'MSEG Release');
  assert.equal(caption({ param: 'env_release' }), 'Release');
  assert.equal(caption({ param: 'wire_stiffness' }), 'Spring Wire Stiffness');
});

test('a parameter with no editor control uses its host name', () => {
  assert.equal(caption({ param: 'size' }), 'Size');
});

test('set rows say what they compare', () => {
  assert.equal(caption({ set: 'exciters', item: 'Bow' }), 'Bow, on Tank');
  assert.equal(caption({ set: 'objects', item: 'Pipe' }), 'Pipe, with Felt Mallet');
  assert.equal(caption({ set: 'exciter-presets', item: 'Bow' }), 'Factory preset');
  assert.equal(caption({ set: 'recipes', item: 'metallic-bass' }), 'Felt Cable Upright: before and after');
  assert.equal(caption({ set: 'exciters' }), 'All exciters, on Tank');
  assert.equal(caption({ set: 'objects' }), 'All objects, with Felt Mallet');
  assert.equal(caption({ set: 'kits' }), 'Drum kits');
  assert.equal(caption({ set: 'exciters', item: 'Bow' }, es), 'Bow, sobre Tank');
  assert.equal(caption({ set: 'kits' }, es), 'Kits de batería');
});

test('an explicit caption wins', () => {
  assert.equal(caption({ param: 'size', caption: 'Object size' }), 'Object size');
});

test('every caption key exists in both languages', () => {
  for (const props of [{ set: 'exciters', item: 'Bow' }, { set: 'kits' }, { set: 'recipes', item: 'metallic-bass' }]) {
    for (const t of [en, es]) assert.doesNotMatch(caption(props, t), /guide\.clips|\{\{/);
  }
});

test('parameter buttons state their value, choices by name', () => {
  const values = (id) => clipsFor({ param: id }, manifest).map((c) => clipValue(c, parameters));
  assert.deepEqual(values('loop_mode'), ['Off', 'Forward', 'Ping-Pong']);
  assert.deepEqual(values('output'), ['-inf dB', '-10.03 dB', '40.00 dB']);
  assert.deepEqual(values('mseg_onset'), ['0.001', '0.5005', '1']);
});

test('set clips have no value', () => {
  const [clip] = clipsFor({ set: 'exciters', item: 'Bow' }, manifest);
  assert.equal(clipValue(clip, parameters), null);
});

test('duplicate captions are found unless the rows play the same clips', () => {
  const rows = [
    { caption: 'Attack', group: 'param:env_attack' },
    { caption: 'Attack', group: 'param:env_attack' },
    { caption: 'Release', group: 'param:env_release' },
    { caption: 'Release', group: 'param:mseg_release' },
  ];
  assert.deepEqual(duplicateCaptions(rows), ['Release']);
});
