//! The app's views: one function per region of the site, mirroring the
//! homepage's design ([`crate::theme`]). Each one is ordinary HTML laid out by
//! Tailwind classes — there is no layout engine here, and no measure counted
//! in cells: inside the session's `.terminal` box one spacing unit is one
//! character cell (`1ch`), so `px-2` is two characters of air and `max-w-88`
//! caps the reading column at eighty-eight of them, and the browser owns the
//! fitting, the wrapping, the truncation and the breakpoints. What a narrow
//! screen loses is decided by CSS at the width the view is actually drawn at,
//! never by this code counting anything.
//!
//! The views read the app through a [`Model`]: one memo per thing a view can
//! depend on, read where it is used. A tab's label subscribes to which tab is
//! in front, a card to which card is selected, the pane's body to which view
//! is up — so a keystroke redraws the runs that show what it changed and
//! nothing else, and a post is parsed once, when it is opened.

use crate::crypt::EncryptedString;
use crate::data;
use crate::encrypted_str;
use crate::hit::HitTarget;
use crate::markdown::{self, Block};
use crate::posts;
use crate::site_path::SitePath;
use crate::style::Line;
use crate::theme;
use crate::{ABOUT_TAB, BLOG_TAB, HELP_TAB, TAB_NAMES, TIMELINE_TAB};
use leptos::attr::custom::custom_attribute;
use leptos::html;
use leptos::prelude::*;

use super::{hanging_line, styled_line, Activate, Model};

/// The site's name as it is spelled over the tabs. Encrypted like the rest of
/// the content, so the binary never spells it out even though the wordmark
/// paints it a cell at a time.
const TITLE: EncryptedString = encrypted_str!("DEV SAM");

// Colors in classes are the ones in [`theme`]: `#2563eb` ACCENT_TEXT,
// `#3b82f6` ACCENT, `#dbeafe` SELECT_BG, `#1e3a8a` SELECT_FG, `#f7f7f7`
// SURFACE, `#d1d5db` BORDER_SUBTLE, `#1c1e21` TEXT, `#4b5563` MUTED, `#374151`
// SUBTLE.
//
// The two list tabs are what the platform already knows how to use: a
// listbox whose options are the cards ([`timeline_tree`], [`blog_tree`]). The
// selected card carries `aria-selected`, and the `aria-selected:` /
// `group-aria-selected:` variants paint the rest; it is also the one card
// focusable with `tabindex` (a roving tabindex), and an effect focuses it as
// the selection moves ([`focus_follows_selection`]) — so the browser scrolls
// it into view by itself, announces it to a screen reader, and moving the
// selection rewrites the two cards' attributes and no other DOM.

fn click(target: HitTarget, on_activate: impl Activate) -> impl Fn(web_sys::MouseEvent) {
    move |event: web_sys::MouseEvent| {
        event.stop_propagation();
        on_activate(&target);
    }
}

/// The pointer moving over a card, as the hover that selects it — but only
/// when the pointer itself moved. Browsers report the pane sliding under a
/// resting pointer as movement too, after a scroll, and a keystroke that just
/// scrolled the selection into view must not have it snatched back by
/// whichever card came to rest under the pointer. A pointer that has not moved
/// is still where it was ([`Model::pointer`]), which is what tells the two
/// apart.
fn hover(model: Model, index: usize, on_activate: impl Activate) -> impl Fn(web_sys::MouseEvent) {
    move |event: web_sys::MouseEvent| {
        let at = (event.screen_x(), event.screen_y());
        if model
            .pointer
            .try_update_value(|last| std::mem::replace(last, at))
            == Some(at)
        {
            return;
        }
        on_activate(&HitTarget::Hover(index));
    }
}

fn style_of(color: theme::Color) -> String {
    format!("color:{};", color.css())
}

/// Whether the card at `index` is the selected one, as a memo of its own: the
/// list's selection is one signal every card derives from, but a card's memo
/// only notifies when its own answer changes — so moving the selection
/// redraws the card it left and the card it landed on, and no other.
fn is_selected(selected: Memo<usize>, index: usize) -> Memo<bool> {
    Memo::new(move |_| selected.get() == index)
}

/// What a screen reader calls each list, encrypted like the rest of the
/// content.
const TIMELINE_LIST_LABEL: EncryptedString = encrypted_str!("Timeline");
const BLOG_LIST_LABEL: EncryptedString = encrypted_str!("Blog posts");

/// Keeps DOM focus on the selected card: the selected option is the one card
/// focusable with `tabindex` (the rest are -1, a roving tabindex), and this
/// effect focuses it as the selection moves. The browser then does what it
/// does for any focus change — scrolls the freshly focused element into view,
/// the least it can — which is how the arrow keys slide the pane by a card
/// rather than by a screenful, and a screen reader announces the option the
/// selection landed on. The first run is the list mounting, under a scroll
/// position the pane is about to be put back to, so that run focuses without
/// scrolling; every later one is a selection move, which is what scrolls.
fn focus_follows_selection(selected: Memo<usize>, cards: &[NodeRef<html::Li>]) {
    let cards = cards.to_vec();
    Effect::new(move |previous: Option<()>| {
        let index = selected.get();
        if let Some(option) = cards.get(index).and_then(|card| card.get()) {
            let mut scroll = web_sys::FocusOptions::new();
            scroll.set_prevent_scroll(previous.is_none());
            let _ = option.focus_with_options(&scroll);
        }
    });
}

/// The app: the header fixed at the top of the screen, the pane filling what
/// is left. `touch` is a host with no keys to press, which leaves the Help
/// tab out of the bar.
pub(crate) fn screen(model: Model, touch: bool, on_activate: impl Activate) -> AnyView {
    view! {
        <div class="flex h-full w-full flex-col">
            {header(model, touch, on_activate)}
            {pane(model, on_activate)}
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
    view! { <div class="text-[#2563eb]">{rows}</div> }.into_any()
}

/// The header: the name over (or beside) the tabs, and the bar's rule under
/// it, run the whole width of the screen. Every folding decision — when the
/// wordmark gives way to tabs alone, when the tabs stop fitting beside it —
/// is a container query against the session itself, so the bar refits the way
/// the site's nav does on the web, with no measuring code anywhere.
fn header(model: Model, touch: bool, on_activate: impl Activate) -> AnyView {
    let tabs: Vec<AnyView> = (0..TAB_NAMES.len())
        .map(|index| {
            let selected = move || model.tab.get() == index;
            let style = move || {
                style_of(if selected() {
                    theme::SELECT_FG
                } else {
                    theme::SUBTLE
                })
            };
            let class = move || {
                if selected() {
                    "cursor-pointer font-bold"
                // Help is a table of key bindings, and a host with no keys to
                // press leaves the tab out of the bar, giving the room to the
                // ones that lead somewhere. It is still named while it is the
                // tab in front, so a visitor who arrives at `/help` is never
                // left reading a bar that marks none of its tabs.
                } else if touch && index == HELP_TAB {
                    "hidden"
                // Past thirty-six characters the bar keeps only the tab in
                // front; the arrow keys and the number keys still reach the
                // rest.
                } else {
                    "cursor-pointer hidden @min-[36ch]:inline"
                }
            };
            let label = TAB_NAMES[index].decrypt();
            view! {
                <span
                    class=class
                    style=style
                    on:click=click(HitTarget::Tab(index), on_activate)
                >{label}</span>
            }
            .into_any()
        })
        .collect();
    view! {
        <header class="shrink-0 border-b border-[#d1d5db]">
            <div class="px-2">
                <div class="mx-auto w-full max-w-88 px-1 pt-row">
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
                    <div class="h-row shrink-0"></div>
                </div>
            </div>
        </header>
    }
    .into_any()
}

// --- The pane -------------------------------------------------------------------

/// The pane: everything under the header's rule. It draws no frame of its own
/// — the rule is the one edge the app's chrome has, and the body hangs off it
/// as a page hangs off a nav bar. The pane is the box the browser scrolls,
/// built once per run of the app: only its body changes hands as the views
/// do, so the box keeps its place across a keystroke that changes nothing
/// about which view is up.
fn pane(model: Model, on_activate: impl Activate) -> AnyView {
    // The reader is the one view that is not a tab's own pane — it fills the
    // Blog tab's — so it is asked about before the tabs are.
    let body = move || match model.reader.get() {
        Some(post) => reader_pane(post, on_activate),
        None => match model.tab.get() {
            TIMELINE_TAB => timeline_tree(model, on_activate),
            BLOG_TAB => blog_tree(model, on_activate),
            ABOUT_TAB => code_listing(about_lines(), on_activate),
            HELP_TAB => help_listing(),
            _ => view! { <div></div> }.into_any(),
        },
    };
    view! {
        <main class="flex min-h-0 w-full flex-1 flex-col">
            // The wrapper holds the body's margin off the screen edges; the
            // scroll box inside it is pinned to the room that is left with
            // `h-full` — the wrapper's own height is definite (it is a
            // `flex-1 min-h-0` child of the pane), so this box is the one that
            // overflows, and the one that scrolls.
            <div class="min-h-0 w-full flex-1 px-2">
                <div class=format!("{} h-full", super::SCROLL) node_ref=model.pane>
                    <div class="mx-auto w-full max-w-88">{body}</div>
                </div>
            </div>
        </main>
    }
    .into_any()
}

fn post_header(post: usize, on_activate: impl Activate) -> AnyView {
    let title = posts::POSTS[post].title().decrypt();
    let date = posts::POSTS[post].formatted_date();
    // The post's heading, as a role rather than a tag: the site's own
    // stylesheet sets `h1` for its prose pages, and the terminal sets every
    // letter of its own type itself. A heading without a level reads as
    // level two, and Leptos has no typed `aria-level`, so the one attribute
    // goes on through the custom-attribute hatch.
    let heading = view! {
        <div
            role="heading"
            class="min-w-0 break-words text-[1.6em] leading-[1.25] font-bold text-[#2563eb]"
        >
            {title}
        </div>
    }
    .add_any_attr(custom_attribute("aria-level", "1"));
    view! {
        <div class="w-full">
            <div class="h-row shrink-0"></div>
            <div class="grid w-full grid-cols-[1fr_3ch] items-start">
                {heading}
                // `leading-[2]` is one line of the title, so the x is centered on it.
                <span
                    class="cursor-pointer whitespace-pre text-center font-bold leading-[2] text-[#2563eb]"
                    on:click=click(HitTarget::Close, on_activate)
                >" x "</span>
            </div>
            <div class="min-h-row whitespace-pre pt-[0.5lh]">
                <span style=style_of(theme::MUTED)>{date}</span>
            </div>
            <div class="h-row shrink-0"></div>
        </div>
    }
    .into_any()
}

// --- Timeline -------------------------------------------------------------------

/// The timeline: a column of cards down the middle of the pane, a rail run
/// down the gutter of each one. The column is the reading measure rather than
/// the whole screen, so a wrapped description keeps a line length the eye can
/// track however wide the window is opened. To the platform it is a listbox
/// and its options, so the arrows, the focus and a screen reader all work on
/// it the way they work on any list.
fn timeline_tree(model: Model, on_activate: impl Activate) -> AnyView {
    let options: Vec<NodeRef<html::Li>> =
        (0..data::TIMELINE.len()).map(|_| NodeRef::new()).collect();
    focus_follows_selection(model.timeline_selected, &options);
    let cards: Vec<AnyView> = data::TIMELINE
        .iter()
        .enumerate()
        .map(|(index, event)| card_tree(event, index, model, options[index], on_activate))
        .collect();
    view! {
        <ul role="listbox" aria-orientation="vertical" aria-label=TIMELINE_LIST_LABEL.decrypt()>
            {cards}
        </ul>
    }
    .into_any()
}

/// One of a card's link buttons, numbered after the digit that opens it on the
/// Timeline tab.
fn link_button_label(index: usize, link: &data::Link) -> String {
    format!("{} {}", index + 1, link.name.decrypt().to_uppercase())
}

/// One card of the timeline: an option of the tab's listbox. The rail is a
/// painted line down the gutter, one unbroken line from the top of the card
/// to the bottom; the marker's own box is painted over it, which is what
/// breaks the rail around the marker exactly as the design has always drawn
/// it.
fn card_tree(
    event: &'static data::TimelineEvent,
    index: usize,
    model: Model,
    option: NodeRef<html::Li>,
    on_activate: impl Activate,
) -> AnyView {
    let selected = is_selected(model.timeline_selected, index);
    // The tag is set in its category's color, which is the one color here the
    // palette picks per card: it is handed to CSS as a variable, so the
    // selected look can still take it over.
    let tag_style = format!("--tag:{};", event.category.color().css());

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
                    <div class="pl-3">
                        <img
                            class="block h-auto max-w-full w-auto max-w-[min(100%,32ch)] max-h-[8lh]"
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
        sections.push(
            view! {
                <div class="mt-row">
                    <div class="whitespace-pre-wrap pl-3">
                        <span class="text-[#374151] group-aria-selected:text-[#1e3a8a]">
                            {detail.decrypt()}
                        </span>
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
                        class="cursor-pointer border border-[#2563eb] px-1 font-bold text-[#2563eb] hover:bg-[#2563eb] hover:text-[#f7f7f7]"
                        on:click=click(HitTarget::Link(link.url.decrypt()), on_activate)
                    >{label}</span>
                }
                .into_any()
            })
            .collect();
        sections.push(
            view! {
                <div class="mt-row">
                    <div class="flex w-full flex-wrap gap-x-1 gap-y-row whitespace-pre pl-3">{buttons}</div>
                </div>
            }
            .into_any(),
        );
    }

    let tag = format!("[{}]", event.category.label());
    view! {
        <li
            role="option"
            class="group relative w-full px-1 aria-selected:bg-[#dbeafe] outline-hidden"
            aria-selected=move || if selected.get() { "true" } else { "false" }
            aria-setsize=data::TIMELINE.len()
            aria-posinset=index + 1
            tabindex=move || if selected.get() { 0 } else { -1 }
            node_ref=option
            on:click=click(HitTarget::Item(index), on_activate)
            on:mousemove=hover(model, index, on_activate)
        >
            <div class="pointer-events-none absolute inset-y-0 left-[calc(1.5ch_-_0.5px)] w-px bg-[#2563eb]"></div>
            // The row of air above the title: what separates one card from the
            // last. On the selected card it is tinted with the rest, so the
            // highlight opens a row above the title rather than cutting flush
            // against it.
            <div class="h-row shrink-0"></div>
            // The title row: the marker holds the gutter, the category tag
            // holds the right edge, and the title takes whatever is left — cut
            // where it runs out, as the homepage card header does.
            <div class="flex w-full whitespace-pre">
                // The marker's box is painted over the rail, in whichever
                // color the card is: that is what breaks the rail around it.
                <div class="w-3 shrink-0 bg-[#f7f7f7] font-bold text-[#2563eb] group-aria-selected:bg-[#dbeafe]">
                    <span class="group-aria-selected:hidden">"●  "</span>
                    <span class="hidden group-aria-selected:inline">"▸  "</span>
                </div>
                <div class="min-w-0 flex-1">
                    <span class="block truncate font-bold text-[#1c1e21] group-aria-selected:text-[#1e3a8a]">
                        {event.title.decrypt()}
                    </span>
                </div>
                <span class="shrink-0 text-(--tag) group-aria-selected:text-[#1e3a8a]" style=tag_style>
                    {tag}
                </span>
            </div>
            <div class="flex w-full whitespace-pre">
                <div class="w-3 shrink-0"></div>
                <span class="text-[#4b5563] group-aria-selected:text-[#1e3a8a]">
                    {event.time.decrypt()}
                </span>
            </div>
            {sections}
            // The row of air that closes the card, so the tint under a
            // selected one ends a row past its last line rather than on it.
            <div class="h-row shrink-0"></div>
        </li>
    }
    .into_any()
}

// --- Blog -----------------------------------------------------------------------

/// The blog index: a centered column of cards, each carrying a post's title
/// and its date and nothing else, as `/blog` reads on the web. The whole card
/// is one click target, so clicking anywhere on it opens the post; the
/// pointer moving onto one selects it, as the arrow keys would. To the
/// platform it is a listbox and its options, like the timeline.
fn blog_tree(model: Model, on_activate: impl Activate) -> AnyView {
    let options: Vec<NodeRef<html::Li>> = (0..posts::POSTS.len()).map(|_| NodeRef::new()).collect();
    focus_follows_selection(model.blog_selected, &options);
    let cards: Vec<AnyView> = posts::POSTS
        .iter()
        .enumerate()
        .map(|(index, post)| post_card_tree(post, index, model, options[index], on_activate))
        .collect();
    view! {
        <ul role="listbox" aria-orientation="vertical" aria-label=BLOG_LIST_LABEL.decrypt()>
            {cards}
        </ul>
    }
    .into_any()
}

/// A blog card: a hairline box carrying the title as its heading and the date
/// under it. A post that lives elsewhere is marked with the same `↗` the web
/// index appends to its title. The option is the card whole — the box a
/// screen reader names and focus lands on — and the hairline frame inside it
/// is the part the pointer acts on.
fn post_card_tree(
    post: &'static posts::Post,
    index: usize,
    model: Model,
    option: NodeRef<html::Li>,
    on_activate: impl Activate,
) -> AnyView {
    let selected = is_selected(model.blog_selected, index);
    let title = if post.is_external() {
        format!("{} ↗", post.title())
    } else {
        post.title().to_string()
    };
    view! {
        <li
            role="option"
            class="group w-full outline-hidden"
            aria-selected=move || if selected.get() { "true" } else { "false" }
            aria-setsize=posts::POSTS.len()
            aria-posinset=index + 1
            tabindex=move || if selected.get() { 0 } else { -1 }
            node_ref=option
        >
            // The row of air above every card: the gap between the web's
            // cards, and what keeps the first one off the pane's title row.
            <div class="h-row shrink-0"></div>
            // A cell for the frame, and a cell of air inside it: the card
            // keeps its text off its own border. The frame is the card as far
            // as the pointer is concerned: what it hovers to select, and what
            // it clicks to open.
            <div
                class="w-full cursor-pointer border border-[#d1d5db] bg-[#f7f7f7] px-2 py-row group-aria-selected:border-[#3b82f6] group-aria-selected:bg-[#dbeafe]"
                on:click=click(HitTarget::Item(index), on_activate)
                on:mousemove=hover(model, index, on_activate)
            >
                // The title is the card's link on the web, and reads as one here.
                <div class="min-h-row">
                    <span class="block truncate font-bold text-[#2563eb] group-aria-selected:text-[#1e3a8a]">
                        {title}
                    </span>
                </div>
                <div class="min-h-row whitespace-pre">
                    <span class="text-[#4b5563] group-aria-selected:text-[#1e3a8a]">
                        {post.formatted_date()}
                    </span>
                </div>
            </div>
        </li>
    }
    .into_any()
}

// --- The reader -----------------------------------------------------------------

/// A post, parsed and laid out once — here, when it is opened; a keystroke
/// that only scrolls it never comes back this way.
fn reader_pane(post: usize, on_activate: impl Activate) -> AnyView {
    let mut blocks = vec![post_header(post, on_activate)];
    blocks.extend(
        markdown::post_blocks(&posts::POSTS[post].body().decrypt())
            .into_iter()
            .map(|block| block_view(block, on_activate)),
    );
    view! { <div>{blocks}</div> }.into_any()
}

/// One block of a post, as the browser lays it out: paragraphs and headings
/// wrap at the measure they land at, quotes hang off a bar, code keeps its
/// lines and scrolls when it has to.
fn block_view(block: Block, on_activate: impl Activate) -> AnyView {
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
                class="block h-auto max-w-full w-auto max-h-[16lh]"
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

/// samlang's indent width: a wrapped line continues one level in from its own.
const INDENT: usize = 2;

/// Where the rest of a line lands when it wraps: past the line's own indent —
/// and, on a line of a block comment, past its ` * ` gutter — then one level
/// in, so the continuation is indented under the line it belongs to rather
/// than lined up with the one below it.
fn hang_of(line: &Line) -> usize {
    let text: String = line.iter().map(|span| span.text.as_str()).collect();
    let indent = text.len() - text.trim_start().len();
    let gutter = if text[indent..].starts_with("* ") {
        2
    } else {
        0
    };
    indent + gutter + INDENT
}

/// The About tab: the doc comment and the program, set exactly as they are
/// written. The listing keeps to its own width and sits centered over the
/// measure, however wide the screen is. A line wider than the screen — the
/// longer URLs of the doc comment, on a phone — wraps under its own indent
/// ([`hanging_line`]) rather than off the edge: the indents survive, which is
/// what makes the code legible, and nothing scrolls sideways behind a
/// scrollbar a phone never shows.
fn code_listing(lines: Vec<Line>, on_activate: impl Activate) -> AnyView {
    let rows: Vec<AnyView> = lines
        .iter()
        .map(|line| hanging_line(line, hang_of(line), on_activate))
        .collect();
    // `w-fit` under `max-w-full`: the listing is as wide as its longest line
    // where there is room for it, and the measure where there is not — the
    // rows wrap inside whichever it is.
    view! {
        <div class="mx-auto w-fit max-w-full">
            <div class="h-row shrink-0"></div>
            <div>{rows}</div>
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
            <div class="h-row shrink-0"></div>
            {rows}
        </div>
    }
    .into_any()
}
