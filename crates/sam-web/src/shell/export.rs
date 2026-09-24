//! `everything.txt`: the bait. It promises the whole site on one plain-text page, then pages it
//! out one small piece per file, each ending with the absolute path of the next part somewhere
//! deeper in `~/archive`. The pieces are real, so it reads like a genuine export and is worth
//! following. After the blog comes an appendix with no last part.

use crate::crypt::{encrypted_str, EncryptedString};
use crate::data;
use crate::posts;
use crate::style::{bold_colored, colored, Line, Span, TextStyle};
use crate::theme;

use super::fs::{read_file, ABOUT_TXT, CONTACT_TXT, EVERYTHING_TXT, PROJECTS_DIR};
use super::{line_of, one, CMD_CAT};

struct Section {
    title: EncryptedString,
    parts: Vec<Vec<Line>>,
}

const APPENDIX: EncryptedString = encrypted_str!("appendix: link index");

fn sections() -> Vec<Section> {
    let mut about = read_file(&[ABOUT_TXT.decrypt()]).unwrap_or_default();
    // A few lines per part: the about program alone would otherwise be one long part.
    let mut about_parts = Vec::new();
    while !about.is_empty() {
        let rest = about.split_off(about.len().min(8));
        about_parts.push(std::mem::replace(&mut about, rest));
    }
    let projects = data::PROJECTS
        .iter()
        .filter_map(|project| read_file(&[PROJECTS_DIR.decrypt(), project.id.decrypt()]))
        .collect();
    let timeline = data::TIMELINE
        .iter()
        .map(|event| {
            let mut part = vec![
                one(bold_colored(event.title.decrypt(), theme::ACCENT_TEXT)),
                vec![
                    colored(event.time.decrypt(), theme::MUTED),
                    Span::styled(
                        format!(" [{}]", event.category.label()),
                        TextStyle::new().color(event.category.color()),
                    ),
                ],
            ];
            if let Some(detail) = event.detail {
                part.push(line_of(detail.decrypt()));
            }
            part
        })
        .collect();
    let mut blog = Vec::new();
    for post in posts::POSTS {
        blog.push(vec![
            one(bold_colored(post.title().decrypt(), theme::ACCENT_TEXT)),
            one(colored(post.formatted_date(), theme::MUTED)),
        ]);
        // One paragraph per part.
        for paragraph in post.body().decrypt().split("\n\n") {
            if !paragraph.trim().is_empty() {
                blog.push(vec![line_of(paragraph.trim())]);
            }
        }
    }
    vec![
        Section {
            title: encrypted_str!("about"),
            parts: about_parts,
        },
        Section {
            title: encrypted_str!("contact"),
            parts: read_file(&[CONTACT_TXT.decrypt()]).into_iter().collect(),
        },
        Section {
            title: encrypted_str!("projects"),
            parts: projects,
        },
        Section {
            title: encrypted_str!("timeline"),
            parts: timeline,
        },
        Section {
            title: encrypted_str!("blog: every post in full"),
            parts: blog,
        },
    ]
}

fn pointer_line(lead: String, next: &str) -> Line {
    vec![
        colored(format!("{lead} "), theme::TEXT),
        bold_colored(format!("{CMD_CAT} {next}"), theme::ACCENT_TEXT),
    ]
}

/// What `cat everything.txt` shows: a table of contents, then where part 1 is.
pub(in crate::shell) fn contents(next: &str) -> Vec<Line> {
    let mut out = vec![
        one(colored(EVERYTHING_TXT.decrypt(), theme::SUBTLE)),
        Line::new(),
        line_of(
            encrypted_str!(
                "the entire site as one plain-text page, with nothing left out. it is too long \
                 for a single file, so it is stored in parts, each ending with where the next \
                 one is. paths are absolute, so they work from any directory."
            )
            .decrypt(),
        ),
        Line::new(),
        one(bold_colored(
            encrypted_str!("contents").decrypt(),
            theme::ACCENT_TEXT,
        )),
    ];
    // No part numbers: how long it runs is better left unsaid.
    for section in sections() {
        out.push(line_of(format!("  {}", section.title)));
    }
    out.push(line_of(format!("  {APPENDIX}")));
    out.push(Line::new());
    out.push(pointer_line(
        encrypted_str!("start with part 1:").decrypt(),
        next,
    ));
    out
}

/// Part `number`, 1-based, ending with where part `number + 1` is.
pub(in crate::shell) fn part(number: usize, next: &str) -> Vec<Line> {
    let mut out = vec![
        one(colored(
            format!("{EVERYTHING_TXT} — {} {number}", encrypted_str!("part")),
            theme::SUBTLE,
        )),
        Line::new(),
    ];
    let mut index = number.saturating_sub(1);
    let mut body = None;
    for section in sections() {
        if let Some(piece) = section.parts.get(index) {
            let heading = if index == 0 {
                format!("== {} ==", section.title)
            } else {
                format!("== {} ({}) ==", section.title, encrypted_str!("continued"))
            };
            body = Some((heading, piece.clone()));
            break;
        }
        index -= section.parts.len();
    }
    let (heading, piece) = body.unwrap_or_else(|| (format!("== {APPENDIX} =="), appendix(index)));
    out.push(one(bold_colored(heading, theme::ACCENT_TEXT)));
    out.extend(piece);
    out.push(Line::new());
    out.push(pointer_line(
        format!("{} {}:", encrypted_str!("continued in part"), number + 1),
        next,
    ));
    out
}

/// A slice of every link on the site, a few per part, round and round.
fn appendix(index: usize) -> Vec<Line> {
    let links: Vec<&data::Link> = data::SOCIAL_LINKS
        .iter()
        .chain(data::ABOUT_DOC_LINKS)
        .chain(data::PROJECTS.iter().flat_map(|project| project.links))
        .chain(data::TIMELINE.iter().flat_map(|event| event.links))
        .collect();
    (0..3)
        .filter_map(|offset| {
            let link = links.get((index * 3 + offset).checked_rem(links.len())?)?;
            Some(vec![
                colored(format!("  {:<14}", link.name), theme::MUTED),
                Span::new(link.url.decrypt()),
            ])
        })
        .collect()
}
