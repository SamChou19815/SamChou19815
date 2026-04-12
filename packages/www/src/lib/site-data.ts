export const GITHUB = "https://github.com/SamChou19815";

export const SOCIAL = {
  github: GITHUB,
  bluesky: "https://bsky.app/profile/developersam.com",
  linkedin: "https://www.linkedin.com/in/sam-zhou-30b91610b/",
} as const;

export const PROJECTS = {
  samlang: {
    tagline: "Sam's programming language with full type-inference.",
    github: `${GITHUB}/samlang`,
    website: "https://samlang.io",
  },
  miniReact: {
    tagline:
      "A simplified version of the React runtime with useState and useEffect hooks, built from scratch.",
    github: `${GITHUB}/mini-react`,
    demo: "https://mini-react.developersam.com",
  },
  samwise: {
    github: "https://github.com/cornell-dti/samwise",
  },
  coursePlan: {
    product: "https://courseplan.io",
    github: "https://github.com/cornell-dti/course-plan",
  },
  ten: {
    github: `${GITHUB}/ten-golang`,
  },
  critterCompiler: {
    github: `${GITHUB}/primitivize`,
  },
  sampl: {
    tagline: "Sam's first programming language. Archived in favor of samlang.",
    github: `${GITHUB}/sampl`,
  },
} as const;

export const WORK = {
  flow: {
    tagline: "Working on Flow's type system, IDE, and performance.",
    website: "https://flow.org",
    github: "https://github.com/facebook/flow",
  },
  pyre: {
    website: "https://pyre-check.org",
    github: "https://github.com/facebook/pyre-check",
  },
  cornellDTI: {
    website: "https://www.cornelldti.org",
  },
} as const;
