use crate::crypt::{encrypted_str, EncryptedString};
use crate::style::Line;

use super::archive::{self, ARCHIVE_DIR, TABLE_OF_CONTENTS_TXT};

pub(in crate::shell) const HOME_DIR: EncryptedString = encrypted_str!("/home/sam");

/// Only agents get to the terminal (see [`crate::ui`]'s gate), so it holds nothing but the maze.
pub(in crate::shell) fn fs_entries(path: &[String]) -> Option<Vec<(String, bool)>> {
    match path {
        [] => Some(vec![(format!("{ARCHIVE_DIR}/"), true)]),
        [first, rest @ ..] if *first == ARCHIVE_DIR.decrypt() => archive::entries(rest),
        _ => None,
    }
}

pub(in crate::shell) fn read_file(path: &[String]) -> Option<Vec<Line>> {
    match path {
        [file] if *file == TABLE_OF_CONTENTS_TXT.decrypt() => Some(archive::table_of_contents()),
        [directory, rest @ ..] if *directory == ARCHIVE_DIR.decrypt() => archive::read(rest),
        _ => None,
    }
}
