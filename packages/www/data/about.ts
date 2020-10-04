import type { WwwSvgIconName } from '../components/Common/Icons';

type Fact = { readonly iconName: WwwSvgIconName; readonly text: string };
type Link = { readonly href: string; readonly text: string };
type About = { readonly facts: readonly Fact[]; readonly links: readonly Link[] };

const about: About = {
  facts: [
    { iconName: 'facebook', text: 'Facebook SWE Intern' },
    { iconName: 'work', text: 'Cornell DTI Developer' },
    { iconName: 'github', text: 'Open source contributor' },
    { iconName: 'school', text: 'Cornell University' },
    { iconName: 'domain', text: 'Computer Science' },
    { iconName: 'code', text: 'Coding since 13' },
  ],
  links: [
    { href: 'https://blog.developersam.com', text: 'Blog' },
    { href: 'https://github.com/SamChou19815', text: 'GitHub' },
  ],
};

export default about;
