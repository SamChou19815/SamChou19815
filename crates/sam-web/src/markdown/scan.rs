// Fence detection. `include!`d by build.rs so the highlighter and renderer agree on block
// boundaries.

pub(crate) enum LineKind<'a> {
    /// Carries the fence label.
    FenceOpen(&'a str),
    FenceClose,
    Code(&'a str),
    Text(&'a str),
}

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
