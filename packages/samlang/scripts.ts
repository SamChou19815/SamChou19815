import generate from 'lib-react-docs/generator';

generate({
  siteTitle: 'samlang',
  sideBarItems: {
    'Language Basics': ['/introduction', '/classes-types', '/expressions', '/type-inference'],
    'Implementation Notes': ['/architecture', '/intermediate-representation'],
  },
});