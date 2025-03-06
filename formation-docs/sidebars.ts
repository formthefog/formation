import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

// This runs in Node.js - Don't use client-side code here (browser APIs, JSX...)

/**
 * Creating a sidebar enables you to:
 - create an ordered group of docs
 - render a sidebar for each doc of that group
 - provide next/previous navigation

 The sidebars can be generated from the filesystem, or explicitly defined here.

 Create as many sidebars as you want.
 */
const sidebars: SidebarsConfig = {
  mainSidebar: [
    {
      type: 'doc',
      id: 'index',
      label: 'Introduction',
    },
    {
      type: 'category',
      label: 'Operator Documentation',
      link: {
        type: 'doc',
        id: 'operator/index',
      },
      items: [
        {
          type: 'category',
          label: 'Getting Started',
          link: {
            type: 'doc',
            id: 'operator/getting-started/index',
          },
          items: [],
        },
        {
          type: 'category',
          label: 'Guides',
          link: {
            type: 'doc',
            id: 'operator/guides/index',
          },
          items: [
            'operator/guides/installation',
            'operator/guides/resource-management',
            'operator/guides/monitoring',
            'operator/guides/maintenance',
            'operator/guides/troubleshooting',
          ],
        },
        {
          type: 'category',
          label: 'Reference',
          link: {
            type: 'doc',
            id: 'operator/reference/index',
          },
          items: [
            'operator/reference/cli-reference',
            'operator/reference/configuration-reference',
            'operator/reference/api-reference',
            'operator/reference/hardware-requirements',
            'operator/reference/resource-management',
            'operator/reference/storage-reference',
            'operator/reference/network-requirements',
            'operator/reference/metrics-reference',
            'operator/reference/log-reference',
            'operator/reference/alert-reference',
            'operator/reference/staking-reference',
            'operator/reference/rewards-reference',
            'operator/reference/pricing-reference',
          ],
        },
        {
          type: 'category',
          label: 'Tutorials',
          link: {
            type: 'doc',
            id: 'operator/tutorials/index',
          },
          items: [],
        },
      ],
    },
    {
      type: 'category',
      label: 'Developer Documentation',
      link: {
        type: 'doc',
        id: 'developer/index',
      },
      items: [
        {
          type: 'category',
          label: 'Getting Started',
          link: {
            type: 'doc',
            id: 'developer/getting-started/index',
          },
          items: [],
        },
        {
          type: 'category',
          label: 'Guides',
          link: {
            type: 'doc',
            id: 'developer/guides/index',
          },
          items: [
            'developer/guides/writing-formfiles',
            'developer/guides/managing-instances',
            'developer/guides/networking',
            'developer/guides/using-form-kit',
            'developer/guides/using-ethereum-wallets',
            'developer/guides/troubleshooting',
          ],
        },
        {
          type: 'category',
          label: 'Reference',
          link: {
            type: 'doc',
            id: 'developer/reference/index',
          },
          items: [
            'developer/reference/formfile-reference',
            'developer/reference/cli-reference',
            'developer/reference/api-reference',
            'developer/reference/configuration-reference',
            'developer/reference/environment-variables',
            'developer/reference/network-reference',
            'developer/reference/resource-spec-reference',
            'developer/reference/error-codes',
            'developer/reference/metrics-reference',
          ],
        },
        {
          type: 'category',
          label: 'Tutorials',
          link: {
            type: 'doc',
            id: 'developer/tutorials/index',
          },
          items: [],
        },
        {
          type: 'category',
          label: 'Examples',
          link: {
            type: 'doc',
            id: 'developer/examples/index',
          },
          items: [],
        },
      ],
    },
    {
      type: 'category',
      label: 'Architecture',
      link: {
        type: 'generated-index',
        title: 'Architecture Documentation',
        slug: '/architecture',
      },
      items: [],
    },
    {
      type: 'category',
      label: 'Inference Engine',
      link: {
        type: 'generated-index',
        title: 'Inference Engine Documentation',
        slug: '/inference-engine',
      },
      items: [],
    },
    {
      type: 'category',
      label: 'Pricing',
      link: {
        type: 'doc',
        id: 'pricing/index',
      },
      items: [],
    },
    {
      type: 'category',
      label: 'API Reference',
      link: {
        type: 'doc',
        id: 'api/index',
      },
      items: [
        {
          type: 'doc',
          id: 'api/vmm/index',
          label: 'VMM Service API',
        },
        {
          type: 'doc',
          id: 'api/state/index',
          label: 'State Service API',
        },
        {
          type: 'doc',
          id: 'api/p2p/index',
          label: 'P2P Service API',
        },
        {
          type: 'doc',
          id: 'api/dns/index',
          label: 'DNS Service API',
        },
        {
          type: 'doc',
          id: 'api/formnet/index',
          label: 'Formnet API',
        },
      ],
    },
  ],
};

export default sidebars;
