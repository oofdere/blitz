<p>
<picture >
  <source media="(prefers-color-scheme: dark)" srcset="https://blitz-website.fly.dev/static/blitz-logo-with-text3-white.svg">
  <img height="70" alt="Blitz" src="https://blitz-website.fly.dev/static/blitz-logo-with-text3.svg">
</picture>
</p>

**A modular HTML/CSS rendering engine**

[![Build Status](https://github.com/dioxuslabs/blitz/actions/workflows/ci.yml/badge.svg)](https://github.com/dioxuslabs/blitz/actions)
[![Crates.io](https://img.shields.io/crates/v/blitz.svg)](https://crates.io/crates/blitz)
[![Docs](https://docs.rs/blitz/badge.svg)](https://docs.rs/blitz)
[![Crates.io License](https://img.shields.io/crates/l/blitz)](#license)
[![dependency status](https://deps.rs/repo/github/dioxuslabs/blitz/status.svg)](https://deps.rs/repo/github/dioxuslabs/blitz)

Talk to us in: the [#native](https://discord.gg/AnNPqT95pu) channel in the [Dioxus Discord](https://discord.gg/AnNPqT95pu)

## This branch: `niso`

This branch is oofdere's fork of [DioxusLabs/blitz](https://github.com/DioxusLabs/blitz)
for [niso](https://github.com/oofdere/niso), whose layout side uses `blitz-dom`
for style and layout. It starts from upstream `main` at
`da271cde1237cad9f7e37e0416464e47833695c7` (12 November 2026) and adds one commit
per patch. It changes nothing else, and the patches are not being sent upstream
for now. The fork's other branches are untouched.

**Licences.** The fork keeps upstream's licences. Blitz is MIT OR Apache-2.0
(`stylo_taffy` is also MPL-2.0), and every patch on this branch is under the same
terms as the code it changes. `packages/blitz-dom/assets/default.css` (MPL-2.0)
and `packages/blitz-dom/assets/moz-bullet-font.otf` are unchanged and keep their
own terms.

### How niso uses it

niso depends on `blitz-dom` with default features off and `system-fonts` on.
It keeps the DOM on its own side and applies each task's changes to a
`BaseDocument` through `DocumentMutator`, with these calls only:

- a node comes into the document: `create_element` or `create_text_node`, then
  `append_children` or `insert_nodes_before`;
- a node in the document moves: `append_children` or `insert_nodes_before` on a
  connected node;
- a node leaves the document with everything under it: `remove_and_drop_node_with`;
- an attribute is set or removed: `set_attribute` or `clear_attribute`;
- a text node's data is set: `set_node_text`.

Each patch below fixes something those calls reach, and each has a test in
`packages/blitz-dom/tests/` that fails without it.

### The patches

1. **A fuzzer of the edits niso makes.** `tests/change_list.rs` makes random
   edits of the kinds above, with layouts between some of them. It fails on a
   panic and counts the seeds whose incremental layout differs from a full
   relayout. On upstream, 2,000 release seeds give 255 panics (`invalid SlotMap
   key used`).
2. **A hidden containing block lets go of its hoisted boxes, and a freed box
   leaves every layout list.** Hiding an element or removing a node made layout
   reach a freed box. Six tests. Brought over unchanged from niso's earlier
   vendored copy of blitz.
3. **A containing block that claims a box takes it from the block that held
   it.** A box could sit in two containing blocks' lists, and freeing it left it
   in one. One test. Brought over unchanged.
4. **A box changed while construction skips it is built when reached.**
   Something changed while hidden, or while a table skipped it, reused an
   inline layout whose boxes had been freed. Two tests. Brought over unchanged.
5. **An element laid out in an inline flow lets go of the box it had.** Its old
   list of layout children and its anonymous blocks stayed behind. Two tests.
   New.
6. **A moved box leaves its old layout parent's list.** A container in a
   `display: none` subtree kept listing a box that had moved out. One test. New.
7. **A feature for the user-agent stylesheet and bullet font.** Both files are
   now behind `default-ua-stylesheet`, which is on by default and on for every
   crate in this workspace. niso turns it off and supplies its own user-agent
   stylesheet through `DocumentConfig::ua_stylesheets`, so neither file is in
   its build. Two tests. New.

With all seven, the fuzzer has no panicking seed in 2,000 debug and 20,000
release seeds.

### Left out on purpose

niso's earlier copy of blitz carried other patches that this branch does not
need:

- `9c94a055` fixed a build with no default features. Upstream builds with
  default features off now, as this branch's tests show.
- `9f7949b6` and `f7004c03` changed only `remove_and_drop_all_children`, which
  niso never calls.
- Style sheets in tree order belong to loading HTML and CSS, a later step of
  niso's, and will come with it if they are still needed.

### Known difference that is not fixed

Incremental layout can still give a different height from a full relayout. The
smallest case: a `position: fixed` container holds an `inline-block` div and a
block div, each with one line of text. After a layout, a div with one line of
text is appended to the block div. The container's height is then 52px after
incremental layout and 48px after a full relayout. Without fixed positioning,
or with a block in place of the inline-block, both give 48px. The fuzzer counts
these cases and does not fail on them.

### Running the tests

```
cargo test -p blitz-dom --no-default-features --features system-fonts \
  --test change_list --test ua_stylesheet
CHANGE_LIST_FUZZ_SEEDS=20000 cargo test --release -p blitz-dom \
  --no-default-features --features system-fonts \
  --test change_list -- --ignored --nocapture change_list_fuzz
```

## Status

Blitz is currently in a **beta** state. It can already render many popular no-JS websites (Wikipedia, (Old) Reddit, etc), and is usable for making apps if you are an early adopter and willing to live on the bleeding edge. But there are also still many bugs and missing features. We are actively working on bringing it up to production quality.

Check out:
- The [downloads page](https://blitz.is/downloads) for runnable builds of the Browser UI (Windows / macOS / Linux / Android)
- The [status page](https://blitz.is/status/css) for current status
- The [roadmap issue](https://github.com/DioxusLabs/blitz/issues/119) for planned work


![screenshot](https://raw.githubusercontent.com/DioxusLabs/screenshots/main/blitz/wikipedia.png)


## Trying it out

1. Clone this repo
2. Run one of our examples:
    - The Browser UI: `cargo run -rp browser`
    - The Markdown viewer: `cargo run -rp rdme ./README.md`
    - TodoMVC app: `cargo run -rp todomvc`
    - WGPU texture integration example: `cargo run -rp wgpu_texture`

Other examples are available in the [examples/](./examples/) folder.

## Goals

Blitz is designed to render HTML and CSS - we *don't* want to support the entirety of browser features (or at least we want to make all such "extra" features opt-in). In our opinion, the browser is bloated for the basic use case of rendering HTML/CSS.

We do intend to support:

- Modern HTML layout (flexbox, grid, table, block, inline, absolute/fixed, etc).
- Advanced CSS (complex selectors, media queries, css variables)
- HTML Form controls
- Accessibility using AccessKit
- Extensibility via custom widgets

Notably we *don't* provide features like webrtc, websockets, bluetooth, localstorage, etc. In a native app, much of this functionality can be fulfilled using regular Rust crates and doesn't need to be coupled with the renderer.

We don't yet have Blitz bindings for other languages (JavaScript, Python, etc) but would accept contributions along those lines.

## Architecture

Blitz consists of a core DOM abstraction; several modular pieces which provide additional functionality like networking, rendering, windows, and state management; and two high-level wrappers that support rendering either a Dioxus application or HTML with a simplified API.

These pieces can be combined together to make a cohesive web engine.

### High-level "wrapper" crates

- **`blitz`** - An HTML/markdown frontend that can render an HTML string. This is useful for previewing HTML and/or markdown files but currently lacks interactivity.
<br /><small><b>Uses: `blitz-dom`, `blitz-html`, `blitz-shell`, `blitz-renderer-vello`</b></small>
- **`dioxus-native`** - A Dioxus frontend that can render a Dioxus VirtualDom. This has full interactivity support via Dioxus's event handling.
<br /><small><b>Uses: `blitz-dom`, `dioxus-core`, `blitz-shell`, `blitz-renderer-vello`</b></small>

Both wrappers can optionally use <b>`blitz-net`</b> to fetch sub-resources.


### Using the git version of Dioxus Native

The latest development version of the Dioxus Native lives in this repository. As Dioxus Native is under rapid development it can be useful to use this version to get access to the latest features and bug fixes sooner than they are available in an official release.

To use the git version of `dioxus-native`:

- Remove your dependency on the `dioxus` crate entirely.
- Add `dioxus-native = { git = "https://github.com/DioxusLabs/blitz", rev = "e64a3d8", features = ["prelude"] }`
- (replace `e64a3d8` with the git commit id of the version you want to use)
- In your rust code change all instances of `use dioxus::prelude::*` to `use dioxus_native::prelude::*`.
- If you need to access additional functionality from the `dioxus` crate that is not exported from the Dioxus Native prelude then you can import it from the individual sub-crates (`dioxus-html`, `dioxus-signals`, `dioxus-router`, etc) instead.

The git versions of Dioxus Native currently depend on Dioxus `0.8.0-alpha.3` from crates.io (commits before `286874c` use the stable v0.7.x version), so any additional libraries that you are using (`dioxus-sdk`, `dioxus-components`, `dioxus-free-icons`, etc) need to be versions that work with Dioxus `0.8.0-alpha.3`.

### Modular Components

#### Core crates

- **`blitz-dom`** - The core DOM abstraction that includes style resolution, layout and event handling (but not parsing, rendering or system integration).
<br /><small><b>Uses: [Stylo](https://github.com/servo/stylo) (CSS parsing/resolution), [Taffy](https://github.com/DioxusLabs/taffy) (box-level layout), [Parley](https://github.com/linebender/parley) (text layout)</b></small>
- **`blitz-traits`** - Minimal base crate containing types and traits to allow the other crates to interoperate without depending on each other

#### Additional crates

- **`blitz-net`** -  Networking that can fetch resources over http, from the file-system or from encoded data URIs.
<br /><small><b>Uses: [reqwest](https://github.com/seanmonstar/reqwest) (HTTP client)</b></small>
- **`blitz-paint`** - Translates a `blitz-dom` tree into `anyrender` draw commands.
<br /><small><b>Uses: [anyrender](https://github.com/dioxuslabs/anyrender) (2D drawing abstraction)</b></small>
- **`blitz-html`** -  Adds HTML parsing to `blitz-dom`
<br /><small><b>Uses: [html5ever](https://github.com/servo/html5ever) (HTML parsing) and [xml5ever](https://github.com/servo/html5ever/tree/main/xml5ever) (XHTML parsing)</b></small>
- **`blitz-shell`** - A shell that allows Blitz to render to a window (integrates a Winit event loop, AccessKit, Muda etc).
<br /><small><b>Uses: [winit](https://github.com/rust-windowing/winit) (windowing/input), [accesskit](https://github.com/AccessKit/accesskit) (accessibility), [muda](https://github.com/tauri-apps/muda) (system menus)</b></small>

The AnyRender rendering abstraction now lives in it's repository over at https://github.com/dioxuslabs/anyrender

## License

This project is dual licensed under the Apache 2.0 and MIT licenses.

The `stylo_taffy` crate is ADDITIONALLY licensed under MPL 2.0 (so it is triple licensed under Apache 2.0, MIT, and MPL 2.0 licenses) for easier interop with the Servo project.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Blitz by you, shall be dual licensed as Apache 2.0 and MIT (and MPL 2.0 if submitted to the `stylo_taffy` crate), without any additional terms or conditions.
