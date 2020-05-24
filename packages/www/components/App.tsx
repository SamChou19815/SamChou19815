import React, { ReactElement } from 'react';

import { Provider as ReactReduxProvider } from 'react-redux';

import { store } from '../store';
import styles from './App.module.css';
import ConsoleSection from './Common/ConsoleSection';
import InformationCard from './Common/InformationCard';
import FirstPageCodeBlock from './FirstPage/FirstPageCodeBlock';
import ProjectsSection from './ProjectsSection';
import TechTalkSection from './TechTalkSection';
import TimelineSection from './TimelineSection';
import WebTerminal from './WebTerminal';

export default (): ReactElement => (
  <ReactReduxProvider store={store}>
    <div className={styles.MainLayout}>
      <div className={styles.SideBar}>
        <FirstPageCodeBlock />
      </div>
      <div className={styles.ContentBlock}>
        <ConsoleSection
          id="about"
          title="dev-sam --about"
          className={styles.FirstPage}
          titleClassName={styles.FirstPageTitle}
        >
          <InformationCard className={styles.InfoCard} />
        </ConsoleSection>
        <ProjectsSection />
        <TechTalkSection />
        <TimelineSection />
      </div>
    </div>
    <WebTerminal />
  </ReactReduxProvider>
);
