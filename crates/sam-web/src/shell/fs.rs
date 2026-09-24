use crate::crypt::{encrypted_str, EncryptedString};
use crate::data;
use crate::highlight;
use crate::style::{bold_colored, colored, Line, Span, TextStyle};
use crate::theme;

use super::maze::{self, ARCHIVE_DIR};
use super::{one, CMD_CAT, CMD_LS};

pub(in crate::shell) const HOME_DIR: EncryptedString = encrypted_str!("/home/sam");
pub(in crate::shell) const PROJECTS_DIR: EncryptedString = encrypted_str!("projects");
pub(in crate::shell) const ABOUT_TXT: EncryptedString = encrypted_str!("about.txt");
pub(in crate::shell) const CONTACT_TXT: EncryptedString = encrypted_str!("contact.txt");
const README_MD: EncryptedString = encrypted_str!("readme.md");
const RESUME_PDF: EncryptedString = encrypted_str!("resume.pdf");
const TIMELINE_TXT: EncryptedString = encrypted_str!("timeline.txt");
/// Bait for bots, see [`super::Shell::open_export`]. Not listed, so people don't stumble on it.
pub(in crate::shell) const EVERYTHING_TXT: EncryptedString = encrypted_str!("everything.txt");

pub(in crate::shell) fn fs_entries(path: &[String]) -> Option<Vec<(String, bool)>> {
    match path {
        [] => Some(vec![
            (format!("{ARCHIVE_DIR}/"), true),
            (format!("{PROJECTS_DIR}/"), true),
            (ABOUT_TXT.decrypt(), false),
            (CONTACT_TXT.decrypt(), false),
            (README_MD.decrypt(), false),
            (RESUME_PDF.decrypt(), false),
            (TIMELINE_TXT.decrypt(), false),
        ]),
        [first, rest @ ..] if *first == ARCHIVE_DIR.decrypt() => maze::entries(rest),
        [only] if *only == PROJECTS_DIR.decrypt() => Some(
            data::PROJECTS
                .iter()
                .map(|project| (project.id.decrypt(), false))
                .collect(),
        ),
        _ => None,
    }
}

fn link_line(mut spans: Line, url: &str) -> Line {
    spans.push(Span::new(url).linked(url));
    spans
}

pub(in crate::shell) fn read_file(path: &[String]) -> Option<Vec<Line>> {
    match path {
        [file] if *file == README_MD.decrypt() => Some(vec![
            one(colored(
                encrypted_str!("Served by Rust, compiled to WASM").decrypt(),
                theme::SUBTLE,
            )),
            Line::new(),
            one(colored(
                encrypted_str!("run dev-sam to launch the app, or explore:").decrypt(),
                theme::TEXT,
            )),
            one(colored(
                format!("  {CMD_CAT} {ABOUT_TXT}"),
                theme::ACCENT_TEXT,
            )),
            one(colored(
                format!("  {CMD_LS} {PROJECTS_DIR}"),
                theme::ACCENT_TEXT,
            )),
            one(colored(
                format!("  {CMD_CAT} {TIMELINE_TXT}"),
                theme::ACCENT_TEXT,
            )),
            one(colored(
                format!("  {CMD_LS} {ARCHIVE_DIR}"),
                theme::ACCENT_TEXT,
            )),
        ]),
        [file] if *file == ABOUT_TXT.decrypt() => {
            let mut out: Vec<Line> = highlight::doc_comment_lines();
            out.push(Line::new());
            out.extend(highlight::program_lines());
            out.push(Line::new());
            for entry in data::ABOUT_DOC_LINKS {
                let url = entry.url.decrypt();
                out.push(link_line(
                    vec![bold_colored(
                        format!("@{}:", entry.name),
                        theme::ACCENT_TEXT,
                    )],
                    &url,
                ));
            }
            Some(out)
        }
        [file] if *file == CONTACT_TXT.decrypt() => {
            let mut out = Vec::new();
            for entry in data::SOCIAL_LINKS {
                let url = entry.url.decrypt();
                out.push(link_line(
                    vec![bold_colored(
                        format!("{:<10}", entry.name),
                        theme::ACCENT_TEXT,
                    )],
                    &url,
                ));
            }
            Some(out)
        }
        [file] if *file == RESUME_PDF.decrypt() => Some(vec![link_line(
            vec![
                line_span(format!("{RESUME_PDF}: {}", encrypted_str!("PDF document"))),
                colored(encrypted_str!("open:").decrypt(), theme::MUTED),
            ],
            &encrypted_str!("https://developersam.com/resume.pdf").decrypt(),
        )]),
        [file] if *file == TIMELINE_TXT.decrypt() => {
            let mut out = Vec::new();
            for (index, event) in data::TIMELINE.iter().enumerate() {
                out.push(vec![
                    colored(
                        format!("{:>3}  {:<15}", index + 1, event.time),
                        theme::MUTED,
                    ),
                    colored(event.title.decrypt(), theme::TEXT),
                    Span::styled(
                        format!(" [{}]", event.category.label()),
                        TextStyle::new().color(event.category.color()),
                    ),
                ]);
            }
            Some(out)
        }
        [directory, name] if *directory == PROJECTS_DIR.decrypt() => {
            let project = data::PROJECTS
                .iter()
                .find(|project| project.id.decrypt() == *name)?;
            let mut out = vec![
                one(bold_colored(project.id.decrypt(), theme::ACCENT_TEXT)),
                one(colored(project.tagline.decrypt(), theme::SUBTLE)),
                Line::new(),
            ];
            for entry in project.links {
                let url = entry.url.decrypt();
                out.push(link_line(
                    vec![bold_colored(
                        format!("  {:<12}", entry.name),
                        theme::ACCENT_TEXT,
                    )],
                    &url,
                ));
            }
            Some(out)
        }
        [directory, rest @ ..] if *directory == ARCHIVE_DIR.decrypt() => maze::read(rest),
        _ => None,
    }
}

fn line_span(text: impl Into<String>) -> Span {
    Span::new(text)
}
