import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'Genex 内部構造ガイド',
  tagline: 'Genexを設計と振る舞いから理解するためのコードリーディングガイド',
  favicon: 'img/favicon.ico',
  url: 'https://localhost',
  baseUrl: '/',
  organizationName: 'genex',
  projectName: 'genex-inspection',
  onBrokenLinks: 'throw',
  i18n: {
    defaultLocale: 'ja',
    locales: ['ja'],
  },
  presets: [
    [
      'classic',
      {
        docs: {
          routeBasePath: '/',
          sidebarPath: './sidebars.ts',
          showLastUpdateTime: false,
          breadcrumbs: true,
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],
  markdown: {
    mermaid: true,
    hooks: {
      onBrokenMarkdownLinks: 'throw',
    },
  },
  themes: ['@docusaurus/theme-mermaid'],
  themeConfig: {
    navbar: {
      title: 'Genex 内部構造ガイド',
      hideOnScroll: true,
    },
    prism: {
      additionalLanguages: ['bash', 'json', 'rust', 'toml', 'sql'],
    },
    docs: {
      sidebar: {
        hideable: true,
        autoCollapseCategories: true,
      },
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
