import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'Genex 内部構造ガイド',
  tagline: 'Genexを設計と振る舞いから理解するためのコードリーディングガイド',
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
      items: [
        {to: '/intro/reading-order', label: '読み方', position: 'left'},
        {to: '/chat/chat-loop', label: 'チャットループ', position: 'left'},
        {to: '/storage/sqlite', label: 'DB', position: 'left'},
        {
          href: 'https://github.com/edge-m/codex-fork',
          label: 'Repository',
          position: 'right',
        },
      ],
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
    footer: {
      style: 'dark',
      links: [
        {
          title: '読む',
          items: [
            {label: '読み方', to: '/intro/reading-order'},
            {label: 'システムマップ', to: '/intro/system-map'},
          ],
        },
        {
          title: '中心テーマ',
          items: [
            {label: 'チャットループ', to: '/chat/chat-loop'},
            {label: 'SQLite', to: '/storage/sqlite'},
          ],
        },
      ],
      copyright: `Genex internal documentation`,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
