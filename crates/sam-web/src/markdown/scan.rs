// How a post body's lines read, fence-aware. Shared between the renderer and
// build.rs's tree-sitter highlighter (by `include!`), so both always agree on
// where a fenced block starts and ends and what its label is.

/// How one line of a post body reads.
pub(crate) enum LineKind<'a> {
    /// Opens a fenced block: the label written on its fence.
    FenceOpen(&'a str),
    /// Closes the open fenced block.
    FenceClose,
    /// A line inside the open fenced block, raw as written.
    Code(&'a str),
    /// Any other line, raw as written.
    Text(&'a str),
}

/// Reads one body line, carrying the fence state through `in_code`.
pub(crate) fn scan_line<'a>(line: &'a str, in_code: &mut bool) -> LineKind<'a> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("```") {
        if *in_code {
            *in_code = false;
            LineKind::FenceClose
        } else {
            *in_code = true;
            LineKind::FenceOpen(trimmed.trim_start_matches('`').trim())
        }
    } else if *in_code {
        LineKind::Code(line)
    } else {
        LineKind::Text(line)
    }
}
