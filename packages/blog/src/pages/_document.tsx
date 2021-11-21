import CommonHeader from 'esbuild-scripts/components/CommonHeader';
import Link from 'esbuild-scripts/components/Link';
import { useLocation } from 'esbuild-scripts/components/router-hooks';
import usePageTracking from 'lib-react-ga';
import React, { ReactNode } from 'react';

import 'infima/dist/css/default/default.min.css';
import './index.css';
import { BLOG_LOGO, BLOG_TITLE, BLOG_URL } from '../constants';

export default function DocumentWrapper({ children }: { children: ReactNode }): JSX.Element {
  usePageTracking();
  let path = useLocation().pathname;
  if (path.endsWith('/')) path = path.substring(0, path.length - 1);

  return (
    <>
      <CommonHeader
        title="Developer Sam Blog"
        description="Developer Sam's Blog"
        shortcutIcon={BLOG_LOGO}
        ogAuthor="Developer Sam"
        ogURL={`${BLOG_URL}${path}`}
      />
      <nav className="navbar navbar--fixed-top">
        <div className="navbar__inner">
          <div className="navbar__items">
            <Link className="navbar__brand" to="/">
              <img className="navbar__logo" src={BLOG_LOGO} alt="Logo" />
              <strong className="navbar__title">{BLOG_TITLE}</strong>
            </Link>
          </div>
          <div className="navbar__items navbar__items--right">
            <a
              className="navbar__item navbar__link"
              href="https://developersam.com"
              target="_blank"
              rel="noreferrer"
            >
              Main Site
            </a>
            <a
              className="navbar__item navbar__link"
              href="https://github.com/SamChou19815/website"
              target="_blank"
              rel="noreferrer"
            >
              GitHub
            </a>
          </div>
        </div>
      </nav>
      <div className="main-wrapper">{children}</div>
      <footer className="footer footer--dark">
        <div className="container">
          <div className="footer__bottom text--center">
            <div className="footer__copyright">Copyright © 2016-2021 Developer Sam.</div>
          </div>
        </div>
      </footer>
    </>
  );
}
