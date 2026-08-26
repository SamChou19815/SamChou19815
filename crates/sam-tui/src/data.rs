pub const COPYRIGHT: &str = "Copyright (C) 2015-2026 Developer Sam.";

use crossterm::style::Color;

pub struct Link {
    pub name: &'static str,
    pub url: &'static str,
}

pub struct Project {
    pub id: &'static str,
    /// Site-root-relative artwork, as on the homepage project cards.
    pub image: Option<&'static str>,
    pub tagline: &'static str,
    pub links: &'static [Link],
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Work,
    Education,
    Project,
    Language,
    Milestone,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Work => "work",
            Category::Education => "education",
            Category::Project => "project",
            Category::Language => "language",
            Category::Milestone => "milestone",
        }
    }

    /// The color this category is painted with across the whole TUI
    /// (tailwind 600-series: readable on the white homepage cards).
    pub fn color(self) -> Color {
        match self {
            Category::Work => Color::Rgb {
                r: 37,
                g: 99,
                b: 235,
            }, // blue-600
            Category::Education => Color::Rgb {
                r: 22,
                g: 163,
                b: 74,
            }, // green-600
            Category::Language => Color::Rgb {
                r: 147,
                g: 51,
                b: 234,
            }, // purple-600
            Category::Project => Color::Rgb {
                r: 217,
                g: 119,
                b: 6,
            }, // amber-600
            Category::Milestone => Color::Rgb {
                r: 8,
                g: 145,
                b: 178,
            }, // cyan-600
        }
    }
}

pub struct TimelineEvent {
    pub title: &'static str,
    pub time: &'static str,
    /// Site-root-relative artwork, as on the homepage timeline cards. Ten of
    /// the twenty-seven events have none.
    pub image: Option<&'static str>,
    pub category: Category,
    pub detail: Option<&'static str>,
    pub links: &'static [Link],
}

/// The samlang program from the homepage's sticky code block.
pub const ABOUT_PROGRAM: &str = r#"import {List} from std.list;

class Developer(
  val github: Str,
  val projects: List<Str>,
) {
  function sam(): Developer = {
    let github = "SamChou19815";
    let projects = List
      .of("samlang")
      .cons("website")
      .cons("...");
    Developer.init(github, projects)
  }
}

class Main {
  function main(): Developer = Developer.sam()
}"#;

/// The doc comment that the homepage renders above the program.
pub const ABOUT_DOC_LINKS: &[Link] = &[
    Link {
        name: "demo",
        url: "https://samlang.io/demo",
    },
    Link {
        name: "github",
        url: "https://github.com/SamChou19815",
    },
    Link {
        name: "blog",
        url: "https://developersam.com/blog",
    },
    Link {
        name: "bsky",
        url: "https://bsky.app/profile/developersam.com",
    },
    Link {
        name: "resume",
        url: "https://developersam.com/resume.pdf",
    },
];

pub const PROJECTS: &[Project] = &[
    Project {
        id: "samlang",
        image: Some("/projects/samlang.webp"),
        tagline: "Sam's programming language with full type-inference.",
        links: &[
            Link { name: "GitHub Repo", url: "https://github.com/SamChou19815/samlang" },
            Link { name: "Docs", url: "https://samlang.io" },
            Link { name: "Demo", url: "https://samlang.io/demo" },
        ],
    },
    Project {
        id: "mini-react",
        image: Some("/projects/mini-react.webp"),
        tagline: "A simplified version of the React runtime with useState and useEffect hooks, built from scratch.",
        links: &[
            Link { name: "GitHub Repo", url: "https://github.com/SamChou19815/mini-react" },
            Link { name: "Demo", url: "https://mini-react.developersam.com" },
            Link { name: "Slides", url: "https://developersam.com/build-simplified-react.pdf" },
        ],
    },
    Project {
        id: "samwise",
        image: Some("/projects/samwise.webp"),
        tagline: "A todo-list app by Cornell DTI, built with React, Redux and Firebase.",
        links: &[Link { name: "GitHub Repo", url: "https://github.com/cornell-dti/samwise" }],
    },
    Project {
        id: "courseplan",
        image: Some("/timeline/courseplan-promotion.png"),
        tagline: "Course planning tool for Cornell students by Cornell DTI.",
        links: &[
            Link { name: "Product", url: "https://courseplan.io" },
            Link { name: "GitHub", url: "https://github.com/cornell-dti/course-plan" },
        ],
    },
    Project {
        id: "ten",
        image: Some("/projects/ten.webp"),
        tagline: "A tiny esoteric language implemented in Go.",
        links: &[Link { name: "GitHub Repo", url: "https://github.com/SamChou19815/ten-golang" }],
    },
    Project {
        id: "critter-compiler",
        image: Some("/timeline/critter-compiler.webp"),
        tagline: "A compiler for the Critter World language from Cornell CS 2112. Proved the language is Turing complete.",
        links: &[
            Link { name: "GitHub Repo", url: "https://github.com/SamChou19815/primitivize" },
            Link {
                name: "Blog Post",
                url: "https://blog.developersam.com/2018/08/27/cw-turing-complete/",
            },
        ],
    },
    Project {
        id: "sampl",
        image: Some("/timeline/sampl.webp"),
        tagline: "Sam's first programming language. Archived in favor of samlang.",
        links: &[
            Link { name: "GitHub Repo", url: "https://github.com/SamChou19815/sampl" },
            Link {
                name: "Blog Post",
                url: "https://blog.developersam.com/2018/06/15/sampl-alpha-design-choices/",
            },
        ],
    },
];

/// Contact channels, used by the contact pane and the shell.
pub const SOCIAL_LINKS: &[Link] = &[
    Link {
        name: "GitHub",
        url: "https://github.com/SamChou19815",
    },
    Link {
        name: "Bluesky",
        url: "https://bsky.app/profile/developersam.com",
    },
    Link {
        name: "LinkedIn",
        url: "https://www.linkedin.com/in/sam-zhou-30b91610b/",
    },
    Link {
        name: "Resume",
        url: "https://developersam.com/resume.pdf",
    },
];

/// The work history pane, authored as markdown (rendered by `markdown.rs`).
pub const WORK_MARKDOWN: &str = r#"## Software Engineer, Flow — Meta
*February 2022 — present*

Working on Flow's type system, IDE, and performance.

- [Website](https://flow.org)
- [GitHub](https://github.com/facebook/flow)

## SWE Intern, Flow — Facebook
*June 2021 — September 2021*

Working on the Flow team. Implemented the extraction code action for Flow LSP.

- [Website](https://flow.org)
- [GitHub](https://github.com/facebook/flow)
- [Tech Talk](https://developersam.com/flow-ide-presentation.pdf)

## SWE Intern, Feed Expressions — Facebook
*June 2020 — September 2020*

Working on the Feed Expressions team.

## SWE Intern, Pyre — Facebook
*May 2019 — August 2019*

Working on the Pyre team.

- [Website](https://pyre-check.org)
- [GitHub](https://github.com/facebook/pyre-check)

## Software Developer / Developer Lead / TPM — Cornell DTI
*September 2018 — December 2021*

Frontend developer of Samwise, then technical product manager of the Samwise subteam, then developer lead and developer on CoursePlan.

- [Cornell DTI Website](https://www.cornelldti.org)
"#;

/// The education pane, authored as markdown.
pub const EDUCATION_MARKDOWN: &str = r#"## Cornell University
Master of Engineering (M.Eng) — graduated *December 2021*

Bachelor of Science (B.S.) — graduated *May 2021*, entered August 2017

## High School
Diploma — graduated June 2016
"#;

/// The contact pane, authored as markdown.
pub const CONTACT_MARKDOWN: &str = r#"Ways to reach Developer Sam:

- [GitHub](https://github.com/SamChou19815)
- [Bluesky](https://bsky.app/profile/developersam.com)
- [LinkedIn](https://www.linkedin.com/in/sam-zhou-30b91610b/)
- [Résumé](https://developersam.com/resume.pdf)

> click a link to open it in a new tab
"#;

/// The homepage timeline, newest first, mirroring `DATASET_TIMELINE`.
pub const TIMELINE: &[TimelineEvent] = &[
    TimelineEvent {
        title: "Became a Canadian permanent resident",
        time: "August 2026",
        image: Some("/timeline/canada.webp"),
        category: Category::Milestone,
        detail: None,
        links: &[],
    },
    TimelineEvent {
        title: "Facebook SWE",
        time: "February 2022",
        image: Some("/timeline/flow.webp"),
        category: Category::Work,
        detail: Some("Working on Flow's type system, IDE, and performance."),
        links: &[
            Link { name: "Website", url: "https://flow.org" },
            Link { name: "GitHub", url: "https://github.com/facebook/flow" },
        ],
    },
    TimelineEvent {
        title: "Graduated from Cornell University, Again",
        time: "December 2021",
        image: Some("/fan-arts/graduation-sam.webp"),
        category: Category::Education,
        detail: Some("Graduated as an M.Eng student."),
        links: &[],
    },
    TimelineEvent {
        title: "samlang emits WASM code",
        time: "October 2021",
        image: None,
        category: Category::Language,
        detail: None,
        links: &[Link {
            name: "Blog Post",
            url: "https://blog.developersam.com/2021/10/29/samlang-wasm-backend",
        }],
    },
    TimelineEvent {
        title: "Facebook SWE Intern",
        time: "June 2021",
        image: Some("/timeline/flow.webp"),
        category: Category::Work,
        detail: Some("Working on the Flow team. Implemented the extraction code action for Flow LSP."),
        links: &[
            Link { name: "Website", url: "https://flow.org" },
            Link { name: "GitHub", url: "https://github.com/facebook/flow" },
            Link { name: "Tech Talk", url: "https://developersam.com/flow-ide-presentation.pdf" },
        ],
    },
    TimelineEvent {
        title: "Cornell DTI Software Developer",
        time: "September 2021",
        image: Some("/timeline/courseplan-promotion.png"),
        category: Category::Work,
        detail: Some("Developer on CoursePlan"),
        links: &[
            Link { name: "Product", url: "https://courseplan.io" },
            Link { name: "GitHub", url: "https://github.com/cornell-dti/course-plan" },
            Link { name: "Tech Talk", url: "https://developersam.com/courseplan-requirement.pdf" },
        ],
    },
    TimelineEvent {
        title: "Graduated from Cornell University",
        time: "May 2021",
        image: Some("/fan-arts/graduation-sam.webp"),
        category: Category::Education,
        detail: Some("Graduated as an undergrad."),
        links: &[],
    },
    TimelineEvent {
        title: "samlang emits LLVM code",
        time: "January 2021",
        image: None,
        category: Category::Language,
        detail: None,
        links: &[Link {
            name: "Blog Post",
            url: "https://blog.developersam.com/2021/01/24/samlang-llvm-backend",
        }],
    },
    TimelineEvent {
        title: "samlang rewritten in TypeScript",
        time: "August 2020",
        image: None,
        category: Category::Language,
        detail: None,
        links: &[Link {
            name: "Blog Post",
            url: "https://blog.developersam.com/2020/08/30/samlang-ts-rewrite",
        }],
    },
    TimelineEvent {
        title: "Facebook SWE Intern",
        time: "June 2020",
        image: Some("/timeline/fb-sticker-pen.webp"),
        category: Category::Work,
        detail: Some("Working on the Feed Expressions team."),
        links: &[],
    },
    TimelineEvent {
        title: "mini-react",
        time: "May 2020",
        image: Some("/projects/mini-react.webp"),
        category: Category::Project,
        detail: Some(
            "A simplified version of the React runtime with useState and useEffect hooks, built from scratch.",
        ),
        links: &[
            Link { name: "GitHub Repo", url: "https://github.com/SamChou19815/mini-react" },
            Link { name: "Demo", url: "https://mini-react.developersam.com" },
            Link { name: "Slides", url: "https://developersam.com/build-simplified-react.pdf" },
        ],
    },
    TimelineEvent {
        title: "Tech Talk: Build your programming language",
        time: "March 2020",
        image: None,
        category: Category::Milestone,
        detail: Some("A tutorial of making a simple programming language derived from lambda-calculus."),
        links: &[Link {
            name: "Slides",
            url: "https://developersam.com/build-your-own-programming-language.pdf",
        }],
    },
    TimelineEvent {
        title: "Cornell DTI Developer Lead",
        time: "June 2019",
        image: None,
        category: Category::Work,
        detail: None,
        links: &[Link { name: "Cornell DTI Website", url: "https://www.cornelldti.org" }],
    },
    TimelineEvent {
        title: "Facebook SWE Intern",
        time: "May 2019",
        image: Some("/timeline/pyre-check.webp"),
        category: Category::Work,
        detail: Some("Working on the Pyre team."),
        links: &[
            Link { name: "Website", url: "https://pyre-check.org" },
            Link { name: "GitHub", url: "https://github.com/facebook/pyre-check" },
        ],
    },
    TimelineEvent {
        title: "Website Redesign v3.0",
        time: "March 2019",
        image: Some("/timeline/website-v3.webp"),
        category: Category::Milestone,
        detail: Some("This is the first version that includes a samlang program about myself."),
        links: &[Link {
            name: "Archive",
            url: "https://web.archive.org/web/20200331050322/https://developersam.com/",
        }],
    },
    TimelineEvent {
        title: "Cornell DTI TPM",
        time: "January 2019",
        image: None,
        category: Category::Work,
        detail: Some("Technical product manager of the Samwise subteam."),
        links: &[Link { name: "Cornell DTI Website", url: "https://www.cornelldti.org" }],
    },
    TimelineEvent {
        title: "samlang",
        time: "January 2019",
        image: Some("/projects/samlang.webp"),
        category: Category::Language,
        detail: Some("Sam's programming language with full type-inference."),
        links: &[
            Link { name: "GitHub Repo", url: "https://github.com/SamChou19815/samlang" },
            Link { name: "Docs", url: "https://samlang.io" },
        ],
    },
    TimelineEvent {
        title: "Cornell DTI Software Developer",
        time: "September 2018",
        image: Some("/projects/samwise.webp"),
        category: Category::Work,
        detail: Some("Frontend Developer of Samwise"),
        links: &[Link { name: "GitHub Repo", url: "https://github.com/cornell-dti/samwise" }],
    },
    TimelineEvent {
        title: "Critter Compiler",
        time: "August 2018",
        image: Some("/timeline/critter-compiler.webp"),
        category: Category::Project,
        detail: None,
        links: &[
            Link { name: "GitHub Repo", url: "https://github.com/SamChou19815/primitivize" },
            Link {
                name: "Blog Post",
                url: "https://blog.developersam.com/2018/08/27/cw-turing-complete/",
            },
        ],
    },
    TimelineEvent {
        title: "Website Redesign v2",
        time: "July 2018",
        image: Some("/timeline/website-v2.webp"),
        category: Category::Milestone,
        detail: None,
        links: &[Link {
            name: "Archive",
            url: "https://web.archive.org/web/20190102202556/https://developersam.com/",
        }],
    },
    TimelineEvent {
        title: "SAMPL",
        time: "June 2018",
        image: Some("/timeline/sampl.webp"),
        category: Category::Language,
        detail: Some("Sam's first programming language. Archived in favor of samlang."),
        links: &[
            Link { name: "GitHub Repo", url: "https://github.com/SamChou19815/sampl" },
            Link {
                name: "Blog Post",
                url: "https://blog.developersam.com/2018/06/15/sampl-alpha-design-choices/",
            },
        ],
    },
    TimelineEvent {
        title: "CS 2112 Critter World",
        time: "December 2017",
        image: Some("/timeline/critter-world.webp"),
        category: Category::Project,
        detail: Some(
            "Sam's critter world implementation. Used as course staff reference solution in Fall 2018 and Fall 2019.",
        ),
        links: &[],
    },
    TimelineEvent {
        title: "Entered Cornell University",
        time: "August 2017",
        image: None,
        category: Category::Education,
        detail: None,
        links: &[],
    },
    TimelineEvent {
        title: "TEN",
        time: "July 2017",
        image: Some("/projects/ten.webp"),
        category: Category::Project,
        detail: None,
        links: &[Link { name: "GitHub Repo", url: "https://github.com/SamChou19815/ten-golang" }],
    },
    TimelineEvent {
        title: "Graduated from High School",
        time: "June 2016",
        image: None,
        category: Category::Education,
        detail: None,
        links: &[Link {
            name: "Blog: CS in High Schools",
            url: "https://blog.developersam.com/2018/12/31/cs-in-high-schools/",
        }],
    },
    TimelineEvent {
        title: "Computerization Club President",
        time: "September 2015",
        image: Some("/timeline/sam-later.webp"),
        category: Category::Milestone,
        detail: None,
        links: &[],
    },
    TimelineEvent {
        title: "Computerization Software Developer",
        time: "February 2015",
        image: None,
        category: Category::Milestone,
        detail: None,
        links: &[],
    },
    TimelineEvent {
        title: "Bought developersam.com",
        time: "February 2015",
        image: None,
        category: Category::Milestone,
        detail: Some("Check how it looks initially!"),
        links: &[Link {
            name: "Initial Version",
            url: "https://web.archive.org/web/20160506203739/http://www.developersam.com/",
        }],
    },
];
