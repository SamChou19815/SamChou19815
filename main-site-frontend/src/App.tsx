import React, { ReactElement } from 'react';
import AppBar from '@material-ui/core/AppBar';
import Toolbar from '@material-ui/core/Toolbar';
import Typography from '@material-ui/core/Typography';
import MuiThemeProvider from '@material-ui/core/styles/MuiThemeProvider';
import { createMuiTheme } from '@material-ui/core/styles';
import styles from './App.module.css';
import ButtonLink from './components/Common/ButtonLink';
import FirstPage from './components/FirstPage';
import ProjectsSection from './components/ProjectsSection';
import TimelineSection from './components/TimelineSection';
import TechTalkSection from './components/TechTalkSection';

const theme = createMuiTheme({ palette: { primary: { main: '#3E7AE2' } } });

export default (): ReactElement => (
  <MuiThemeProvider theme={theme}>
    <div className={styles.App}>
      <AppBar position="fixed" className={styles.AppBar}>
        <Toolbar>
          <Typography variant="h6" color="inherit" className={styles.Title}>
            Developer Sam
          </Typography>
          <ButtonLink href="/resume.pdf" color="inherit" openInNewTab className={styles.Resume}>
            Resume
          </ButtonLink>
          <ButtonLink href="https://blog.developersam.com" color="inherit" openInNewTab>
            Blog
          </ButtonLink>
          <ButtonLink href="https://github.com/SamChou19815" color="inherit" openInNewTab>
            GitHub
          </ButtonLink>
        </Toolbar>
      </AppBar>
      <FirstPage />
      <ProjectsSection />
      <TechTalkSection />
      <TimelineSection />
    </div>
  </MuiThemeProvider>
);
