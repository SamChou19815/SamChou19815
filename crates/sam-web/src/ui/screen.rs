//! The app's views: one function per region of the site, mirroring the
//! homepage's design ([`crate::theme`]). Each one is ordinary HTML laid out by
//! Tailwind classes — there is no layout engine here, and no measure counted
//! in cells: inside the session's `.terminal` box one spacing unit is one
//! character cell (`1ch`), so `px-2` is two characters of air and `max-w-88`
//! caps the reading column at eighty-eight of them, and the browser owns the
//! fitting, the wrapping, the truncation and the breakpoints. What a narrow
//! screen loses is decided by CSS at the width the view is actually drawn at,
//! never by this code counting anything.

use crate::crypt::EncryptedString;
use crate::data;
use crate::encrypted_str;
use crate::hit::HitTarget;
use crate::markdown::{self, Block};
use crate::posts;
use crate::site_path::SitePath;
use crate::style::Line;
use crate::theme;
use crate::{App, ABOUT_TAB, BLOG_TAB, HELP_TAB, TAB_NAMES, TIMELINE_TAB};
use leptos::prelude::*;

use super::styled_line;

/// The site's name as it is spelled over the tabs. Encrypted like the rest of
/// the content, so the binary never spells it out even though the wordmark
/// paints it a cell at a time.
const TITLE: EncryptedString = encrypted_str!("DEV SAM");

/// The column everything the app draws is laid out down: the reading measure,
/// centered. Its parent holds it two cells off the screen edges, so it is as
/// wide as fits and never wider than eighty-eight characters — the measure
/// that keeps prose trackable on a maximized window.
const COLUMN: &str = "mx-auto w-full max-w-88";

/// The air the body keeps off the left and right edges of the screen.
const BODY: &str = "px-2";

/// The air a card keeps inside its own edges. A card is only tinted when it is
/// the selected one, and this is what holds that tint off its marker and its
/// category tag instead of running it flush to both.
const CARD_PAD: &str = "px-1";

/// A row of air: a box one row of the type tall, with nothing in it. Vertical
/// measures are rows for the same reason horizontal ones are characters: they
/// are the units the type itself sets.
const AIR_ROW: &str = "h-row shrink-0";

/// The gutter a timeline card's body is held off the rail by.
const GUTTER: &str = "pl-3";

/// Color classes for boxes, lifted straight from [`theme`]. Text colors stay
/// inline styles — they are picked per run as the view is built — but these
/// belong to boxes, and a box's class can name its color.
const RULE_BORDER: &str = "border-[#d1d5db]"; // theme::BORDER_SUBTLE
const ACCENT_TEXT_CLASS: &str = "text-[#2563eb]"; // theme::ACCENT_TEXT
const RAIL_CLASS: &str = "bg-[#2563eb]"; // theme::ACCENT_TEXT
const SELECT_BG_CLASS: &str = "bg-[#dbeafe]"; // theme::SELECT_BG
const SURFACE_CLASS: &str = "bg-[#f7f7f7]"; // theme::SURFACE

fn click(
    target: HitTarget,
    on_activate: impl Fn(&HitTarget) + Copy + 'static,
) -> impl Fn(web_sys::MouseEvent) {
    move |event: web_sys::MouseEvent| {
        event.stop_propagation();
        on_activate(&target);
    }
}

fn style_of(color: theme::Color) -> String {
    format!("color:{};", color.css())
}

/// The header bar and the pane under it: everything the app draws for one
/// state. `touch` is the build that hands a whole view to the browser to
/// scroll, rather than keeping the header fixed over a pane of its own.
pub fn chrome(
    app: &App,
    touch: bool,
    scrolls: bool,
    on_activate: impl Fn(&HitTarget) + Copy + 'static,
) -> Vec<AnyView> {
    vec![
        header(app, touch, on_activate).into_any(),
        pane(app, scrolls, on_activate).into_any(),
    ]
}

/// The full-screen app: the header fixed at the top of the screen, the pane
/// filling what is left.
pub fn screen(
    app: &App,
    touch: bool,
    on_activate: impl Fn(&HitTarget) + Copy + 'static,
) -> AnyView {
    let children = chrome(app, touch, true, on_activate);
    view! { <div class="flex h-full w-full flex-col">{children}</div> }.into_any()
}

/// The view at `path` as one page, for the touch build to scroll: the same
/// chrome, with the scroll box around all of it, so the header slides away
/// under a scrolling finger.
pub fn page(path: &SitePath, on_activate: impl Fn(&HitTarget) + Copy + 'static) -> AnyView {
    let app = App::page(path);
    crate::publish_route(app.route());
    let children = chrome(&app, true, false, on_activate);
    view! {
        // The session's own screen is a fixed box, so the scroll box pinned to
        // it with `h-full w-full` is the one that overflows: the whole page,
        // header and all, slides under a scrolling finger.
        <div class=format!("{} h-full w-full", super::SCROLL) data-pane="">
            {children}
        </div>
    }
    .into_any()
}

// --- Header ---------------------------------------------------------------------

/// The wordmark's letters, three cells wide and three rows tall, drawn out of
/// half blocks. The type is set at one size — the size a line of the shell is
/// set at — so the only way to set the site's name at anything like a
/// heading's size is to draw the letters out of cells. Three rows is what it
/// takes for a letter to still read as that letter: at two the crossbars have
/// nowhere to go and the name stops being legible.
const GLYPHS: [(char, [&str; 3]); 11] = [
    ('A', ["█▀█", "█▀█", "▀ ▀"]),
    ('D', ["█▀▄", "█ █", "▀▀ "]),
    ('E', ["█▀▀", "█▀ ", "▀▀▀"]),
    ('L', ["█  ", "█  ", "▀▀▀"]),
    ('M', ["█▄█", "█ █", "▀ ▀"]),
    ('O', ["█▀█", "█ █", "▀▀▀"]),
    ('P', ["█▀█", "█▀▀", "▀  "]),
    ('R', ["█▀█", "█▀▄", "▀ ▀"]),
    ('S', ["█▀▀", "▀▀█", "▀▀▀"]),
    ('V', ["█ █", "█ █", " ▀ "]),
    (' ', [" ", " ", " "]),
];

/// `text` set in [`GLYPHS`], a cell of air between letters.
fn banner_rows(text: &str) -> [String; 3] {
    let mut rows: [String; 3] = Default::default();
    for character in text.chars() {
        let glyph = &GLYPHS
            .iter()
            .find(|(letter, _)| *letter == character)
            .unwrap_or_else(|| panic!("{character} is not a letter the wordmark spells"))
            .1;
        for (row, cells) in rows.iter_mut().zip(glyph) {
            if !row.is_empty() {
                row.push(' ');
            }
            row.push_str(cells);
        }
    }
    rows
}

/// One cell of the wordmark: a painted rectangle rather than a glyph, so the
/// letters come out solid where a font's own blocks would leave a hairline of
/// daylight between one cell and the next.
fn block_cell(character: char) -> AnyView {
    let fill = match character {
        '█' => "bg-[#2563eb]".to_string(),
        '▀' => "bg-[linear-gradient(#2563eb_50%,transparent_50%)]".to_string(),
        '▄' => "bg-[linear-gradient(to_bottom,transparent_50%,#2563eb_50%)]".to_string(),
        _ => String::new(),
    };
    view! { <span class=format!("inline-block h-row w-[1ch] {fill}")></span> }.into_any()
}

/// The wordmark, three rows tall, in the accent the tabs are named in.
fn wordmark() -> AnyView {
    let rows: Vec<AnyView> = banner_rows(&TITLE.decrypt())
        .map(|row| {
            let cells: Vec<AnyView> = row.chars().map(block_cell).collect();
            view! { <div class="flex">{cells}</div> }.into_any()
        })
        .into_iter()
        .collect();
    view! { <div class=ACCENT_TEXT_CLASS>{rows}</div> }.into_any()
}

/// The header: the name over (or beside) the tabs, and the bar's rule under
/// it, run the whole width of the screen. Every folding decision — when the
/// wordmark gives way to tabs alone, when the tabs stop fitting beside it —
/// is a container query against the session itself, so the bar refits the way
/// the site's nav does on the web, with no measuring code anywhere.
fn header(app: &App, touch: bool, on_activate: impl Fn(&HitTarget) + Copy + 'static) -> AnyView {
    let tab = app.tab();
    // Help is a table of key bindings, and the touch build's host has no keys
    // to press: it leaves the tab out of the bar and gives the room to the
    // ones that lead somewhere. It is still named while it is the tab in
    // front, so a visitor who arrives at `/help` is never left reading a bar
    // that marks none of its tabs.
    let labels: Vec<usize> = (0..TAB_NAMES.len())
        .filter(|index| !touch || *index == tab || *index != HELP_TAB)
        .collect();
    let tabs: Vec<AnyView> = labels
        .iter()
        .map(|index| {
            let selected = *index == tab;
            let color = if selected {
                theme::SELECT_FG
            } else {
                theme::SUBTLE
            };
            // Past thirty-six characters the bar keeps only the tab in front;
            // the arrow keys and the number keys still reach the rest.
            let mut class = String::from("cursor-pointer");
            if selected {
                class.push_str(" font-bold");
            } else {
                class.push_str(" hidden @min-[36ch]:inline");
            }
            let label = TAB_NAMES[*index].decrypt();
            view! {
                <span
                    class=class
                    style=style_of(color)
                    on:click=click(HitTarget::Tab(*index), on_activate)
                >{label}</span>
            }
            .into_any()
        })
        .collect();
    view! {
        <header class=format!("shrink-0 border-b {RULE_BORDER}")>
            <div class=BODY>
                <div class=format!("{COLUMN} {CARD_PAD} pt-row")>
                    <div class="flex flex-wrap items-center">
                        // The name gives way below sixty characters — or on a
                        // screen too short to spend three rows on a wordmark —
                        // where the tabs alone tell the visitor where they are.
                        <div class="hidden [@media(min-width:542px)_and_(min-height:432px)]:@min-[60ch]:block">
                            {wordmark()}
                        </div>
                        // Beside the name when the two fit with air to spare,
                        // centered in what the name leaves; on their own row,
                        // starting at the column, when they would crowd it.
                        <div class="flex min-w-max flex-1 items-center justify-center gap-3 @max-[67ch]:justify-start">
                            {tabs}
                        </div>
                    </div>
                    // The air between the bar's last row and its rule: under
                    // the wordmark it is half a painted cell and half this
                    // row; the tabs and a plain row of type sit on it
                    // directly, the way a nav sits on the line under it.
                    <div class=AIR_ROW></div>
                </div>
            </div>
        </header>
    }
    .into_any()
}

// --- The pane -------------------------------------------------------------------

/// The pane: everything under the header's rule. It draws no frame of its own
/// — the rule is the one edge the app's chrome has, and the body hangs off it
/// as a page hangs off a nav bar. In the full-screen app the pane is the box
/// the browser scrolls; on the touch page the browser scrolls the whole page,
/// and the pane is just the body.
fn pane(app: &App, scrolls: bool, on_activate: impl Fn(&HitTarget) + Copy + 'static) -> AnyView {
    let title = app.reader().map(|post| title_row(post, on_activate));
    let body: Vec<AnyView> = match app.reader() {
        Some(post) => reader_pane(post, scrolls, on_activate),
        None => vec![tab_pane(app, scrolls, on_activate)],
    };
    view! {
        <main class="flex min-h-0 w-full flex-1 flex-col">
            {title}
            {body}
        </main>
    }
    .into_any()
}

/// The reader's close button, padded either side so the target is three cells
/// wide rather than one — it is aimed at with a fingertip.
const CLOSE_LABEL: &str = " x ";

/// What the pane holds for the tab in front. The reader is the one view that
/// is not a tab's own pane — it fills the Blog tab's — so it is asked about
/// before the tabs are.
fn tab_pane(
    app: &App,
    scrolls: bool,
    on_activate: impl Fn(&HitTarget) + Copy + 'static,
) -> AnyView {
    let body = match app.tab() {
        TIMELINE_TAB => timeline_tree(app, on_activate),
        BLOG_TAB => blog_tree(app, on_activate),
        ABOUT_TAB => code_listing(about_lines(), on_activate),
        HELP_TAB => help_listing(),
        _ => view! { <div></div> }.into_any(),
    };
    if scrolls {
        view! {
            // The wrapper holds the body's margin off the screen edges; the
            // scroll box inside it is pinned to the room that is left with
            // `h-full` — the wrapper's own height is definite (it is a
            // `flex-1 min-h-0` child of the pane), so this box is the one that
            // overflows, and the one that scrolls.
            <div class=format!("min-h-0 w-full flex-1 {BODY}")>
                <div class=format!("{} h-full", super::SCROLL) data-pane="">
                    <div class=COLUMN>{body}</div>
                </div>
            </div>
        }
        .into_any()
    } else {
        view! {
            <div class="w-full">
                <div class=BODY>
                    <div class=COLUMN>{body}</div>
                </div>
            </div>
        }
        .into_any()
    }
}

/// The pane's title row. Only the reader names itself, centered over the pane:
/// a tab is already named — and marked as the one in front — by the header a
/// row above. A spacer as wide as the close button balances the row, so the
/// title is centered over the pane, not over the room the button leaves.
fn title_row(post: usize, on_activate: impl Fn(&HitTarget) + Copy + 'static) -> AnyView {
    let title = posts::POSTS[post].title().decrypt();
    view! {
        <div class=BODY>
            <div class=format!("{COLUMN} {CARD_PAD}")>
                <div class="grid w-full grid-cols-[3ch_1fr_3ch] items-center whitespace-pre">
                    <span></span>
                    <span class=format!("min-w-0 truncate text-center font-bold {ACCENT_TEXT_CLASS}")>
                        {format!(" {title} ")}
                    </span>
                    <span
                        class=format!("cursor-pointer text-center font-bold {ACCENT_TEXT_CLASS}")
                        on:click=click(HitTarget::Close, on_activate)
                    >{CLOSE_LABEL}</span>
                </div>
            </div>
        </div>
    }
    .into_any()
}

// --- Timeline -------------------------------------------------------------------

/// The timeline: a column of cards down the middle of the pane, a rail run
/// down the gutter of each one. The column is the reading measure rather than
/// the whole screen, so a wrapped description keeps a line length the eye can
/// track however wide the window is opened.
fn timeline_tree(app: &App, on_activate: impl Fn(&HitTarget) + Copy + 'static) -> AnyView {
    let selected = app.selected(TIMELINE_TAB);
    let cards: Vec<AnyView> = data::TIMELINE
        .iter()
        .enumerate()
        .map(|(index, event)| card_tree(event, index, index == selected, on_activate))
        .collect();
    view! { <div>{cards}</div> }.into_any()
}

/// The largest a card's thumbnail is drawn at. Where it lands inside that is
/// the picture's own business: the browser has the file and knows its shape.
const THUMB_MAX_W: &str = "max-w-[min(100%,32ch)]";
const THUMB_MAX_H: &str = "max-h-[8lh]";

/// The same, for a post's artwork.
const HERO_MAX_H: &str = "max-h-[16lh]";

/// One of a card's link buttons, numbered after the digit that opens it on the
/// Timeline tab.
fn link_button_label(index: usize, link: &data::Link) -> String {
    format!("{} {} ", index + 1, link.name.decrypt().to_uppercase())
}

/// One card of the timeline. The rail is a painted line down the gutter, one
/// unbroken line from the top of the card to the bottom; the marker's own box
/// is painted over it, which is what breaks the rail around the marker
/// exactly as the design has always drawn it.
fn card_tree(
    event: &data::TimelineEvent,
    index: usize,
    selected: bool,
    on_activate: impl Fn(&HitTarget) + Copy + 'static,
) -> AnyView {
    let pick = |plain: theme::Color| {
        if selected {
            theme::SELECT_FG
        } else {
            plain
        }
    };
    let title_style = style_of(pick(theme::TEXT));
    let time_style = style_of(pick(theme::MUTED));
    let tag_style = style_of(pick(event.category.color()));
    let marker_box = if selected {
        SELECT_BG_CLASS
    } else {
        SURFACE_CLASS
    };
    let marker = if selected { "▸  " } else { "●  " };
    let mut card_class = format!("relative w-full {CARD_PAD}");
    if selected {
        card_class.push(' ');
        card_class.push_str(SELECT_BG_CLASS);
    }

    // What the card carries under its time: the artwork first, as the homepage
    // card leads with its media, then the description and the links. Each is
    // opened by a row of air, so a card reads as stacked blocks rather than
    // one paragraph of mixed content — and a card that leaves a section out
    // spends no rows on it, spacer included.
    let mut sections: Vec<AnyView> = Vec::new();
    if let Some(image) = event.image {
        let url = SitePath::new(image.decrypt());
        sections.push(
            view! {
                <div class="mt-row">
                    <div class=GUTTER>
                        <img
                            class=format!("block h-auto max-w-full w-auto {THUMB_MAX_W} {THUMB_MAX_H}")
                            src=url.to_string()
                            alt=""
                        />
                    </div>
                </div>
            }
            .into_any(),
        );
    }
    if let Some(detail) = event.detail {
        let detail_style = style_of(pick(theme::SUBTLE));
        sections.push(
            view! {
                <div class="mt-row">
                    <div class=format!("whitespace-pre-wrap {GUTTER}")>
                        <span style=detail_style>{detail.decrypt()}</span>
                    </div>
                </div>
            }
            .into_any(),
        );
    }
    if !event.links.is_empty() {
        let buttons: Vec<AnyView> = event
            .links
            .iter()
            .enumerate()
            .map(|(link_index, link)| {
                let label = link_button_label(link_index, link);
                view! {
                    <span
                        class=format!("cursor-pointer font-bold {ACCENT_TEXT_CLASS}")
                        on:click=click(HitTarget::Link(link.url.decrypt()), on_activate)
                    >{label}</span>
                }
                .into_any()
            })
            .collect();
        sections.push(
            view! {
                <div class="mt-row">
                    // `whitespace-pre` keeps the air at the end of each label:
                    // it is the space between one button and the next.
                    <div class=format!("flex w-full flex-wrap whitespace-pre {GUTTER}")>{buttons}</div>
                </div>
            }
            .into_any(),
        );
    }

    let tag = format!("[{}]", event.category.label());
    view! {
        <div
            class=card_class
            data-card=index.to_string()
            data-selected=selected.then_some("")
            on:click=click(HitTarget::Item(index), on_activate)
        >
            <div class=format!("pointer-events-none absolute inset-y-0 left-[calc(1.5ch_-_0.5px)] w-px {RAIL_CLASS}")></div>
            // The row of air above the title: what separates one card from the
            // last. On the selected card it is tinted with the rest, so the
            // highlight opens a row above the title rather than cutting flush
            // against it.
            <div class=AIR_ROW></div>
            // The title row: the marker holds the gutter, the category tag
            // holds the right edge, and the title takes whatever is left — cut
            // where it runs out, as the homepage card header does.
            <div class="flex w-full whitespace-pre">
                <div class=format!("w-3 shrink-0 {marker_box}")>
                    <span class=format!("font-bold {ACCENT_TEXT_CLASS}")>{marker}</span>
                </div>
                <div class="min-w-0 flex-1">
                    <span class="block truncate font-bold" style=title_style>
                        {event.title.decrypt()}
                    </span>
                </div>
                <span class="shrink-0" style=tag_style>{tag}</span>
            </div>
            <div class="flex w-full whitespace-pre">
                <div class="w-3 shrink-0"></div>
                <span style=time_style>{event.time.decrypt()}</span>
            </div>
            {sections}
            // The row of air that closes the card, so the tint under a
            // selected one ends a row past its last line rather than on it.
            <div class=AIR_ROW></div>
        </div>
    }
    .into_any()
}

// --- Blog -----------------------------------------------------------------------

/// The blog index: a centered column of cards, each carrying a post's title
/// and its date and nothing else, as `/blog` reads on the web. The whole card
/// is one click target, so clicking anywhere on it opens the post.
fn blog_tree(app: &App, on_activate: impl Fn(&HitTarget) + Copy + 'static) -> AnyView {
    let selected = app.selected(BLOG_TAB);
    let cards: Vec<AnyView> = posts::POSTS
        .iter()
        .enumerate()
        .map(|(index, post)| post_card_tree(post, index, index == selected, on_activate))
        .collect();
    view! { <div>{cards}</div> }.into_any()
}

/// A blog card: a hairline box carrying the title as its heading and the date
/// under it. A post that lives elsewhere is marked with the same `↗` the web
/// index appends to its title.
fn post_card_tree(
    post: &posts::Post,
    index: usize,
    selected: bool,
    on_activate: impl Fn(&HitTarget) + Copy + 'static,
) -> AnyView {
    // The title is the card's link on the web, and reads as one here.
    let title_style = style_of(if selected {
        theme::SELECT_FG
    } else {
        theme::ACCENT_TEXT
    });
    let date_style = style_of(if selected {
        theme::SELECT_FG
    } else {
        theme::MUTED
    });
    // A cell for the frame, and a cell of air inside it: the card keeps its
    // text off its own border.
    let mut frame_class = String::from("w-full border px-2 py-row ");
    frame_class.push_str(if selected {
        "border-[#3b82f6]"
    } else {
        RULE_BORDER
    });
    frame_class.push(' ');
    frame_class.push_str(if selected {
        SELECT_BG_CLASS
    } else {
        SURFACE_CLASS
    });
    let title = if post.is_external() {
        format!("{} ↗", post.title())
    } else {
        post.title().to_string()
    };
    view! {
        <div
            class="w-full"
            data-card=index.to_string()
            data-selected=selected.then_some("")
            on:click=click(HitTarget::Item(index), on_activate)
        >
            // The row of air above every card: the gap between the web's
            // cards, and what keeps the first one off the pane's title row.
            <div class=AIR_ROW></div>
            <div class=frame_class>
                <div class="min-h-row">
                    <span class="block truncate font-bold" style=title_style>{title}</span>
                </div>
                <div class="min-h-row whitespace-pre">
                    <span style=date_style>{post.formatted_date()}</span>
                </div>
            </div>
        </div>
    }
    .into_any()
}

// --- The reader -----------------------------------------------------------------

/// The reader: the post's date pinned over the scrolling body, as the web's
/// post header carries it, with a row of air under the date.
fn reader_pane(
    post: usize,
    scrolls: bool,
    on_activate: impl Fn(&HitTarget) + Copy + 'static,
) -> Vec<AnyView> {
    let date = posts::POSTS[post].formatted_date();
    let date_row = view! {
        <div class="w-full shrink-0">
            <div class=BODY>
                <div class=COLUMN>
                    <div class="min-h-row whitespace-pre">
                        <span style=style_of(theme::MUTED)>{date}</span>
                    </div>
                    <div class=AIR_ROW></div>
                </div>
            </div>
        </div>
    };
    let blocks: Vec<AnyView> = markdown::post_blocks(&posts::POSTS[post].body().decrypt())
        .into_iter()
        .map(|block| block_view(block, on_activate))
        .collect();
    let scroller = if scrolls {
        view! {
            // Pinned with `h-full` for the same reason as the tabs' pane: the
            // wrapper is a `flex-1 min-h-0` child of the pane, so its height
            // is definite and this box is the one that overflows.
            <div class=format!("min-h-0 w-full flex-1 {BODY}")>
                <div class=format!("{} h-full", super::SCROLL) data-pane="">
                    <div class=COLUMN>{blocks}</div>
                </div>
            </div>
        }
        .into_any()
    } else {
        view! {
            <div class="w-full">
                <div class=BODY>
                    <div class=COLUMN>{blocks}</div>
                </div>
            </div>
        }
        .into_any()
    };
    vec![date_row.into_any(), scroller]
}

/// One block of a post, as the browser lays it out: paragraphs and headings
/// wrap at the measure they land at, quotes hang off a bar, code keeps its
/// lines and scrolls when it has to.
fn block_view(block: Block, on_activate: impl Fn(&HitTarget) + Copy + 'static) -> AnyView {
    match block {
        Block::Line(line) => styled_line(&line, true, on_activate),
        Block::Bullet(line) => {
            // The marker hangs in the gutter the block indents by, so a
            // wrapped line comes back under the text, not under the bullet.
            let runs: Vec<AnyView> = line
                .iter()
                .map(|span| super::span_view(span, on_activate))
                .collect();
            view! {
                <div class="min-h-row whitespace-pre-wrap pl-[2ch]">
                    <span class="-ml-[2ch]">{runs}</span>
                </div>
            }
            .into_any()
        }
        Block::Quote(line) => {
            // The bar is drawn, not spelled out in `│`: one unbroken line down
            // however many rows the quote takes.
            let runs: Vec<AnyView> = line
                .iter()
                .map(|span| super::span_view(span, on_activate))
                .collect();
            view! {
                <div class="min-h-row whitespace-pre-wrap border-l border-[#6b7280] pl-[2ch]">
                    {runs}
                </div>
            }
            .into_any()
        }
        Block::Code {
            lang,
            lines,
            closed,
        } => {
            // The fences' labeled rules: the label, then a hairline run out to
            // the measure's far edge. The block opens with its own and, when
            // the fence was closed, closes with the bare one.
            let fence_rule = |label: String| {
                view! {
                    <div class="flex w-full items-center">
                        <span class="shrink-0 whitespace-pre" style=style_of(theme::BORDER)>
                            {label}
                        </span>
                        <div class="h-px min-w-0 flex-1" style=format!("background:{};", theme::BORDER.css())></div>
                    </div>
                }
                .into_any()
            };
            let rows: Vec<AnyView> = lines
                .iter()
                .map(|line| styled_line(line, false, on_activate))
                .collect();
            // Code is never wrapped — an indent surviving is worth more than a
            // line that fits — so the block scrolls when a line is longer than
            // the measure, the way a code block on the web does.
            view! {
                <div class="overflow-x-auto">
                    {fence_rule(format!("── {lang} "))}
                    {rows}
                    {closed.then(|| fence_rule(String::from("──  ")))}
                </div>
            }
            .into_any()
        }
        Block::Image { url } => view! {
            <img
                class=format!("block h-auto max-w-full w-auto {HERO_MAX_H}")
                src=url.to_string()
                alt=""
            />
        }
        .into_any(),
    }
}

// --- Listings -------------------------------------------------------------------

fn about_lines() -> Vec<Line> {
    let mut lines = crate::highlight::doc_comment_lines();
    lines.push(Line::new());
    lines.extend(crate::highlight::program_lines());
    lines
}

/// The About tab: the doc comment and the program, set exactly as they are
/// written. The listing keeps to its own width and sits centered over the
/// measure, however wide the screen is; the lines never wrap — an indent
/// surviving is worth more than a line that fits — so it scrolls when the
/// screen is narrower than it, the way a code block on the web does.
fn code_listing(lines: Vec<Line>, on_activate: impl Fn(&HitTarget) + Copy + 'static) -> AnyView {
    let rows: Vec<AnyView> = lines
        .iter()
        .map(|line| styled_line(line, false, on_activate))
        .collect();
    view! {
        <div class="mx-auto w-fit max-w-full overflow-x-auto">
            <div class=AIR_ROW></div>
            <div class="whitespace-pre">{rows}</div>
        </div>
    }
    .into_any()
}

/// The Help tab: every key the app listens for. The key column only earns its
/// keep while the description still fits beside it; below that, the
/// description stacks on its own line — which is the browser's call, made at
/// the width the tab is drawn at.
fn help_listing() -> AnyView {
    let rows: Vec<AnyView> = [
        (
            encrypted_str!("←/→ or h/l"),
            encrypted_str!("switch between tabs"),
        ),
        (
            encrypted_str!("1 … 9"),
            encrypted_str!("open a link of the selected card (timeline) · jump to a tab elsewhere"),
        ),
        (
            encrypted_str!("↑/↓ or j/k"),
            encrypted_str!("move selection / scroll"),
        ),
        (
            encrypted_str!("Enter"),
            encrypted_str!("read a post / open a card's link"),
        ),
        (
            encrypted_str!("g / G"),
            encrypted_str!("jump to top / bottom"),
        ),
        (encrypted_str!("Esc"), encrypted_str!("close the reader")),
        (encrypted_str!("?"), encrypted_str!("open this tab")),
        (encrypted_str!("q / Ctrl+C"), encrypted_str!("quit")),
        (
            encrypted_str!("mouse"),
            encrypted_str!("click tabs, cards and links · the pane scrolls"),
        ),
    ]
    .iter()
    .map(|(keys, description)| {
        let keys = keys.decrypt();
        let description = description.decrypt();
        let keys_style = style_of(theme::ACCENT_TEXT);
        let description_style = style_of(theme::SUBTLE);
        let keys_wide = format!("  {keys:<14}");
        let keys_narrow = format!("  {keys}");
        let description_indented = format!("    {description}");
        view! {
            <div>
                <div class="min-h-row whitespace-pre-wrap break-words @max-[56ch]:hidden">
                    <span class="font-bold" style=keys_style.clone()>{keys_wide}</span>
                    <span style=description_style.clone()>{description}</span>
                </div>
                <div class="@min-[56ch]:hidden">
                    <div class="min-h-row whitespace-pre-wrap">
                        <span class="font-bold" style=keys_style.clone()>{keys_narrow}</span>
                    </div>
                    <div class="min-h-row whitespace-pre-wrap break-words">
                        <span style=description_style.clone()>{description_indented}</span>
                    </div>
                </div>
            </div>
        }
        .into_any()
    })
    .collect();
    view! {
        <div>
            <div class=AIR_ROW></div>
            {rows}
        </div>
    }
    .into_any()
}
