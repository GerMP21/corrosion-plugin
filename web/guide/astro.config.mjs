// The user guide (`add-user-guide`): Astro Starlight under /wiki/, English at
// the root and Spanish under /wiki/es/, dark-only in the site's tokens.
// `npm run build` writes into ../dist/wiki/, next to the home page and the
// web app that web/scripts/build.sh assembles in ../dist/.
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightLinksValidator from 'starlight-links-validator';
import { sidebar } from './sidebar.mjs';

export default defineConfig({
  site: 'https://corrosion-vst.org',
  base: '/wiki',
  outDir: '../dist/wiki',
  trailingSlash: 'always',
  integrations: [
    starlight({
      title: { en: 'Corrosion Wiki', es: 'Wiki de Corrosion' },
      defaultLocale: 'root',
      locales: {
        root: { label: 'English', lang: 'en' },
        es: { label: 'Español', lang: 'es' },
      },
      favicon: '/favicon.svg',
      customCss: ['../theme/tokens.css', './src/styles/guide.css'],
      components: {
        ThemeProvider: './src/components/DarkThemeProvider.astro',
        ThemeSelect: './src/components/NoThemeSelect.astro',
        SocialIcons: './src/components/SiteLinks.astro',
      },
      sidebar,
      pagefind: true,
      lastUpdated: false,
      plugins: [starlightLinksValidator({ errorOnLocalLinks: true })],
    }),
  ],
});
