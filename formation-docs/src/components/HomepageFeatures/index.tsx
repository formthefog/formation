import React from 'react';
import type {ReactNode} from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';
import Heading from '@theme/Heading';
import styles from './styles.module.css';

type FeatureItem = {
  title: string;
  description: ReactNode;
  linkTo: string;
  linkText: string;
  iconClass: string;
  emoji: string;
};

const FeatureList: FeatureItem[] = [
  {
    title: 'For Developers',
    description: (
      <>
        Build and deploy secure, confidential applications using Formation's distributed network. Access powerful APIs, SDK, and comprehensive documentation.
      </>
    ),
    emoji: '👩‍💻',
    linkTo: '/developer/',
    linkText: 'Developer Getting Started',
    iconClass: 'code'
  },
  {
    title: 'For Operators',
    description: (
      <>
        Join the Formation network by contributing compute resources. Earn rewards by helping power the decentralized cloud.
      </>
    ),
    emoji: '🖥️',
    linkTo: '/operator/',
    linkText: 'Operator Getting Started',
    iconClass: 'server'
  },
  {
    title: 'Architecture',
    description: (
      <>
        Discover the technical foundation of Formation's decentralized protocol, self-replicating mechanisms and governance.
      </>
    ),
    emoji: '🏗️',
    linkTo: '/architecture/',
    linkText: 'Explore Architecture',
    iconClass: 'sitemap'
  },
];

function Feature({title, description, linkTo, linkText, emoji}: FeatureItem) {
  return (
    <div className={styles.featureCard}>
      <div className={styles.featureEmoji}>{emoji}</div>
      <Heading as="h3" className={styles.featureTitle}>{title}</Heading>
      <div className={styles.featureDescription}>{description}</div>
      <div className={styles.featureAction}>
        <Link to={linkTo} className={styles.featureButton}>
          {linkText}
        </Link>
      </div>
    </div>
  );
}

export default function HomepageFeatures(): ReactNode {
  return (
    <section className={styles.featuresSection}>
      <div className={styles.featuresContainer}>
        {FeatureList.map((props, idx) => (
          <div key={idx} className={styles.featureWrapper}>
            <Feature {...props} />
          </div>
        ))}
      </div>
    </section>
  );
}
