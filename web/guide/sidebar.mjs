// The guide's sidebar (`add-user-guide`): the agreed sitemap, in order, with
// Spanish labels. Groups list their directory's pages; a page's
// `sidebar.order` front matter sets its place in the group.
const group = (label, es, directory, collapsed = false) => ({
  label,
  translations: { es },
  collapsed,
  items: [{ autogenerate: { directory } }],
});
const page = (slug) => ({ slug });

export const sidebar = [
  group('Getting started', 'Empezar', 'getting-started'),
  page('how-it-sounds'),
  page('editor'),
  group('Exciters', 'Excitadores', 'exciters', true),
  group('Objects', 'Objetos', 'objects', true),
  group('Output chain', 'Cadena de salida', 'output-chain', true),
  page('macros'),
  page('randomizer'),
  group('Note modes', 'Modos de nota', 'note-modes', true),
  group('Presets', 'Presets', 'presets', true),
  group('MIDI and expression', 'MIDI y expresión', 'midi', true),
  page('quality'),
  page('ui-scale'),
  page('recipes'),
  page('troubleshooting'),
  page('glossary'),
  group('Parameter reference', 'Referencia de parámetros', 'reference', true),
];
