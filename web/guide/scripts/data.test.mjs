// Tests for the values <Data> renders (`write-user-guide-content` 1.2).
// Usage: node --test scripts/data.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { dataCount, dataParam } from '../src/lib/data-values.mjs';

const read = (name) => JSON.parse(readFileSync(new URL(`../src/data/${name}`, import.meta.url), 'utf8'));
const models = read('models.json');
const parameters = read('parameters.json').parameters;

test('counts come from models.json', () => {
  // The bank's files: 214 when this was written, the drum kits included.
  const files = readdirSync(new URL('../../../presets/factory/', import.meta.url), { recursive: true }).filter((f) =>
    String(f).endsWith('.corrosion-preset'),
  );
  assert.equal(dataCount('factoryPresets', models, parameters), files.length);
  assert.equal(dataCount('drumKits', models, parameters), models.factory.drum_kits.length);
  assert.equal(dataCount('exciters', models, parameters), 16);
  assert.equal(dataCount('objects', models, parameters), 9);
  assert.equal(dataCount('parameters', models, parameters), parameters.length);
});

test('a parameter field comes from parameters.json', () => {
  assert.equal(dataParam('position_wander_rate', 'unit', parameters), 'Hz');
  const p = parameters.find((q) => q.id === 'position_wander_rate');
  assert.equal(dataParam('position_wander_rate', 'range', parameters), `${p.min}–${p.max} Hz`);
  assert.equal(dataParam('position_wander_rate', 'default', parameters), `${p.default} Hz`);
  assert.equal(dataParam('filter_cutoff', 'name', parameters), 'Filter Cutoff');
});

test('a selector lists its options', () => {
  assert.equal(dataParam('space_mode', 'options', parameters), 'Off, Factory, Spring, Echo');
});

test('unknown names fail the build rather than print nothing', () => {
  assert.throws(() => dataCount('presetz', models, parameters), /unknown count `presetz`/);
  assert.throws(() => dataParam('cutof', 'range', parameters), /unknown parameter `cutof`/);
  assert.throws(() => dataParam('filter_cutoff', 'colour', parameters), /unknown field `colour`/);
});
