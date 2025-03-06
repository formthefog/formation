import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

// This runs in Node.js - Don't use client-side code here (browser APIs, JSX...)

const config: Config = {
  title: 'Formation Protocol Documentation',
  tagline: 'A public verifiable and self-replicating protocol for trustless, confidential virtual private servers and scalable, peer to peer, affordable inference with state of the art AI models',
  favicon: 'img/logo/Formation_Logomark-1.png',

  // Set the production url of your site here
  url: 'https://formation.cloud',
  // Set the /<baseUrl>/ pathname under which your site is served
  // For GitHub pages deployment, it is often '/<projectName>/'
  baseUrl: '/',

  // GitHub pages deployment config.
  // If you aren't using GitHub pages, you don't need these.
  organizationName: 'formthefog', // GitHub org/user name.
  projectName: 'formation-docs', // Repo name.

  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',

  // Even if you don't use internationalization, you can use this field to set
  // useful metadata like html lang. For example, if your site is Chinese, you
  // may want to replace "en" with "zh-Hans".
  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  // Add FontAwesome script to the head
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
          // Please change this to your repo.
          // Remove this to remove the "edit this page" links.
          editUrl:
            'https://github.com/formthefog/formation-docs/tree/main/',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    // Replace with your project's social card
    image: 'img/logo/Formation_Logo-1.png',
    navbar: {
      title: '',
      logo: {
        alt: 'Formation Logo',
        src: 'img/logo/Formation_Logomark-1.svg',
      },
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'mainSidebar',
          position: 'left',
          label: 'Operator Docs',
          to: '/operator/',
        },
        {
          type: 'docSidebar',
          sidebarId: 'mainSidebar',
          position: 'left',
          label: 'Developer Docs',
          to: '/developer/',
        },
        {
          type: 'docSidebar',
          sidebarId: 'mainSidebar',
          position: 'left',
          label: 'Architecture',
          to: '/architecture/',
        },
        {
          type: 'docSidebar',
          sidebarId: 'mainSidebar',
          position: 'left',
          label: 'Inference Engine',
          to: '/inference-engine/',
        },
        {
          type: 'docSidebar',
          sidebarId: 'mainSidebar',
          position: 'left',
          label: 'Pricing',
          to: '/pricing/',
        },
        {
          type: 'docSidebar',
          sidebarId: 'mainSidebar',
          position: 'left',
          label: 'API Reference',
          to: '/api/',
        },
        {
          href: 'https://github.com/formthefog/formation-docs',
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
              to: '/operator/',
            },
            {
              label: 'Developer Docs',
              to: '/developer/',
            },
            {
              label: 'Architecture',
              to: '/architecture/',
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
