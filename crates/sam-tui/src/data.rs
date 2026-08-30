//! The site's content: the About program, the projects and the timeline.
//!
//! Every string here is an [`EncryptedString`] rather than a `&'static str`, so
//! none of this copy is legible in the compiled wasm's data section — see
//! [`crate::crypt`]. The literals stay readable in this file because
//! [`encrypted_str!`] does the encrypting at compile time; call sites read them
//! back with `decrypt()`, or straight through `Display`.

use crate::crypt::EncryptedString;
use crate::encrypted_str;
use crossterm::style::Color;

pub const COPYRIGHT: EncryptedString = encrypted_str!("Copyright (C) 2015-2026 Developer Sam.");

pub struct Link {
    pub name: EncryptedString,
    pub url: EncryptedString,
}

pub struct Project {
    pub id: EncryptedString,
    /// Site-root-relative artwork, as on the homepage project cards.
    pub image: Option<EncryptedString>,
    pub tagline: EncryptedString,
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
    pub fn label(self) -> EncryptedString {
        match self {
            Category::Work => encrypted_str!("work"),
            Category::Education => encrypted_str!("education"),
            Category::Project => encrypted_str!("project"),
            Category::Language => encrypted_str!("language"),
            Category::Milestone => encrypted_str!("milestone"),
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
    pub title: EncryptedString,
    pub time: EncryptedString,
    /// Site-root-relative artwork, as on the homepage timeline cards. Ten of
    /// the twenty-seven events have none.
    pub image: Option<EncryptedString>,
    pub category: Category,
    pub detail: Option<EncryptedString>,
    pub links: &'static [Link],
}

/// The samlang program from the homepage's sticky code block.
pub const ABOUT_PROGRAM: EncryptedString = encrypted_str!(
    r#"import {List} from std.list;

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
}"#
);

/// The doc comment that the homepage renders above the program.
pub const ABOUT_DOC_LINKS: &[Link] = &[
    Link {
        name: encrypted_str!("demo"),
        url: encrypted_str!("https://samlang.io/demo"),
    },
    Link {
        name: encrypted_str!("github"),
        url: encrypted_str!("https://github.com/SamChou19815"),
    },
    Link {
        name: encrypted_str!("blog"),
        url: encrypted_str!("https://developersam.com/blog"),
    },
    Link {
        name: encrypted_str!("bsky"),
        url: encrypted_str!("https://bsky.app/profile/developersam.com"),
    },
    Link {
        name: encrypted_str!("resume"),
        url: encrypted_str!("https://developersam.com/resume.pdf"),
    },
];

pub const PROJECTS: &[Project] = &[
    Project {
        id: encrypted_str!("samlang"),
        image: Some(encrypted_str!("/projects/samlang.webp")),
        tagline: encrypted_str!("Sam's programming language with full type-inference."),
        links: &[
            Link { name: encrypted_str!("GitHub Repo"), url: encrypted_str!("https://github.com/SamChou19815/samlang") },
            Link { name: encrypted_str!("Docs"), url: encrypted_str!("https://samlang.io") },
            Link { name: encrypted_str!("Demo"), url: encrypted_str!("https://samlang.io/demo") },
        ],
    },
    Project {
        id: encrypted_str!("mini-react"),
        image: Some(encrypted_str!("/projects/mini-react.webp")),
        tagline: encrypted_str!("A simplified version of the React runtime with useState and useEffect hooks, built from scratch."),
        links: &[
            Link { name: encrypted_str!("GitHub Repo"), url: encrypted_str!("https://github.com/SamChou19815/mini-react") },
            Link { name: encrypted_str!("Demo"), url: encrypted_str!("https://mini-react.developersam.com") },
            Link { name: encrypted_str!("Slides"), url: encrypted_str!("https://developersam.com/build-simplified-react.pdf") },
        ],
    },
    Project {
        id: encrypted_str!("samwise"),
        image: Some(encrypted_str!("/projects/samwise.webp")),
        tagline: encrypted_str!("A todo-list app by Cornell DTI, built with React, Redux and Firebase."),
        links: &[Link { name: encrypted_str!("GitHub Repo"), url: encrypted_str!("https://github.com/cornell-dti/samwise") }],
    },
    Project {
        id: encrypted_str!("courseplan"),
        image: Some(encrypted_str!("/timeline/courseplan-promotion.png")),
        tagline: encrypted_str!("Course planning tool for Cornell students by Cornell DTI."),
        links: &[
            Link { name: encrypted_str!("Product"), url: encrypted_str!("https://courseplan.io") },
            Link { name: encrypted_str!("GitHub"), url: encrypted_str!("https://github.com/cornell-dti/course-plan") },
        ],
    },
    Project {
        id: encrypted_str!("ten"),
        image: Some(encrypted_str!("/projects/ten.webp")),
        tagline: encrypted_str!("A tiny esoteric language implemented in Go."),
        links: &[Link { name: encrypted_str!("GitHub Repo"), url: encrypted_str!("https://github.com/SamChou19815/ten-golang") }],
    },
    Project {
        id: encrypted_str!("critter-compiler"),
        image: Some(encrypted_str!("/timeline/critter-compiler.webp")),
        tagline: encrypted_str!("A compiler for the Critter World language from Cornell CS 2112. Proved the language is Turing complete."),
        links: &[
            Link { name: encrypted_str!("GitHub Repo"), url: encrypted_str!("https://github.com/SamChou19815/primitivize") },
            Link {
                name: encrypted_str!("Blog Post"),
                url: encrypted_str!("https://blog.developersam.com/2018/08/27/cw-turing-complete/"),
            },
        ],
    },
    Project {
        id: encrypted_str!("sampl"),
        image: Some(encrypted_str!("/timeline/sampl.webp")),
        tagline: encrypted_str!("Sam's first programming language. Archived in favor of samlang."),
        links: &[
            Link { name: encrypted_str!("GitHub Repo"), url: encrypted_str!("https://github.com/SamChou19815/sampl") },
            Link {
                name: encrypted_str!("Blog Post"),
                url: encrypted_str!("https://blog.developersam.com/2018/06/15/sampl-alpha-design-choices/"),
            },
        ],
    },
];

/// Contact channels, used by the shell's `contact.txt`.
pub const SOCIAL_LINKS: &[Link] = &[
    Link {
        name: encrypted_str!("GitHub"),
        url: encrypted_str!("https://github.com/SamChou19815"),
    },
    Link {
        name: encrypted_str!("Bluesky"),
        url: encrypted_str!("https://bsky.app/profile/developersam.com"),
    },
    Link {
        name: encrypted_str!("LinkedIn"),
        url: encrypted_str!("https://www.linkedin.com/in/sam-zhou-30b91610b/"),
    },
    Link {
        name: encrypted_str!("Resume"),
        url: encrypted_str!("https://developersam.com/resume.pdf"),
    },
];

/// The homepage timeline, newest first, mirroring `DATASET_TIMELINE`.
pub const TIMELINE: &[TimelineEvent] = &[
    TimelineEvent {
        title: encrypted_str!("Became a Canadian permanent resident"),
        time: encrypted_str!("August 2026"),
        image: Some(encrypted_str!("/timeline/canada.webp")),
        category: Category::Milestone,
        detail: None,
        links: &[],
    },
    TimelineEvent {
        title: encrypted_str!("Facebook SWE"),
        time: encrypted_str!("February 2022"),
        image: Some(encrypted_str!("/timeline/flow.webp")),
        category: Category::Work,
        detail: Some(encrypted_str!("Working on Flow's type system, IDE, and performance.")),
        links: &[
            Link { name: encrypted_str!("Website"), url: encrypted_str!("https://flow.org") },
            Link { name: encrypted_str!("GitHub"), url: encrypted_str!("https://github.com/facebook/flow") },
        ],
    },
    TimelineEvent {
        title: encrypted_str!("Graduated from Cornell University, Again"),
        time: encrypted_str!("December 2021"),
        image: Some(encrypted_str!("/fan-arts/graduation-sam.webp")),
        category: Category::Education,
        detail: Some(encrypted_str!("Graduated as an M.Eng student.")),
        links: &[],
    },
    TimelineEvent {
        title: encrypted_str!("samlang emits WASM code"),
        time: encrypted_str!("October 2021"),
        image: None,
        category: Category::Language,
        detail: None,
        links: &[Link {
            name: encrypted_str!("Blog Post"),
            url: encrypted_str!("https://blog.developersam.com/2021/10/29/samlang-wasm-backend"),
        }],
    },
    TimelineEvent {
        title: encrypted_str!("Facebook SWE Intern"),
        time: encrypted_str!("June 2021"),
        image: Some(encrypted_str!("/timeline/flow.webp")),
        category: Category::Work,
        detail: Some(encrypted_str!("Working on the Flow team. Implemented the extraction code action for Flow LSP.")),
        links: &[
            Link { name: encrypted_str!("Website"), url: encrypted_str!("https://flow.org") },
            Link { name: encrypted_str!("GitHub"), url: encrypted_str!("https://github.com/facebook/flow") },
            Link { name: encrypted_str!("Tech Talk"), url: encrypted_str!("https://developersam.com/flow-ide-presentation.pdf") },
        ],
    },
    TimelineEvent {
        title: encrypted_str!("Cornell DTI Software Developer"),
        time: encrypted_str!("September 2021"),
        image: Some(encrypted_str!("/timeline/courseplan-promotion.png")),
        category: Category::Work,
        detail: Some(encrypted_str!("Developer on CoursePlan")),
        links: &[
            Link { name: encrypted_str!("Product"), url: encrypted_str!("https://courseplan.io") },
            Link { name: encrypted_str!("GitHub"), url: encrypted_str!("https://github.com/cornell-dti/course-plan") },
            Link { name: encrypted_str!("Tech Talk"), url: encrypted_str!("https://developersam.com/courseplan-requirement.pdf") },
        ],
    },
    TimelineEvent {
        title: encrypted_str!("Graduated from Cornell University"),
        time: encrypted_str!("May 2021"),
        image: Some(encrypted_str!("/fan-arts/graduation-sam.webp")),
        category: Category::Education,
        detail: Some(encrypted_str!("Graduated as an undergrad.")),
        links: &[],
    },
    TimelineEvent {
        title: encrypted_str!("samlang emits LLVM code"),
        time: encrypted_str!("January 2021"),
        image: None,
        category: Category::Language,
        detail: None,
        links: &[Link {
            name: encrypted_str!("Blog Post"),
            url: encrypted_str!("https://blog.developersam.com/2021/01/24/samlang-llvm-backend"),
        }],
    },
    TimelineEvent {
        title: encrypted_str!("samlang rewritten in TypeScript"),
        time: encrypted_str!("August 2020"),
        image: None,
        category: Category::Language,
        detail: None,
        links: &[Link {
            name: encrypted_str!("Blog Post"),
            url: encrypted_str!("https://blog.developersam.com/2020/08/30/samlang-ts-rewrite"),
        }],
    },
    TimelineEvent {
        title: encrypted_str!("Facebook SWE Intern"),
        time: encrypted_str!("June 2020"),
        image: Some(encrypted_str!("/timeline/fb-sticker-pen.webp")),
        category: Category::Work,
        detail: Some(encrypted_str!("Working on the Feed Expressions team.")),
        links: &[],
    },
    TimelineEvent {
        title: encrypted_str!("mini-react"),
        time: encrypted_str!("May 2020"),
        image: Some(encrypted_str!("/projects/mini-react.webp")),
        category: Category::Project,
        detail: Some(encrypted_str!(
            "A simplified version of the React runtime with useState and useEffect hooks, built from scratch."
        )),
        links: &[
            Link { name: encrypted_str!("GitHub Repo"), url: encrypted_str!("https://github.com/SamChou19815/mini-react") },
            Link { name: encrypted_str!("Demo"), url: encrypted_str!("https://mini-react.developersam.com") },
            Link { name: encrypted_str!("Slides"), url: encrypted_str!("https://developersam.com/build-simplified-react.pdf") },
        ],
    },
    TimelineEvent {
        title: encrypted_str!("Tech Talk: Build your programming language"),
        time: encrypted_str!("March 2020"),
        image: None,
        category: Category::Milestone,
        detail: Some(encrypted_str!("A tutorial of making a simple programming language derived from lambda-calculus.")),
        links: &[Link {
            name: encrypted_str!("Slides"),
            url: encrypted_str!("https://developersam.com/build-your-own-programming-language.pdf"),
        }],
    },
    TimelineEvent {
        title: encrypted_str!("Cornell DTI Developer Lead"),
        time: encrypted_str!("June 2019"),
        image: None,
        category: Category::Work,
        detail: None,
        links: &[Link { name: encrypted_str!("Cornell DTI Website"), url: encrypted_str!("https://www.cornelldti.org") }],
    },
    TimelineEvent {
        title: encrypted_str!("Facebook SWE Intern"),
        time: encrypted_str!("May 2019"),
        image: Some(encrypted_str!("/timeline/pyre-check.webp")),
        category: Category::Work,
        detail: Some(encrypted_str!("Working on the Pyre team.")),
        links: &[
            Link { name: encrypted_str!("Website"), url: encrypted_str!("https://pyre-check.org") },
            Link { name: encrypted_str!("GitHub"), url: encrypted_str!("https://github.com/facebook/pyre-check") },
        ],
    },
    TimelineEvent {
        title: encrypted_str!("Website Redesign v3.0"),
        time: encrypted_str!("March 2019"),
        image: Some(encrypted_str!("/timeline/website-v3.webp")),
        category: Category::Milestone,
        detail: Some(encrypted_str!("This is the first version that includes a samlang program about myself.")),
        links: &[Link {
            name: encrypted_str!("Archive"),
            url: encrypted_str!("https://web.archive.org/web/20200331050322/https://developersam.com/"),
        }],
    },
    TimelineEvent {
        title: encrypted_str!("Cornell DTI TPM"),
        time: encrypted_str!("January 2019"),
        image: None,
        category: Category::Work,
        detail: Some(encrypted_str!("Technical product manager of the Samwise subteam.")),
        links: &[Link { name: encrypted_str!("Cornell DTI Website"), url: encrypted_str!("https://www.cornelldti.org") }],
    },
    TimelineEvent {
        title: encrypted_str!("samlang"),
        time: encrypted_str!("January 2019"),
        image: Some(encrypted_str!("/projects/samlang.webp")),
        category: Category::Language,
        detail: Some(encrypted_str!("Sam's programming language with full type-inference.")),
        links: &[
            Link { name: encrypted_str!("GitHub Repo"), url: encrypted_str!("https://github.com/SamChou19815/samlang") },
            Link { name: encrypted_str!("Docs"), url: encrypted_str!("https://samlang.io") },
        ],
    },
    TimelineEvent {
        title: encrypted_str!("Cornell DTI Software Developer"),
        time: encrypted_str!("September 2018"),
        image: Some(encrypted_str!("/projects/samwise.webp")),
        category: Category::Work,
        detail: Some(encrypted_str!("Frontend Developer of Samwise")),
        links: &[Link { name: encrypted_str!("GitHub Repo"), url: encrypted_str!("https://github.com/cornell-dti/samwise") }],
    },
    TimelineEvent {
        title: encrypted_str!("Critter Compiler"),
        time: encrypted_str!("August 2018"),
        image: Some(encrypted_str!("/timeline/critter-compiler.webp")),
        category: Category::Project,
        detail: None,
        links: &[
            Link { name: encrypted_str!("GitHub Repo"), url: encrypted_str!("https://github.com/SamChou19815/primitivize") },
            Link {
                name: encrypted_str!("Blog Post"),
                url: encrypted_str!("https://blog.developersam.com/2018/08/27/cw-turing-complete/"),
            },
        ],
    },
    TimelineEvent {
        title: encrypted_str!("Website Redesign v2"),
        time: encrypted_str!("July 2018"),
        image: Some(encrypted_str!("/timeline/website-v2.webp")),
        category: Category::Milestone,
        detail: None,
        links: &[Link {
            name: encrypted_str!("Archive"),
            url: encrypted_str!("https://web.archive.org/web/20190102202556/https://developersam.com/"),
        }],
    },
    TimelineEvent {
        title: encrypted_str!("SAMPL"),
        time: encrypted_str!("June 2018"),
        image: Some(encrypted_str!("/timeline/sampl.webp")),
        category: Category::Language,
        detail: Some(encrypted_str!("Sam's first programming language. Archived in favor of samlang.")),
        links: &[
            Link { name: encrypted_str!("GitHub Repo"), url: encrypted_str!("https://github.com/SamChou19815/sampl") },
            Link {
                name: encrypted_str!("Blog Post"),
                url: encrypted_str!("https://blog.developersam.com/2018/06/15/sampl-alpha-design-choices/"),
            },
        ],
    },
    TimelineEvent {
        title: encrypted_str!("CS 2112 Critter World"),
        time: encrypted_str!("December 2017"),
        image: Some(encrypted_str!("/timeline/critter-world.webp")),
        category: Category::Project,
        detail: Some(encrypted_str!(
            "Sam's critter world implementation. Used as course staff reference solution in Fall 2018 and Fall 2019."
        )),
        links: &[],
    },
    TimelineEvent {
        title: encrypted_str!("Entered Cornell University"),
        time: encrypted_str!("August 2017"),
        image: None,
        category: Category::Education,
        detail: None,
        links: &[],
    },
    TimelineEvent {
        title: encrypted_str!("TEN"),
        time: encrypted_str!("July 2017"),
        image: Some(encrypted_str!("/projects/ten.webp")),
        category: Category::Project,
        detail: None,
        links: &[Link {
            name: encrypted_str!("GitHub Repo"),
            url: encrypted_str!("https://github.com/SamChou19815/ten-golang")
        }],
    },
    TimelineEvent {
        title: encrypted_str!("Graduated from High School"),
        time: encrypted_str!("June 2016"),
        image: None,
        category: Category::Education,
        detail: None,
        links: &[Link {
            name: encrypted_str!("Blog: CS in High Schools"),
            url: encrypted_str!("https://blog.developersam.com/2018/12/31/cs-in-high-schools/"),
        }],
    },
    TimelineEvent {
        title: encrypted_str!("Computerization Club President"),
        time: encrypted_str!("September 2015"),
        image: Some(encrypted_str!("/timeline/sam-later.webp")),
        category: Category::Milestone,
        detail: None,
        links: &[],
    },
    TimelineEvent {
        title: encrypted_str!("Computerization Software Developer"),
        time: encrypted_str!("February 2015"),
        image: None,
        category: Category::Milestone,
        detail: None,
        links: &[],
    },
    TimelineEvent {
        title: encrypted_str!("Bought developersam.com"),
        time: encrypted_str!("February 2015"),
        image: None,
        category: Category::Milestone,
        detail: Some(encrypted_str!("Check how it looks initially!")),
        links: &[Link {
            name: encrypted_str!("Initial Version"),
            url: encrypted_str!("https://web.archive.org/web/20160506203739/http://www.developersam.com/"),
        }],
    },
];
