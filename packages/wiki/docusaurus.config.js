/**
 * Copyright (c) 2019-present, Developer Sam.
 *
 * This source code is licensed under the AGPLv3 license found in the
 * LICENSE file in the root directory of this source tree.
 */

const theme = require('lib-react/prism-theme.json');

module.exports = {
  title: 'Wiki',
  tagline: 'Documentation for dev-sam',
  url: 'https://wiki.developersam.com',
  baseUrl: '/',
  favicon: 'https://developersam.com/favicon.ico',
  themeConfig: {
    prism: { theme },
    navbar: { title: 'Wiki', items: [] },
  },
  presets: [
    [
      require.resolve('@docusaurus/preset-classic'),
      {
        docs: { sidebarPath: require.resolve('./sidebars.json') },
        debug: true,
        theme: { customCss: require.resolve('./src/css/custom.css') },
      },
    ],
  ],
  plugins: [require.resolve('lib-docusaurus-plugin')],
};
