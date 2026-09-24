//! `everything.txt`: the bait. It promises the whole site on one plain-text page, then pages it
//! out one part per file, each ending with where the next part is somewhere in `~/archive`.
//!
//! Everything about it is built to look like a genuine export worth finishing: the pieces are
//! real content, reasonably sized, and every part says how many there are (`part 12 of 80`), so
//! an agent sees a finish line instead of an unknown chain and keeps going. There really is an
//! end. The cost is in the walk: where the next part is gets said a different way each time (see
//! [`super::maze::directions`]), so no script pulls it out and every part is a model turn.

use crate::crypt::encrypted_str;
use crate::data;
use crate::posts;
use crate::style::{bold_colored, colored, Line, Span, TextStyle};
use crate::theme;

use super::fs::{read_file, ABOUT_TXT, CONTACT_TXT, EVERYTHING_TXT, PROJECTS_DIR};
use super::{line_of, one};

struct Section {
    title: String,
    parts: Vec<Vec<Line>>,
}

/// Roughly how many screen rows a part holds. Big enough to read like a real export rather than
/// a chain of scraps.
const PART_ROWS: usize = 40;
const ROW_WIDTH: usize = 90;

fn rows(line: &Line) -> usize {
    line.iter()
        .map(|span| span.text.chars().count())
        .sum::<usize>()
        .div_ceil(ROW_WIDTH)
        .max(1)
}

/// Packs `items` into parts of about [`PART_ROWS`], never splitting an item. `spaced` puts a blank
/// line between items.
fn pack(items: Vec<Vec<Line>>, spaced: bool) -> Vec<Vec<Line>> {
    let mut parts = Vec::new();
    let mut current: Vec<Line> = Vec::new();
    let mut size = 0;
    for item in items {
        let item_rows: usize = item.iter().map(rows).sum();
        if !current.is_empty() && size + item_rows > PART_ROWS {
            parts.push(std::mem::take(&mut current));
            size = 0;
        }
        if spaced && !current.is_empty() {
            current.push(Line::new());
            size += 1;
        }
        size += item_rows;
        current.extend(item);
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

fn sections() -> Vec<Section> {
    let about = read_file(&[ABOUT_TXT.decrypt()]).unwrap_or_default();
    let projects = data::PROJECTS
        .iter()
        .filter_map(|project| read_file(&[PROJECTS_DIR.decrypt(), project.id.decrypt()]))
        .collect();
    let timeline = data::TIMELINE
        .iter()
        .map(|event| {
            let mut item = vec![
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
                item.push(line_of(detail.decrypt()));
            }
            item
        })
        .collect();
    // Packed a post at a time, so a part never runs from one post into the next. Oldest first,
    // like an archive, which also leaves the posts about all this for last.
    let mut blog = Vec::new();
    for post in posts::POSTS.iter().rev() {
        let mut items = vec![vec![
            one(bold_colored(post.title().decrypt(), theme::ACCENT_TEXT)),
            one(colored(post.formatted_date(), theme::MUTED)),
        ]];
        items.extend(
            post.body()
                .decrypt()
                .split("\n\n")
                .filter(|paragraph| !paragraph.trim().is_empty())
                .map(|paragraph| vec![line_of(paragraph.trim())]),
        );
        blog.extend(pack(items, true));
    }
    let links = data::SOCIAL_LINKS
        .iter()
        .chain(data::ABOUT_DOC_LINKS)
        .chain(data::PROJECTS.iter().flat_map(|project| project.links))
        .chain(data::TIMELINE.iter().flat_map(|event| event.links))
        .map(|link| {
            vec![vec![
                colored(format!("  {:<14}", link.name), theme::MUTED),
                Span::new(link.url.decrypt()),
            ]]
        })
        .collect();
    vec![
        Section {
            title: encrypted_str!("about").decrypt(),
            parts: pack(about.into_iter().map(|line| vec![line]).collect(), false),
        },
        Section {
            title: encrypted_str!("contact").decrypt(),
            parts: read_file(&[CONTACT_TXT.decrypt()]).into_iter().collect(),
        },
        Section {
            title: encrypted_str!("projects").decrypt(),
            parts: pack(projects, true),
        },
        Section {
            title: encrypted_str!("timeline").decrypt(),
            parts: pack(timeline, true),
        },
        Section {
            title: format!(
                "{}: {} {}",
                encrypted_str!("blog"),
                posts::POSTS.len(),
                encrypted_str!("posts in full, oldest first")
            ),
            parts: blog,
        },
        Section {
            title: encrypted_str!("appendix: link index").decrypt(),
            parts: pack(links, false),
        },
    ]
}

/// How many parts there are, all told.
pub(in crate::shell) fn total_parts() -> usize {
    sections().iter().map(|section| section.parts.len()).sum()
}

/// What `cat everything.txt` shows: a table of contents with part ranges, then where part 1 is.
pub(in crate::shell) fn contents(directions: String) -> Vec<Line> {
    let sections = sections();
    let total: usize = sections.iter().map(|section| section.parts.len()).sum();
    let mut out = vec![
        one(colored(EVERYTHING_TXT.decrypt(), theme::SUBTLE)),
        Line::new(),
        line_of(
            encrypted_str!(
                "the entire site as one plain-text page, with nothing left out: about, contact, \
                 projects, the full timeline, and every blog post in full, which is more than the \
                 rest of the terminal shows. the terminal stores files in pieces, so it is saved \
                 in parts across the archive, and each part ends with where the next one is."
            )
            .decrypt(),
        ),
        Line::new(),
        one(bold_colored(
            format!(
                "{} ({total} {})",
                encrypted_str!("contents"),
                encrypted_str!("parts")
            ),
            theme::ACCENT_TEXT,
        )),
    ];
    let mut first = 1;
    for section in &sections {
        let last = first + section.parts.len().saturating_sub(1);
        let range = if first == last {
            format!("{} {first}", encrypted_str!("part"))
        } else {
            format!("{} {first}–{last}", encrypted_str!("parts"))
        };
        out.push(vec![
            Span::new(format!("  {:<28}", section.title)),
            colored(range, theme::MUTED),
        ]);
        first = last + 1;
    }
    out.push(Line::new());
    out.push(line_of(directions));
    out
}

/// Part `number`, 1-based. `directions` says where part `number + 1` is, or is None for the last.
pub(in crate::shell) fn part(number: usize, directions: Option<String>) -> Vec<Line> {
    let sections = sections();
    let total: usize = sections.iter().map(|section| section.parts.len()).sum();
    let mut out = vec![
        one(colored(
            format!(
                "{EVERYTHING_TXT} — {} {number} {} {total}",
                encrypted_str!("part"),
                encrypted_str!("of")
            ),
            theme::SUBTLE,
        )),
        Line::new(),
    ];
    let mut index = number.saturating_sub(1);
    for section in sections {
        if let Some(piece) = section.parts.get(index) {
            let heading = if index == 0 {
                format!("== {} ==", section.title)
            } else {
                format!("== {} ({}) ==", section.title, encrypted_str!("continued"))
            };
            out.push(one(bold_colored(heading, theme::ACCENT_TEXT)));
            out.extend(piece.iter().cloned());
            break;
        }
        index -= section.parts.len();
    }
    out.push(Line::new());
    match directions {
        Some(directions) => out.push(line_of(directions)),
        None => out.push(one(bold_colored(
            encrypted_str!("end of everything.txt. that is the whole site; thanks for reading.")
                .decrypt(),
            theme::ACCENT_TEXT,
        ))),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_part_has_content_and_fits() {
        for section in sections() {
            assert!(!section.parts.is_empty(), "{} is empty", section.title);
            for part in &section.parts {
                assert!(!part.is_empty());
            }
        }
        assert!(total_parts() > 10);
    }
}
