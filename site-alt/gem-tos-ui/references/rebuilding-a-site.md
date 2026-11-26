# Rebuilding an existing site in GEM/TOS

The procedure for taking a live site (or a static generator's output) and giving it the GEM/TOS skin without losing content, URLs, tags or the feed.

## 1. Pick the real source of truth

- Prefer the repo's generator source only if it is current. A checked-in `src/` can lag the live site; rebuilding from it silently drops the newest posts. When in doubt, scrape the live HTML.
- Snapshot every page into `content/` once, then build offline. The build then does not depend on the origin being up, and a failed fetch cannot half-write the site.
- Map the routes before writing anything: home, post index, each post, tag index, each tag, RSS. Keep the paths (`/posts/<slug>/`, `/tags/<tag>/`) so links and the feed survive.

## 2. Extract without rewriting

- Parse with a small `html.parser` tree (`Node` with tag/attrs/children/text) and serialize from that. Regex over attributes breaks on minified HTML.
- Post metadata: `h1`, `time[datetime]`, the reading-time span, and tag links whose href contains `/tags/`.
- Content starts after the post header; skip the generated table of contents. Keep the rest in document order.
- Unwrap generator artifacts:
  - heading anchor links (`<a href="#…">` with no text) disappear;
  - syntax-highlighter wrappers (`div.highlight` around a chroma `pre`) become `<pre><code class="language-…">` using the text, not the spans;
  - `paragraph-header` classes, inline styles and empty spans are dropped.
- Map content headings when the page title is the only `h1`: source `h3` → `h2`, `h4` → `h3`, so the outline stays valid.
- Rewrite the canonical origin to root-relative paths; keep external links with `rel="noopener"`.
- Preserve the copy verbatim. If the design needs a note, append an edition line to the colophon rather than editing the author's prose. Replace emoji with ASCII punctuation (the 8x16 face has no emoji), and leave a short factual note if the colophon describes the old stack.
- Home extras (about, useful links, colophon) can be serialized from the live page; do not retype them from memory.

## 3. Render the pages

- One document window per route: title `<SLUG>.TXT`, info line with date · reading (post) or counts (index), body, and the same contents sidebar everywhere.
- Post rows: a fixed date column and a title column, 1px rule per row, hover inverse; stack date above title under 480px.
- Tags index: chips with counts in parentheses and a one-line explanation; tag pages are the filtered post list.
- Keep the feed absolute to the canonical domain, not to the preview host.
- TOS 4 default, checked "TOS 4 colours" toggle, teal `theme-color`; classic green is the alternate.
- Self-contained assets next to the HTML: `gem.css`, the page CSS, the WOFF2, the favicon.

## 4. Build and verify

- A `build.py` that wipes `site/`, regenerates every route and copies the assets, so a stale page cannot survive a rebuild.
- Crawl every internal `href` from the generated HTML and assert the target exists; parse the RSS with an XML parser.
- Serve over HTTP before judging the look. Root-absolute asset paths on `file://` resolve to the filesystem root, so the page renders unstyled even though the CSS is fine.
- Check `document.documentElement.scrollWidth === clientWidth` at 393px, then screenshot home, a post, a tag page, and a post with a table/image.
- On a tunnel, request the HTML and each asset separately; a 200 page with a 404 stylesheet is the unstyled failure.

## Pitfalls

- **Generator leftovers**: TOC blocks, heading anchors, chroma spans and `paragraph-header` classes leak into the new page if not unwrapped.
- **Code wrapping**: prose is 16/24 and code blocks should be `white-space: pre` with `overflow-x: auto`; `pre-wrap` on long shell lines produces unreadable ragged blocks.
- **Emoji**: `✨` and friends render as tofu in the 8x16 face; replace with punctuation or an intentional emoji span.
- **Modern favicon**: a rounded, anti-aliased PNG mark clashes with the 1px chrome. Redraw it as a 32px block (text mark or pixel art) and keep the PNG only as the favicon.
- **Inline code**: one full cell (8px) of padding and `font: inherit`; otherwise the chip hugs the surrounding glyphs.
- **Rightmost menu**: `.gem-dropdown--right`, or the panel opens off-screen.
- **Cached CSS on the tunnel**: Cloudflare caches `.css` by extension; bump `?v=` on every stylesheet change or the old skin survives for hours.
