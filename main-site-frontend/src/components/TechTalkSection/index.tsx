import React, { ReactElement } from 'react';
import Card from '@material-ui/core/Card';
import CardHeader from '@material-ui/core/CardHeader';
import CardActions from '@material-ui/core/CardActions';
import CardContent from '@material-ui/core/CardContent';
import Icon from '@material-ui/core/Icon';
import MaterialButtonLink from 'sam-react-common/MaterialButtonLink';
import styles from './index.module.css';
import ConsoleSection from '../Common/ConsoleSection';

const schoolIcon = <Icon>school</Icon>;

export default (): ReactElement => (
  <ConsoleSection id="tech-talks" title="./tech-talks -all">
    <div className={styles.TechTalkContainer}>
      <Card className={styles.TechTalkCard}>
        <CardHeader avatar={schoolIcon} title="How to scale" subheader="Learning Series" />
        <CardContent>
          {'Tips on scaling your codebase and your workload,'}
          {" with lessons learned from Samwise's codebase."}
        </CardContent>
        <CardActions>
          <MaterialButtonLink href="/how-to-scale.pdf" openInNewTab>
            Slides
          </MaterialButtonLink>
        </CardActions>
      </Card>
      <Card className={styles.TechTalkCard}>
        <CardHeader avatar={schoolIcon} title="Intro to Firebase" subheader="DevSesh" />
        <CardContent>
          Tech stack discussion on Firebase, and why Samwise switched to Firebase.
        </CardContent>
        <CardActions>
          <MaterialButtonLink href="/intro-to-firebase.pdf" openInNewTab>
            Slides
          </MaterialButtonLink>
          <MaterialButtonLink href="https://jessicahong9.github.io/" openInNewTab>
            {"Co-speaker Jessica's website"}
          </MaterialButtonLink>
        </CardActions>
      </Card>
    </div>
  </ConsoleSection>
);
