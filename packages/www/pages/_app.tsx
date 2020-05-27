// Adapted from: https://github.com/mui-org/material-ui/tree/master/examples/nextjs-with-typescript

import React, { ReactElement, useEffect } from 'react';

import { AppProps } from 'next/app';
import Head from 'next/head';

import 'infima/dist/css/default/default.min.css';
import './index.css';

const description = 'Explore the portfolio and projects created and open sourced by Developer Sam.';

const MaterialUIApp = (props: AppProps): ReactElement => {
  const { Component, pageProps } = props;

  useEffect(() => {
    // Remove the server-side injected CSS.
    const jssStyles = document.querySelector('#jss-server-side');
    if (jssStyles != null) {
      jssStyles.parentElement?.removeChild(jssStyles);
    }
  }, []);

  return (
    <>
      <Head>
        <meta charSet="utf-8" />
        <link rel="shortcut icon" href="/favicon.ico" />
        <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no" />
        <meta name="theme-color" content="#F7F7F7" />
        <meta name="keywords" content="Sam, Developer Sam, developer, web apps, open source" />
        <meta name="description" content={description} />
        <meta property="og:title" content="Developer Sam" />
        <meta property="og:type" content="profile" />
        <meta property="og:url" content="https://developersam.com/" />
        <meta property="og:image" content="https://developersam.com/timeline/fb-hacker-way.jpg" />
        <meta property="og:description" content={description} />
        <meta name="author" content="Developer Sam" />
        <link rel="canonical" href="https://developersam.com/" />
        <link rel="manifest" href="/manifest.json" />
        <link
          rel="stylesheet"
          href="https://fonts.googleapis.com/css?family=Roboto:300,400,500,700"
        />
        <link rel="stylesheet" href="https://fonts.googleapis.com/css?family=Roboto+Mono:400,500" />
        <title>Developer Sam</title>
        <script
          type="application/ld+json"
          // eslint-disable-next-line react/no-danger
          dangerouslySetInnerHTML={{
            __html: JSON.stringify({
              '@context': 'http://schema.org',
              '@type': 'Organization',
              url: 'https://developersam.com',
              logo: 'https://developersam.com/logo.png',
            }),
          }}
        />
        <script
          type="application/ld+json"
          // eslint-disable-next-line react/no-danger
          dangerouslySetInnerHTML={{
            __html: JSON.stringify({
              '@context': 'http://schema.org',
              '@type': 'Person',
              name: 'Developer Sam',
              url: 'https://developersam.com',
              sameAs: [
                'https://www.developersam.com',
                'https://blog.developersam.com',
                'https://www.facebook.com/SamChou19815',
                'https://twitter.com/SamChou19815',
                'https://github.com/SamChou19815',
                'https://www.linkedin.com/in/sam-zhou-30b91610b/',
              ],
            }),
          }}
        />
      </Head>
      <Component
        // eslint-disable-next-line react/jsx-props-no-spreading
        {...pageProps}
      />
    </>
  );
};

export default MaterialUIApp;
