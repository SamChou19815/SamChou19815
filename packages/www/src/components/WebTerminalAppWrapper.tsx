import React, { useEffect } from 'react';

import WebTerminal from './web-terminal';
import { WebTerminalCommandsContextProvider } from './web-terminal/WebTerminalCommandsContext';
import baseCommands from './web-terminal/base-commands';
import type { Commands } from './web-terminal/types';

const devSam = () =>
  `Copyright (C) 2015-${new Date().getFullYear()} Developer Sam. All rights reserved.`;

const TIME_OF_OCT_14_2020_7PM = 1602630000000;

function DevMegan(): JSX.Element {
  const timeTogetherInDays = Math.floor(
    (new Date().getTime() - TIME_OF_OCT_14_2020_7PM) / 86400000
  );
  const timeTogetherInYears = Math.floor(timeTogetherInDays / 365);
  return (
    <>
      <div>
        Time together: {timeTogetherInYears} year and {timeTogetherInDays % 365} days.
      </div>
      <div>{"Sam loves Megan's drawings. 💕"}</div>
      <div>
        <img
          src="/fan-arts/dev-sam-birthday-edition.webp"
          height={200}
          alt="@dev-sam/fan-art Birthday Edition"
          title="@dev-sam/fan-art Birthday Edition"
        />
        <img
          src="/fan-arts/dev-sam-fan-art-3.webp"
          height={200}
          alt="@dev-sam/fan-art Iteration 3"
          title="@dev-sam/fan-art Iteration 3"
        />
        <img
          src="/fan-arts/dev-sam-fan-art-2.webp"
          height={200}
          alt="@dev-sam/fan-art Iteration 2"
          title="@dev-sam/fan-art Iteration 2"
        />
        <img
          src="/fan-arts/dev-sam-fan-art-1.webp"
          height={200}
          alt="@dev-sam/fan-art Iteration 1"
          title="@dev-sam/fan-art Iteration 1"
        />
      </div>
      <a href="https://meganyin.com">{"Visit Megan's Website!"}</a>
    </>
  );
}

function Home(): JSX.Element {
  useEffect(() => {
    window.location.href = '/';
  }, []);
  return <div>Redirecting...</div>;
}

const commands: Commands = {
  ...baseCommands,
  home: { fn: () => <Home />, description: 'Redirect to homepage.' },
  'dev-sam': { fn: devSam, description: 'You guess what it is.' },
  'dev-megan': { fn: DevMegan, description: `${'💕'}` },
};

export default function WebTerminalAppWrapper(): JSX.Element {
  return (
    <WebTerminalCommandsContextProvider value={commands}>
      <WebTerminal />
    </WebTerminalCommandsContextProvider>
  );
}
