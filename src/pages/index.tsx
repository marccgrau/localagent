import {useState} from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import HomepageFeatures from '@site/src/components/HomepageFeatures';

import styles from './index.module.css';

function InstallCommand() {
  const [copied, setCopied] = useState(false);
  const command = 'cargo install localgpt';

  const handleCopy = () => {
    navigator.clipboard.writeText(command);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className={styles.installWrap}>
      <div className={styles.installCmd} onClick={handleCopy}>
        <code>$ {command}</code>
        <button className={styles.copyBtn} title="Copy to clipboard">
          {copied ? 'Copied!' : 'Copy'}
        </button>
      </div>
    </div>
  );
}

function HomepageHeader() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={clsx('hero hero--dark', styles.heroBanner)}>
      <div className="container">
        <img
          src="/logo/localgpt-icon.png"
          alt={siteConfig.title}
          className={styles.heroIcon}
        />
        <img
          src="/logo/localgpt.gif"
          alt={siteConfig.title}
          className={styles.heroLogo}
        />
        <InstallCommand />
        <p className="hero__subtitle">
          A local AI assistant with persistent memory, autonomous tasks, semantic search, and explorable world generation.
          <br />
          Single binary, no runtime dependencies.
        </p>
        <div className={styles.buttons}>
          <Link
            className="button button--secondary button--lg"
            to="/docs/intro">
            Get Started
          </Link>
        </div>
      </div>
    </header>
  );
}

export default function Home(): JSX.Element {
  const {siteConfig} = useDocusaurusContext();
  return (
    <Layout
      title="Home"
      description="LocalGPT - A local AI assistant with persistent memory, autonomous tasks, semantic search, and explorable world generation. Single binary, no runtime dependencies.">
      <HomepageHeader />
      <main>
        <HomepageFeatures />
      </main>
    </Layout>
  );
}
