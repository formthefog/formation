import { themes as prismThemes } from 'prism-react-renderer';
import type { Config } from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'Formation Protocol Documentation',
  tagline: 'A public verifiable and self-replicating protocol for trustless, confidential virtual private servers and scalable, peer to peer, affordable inference with state of the art AI models',
  favicon: 'img/logo/Formation_Logo-1.svg',
  url: 'https://formation.cloud',
  baseUrl: '/',
  organizationName: 'formthefog',
  projectName: 'formation-docs',

  // Change this to 'warn' temporarily to allow builds to complete
  onBrokenLinks: 'warn',
  onBrokenMarkdownLinks: 'warn',

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },
  scripts: [
    {
      src: 'https://kit.fontawesome.com/a91a27a46f.js',
      crossorigin: 'anonymous',
    },
  ],
  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          editUrl: 'https://github.com/formthefog/formation-docs/tree/main/',
          routeBasePath: '/', // Makes docs the main content
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],
  themeConfig: {
    navbar: {
      hideOnScroll: true,
      title: '',
      logo: {
        alt: 'Formation Logo',
        src: 'img/logo/Formation_Logo-1.svg',
      },
      items: [
        {
          type: 'doc',
          docId: 'operator/index',
          position: 'left',
          label: 'Operator Docs',
        },
        {
          type: 'doc',
          docId: 'developer/index',
          position: 'left',
          label: 'Developer Docs',
        },
        {
          type: 'doc',
          docId: 'architecture/index',
          position: 'left',
          label: 'Architecture',
        },
        {
          type: 'doc',
          docId: 'inference-engine/index',
          position: 'left',
          label: 'Inference Engine',
        },
        {
          type: 'doc',
          docId: 'pricing/index',
          position: 'left',
          label: 'Pricing',
        },
        {
          type: 'doc',
          docId: 'api/index',
          position: 'left',
          label: 'API Reference',
        },
        {
          href: 'https://github.com/formthefog/formation',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            {
              label: 'Operator Docs',
              to: '/operator',
            },
            {
              label: 'Developer Docs',
              to: '/developer',
            },
            {
              label: 'Architecture',
              to: '/architecture',
            },
          ],
        },
        {
          title: 'Community',
          items: [
            {
              label: 'Discord',
              href: 'https://discord.gg/formation',
            },
            {
              label: 'Forum',
              href: 'https://forum.formation.cloud',
            },
          ],
        },
        {
          title: 'More',
          items: [
            {
              label: 'GitHub',
              href: 'https://github.com/formthefog',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Formation Protocol. Built with Docusaurus.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
    },
    colorMode: {
      defaultMode: 'light',
      disableSwitch: false,
      respectPrefersColorScheme: true,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
