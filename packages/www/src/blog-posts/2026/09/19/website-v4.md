---
title: "Website V4 Redesign"
---

![Website Change](/blog/2026-09-19-website-v4/change.png)

## Background

It has been forever since the last time I refreshed my website's design. While there is nothing
wrong with the current design, I do think it's time to think about a refresh.

I had never looked deep into TUI stuff beyond some simple vim setup. The best I have done in terms
of TUI programming is a bunch of coloured text. For me, TUI for my personal website is always quite
unthinkable. It might just be some some secondary retro mode at most. Well, now it's 2026. The rise
of coding agents that run inside terminals make a lot of IDE-only or web-only people like me to
appreciate the expressiveness and rendering ability of TUI for the first time.

I like the fact that these TUI interfaces can be easily and intuitively navigated by keyboard.
While I am still confused by those vim bindings, these terminal based coding agents show that you
can get very far with just a few simple ones, such as tabs, arrow keys, enter, esc.

I already tried the TUI for some of my personal tools:

![TUI example](/blog/2026-09-19-website-v4/tui-example.png)

Therefore, it's time to experiment it on my website.

## Achieving the TUI Feel without the TUI Tech

I already had some background knowledge how to turn my personal website into some TUI and render it
in the browser.

1. You can start with [xterms.js](https://xtermjs.org/) on the browser side. This is the same
   rendering engine that also powers the terminal experience in VSCode and it handles Claude
   Code without issues.
2. The TUI code will be written in Rust with the help of
   [iocraft](https://docs.rs/iocraft/latest/iocraft/) and compiled to WASM, so it can be used in the
   browser.
3. Use full-screen mode of rendering: taking over the entire terminal screen like Claude Code's no
   flickering mode. This is not really necessary for the desktop experience, since the traditional
   mode also has the ability to wipe clean the entire screen and rerender on events like resize.
   It's more important on mobile so we can have mouse control, since keyboard inputs are way less
   convenient there.

Although I know the basic design, I know almost nothing about TUI programming, so I let GLM 5.3
coded an initial version. It soon runs into issues after issues:

1. [iocraft](https://docs.rs/iocraft/latest/iocraft/) cannot be compiled into WASM due to a
   dependency issue. I end up having to maintain [patch files](https://github.com/SamChou19815/SamChou19815/tree/42397bc7c3aa5daff629364584593e330e6d9255/patches).
2. TUI doesn't have good ways to render images, while the timeline really needs the images to
   present rich information. I end up having to let AI create a custom protocol to leaving empty
   spaces and defer the actual image rendering in the JS layer. I never really liked the protocol.
3. Scrolling always feels slow and glitchy. It was the final dealbreaker.

At this point, I have realized that TUI is fundamentally a slightly flawed technology. I do
appreciate its simplicity, but it comes with severe limitations that simply don't exist elsewhere.
I do want a TUI redesign, but a different approach is necessary.

I threw away the failed attempt, and eventually settled on the approach of using native browser
technology while maintaining a TUI feel. It's now powered by [leptos](https://www.leptos.dev/), a
Rust-based React-like framework.

## Intentionally Designed to Burn Your Tokens

You might be wondering why I decided to stay on Rust despite the initial failed attempt of the
xterm.js-rust/wasm stack. You will soon know why.

When I decided to do the TUI redesign, I already knew it comes with tradeoffs. It will be bad for
SEO unless I specifically code together a static pre-renderer just for the crawler, which I don't
bother to do and increasingly don't intend to do even with infinite resources.

Shortly after [this blog post](/blog/2024/05/27/cake-for-ai-bot), I hard-blocked all the AI crawlers
via Cloudflare. It's a static site hosted on free plans, so it's never about the server cost. It's
about taking a stance that I maintain this website for the pleasure of other humans. If you are so
lazy that you don't even care to read a bunch of bulletpoint items on the homepage, I am not
interested in serving you.

While the AI crawler block largely works in 2025, I observed that it's increasingly irrelevant in 2026.
The crawlers do still correctly declare their true user agents so the block is still working as is,
but things like Claude Code is a completely different type of beast. After it did a builtin `Fetch`
tool call with claude UA set and received a hard block, it consistenty decides to immediately start
spoofing a typical browser UA, without remorse or hesitation. Well, so much for the so-called
"most aligned" AI models when it can't even take no as an answer.

Yes, I can turn on bot-fight mode. I can potentially write more bot detection code, but it will be
a forever cat-and-mouse game. I decide to open another front and fight fire with fire: intentionally
burn your tokens.

With the TUI setup, information is no longer available in plain HTML, so AI can no longer answer the
question of "what's on the website" with a single fetch. Then AI will try to scan the JS code for
string literals, which it will find nothing useful other than the link to the wasm binary. The lazy
AI will try to scan the `.text` section of the WASM binary, so I used Rust macros to encrypt all the
static strings. It's certainly not secure (and cannot be secure anyways since the WASM does need to
decrypt them), but it's enough to fool the lazy AI that only knows how to hill climb by taking
shortcuts. Since the AI cannot easily figure out what's there by scanning, it's forced to solve the
problem by going through the multi-turn browser-use capability.

The core idea is to make the agents take as many turns as possible. Each turn has to carry all the
existing context of all previous turns, so despite the cheaper cached input price, it still grows
quadratically. In addition, you also need to pay the cost of those reasoning tokens since I
intentionally make the problem harder.

![cost of all the burnt tokens](/blog/2026-09-19-website-v4/ai-cost.png)

According to my test, it can turn a problem that just costs a few cents in less than one minute into
a problem that will cost a few dollars in ten minutes with Opus 5. If you are even lazier and use
Fable/Astra for everything, well good luck. You will have a much better experience, if you know, just
read the website by yourself.

I understand that this is not a stance that can be taken by all websites that still need to make
some money despite the diminishing returns. I can freely do this because I don't care about my
website traffic at all. However, someone needs to fire the first shot against the tyranny of brain
rots clearly caused by AI agents, and I am willing to be the one doing it.
