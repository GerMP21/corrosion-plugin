import { defineCollection } from 'astro:content';
import { z } from 'astro/zod';
import { docsLoader, i18nLoader } from '@astrojs/starlight/loaders';
import { docsSchema, i18nSchema } from '@astrojs/starlight/schema';

// `covers` declares which models, modes, folders, features and reference
// panels a page documents; `npm run check` reads it (Decision 5).
const covers = z
  .object({
    exciters: z.array(z.string()).optional(),
    objects: z.array(z.string()).optional(),
    modes: z.record(z.string(), z.array(z.string())).optional(),
    folders: z.array(z.string()).optional(),
    features: z.array(z.string()).optional(),
    panels: z.array(z.string()).optional(),
  })
  .strict();

export const collections = {
  docs: defineCollection({
    loader: docsLoader(),
    schema: docsSchema({ extend: z.object({ covers: covers.optional() }) }),
  }),
  // UI strings of the guide's own components, beside Starlight's.
  i18n: defineCollection({ loader: i18nLoader(), schema: i18nSchema() }),
};
