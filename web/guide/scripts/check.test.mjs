// Tests for check.mjs's model-page rules (`write-user-guide-content` 1.1):
// each exciter and object page has the seven sections in order (with the
// Spanish headings mapped), a written page's Controls section links every
// control its model shows, and its Listen section has the model's clips.
// Each case copies the real pages to a temporary folder, changes one page,
// runs the check against it and looks for the expected message.
// Usage: node --test scripts/check.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const docs = join(root, 'src/content/docs');
const parameters = JSON.parse(readFileSync(join(root, 'src/data/parameters.json'), 'utf8')).parameters;

/** Run the check on a copy of the pages after `edit(dir)`; returns its output. */
function checkWith(edit) {
  const dir = mkdtempSync(join(tmpdir(), 'guide-check-'));
  try {
    cpSync(docs, dir, { recursive: true });
    edit(dir);
    const run = spawnSync(process.execPath, [join(root, 'scripts/check.mjs'), `--docs=${dir}`], { encoding: 'utf8' });
    return { status: run.status, output: run.stdout + run.stderr };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const edit = (dir, page, change) => {
  const path = join(dir, page);
  writeFileSync(path, change(readFileSync(path, 'utf8')));
};

/** The reference link for a parameter, as a page would write it. */
const link = (id) => {
  const p = parameters.find((q) => q.id === id);
  return `[${p.name}](/wiki/reference/${p.panel}/#${id})`;
};

/** The ids of the controls the Bow page must link. */
const bowControls = [
  'bow_pressure',
  'bow_speed',
  'rosin_grip',
  'slip_curve',
  'exciter_pressure',
  'exciter_speed',
  'exciter_roughness',
  'env_amount',
  'velocity_to_peak',
  'global_time_scale',
  'velocity_to_level',
  'velocity_to_time',
  'curve_tension',
  'mseg_onset',
  'mseg_onset_level',
  'mseg_attack',
  'mseg_hold',
  'mseg_decay',
  'mseg_sustain',
  'mseg_release',
  'loop_mode',
  'loop_start_stage',
  'loop_end_stage',
];

/** Write the Bow page: no placeholder, its Controls section holding only
 * links to `ids` (whatever the page's own Controls section says). */
const writeBow = (ids) => (text) =>
  text
    .replace(/^<Stub \/>\n/m, '')
    .replace(/^import Stub .*\n/m, '')
    .replace(/## Controls\n[\s\S]*?(?=\n## )/, `## Controls\n\n${ids.map(link).join(', ')}\n`);

test('the pages as they are pass the model-page rules', () => {
  const { output } = checkWith(() => {});
  assert.doesNotMatch(output, /section|does not link|clip for/);
});

test('a model page with a section out of shape fails, naming it', () => {
  const { status, output } = checkWith((dir) =>
    edit(dir, 'exciters/bow.mdx', (t) => t.replace('## Pairings', '## Partners')),
  );
  assert.equal(status, 1);
  assert.match(output, /exciters\/bow\.mdx.*sections/);
  assert.match(output, /Pairings/);
});

test('the Spanish twin needs the Spanish headings', () => {
  const { status, output } = checkWith((dir) =>
    edit(dir, 'es/objects/pipe.mdx', (t) => t.replace('## Consejos', '## Tips')),
  );
  assert.equal(status, 1);
  assert.match(output, /es\/objects\/pipe\.mdx.*sections/);
  assert.match(output, /Consejos/);
});

test('a written Bow page without Bow Pressure fails, naming the control', () => {
  const { status, output } = checkWith((dir) =>
    edit(dir, 'exciters/bow.mdx', writeBow(bowControls.filter((id) => id !== 'bow_pressure'))),
  );
  assert.equal(status, 1);
  assert.match(output, /exciters\/bow\.mdx.*does not link.*Bow Pressure \(bow_pressure\)/);
  assert.doesNotMatch(output, /Bow Speed \(bow_speed\)/);
});

test('a written Bow page linking every control passes', () => {
  const { output } = checkWith((dir) => edit(dir, 'exciters/bow.mdx', writeBow(bowControls)));
  assert.doesNotMatch(output, /bow\.mdx/);
});

test('<ModelControls> in the Controls section links them all', () => {
  const { output } = checkWith((dir) =>
    edit(dir, 'exciters/bow.mdx', (t) =>
      writeBow([])(t)
        .replace(/^(import Clips .*)$/m, "$1\nimport ModelControls from '../../../components/ModelControls.astro';")
        .replace('## Controls\n', '## Controls\n\n<ModelControls exciter="Bow" />\n')
        .replace(/^(import ModelControls .*\n)\1/m, '$1'),
    ),
  );
  assert.doesNotMatch(output, /bow\.mdx/);
});

test('a Hand Strike page needs the AD envelope, not the MSEG', () => {
  const { output } = checkWith((dir) => edit(dir, 'exciters/hand-strike.mdx', writeBow([]).bind(null)));
  assert.match(output, /hand-strike\.mdx.*does not link.*Hand Mass \(hand_mass\)/);
  assert.match(output, /Attack \(env_attack\)/);
  assert.doesNotMatch(output, /hand-strike\.mdx.*MSEG Attack/);
});

test('an object page links its slots and the resonator controls', () => {
  const { output } = checkWith((dir) => edit(dir, 'objects/pipe.mdx', writeBow([])));
  assert.match(output, /objects\/pipe\.mdx.*does not link.*Pipe Diameter \(pipe_diameter\)/);
  assert.match(output, /Rust \(rust\)/);
  assert.doesNotMatch(output, /pipe\.mdx.*\(object\)/);
});

test('the Listen section needs the model comparison clip', () => {
  const { status, output } = checkWith((dir) =>
    edit(dir, 'exciters/bow.mdx', (t) => t.replace('<Clips set="exciters" item="Bow" />\n', '')),
  );
  assert.equal(status, 1);
  assert.match(output, /exciters\/bow\.mdx.*Listen.*exciters.*Bow/);
});

test('a bare number next to a parameter name warns, without failing', () => {
  const { status, output } = checkWith((dir) =>
    edit(dir, 'exciters/common-controls.mdx', (t) => `${t}\nFilter Cutoff goes up to 20000 at most.\n`),
  );
  assert.equal(status, 0);
  assert.match(output, /warning: .*common-controls\.mdx.*Filter Cutoff.*20000.*<Data param="filter_cutoff"/);
});

test('a number from <Data> does not warn', () => {
  const { output } = checkWith((dir) =>
    edit(dir, 'exciters/common-controls.mdx', (t) => `${t}\nFilter Cutoff runs <Data param="filter_cutoff" field="range" />.\n`),
  );
  assert.doesNotMatch(output, /warning: .*Filter Cutoff/);
});
