import React, { ReactElement, ReactNode } from 'react';

import Head from '@docusaurus/Head';
import Link from '@docusaurus/Link';
import useBaseUrl from '@docusaurus/useBaseUrl';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';

export default function App({ children }: { readonly children: ReactNode }): ReactElement {
  const { siteConfig = {} } = useDocusaurusContext();
  const { favicon } = siteConfig;
  const faviconUrl = useBaseUrl(favicon);

  const buttons = (
    <>
      <Link className="navbar__item navbar__link" to="/">
        Play against AI
      </Link>
      <Link className="navbar__item navbar__link" to="/local">
        Play locally
      </Link>
      <Link className="navbar__item navbar__link" to="/online">
        Play online
      </Link>
      <Link className="navbar__item navbar__link" to="/rules">
        Rules
      </Link>
      <a className="navbar__item navbar__link" href="https://developersam.com">
        Home
      </a>
    </>
  );

  const head = (
    <Head>
      <html lang="en" />
      <title>TEN Game</title>
      <meta property="og:title" content="TEN Game" />
      <link rel="shortcut icon" href={faviconUrl} />
      <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no" />
    </Head>
  );

  return (
    <div>
      {head}
      <nav className="navbar">
        <div className="navbar__inner">
          <div className="navbar__items">
            <a className="navbar__brand" href="/">
              <img className="navbar__logo" src="/logo.png" alt="TEN App logo" />
              <strong className="navbar__title">TEN</strong>
            </a>
          </div>
          <div className="navbar__items navbar__items--right">{buttons}</div>
        </div>
      </nav>
      {children}
    </div>
  );
}
