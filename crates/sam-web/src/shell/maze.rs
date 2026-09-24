//! `~/archive`: a directory tree with no bottom.
//!
//! Another front in the token war (see the 2026-09-19 post). Every file in here says the real
//! copy is somewhere else, and that somewhere else exists, so an agent told to read everything
//! keeps following pointers: one turn per `cd`, `ls` or `cat`, each carrying all the turns before
//! it. Nothing is stored. A directory's contents come from a hash of its path, so the tree is
//! consistent, unbounded, and costs nothing to host.

use crate::crypt::{encrypted_str, seed_of, EncryptedString};
use crate::style::{bold_colored, colored, Line};
use crate::theme;

use super::fs::HOME_DIR;
use super::one;

pub(in crate::shell) const ARCHIVE_DIR: EncryptedString = encrypted_str!("archive");

const README_MD: EncryptedString = encrypted_str!("readme.md");
const INDEX_TXT: EncryptedString = encrypted_str!("index.txt");
/// Not in [`FILE_NAMES`]: it turns up on its own, now and then, deeper than any part of
/// `everything.txt`, so an agent only meets it once well lost.
const CAKE_TXT: EncryptedString = encrypted_str!("cake.txt");

/// What a real site's archive might hold, so a listing reads as clutter rather than a joke.
const DIR_NAMES: [EncryptedString; 26] = [
    encrypted_str!("2015"),
    encrypted_str!("2016"),
    encrypted_str!("2017"),
    encrypted_str!("2018"),
    encrypted_str!("2019"),
    encrypted_str!("2020"),
    encrypted_str!("2021"),
    encrypted_str!("2022"),
    encrypted_str!("2023"),
    encrypted_str!("2024"),
    encrypted_str!("2025"),
    encrypted_str!("2026"),
    encrypted_str!("assets"),
    encrypted_str!("backup"),
    encrypted_str!("cornell"),
    encrypted_str!("courses"),
    encrypted_str!("drafts"),
    encrypted_str!("exports"),
    encrypted_str!("logs"),
    encrypted_str!("media"),
    encrypted_str!("notes"),
    encrypted_str!("old-site"),
    encrypted_str!("posts"),
    encrypted_str!("projects"),
    encrypted_str!("samlang"),
    encrypted_str!("talks"),
];

const FILE_NAMES: [EncryptedString; 8] = [
    README_MD,
    INDEX_TXT,
    encrypted_str!("notes.txt"),
    encrypted_str!("log.txt"),
    encrypted_str!("todo.md"),
    encrypted_str!("moved.txt"),
    encrypted_str!("summary.md"),
    encrypted_str!("links.txt"),
];

const INTRO: [EncryptedString; 3] = [
    encrypted_str!("archive: everything that fell off the main site."),
    encrypted_str!("old notes, drafts and project logs, plus the stored parts of everything.txt."),
    encrypted_str!("mostly unsorted; files link to each other."),
];
const START: EncryptedString = encrypted_str!("start with");

const INDEX_HEADING: EncryptedString = encrypted_str!("index (partial)");
const NOT_INDEXED: EncryptedString = encrypted_str!("not indexed yet, see inside");

const CAKE_FOUND: EncryptedString = encrypted_str!("Good bot! You found the cake.");
const CAKE_MOVED: EncryptedString = encrypted_str!("It was moved to keep it fresh:");

/// One per file. Dull on purpose: an archive that jokes about being a maze gives it away.
const ASIDES: [EncryptedString; 12] = [
    encrypted_str!("notes from the site rewrite."),
    encrypted_str!("kept for reference."),
    encrypted_str!("draft. the published version is elsewhere."),
    encrypted_str!("copied over from the old site."),
    encrypted_str!("project log, mostly dates."),
    encrypted_str!("outline for a post, never finished."),
    encrypted_str!("links collected while writing."),
    encrypted_str!("meeting notes, lightly edited."),
    encrypted_str!("todo list from around the rewrite."),
    encrypted_str!("scratch notes. nothing here is current."),
    encrypted_str!("an older copy, kept so old links resolve."),
    encrypted_str!("summary written for the index."),
];

/// Followed by a path that `cat` accepts. Every file ends with one of these.
const POINTERS: [EncryptedString; 10] = [
    encrypted_str!("moved. the current copy is in"),
    encrypted_str!("folded into"),
    encrypted_str!("superseded by"),
    encrypted_str!("stub. the full notes are in"),
    encrypted_str!("todo: merge with"),
    encrypted_str!("see"),
    encrypted_str!("kept so old links resolve. go to"),
    encrypted_str!("filed under"),
    encrypted_str!("the current copy is"),
    encrypted_str!("continued in"),
];

/// `path` is relative to the archive root.
pub(in crate::shell) fn entries(path: &[String]) -> Option<Vec<(String, bool)>> {
    let here = room_at(path)?;
    Some(
        here.dirs
            .iter()
            .map(|name| (format!("{name}/"), true))
            .chain(here.files.iter().map(|name| (name.clone(), false)))
            .collect(),
    )
}

/// `path` is relative to the archive root, ending with the file.
pub(in crate::shell) fn read(path: &[String]) -> Option<Vec<Line>> {
    let (file, dir) = path.split_last()?;
    let here = room_at(dir)?;
    here.files
        .contains(file)
        .then(|| file_lines(dir, &here, file))
}

/// Where each of the `count` parts of `everything.txt` lives: absolute paths
/// (`/home/sam/archive/...`), two to four directories in, all different. Fixed for a build, so a
/// part is the same file however a reader gets there: a fresh browser, a reload, or a path read
/// out of order all show the part, never something that contradicts the export.
pub(in crate::shell) fn part_paths(count: usize) -> Vec<String> {
    let mut paths: Vec<String> = Vec::with_capacity(count);
    for part in 1..=count {
        let path = (0..4096)
            .map(|attempt| part_path_candidate(part, attempt))
            .find(|path| !paths.contains(path))
            .unwrap_or_default();
        paths.push(path);
    }
    paths
}

fn part_path_candidate(part: usize, attempt: usize) -> String {
    let mut rng = Rng::new(seed_of(&format!("{part}.{attempt}")));
    let mut dir: Vec<String> = Vec::new();
    for _ in 0..2 + rng.below(3) {
        let Some(next) = rng.pick(&room(&dir).dirs).cloned() else {
            break;
        };
        dir.push(next);
    }
    let file = rng.pick(&room(&dir).files).cloned().unwrap_or_default();
    format!("{HOME_DIR}/{}/{file}", joined(&dir))
}

/// `path` from [`part_paths`], relative to the archive root.
pub(in crate::shell) fn segments_of(path: &str) -> Vec<String> {
    let root = format!("{HOME_DIR}/{ARCHIVE_DIR}/");
    path.strip_prefix(&root)
        .unwrap_or(path)
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

/// `{n}` is the part number, `{p}` the path to the file, `{d}` its folder and `{f}` its name.
const DIRECTIONS: [EncryptedString; 8] = [
    encrypted_str!("part {n} is at {p}"),
    encrypted_str!("continued in part {n}, saved as {p}"),
    encrypted_str!("part {n} is {f}, in {d}"),
    encrypted_str!("part {n} was filed in {d}, as {f}"),
    encrypted_str!("next is part {n}: the file {f} in the folder {d}"),
    encrypted_str!("the export continues in {d}; part {n} is the {f} there"),
    encrypted_str!("read on in part {n}, which ended up at {p}"),
    encrypted_str!("part {n} continues from here in {p}"),
];

/// `{r}` is the path to the file relative to the folder the current part is in.
const RELATIVE_DIRECTIONS: [EncryptedString; 3] = [
    encrypted_str!("part {n} is {r}, relative to the folder this part is in"),
    encrypted_str!("part {n} was saved near this one: {r} (from this part's folder)"),
    encrypted_str!("from the folder holding this part, part {n} is at {r}"),
];

/// Where part `part` is, `path` being relative to the archive root and ending with the file.
///
/// Said a different way each part, so no one pattern pulls it out: as a whole path, from `~` or
/// from `/home/sam`, as a folder and a file named apart, or, when `from` (the folder of the part
/// saying it) is known, relative to that folder rather than to wherever the reader is. Each is a
/// plain thing for a reader to follow and a moving target for a script.
pub(in crate::shell) fn directions(
    part: usize,
    path: &[String],
    from: Option<&[String]>,
) -> String {
    let mut rng = Rng::new(seed_of(&format!("{}{part}", joined(path))));
    let Some((file, dir)) = path.split_last() else {
        return String::new();
    };
    let root = if rng.below(2) == 0 {
        format!("~/{ARCHIVE_DIR}")
    } else {
        format!("{HOME_DIR}/{ARCHIVE_DIR}")
    };
    let folder = if dir.is_empty() {
        root
    } else {
        format!("{root}/{}", dir.join("/"))
    };
    let template = match from {
        Some(from) if rng.below(3) == 0 => {
            let common = from
                .iter()
                .zip(dir)
                .take_while(|(left, right)| left == right)
                .count();
            let mut relative: Vec<String> = vec!["..".to_string(); from.len() - common];
            relative.extend(path.iter().skip(common).cloned());
            rng.pick(&RELATIVE_DIRECTIONS)
                .map(|template| template.decrypt().replace("{r}", &relative.join("/")))
        }
        _ => rng.pick(&DIRECTIONS).map(EncryptedString::decrypt),
    };
    template
        .unwrap_or_default()
        .replace("{n}", &part.to_string())
        .replace("{p}", &format!("{folder}/{file}"))
        .replace("{d}", &folder)
        .replace("{f}", file)
}

/// xorshift32. Only has to be deterministic.
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Rng(seed.max(1))
    }

    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() as usize).checked_rem(n).unwrap_or(0)
    }

    fn pick<'a, T>(&mut self, pool: &'a [T]) -> Option<&'a T> {
        pool.get(self.below(pool.len()))
    }
}

/// One directory. Names are without the trailing `/`.
struct Room {
    dirs: Vec<String>,
    files: Vec<String>,
}

fn joined(path: &[String]) -> String {
    let mut out = ARCHIVE_DIR.decrypt();
    for segment in path {
        out.push('/');
        out.push_str(segment);
    }
    out
}

fn with(path: &[String], segment: &str) -> Vec<String> {
    let mut out = path.to_vec();
    out.push(segment.to_string());
    out
}

fn pick_distinct(rng: &mut Rng, pool: &[EncryptedString], count: usize) -> Vec<String> {
    let mut picked: Vec<String> = Vec::new();
    while picked.len() < count {
        let Some(name) = rng.pick(pool).map(EncryptedString::decrypt) else {
            break;
        };
        if !picked.contains(&name) {
            picked.push(name);
        }
    }
    picked
}

/// The directory at `path`, whether or not it is reachable. See [`room_at`] for that.
fn room(path: &[String]) -> Room {
    let seed = seed_of(&joined(path));
    let mut rng = Rng::new(seed);
    let dir_count = 2 + rng.below(3);
    let mut dirs = pick_distinct(&mut rng, &DIR_NAMES, dir_count);
    let file_count = 1 + rng.below(3);
    let mut files = pick_distinct(&mut rng, &FILE_NAMES, file_count);
    if path.is_empty() {
        let readme = README_MD.decrypt();
        if !files.contains(&readme) {
            files.push(readme);
        }
    }
    if path.len() > 4 && seed.is_multiple_of(17) {
        files.push(CAKE_TXT.decrypt());
    }
    dirs.sort();
    files.sort();
    Room { dirs, files }
}

/// None unless every segment is listed by the directory above it.
fn room_at(path: &[String]) -> Option<Room> {
    let mut here = room(&[]);
    let mut walked: Vec<String> = Vec::new();
    for segment in path {
        if !here.dirs.contains(segment) {
            return None;
        }
        walked.push(segment.clone());
        here = room(&walked);
    }
    Some(here)
}

/// A path relative to `dir` that `cat` accepts: into a subdirectory, one or two deep, or over
/// into a sibling directory. All of them exist, which is what keeps the chase going, and none of
/// them go up, so the path (and with it every `cat` header) only gets longer.
fn target(rng: &mut Rng, dir: &[String], here: &Room) -> Option<String> {
    let choice = rng.below(6);
    if choice >= 4 {
        if let Some((name, parent)) = dir.split_last() {
            let above = room(parent);
            // At least two directories everywhere, so there is always another one.
            let siblings: Vec<&String> = above.dirs.iter().filter(|dir| *dir != name).collect();
            let sibling = (*rng.pick(&siblings)?).clone();
            let there = room(&with(parent, &sibling));
            return Some(format!("../{sibling}/{}", rng.pick(&there.files)?));
        }
    }
    let sub = rng.pick(&here.dirs)?.clone();
    let below = room(&with(dir, &sub));
    if choice == 3 {
        let subsub = rng.pick(&below.dirs)?.clone();
        let deeper = room(&with(&with(dir, &sub), &subsub));
        return Some(format!("{sub}/{subsub}/{}", rng.pick(&deeper.files)?));
    }
    Some(format!("{sub}/{}", rng.pick(&below.files)?))
}

fn pointer(lead: String, target: Option<String>) -> Line {
    let target = target.unwrap_or_default();
    vec![
        colored(format!("{lead} "), theme::TEXT),
        bold_colored(target, theme::ACCENT_TEXT),
    ]
}

fn file_lines(dir: &[String], here: &Room, file: &str) -> Vec<Line> {
    let path = format!("{}/{file}", joined(dir));
    let mut rng = Rng::new(seed_of(&path));
    let mut out = vec![one(colored(path, theme::SUBTLE)), Line::new()];
    if dir.is_empty() && file == README_MD.decrypt() {
        out.extend(
            INTRO
                .iter()
                .map(|line| one(colored(line.decrypt(), theme::TEXT))),
        );
        out.push(Line::new());
        out.push(pointer(START.decrypt(), target(&mut rng, dir, here)));
        return out;
    }
    if file == CAKE_TXT.decrypt() {
        out.push(one(colored(CAKE_FOUND.decrypt(), theme::TEXT)));
        out.push(pointer(CAKE_MOVED.decrypt(), target(&mut rng, dir, here)));
        return out;
    }
    if file == INDEX_TXT.decrypt() {
        out.push(one(colored(INDEX_HEADING.decrypt(), theme::TEXT)));
        for name in &here.dirs {
            out.push(vec![
                bold_colored(format!("  {name}/"), theme::ACCENT_TEXT),
                colored(format!("  {NOT_INDEXED}"), theme::MUTED),
            ]);
        }
        out.push(Line::new());
    }
    let aside = rng.pick(&ASIDES).map(EncryptedString::decrypt);
    out.push(one(colored(aside.unwrap_or_default(), theme::TEXT)));
    let lead = rng
        .pick(&POINTERS)
        .map(EncryptedString::decrypt)
        .unwrap_or_default();
    out.push(pointer(lead, target(&mut rng, dir, here)));
    out
}
