// The controls a model shows in the editor, from the generated data: its
// own slots, its panel's shared controls (not the model selector) and, for
// an exciter, its family's envelope. Shared by the guide check (model pages
// must link them) and <ModelControls> (which renders them as links).

/**
 * @param {'exciters' | 'objects'} kind
 * @param {string} name the model's name, as the plugin shows it
 * @param {any[]} parameters parameters.json's `parameters`
 * @param {any} models models.json
 */
export function modelControls(kind, name, parameters, models) {
  const panel = kind === 'exciters' ? 'exciter' : 'resonator';
  const selector = kind === 'exciters' ? 'exciter' : 'object';
  const family = models.exciters.find((e) => e.name === name)?.family;
  return parameters.filter((p) => {
    if (p.panel !== panel || p.id === selector) return false;
    if ((p.slots ?? []).some((s) => s.model === name)) return true;
    return (p.controls ?? []).some((c) => c.shown === 'always' || (family && c.shown === `family:${family}`));
  });
}
