import esbuildPlugins from './esbuild-plugins';

import type { BuildOptions } from 'esbuild';

const baseESBuildConfig = ({
  isServer = false,
  isProd = false,
}: {
  readonly isServer?: boolean;
  readonly isProd?: boolean;
  readonly noThemeSwitch?: boolean;
}): BuildOptions => ({
  define: {
    __dirname: '""',
    __SERVER__: String(isServer),
    'process.env.NODE_ENV': isProd ? '"production"' : '"development"',
  },
  bundle: true,
  minify: false,
  legalComments: 'linked',
  platform: 'browser',
  target: 'es2019',
  logLevel: 'error',
  plugins: esbuildPlugins,
});

export default baseESBuildConfig;
