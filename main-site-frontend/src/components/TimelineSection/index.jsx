// @flow strict
/* eslint-disable react/no-array-index-key */

import type { Node } from 'react';
import React from 'react';
// $FlowFixMe
import FormGroup from '@material-ui/core/FormGroup';
// $FlowFixMe
import FormControlLabel from '@material-ui/core/FormControlLabel';
// $FlowFixMe
import Checkbox from '@material-ui/core/Checkbox';
import ConsoleSection from '../Common/ConsoleSection';
import type { TimelineItem } from './items';
import items from './items';
import TimelineItemCard from './TimelineItemCard';
import styles from './TimelineSection.module.css';

type CheckboxProps = {| +label: string; +checked: boolean; onChange: () => void |};

const ControlledCheckbox = ({ checked, onChange, label }: CheckboxProps): Node => (
  <FormControlLabel
    control={(
      <Checkbox
        checked={checked}
        onChange={onChange}
        color="primary"
      />
    )}
    label={label}
  />
);

export default (): Node => {
  const [workChecked, setWorkChecked] = React.useState(true);
  const [projectsChecked, setProjectsChecked] = React.useState(true);
  const [eventsChecked, setEventsChecked] = React.useState(true);

  const workOnChange = () => setWorkChecked(prev => !prev);
  const projectsOnChange = () => setProjectsChecked(prev => !prev);
  const eventsOnChange = () => setEventsChecked(prev => !prev);

  let title = './timeline --fancy-display';
  if (!(workChecked && projectsChecked && eventsChecked)) {
    if (!workChecked && !projectsChecked && !eventsChecked) {
      title += ' --none';
    } else {
      title += ' --only';
      if (workChecked) {
        title += ' work';
      }
      if (projectsChecked) {
        title += ' projects';
      }
      if (eventsChecked) {
        title += ' events';
      }
    }
  }

  const filteredItems = items.filter(({ type }: TimelineItem) => {
    if (type === 'work' && workChecked) {
      return true;
    }
    if (type === 'project' && projectsChecked) {
      return true;
    }
    return type === 'event' && eventsChecked;
  });

  return (
    <ConsoleSection id="timeline" title={title}>
      <FormGroup row className={styles.ControlSection}>
        <h3 className={styles.ControlSectionTitle}>Filters:</h3>
        <ControlledCheckbox label="Work & Interns" checked={workChecked} onChange={workOnChange} />
        <ControlledCheckbox
          label="Projects"
          checked={projectsChecked}
          onChange={projectsOnChange}
        />
        <ControlledCheckbox label="Events" checked={eventsChecked} onChange={eventsOnChange} />
      </FormGroup>
      <div className={styles.TimelineSection}>
        <div className={styles.VerticalBar} />
        {filteredItems.map((item, index) => <TimelineItemCard key={index} item={item} />)}
      </div>
    </ConsoleSection>
  );
};
