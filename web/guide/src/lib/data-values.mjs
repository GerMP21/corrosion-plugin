// The values <Data> renders, from the generated data
// (`write-user-guide-content` Decision 3). An unknown name throws, so a
// typo fails the build instead of printing nothing.

/** Counts a page can state. */
const COUNTS = {
  // Every preset in the bank's folders; `drums` holds the drum kits.
  factoryPresets: (m) => m.factory.folders.reduce((n, f) => n + f.presets.length, 0),
  drumKits: (m) => m.factory.drum_kits.length,
  exciters: (m) => m.exciters.length,
  objects: (m) => m.objects.length,
  parameters: (_m, p) => p.length,
};

/** @returns {number} */
export function dataCount(name, models, parameters) {
  const count = COUNTS[name];
  if (!count) throw new Error(`<Data>: unknown count \`${name}\` (one of ${Object.keys(COUNTS).join(', ')})`);
  return count(models, parameters);
}

const withUnit = (value, unit) => (unit ? `${value} ${unit}` : `${value}`);

/** Fields of a parameter a page can state. */
const FIELDS = {
  name: (p) => p.name,
  unit: (p) => p.unit ?? '',
  min: (p) => withUnit(p.min, p.unit),
  max: (p) => withUnit(p.max, p.unit),
  range: (p) => withUnit(`${p.min}–${p.max}`, p.unit),
  default: (p) => withUnit(p.default, p.unit),
  options: (p) => (p.options ?? []).map((o) => o.name).join(', '),
};

/** @returns {string} */
export function dataParam(id, field, parameters) {
  const p = parameters.find((q) => q.id === id);
  if (!p) throw new Error(`<Data>: unknown parameter \`${id}\``);
  const get = FIELDS[field];
  if (!get) throw new Error(`<Data>: unknown field \`${field}\` (one of ${Object.keys(FIELDS).join(', ')})`);
  if (field === 'options' && !p.options) throw new Error(`<Data>: \`${id}\` has no options`);
  if (['min', 'max', 'range'].includes(field) && p.min === undefined) {
    throw new Error(`<Data>: \`${id}\` has no range (it's a selector; use field="options")`);
  }
  return get(p);
}
