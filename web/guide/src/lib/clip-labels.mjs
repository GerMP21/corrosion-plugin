// What a row of guide clips says (`refine-wiki-naming-and-clips`): the
// row's caption and, for a parameter row, the value each button plays.
// Shared by <Clips> (which renders them) and the guide check (which fails
// on a row without a caption, or two rows on a page that share one).

/** The manifest clips a `<Clips>` row plays. */
export function clipsFor({ param, set, item }, manifest) {
  const group = param ? `param:${param}` : `set:${set}${item ? `:${item}` : ''}`;
  return manifest.clips.filter((c) => c.ref === group || c.ref.startsWith(`${group}:`));
}

/** The group a row plays, the key two rows compare to tell repeats apart. */
export const rowGroup = ({ param, set, item }) => (param ? `param:${param}` : `set:${set}${item ? `:${item}` : ''}`);

/** A `t(key, vars)` over one of the guide's i18n files, for scripts. */
export function translator(strings) {
  return (key, vars = {}) => (strings[key] ?? key).replace(/\{\{(\w+)\}\}/g, (_, name) => String(vars[name] ?? ''));
}

/**
 * The label the editor shows for a parameter: its first control, else its
 * model slots' label when they all share one (Size is "Pipe Length" on a
 * Pipe and something else on each object, so it has none).
 */
function editorLabel(p) {
  if (p.controls?.length) return p.controls[0].label;
  const labels = new Set((p.slots ?? []).map((s) => s.label));
  return labels.size === 1 ? [...labels][0] : undefined;
}

/** Labels shown on more than one parameter (the MSEG and the AD envelope both say Attack). */
const sharedLabels = new WeakMap();
function isShared(label, parameters) {
  if (!sharedLabels.has(parameters)) {
    const owners = new Map();
    for (const p of parameters) {
      for (const shown of [...(p.controls ?? []), ...(p.slots ?? [])]) {
        owners.set(shown.label, (owners.get(shown.label) ?? new Set()).add(p.id));
      }
    }
    sharedLabels.set(parameters, new Set([...owners].filter(([, ids]) => ids.size > 1).map(([label]) => label)));
  }
  return sharedLabels.get(parameters).has(label);
}

/** What follows `on` / `with` in a model clip's source ("Bow on Tank"). */
const after = (clips, word) => clips[0]?.source.split(` ${word} `).at(-1) ?? '';

/**
 * The caption that opens a clip row.
 * @param {{param?: string, set?: string, item?: string, caption?: string}} props
 * @param {any[]} clips the row's clips (`clipsFor`)
 * @param {any[]} parameters parameters.json's `parameters`
 * @param {(key: string, vars?: object) => string} t
 */
export function rowCaption({ param, set, item, caption }, clips, parameters, t) {
  if (caption) return caption;
  if (param) {
    const p = parameters.find((q) => q.id === param);
    if (!p) return '';
    const label = editorLabel(p);
    // Host names are unique, so they tell two same-labelled faders apart.
    return label && !isShared(label, parameters) ? label : p.name;
  }
  switch (set) {
    case 'exciters':
      return item
        ? t('guide.clips.caption.exciter', { name: item, object: after(clips, 'on') })
        : t('guide.clips.caption.exciters', { object: after(clips, 'on') });
    case 'objects':
      return item
        ? t('guide.clips.caption.object', { name: item, exciter: after(clips, 'with') })
        : t('guide.clips.caption.objects', { exciter: after(clips, 'with') });
    case 'exciter-presets':
    case 'object-presets':
      return t('guide.clips.caption.preset');
    case 'recipes': {
      const preset = (clips[0]?.source ?? '').replace(/^preset /, '').replace(/ \((before|after)\)$/, '');
      return t('guide.clips.caption.recipe', { name: preset });
    }
    case 'kits':
      return t('guide.clips.caption.kits');
    default:
      return '';
  }
}

/**
 * The value a parameter clip plays, as the plugin shows it (the manifest's
 * `id = value`), with a choice's option name for its index. `null` for a
 * clip that isn't a parameter's.
 */
export function clipValue(clip, parameters) {
  if (!clip.ref.startsWith('param:')) return null;
  const id = clip.ref.split(':')[1];
  const shown = clip.source.slice(clip.source.indexOf(' = ') + 3);
  const option = parameters.find((p) => p.id === id)?.options?.find((o) => String(o.value) === shown);
  return option ? option.name : shown;
}

/** Captions used by two rows on one page that play different clips. */
export function duplicateCaptions(rows) {
  const groups = new Map();
  for (const { caption, group } of rows) groups.set(caption, (groups.get(caption) ?? new Set()).add(group));
  return [...groups].filter(([, g]) => g.size > 1).map(([caption]) => caption);
}
