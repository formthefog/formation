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
  projectName: 'formation',
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
          editUrl: 'https://github.com/formthefog/formation/tree/main/formation-docs',
          routeBasePath: '/', // Makes docs the main content
          sidebarCollapsible: true,
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],
  themeConfig: {
    image: 'img/logo/Formation_Logo-1.png',
    // Important: Disable hiding the navbar when scrolling
    navbar: {
      hideOnScroll: false,
      style: 'dark',
      logo: {
        alt: 'Formation Logo',
        src: 'img/logo/Formation_Logo-1.svg',
        srcDark: 'img/logo/Formation_Logo-1.svg',
        width: 40,
        height: 40,
      },
      items: [
        {
          type: 'doc',
          docId: 'operator/index',
          position: 'left',
          label: 'Operator Docs',
          className: 'navbar-item-custom',
        },
        {
          type: 'doc',
          docId: 'developer/index',
          position: 'left',
          label: 'Developer Docs',
          className: 'navbar-item-custom',
        },
        {
          type: 'doc',
          docId: 'architecture/index',
          position: 'left',
          label: 'Architecture',
          className: 'navbar-item-custom',
        },
        {
          type: 'doc',
          docId: 'inference-engine/index',
          position: 'left',
          label: 'Inference Engine',
          className: 'navbar-item-custom',
        },
        {
          type: 'doc',
          docId: 'pricing/index',
          position: 'left',
          label: 'Pricing',
          className: 'navbar-item-custom',
        },
        {
          type: 'doc',
          docId: 'api/index',
          position: 'left',
          label: 'API Reference',
          className: 'navbar-item-custom',
        },
        {
          href: 'https://github.com/formthefog/formation',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    // Configure sidebar properly
    docs: {
      sidebar: {
        hideable: true,
        autoCollapseCategories: false,
      },
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
      defaultMode: 'dark', // Set default to dark mode
      disableSwitch: false,
      respectPrefersColorScheme: true,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
