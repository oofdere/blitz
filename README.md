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
keeps its own terms; patch 26 adds the HTML Standard's two rules for hidden
elements to it, and nothing else changes it. `packages/blitz-dom/assets/moz-bullet-font.otf`
is unchanged and keeps its own terms.

### How niso uses it

niso depends on `blitz-dom` with default features off and `system-fonts` and
`floats` on.
It keeps the DOM on its own side and applies each task's changes to a
`BaseDocument` through `DocumentMutator`, with these calls only:

- a node comes into the document: `create_element` or `create_text_node`, then
  `append_children` or `insert_nodes_before`;
- a node in the document moves: `append_children` or `insert_nodes_before` on a
  connected node;
- a node leaves the document with everything under it: `remove_and_drop_node_with`;
- an attribute is set or removed: `set_attribute` or `clear_attribute`;
- a text node's data is set: `set_node_text`.

None of these calls checks that a node is not put under itself, and once one
is, its parents make a loop and blitz's walks up the tree never end, as
`reset_form_owner` does for each node put in. niso never makes one: its tree
refuses to put a node under itself (the DOM's pre-insert validity, a
`HierarchyRequestError`), and a task's changes are a record in the order they
were made, so the copy is a valid tree after each change. Only a change list cut
down by hand, as a minimizer makes them, gets there.

Each patch below fixes something those calls reach, and each has a test in
`packages/blitz-dom/tests/` that fails without it.

### The patches

1. **A fuzzer of the edits niso makes.** `tests/change_list.rs` makes random
   edits of the kinds above, with layouts between some of them. It fails on a
   panic and counts the seeds whose incremental layout differs from a full
   relayout. On upstream, 2,000 release seeds give 255 panics (`invalid SlotMap
   key used`). Since patch 8 it also gives elements `display: contents`, and
   after the last layout it checks every layout list and every hoisted list. A
   seed that breaks one of their rules is broken, and fails the fuzzer as a
   panic does.
2. **A hidden containing block lets go of its hoisted boxes, and a freed box
   leaves every layout list.** Hiding an element or removing a node made layout
   reach a freed box. Six tests. Brought over unchanged from niso's earlier
   vendored copy of blitz.
3. **A containing block that claims a box takes it from the block that held
   it.** A box could sit in two containing blocks' lists, and freeing it left it
   in one. One test. Brought over unchanged. That test stopped pinning this
   patch once patch 4 landed: with patch 4 in place it passes without this one,
   so its commit message claims more than the test shows. Patch 9's test pins
   it instead.
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
   its build. Two tests. New. This workspace's own dependency on `blitz-dom`
   turns the feature on, so linking another crate of this fork (`blitz-paint`,
   `blitz-html`, `blitz-shell`) would bring both files back into niso's
   build. Nothing to change while niso uses `blitz-dom` alone.
8. **An element that becomes `display: contents` lets go of its box.** Its
   children were hoisted into the nearest container with a box, but it kept
   its own list of layout children and its anonymous blocks, so that list named
   boxes laid out elsewhere. Three tests. New.
9. **A test that pins patch 3.** No code change. A box inside an absolutely
   positioned box whose containing block becomes `display: contents` must be
   in one hoisted list. Without patch 3, the fuzzer finds 126 broken seeds in
   5,000. New.
10. **Freeing many boxes of one container stays linear.** Patch 2 took each
    freed box out of its container's lists with a pass over each whole list.
    Now the containers are recorded and each one is swept once before
    construction and again before layout. 20,000 boxes of one containing block
    took 209 ms to remove in a release build, and now take 2.6 ms (2.9 ms on
    upstream). One test. New.
11. **A moved element takes the boxes under it out of their lists.** Moving an
    element cleared only its own layout parent. The children of a
    `display: contents` element and the boxes of an inline element are listed
    by an ancestor's box, and stayed in that list after the move. The move now
    walks the subtree, and every node whose layout parent is outside it leaves
    that parent's list. Two tests. New.
12. **One mutator that puts an element in and takes it out does not panic.**
    The mutator queues `style`, `title` and form-control elements to process
    when it is dropped, and it reached the ones already freed. It now skips
    them. Three tests. New.
13. **A replaced element gets no layout children, whatever its `display`.**
    An `img`, `svg`, `canvas`, `video`, `embed` or `iframe` with
    `display: table` and a child was built as a table and reached an
    unreachable branch. One test. New.
14. **Freeing a box clears the layout parent of the inline content that
    points at it.** A read under a freed anonymous block reached it. One test.
    New.
15. **A read stops at a node that was freed.** The walk from an inline element
    to its inline root used ids that a table had left out and that were
    freed. One test. New.
16. **A box that was never built is built when construction reaches it.** An
    element put into a `display: none` subtree has no construction damage, so
    it was not built when shown, and kept height 0. One test. New.
17. **`Viewport` and `ColorScheme` are exported from `blitz-dom`.** niso sets a
    viewport without naming `blitz-traits`. One test. New.
18. **`get_client_bounding_rect` applies transforms.** The rect is mapped
    through each containing block's position, scroll offset and `transform`,
    as CSSOM View asks. The `offset*` path still ignores transforms. Five
    tests. New.
19. **An inline element reads its content area, not its line box.** Its rect
    spans the font's ascent and descent around the baseline, which
    `line-height` does not change, as CSS 2 10.6.1 has it. One test. New.
20. **A child that a table leaves out reads as never laid out.** blitz builds
    no anonymous rows and cells, so a block in a `display: table` has no box.
    It kept the layout it had before the container became a table. One test.
    New.
21. **The children of a form control or replaced element read as never laid
    out.** A block moved into one kept its old layout. One test. New.
22. **A size read from the layout cache keeps its baselines.** Taffy's cache
    keeps only a size for a measurement. An inline-block measured from the
    cache had no baseline, sat its bottom on the line's baseline, and made the
    line taller. A second cache keeps the baselines. One test. New.
23. **An element that lets go of its box lets go of all of it.** One that
    became inline kept its inline-root flag, inline layout and layout, and a
    read used them. One test. New.
24. **A box in a `display: none` subtree lets go of its box.** A block moved
    into a hidden subtree and changed there reused its old box when moved out.
    Construction of a hidden element now leaves its children out of layout,
    and a node moved into a parent with no box leaves layout. One test. New.
25. **A box whose layout changed resolves its transform again.** The
    transform pass skipped a box with no damage of its own, so a box resized
    by a change elsewhere kept the transform origin of its old size. A box
    whose rounded layout changed now gets overflow damage, and so does every
    box it is in. One test. New.
26. **`hidden` is hidden by the user-agent sheet.** A presentational hint made
    every `hidden` attribute `display: none`, so `hidden=until-found` and a
    hidden `embed` had no box, and `display: revert` could not undo it. The
    hint is gone, and the default sheet has the standard's two rules for hidden
    elements. Two tests. New.
27. **A `display: contents` element whose last box was its `::before` lets go
    of it.** Construction stopped early for an element left with no children
    and no pseudo-elements, before it let go of its box, so its list kept the
    freed `::before`. One test. New.
28. **A row in a table lets go of the box it had outside one.** A table-row
    outside a table is laid out as a block; once its parent became a table, it
    still listed a child the table gives no box. Rows, row groups and
    `display: contents` elements in a table now let go of their own box. One
    test. New.
29. **Nothing under a `display: none` element in an inline flow has a box.**
    The inline flow skipped such an element without leaving its children out
    of layout, as a block container does since patch 24, so a block moved under
    it kept a layout cache that named an out-of-flow box that had moved away.
    One test. New.
30. **A move into an inline element that is laid out keeps the box.** Patch 24
    took a node moved into any parent with no list of its own out of layout,
    which included a span in a paragraph: the moved subtree was laid out again
    from scratch. Only a parent that also has no layout parent (hidden, or never
    built) counts now. One test. New.
31. **A box built again builds its paint lists again.** A box moved into a
    `display: none` element lets go of its box there, and its damage is cleared
    without the paint tree reaching it. Moved back out, it was built again with
    no damage, and an empty box in a shrink-to-fit parent gets none from layout
    either, so the paint tree kept its old list and painted a child that had
    moved away twice. A box built again now carries the damage a new box does.
    One test. New.
32. **A list item that joins an inline flow loses its marker.** A list item gets
    its marker when its parent's box is constructed. When that parent stopped
    having a box of its own and was laid out in an inline formatting context
    instead (it became `display: contents` inside an inline element, or an
    inline element around it lost its last block), the list item kept its
    marker, where a page built fresh gives it none. Construction now drops the
    marker of each child of an element it lays out in the flow. Like upstream,
    a list item inside an inline element still gets no marker, where a browser
    gives it one. Two tests. New.
33. **Outside markers are laid out again when the scale changes.** An outside
    marker is shaped in device pixels when its list is constructed. A scale
    change built every inline layout again but not markers, so a marker kept
    the size it had at the old scale. It is now laid out again with them. One
    test. New.
34. **A box whose block formatting context has floats is laid out again.** The
    layout cache's key does not include the floats a box reads from its parent's
    block formatting context or places into it, and a cached result places no
    floats. After an edit elsewhere in that context, a cached box's floats were
    left out and lines kept the room of floats that had moved. Such a box now
    uses and keeps cached results only while its context has no floats, as a
    fresh page lays it out. One test. New.
35. **A node that lets go of its box drops its transform.** A node left out of
    layout (moved into a canvas, a form control or a `display: none` element,
    or left out of a table) kept the transform resolved against its old box,
    and its bounding rect went through it. It now reads as a node never laid
    out. Three tests. New.
36. **A change to `text-wrap-mode` builds the text again.** The wrap mode is
    part of the text's built layout, and a change to it (`white-space: nowrap`)
    only laid the old lines out again at the new width. It now gets the damage
    the other text properties that break lines get. Two tests. New.
37. **The pseudo-elements of a `display: contents` child stay in its flow.**
    A container's first construction read them before they were made, and the
    inline flow left out those of a `display: contents` element, so a box
    built again lost its `::before` text. Both now see them. Three tests. New.
38. **A `<style>` element's sheet is its own text children.** Its sheet was
    the text of all its descendants, and text put under an element inside it
    reached a page built fresh but not one edited after layout. It is now its
    child text content, as HTML says. One test. New.
39. **A scale change recascades the tree.** Border widths are snapped to
    device pixels when their style is computed, and a scale change kept the
    old snapping. It now recascades, as a colour-scheme change does. One test.
    New.
40. **A measurement does not leave its lines in a laid-out box.** Measuring a
    box broke its lines at the measured width, and a layout taken from the
    cache did not break them again. A box already laid out now gets its lines
    back after a measurement. One test. New.
41. **The pseudo-elements of an inline in an inline flow are made and counted.**
    Those of an inline nested in another inline were never made, a container's
    first construction read an inline child's before they were, and a block
    `::before` of an inline did not break its line. They are now made before
    the container's children are classified, and a block one breaks the line.
    Two tests. New.
42. **A box's own scroll offset and the viewport's do not move a fixed box in
    reads.** `getBoundingClientRect` took a box's own scroll offset off the
    box, after its transform, and the viewport's scroll off a fixed box the
    root element holds. A containing block's scroll now moves what it holds,
    inside its transform, and a box's own offset moves only its inline
    fragments. Five tests. New.
43. **The viewport scrolls over the document's scrollable overflow.** It
    scrolled over the root element's box and its in-flow overflow, from the
    top left. It now takes the root element's margin box and the overflow of
    what the root element holds, absolute boxes of the initial containing
    block included, from the origin the root element's writing mode gives,
    so a right-to-left page scrolls to the left. `viewport_scroll_area`
    gives that area, for the scroll clamp and the scroll events. Four tests.
    New.
44. **A fixed box adds nothing to what the viewport scrolls over.** The root
    element's scrollable overflow holds the fixed boxes hoisted into it, and
    `resolve` now keeps that overflow without them for the viewport. One
    test. New.
45. **The body gives its overflow to the viewport.** When the root element's
    overflow is `visible` on both axes, the viewport takes the body's (CSS
    Overflow 3, section 3.3), and the body's box uses `visible`: it is no
    scroll container, does not clip, and is laid out, scrolled, painted and
    hit as visible. `Node::used_overflow` gives what a box uses and
    `BaseDocument::viewport_overflow` what the viewport takes, which also
    decides on which axes the user can scroll the page. Four tests. New.
46. **The root element's box uses `visible`, and `clip` on the viewport is
    `hidden`.** The root element's overflow always goes to the viewport, so
    its own box uses `visible`, as the body's does once it gives its
    overflow. `clip` on the viewport is read as `hidden` (the same
    section): code can scroll the page and the user cannot. Two tests. New.
47. **A float in an inline flow keeps its scrollable overflow.** Inline layout
    places a float it meets in the block formatting context and set its size,
    place and margins, but not the scrollable overflow its own layout gave. A
    floated scroll container in a `span` read none on a page built fresh, and
    the overflow of an older layout on a page edited after one. Two tests,
    with the `floats` feature. New.
48. **The root element's box and the initial containing block use the
    principal writing mode.** CSS Writing Modes 4 (section 8) takes the used
    `writing-mode` and `direction` of an `html` root element from its first
    displayed `body` child, unless either has containment, and propagates them
    to the initial containing block and the viewport. Only the `writing-mode`
    feature did, for the root element's layout, and without the `display` and
    `html` checks. The root element's box and the initial containing block now
    take the body's direction, and the viewport scrolls from the origin it
    gives, with or without the feature. Six tests. New.
49. **What a box clips adds nothing to its parent's scrollable overflow.** A box
    gave its parent its whole scrollable overflow, also on an axis it clips, so
    the viewport of patch 43 scrolled over what a scroll container holds: a
    100px list with 2,000px in it made the page scroll 1,400px. A box now gives
    only its border box on an axis whose used `overflow` is not `visible`. One
    test. New.
50. **Containment keeps the body's `overflow` its own.** CSS Containment 2
    (section 3) turns off what the body gives the viewport when the root
    element or the body has containment, its `overflow` as well as its writing
    mode (patch 48). Patch 45 did not check. One test. New.
51. **A transformed root's fixed boxes scroll with the page.** A root element
    with a transform, a filter or containment is the containing block of its
    fixed-position descendants (CSS Transforms 1, section 2), and layout placed
    them against its padding box, but reads, the page's scrollable overflow,
    hit testing and paint took every fixed box the root element holds to be
    placed against the viewport. `Node::is_fixed_to_viewport` now decides that
    in one place. Hit testing and the paint position of a hoisted fixed box
    never recognised the root element at all, so in hits a fixed box an
    untransformed root holds moved with the page too. Five tests. New.
52. **getBoundingClientRect reads vertical writing modes physically.** With
    the `writing-mode` feature, a box's unrounded layout is in the writing
    mode of the box that places it, and the read used it as it was, so a box
    placed by a vertical one came back with its width and height swapped and,
    under a `*-rl` mode, not mirrored. Each box now goes through the
    conversion `physical_unrounded_geometry` uses. Three tests, with the
    feature. New.
53. **`elementFromPoint` takes viewport coordinates.** It handed the point to
    `hit`, which takes page coordinates, so once the page scrolled it found
    what had been under the point before. Two tests. New.
54. **Only the first `body` gives the principal writing mode.** Patch 48 took
    the first `body` child that is displayed, as the editor's draft of CSS
    Writing Modes 4 (section 8) says, a later one too. Blink and Gecko both
    look at the first `body` child only; Blink, as the draft, skips it when
    its `display` is `none`, and Gecko does not. This follows Blink: the
    first `body`, unless it is not displayed. One test changed, one case
    added.
55. **A table builds its grid again when its writing mode changes.** The
    table context holds the styles of the table and its parts converted into
    the table's writing mode, and a change to `writing-mode` only relaid the
    table out, so a table that turned vertical after a layout kept its
    horizontal sizes: `height: 100%` stayed auto instead of becoming the
    inline size. Found by niso's frames harness with vertical writing modes
    (seed 2050). Two tests. New.
56. **A box's layout cache holds only in its placer's writing mode.** Cached
    results are written in the axes of the box that placed the node, and
    nothing cleared them when that box's writing mode changed while the
    node's own did not: a `vertical-rl` box that stopped being orthogonal to
    its parent when the root turned vertical kept its old size. The cache now
    records the placer's writing mode and is not used in another. Found by
    the same harness (seed 2056). One test. New.
57. **Below `display: contents`, a box follows its layout parent's writing
    mode.** Stylo makes an inline box whose writing mode differs from its
    layout parent's an inline-block. Below a `display: contents` element that
    parent is further up, and Stylo stopped cascading at the `display:
    contents` element when its own style came out unchanged, so the box kept
    the display it had under the parent's old writing mode. When the style
    traversal reaches a `display: contents` element, the boxes below it
    whose `display` was adjusted for a writing mode other than their layout
    parent's are cascaded again, and only those. Found by the same harness
    (seed 2473). Two tests. New.
58. **An inline element's margin, border and padding take up width in its
    line.** Parley has no edge spacing on a span, so they took none: text
    after a padded link sat where it would without the padding. Each
    non-atomic inline element whose inline-start or inline-end margin, or
    border and padding, is not zero now gets a spacer at that edge: a
    zero-high in-flow inline box as wide as that margin, or as that border
    and padding, sized when the line is laid out, so percentages are of the
    containing block. Start and end follow the element's `direction`. Word
    joiners, and a line-break override that keeps Parley from breaking beside
    one, keep each spacer to its element's content and to what comes before
    or after it unless white space comes between them, and white space after
    a spacer collapses with white space before it. The margin spacers are not
    part of the element's client rectangles, the others are, and the
    selected text and `innerText` leave the spacers and their joiners out.
    `InlineSpacer::from_id` tells a spacer's inline box from an atomic
    inline's. Twelve tests. New.
59. **A style sheet answers only the `<link>` still waiting for it.** The
    answer to a link's request went to the link's node whatever had become
    of it: a link taken out of the document before its sheet came made blitz
    index a dead node and panic, and a link whose `href` had asked for
    another sheet, or that had been moved and asked again, took whichever
    answer came last. The document now keeps the request each link waits on,
    and passes over any other answer. A request given up, by a removal or a
    new request, no longer holds rendering up as a critical resource. Three
    tests. New.
60. **An `@import` whose URL resolves against nothing is left out.** Its
    rule unwrapped the resolved URL and panicked: on a page with no base URL
    yet, and on one whose own URL is a `data:` URL, which nothing resolves
    against. It is now refused like an import past the depth limit, logged
    under the `tracing` feature, and the rest of its sheet applies. Three
    tests. New.

Patches 27 to 29 come from Lens's fuzzer (`lens_fuzz`, seeds 234 and 244, 1442,
and 600), patch 30 from measuring patch 24's cost, patch 31 from Lens's fuzzer
over niso's frames (seed 172), patches 32 and 33 from the same fuzzer (seed
2104, and its mode that changes the viewport between tasks), and patches 34 and 35
from niso's copy harness, which reads a kept page and a fresh one (59 of 2,000
seeds, and seeds 278, 978 and 1944). Patches 36 and 37 come from Lens's fuzzer
(seeds 96, 612, 1221 and 1822, and 1137 and 1658), patch 38 from the frames
fuzzer's mode that changes a style sheet (seed 1210), and patches 39 and 40 from
its viewport mode (14 seeds, and seed 59). Patch 41 comes from the same mode
as patch 38 (seed 1223). Patches 42 to 44 come from Lens's reads of niso's
scroll offsets (`read77`), patch 45 from propagating the body's overflow as
CSS Overflow 3 says, patch 46 from Lens's two gaps in patch 45, and patch 47
from niso's frames fuzzer (seed 2121). Patch 48 is the first of niso's list for
later, and patches 49 and 50 came up while it was made.

With all of them, the fuzzer has no panicking or broken seed in 2,000 debug and
20,000 release seeds, and Lens's fuzzer has no panicking seed and no broken list
in 2,000 debug and 2,000 release seeds (with an 800×600 viewport and with none)
and 300 seeds of its wide mode.

Lens's fuzzer has no stale seed in 2,000 release seeds, with floats and without,
and the frames fuzzer none in 200 seeds, nor in 300 of its viewport mode or of its
mode that changes a style sheet.

### Left out on purpose

niso's earlier copy of blitz carried other patches that this branch does not
need:

- `9c94a055` fixed a build with no default features. Upstream builds with
  default features off now, as this branch's tests show.
- `9f7949b6` and `f7004c03` changed only `remove_and_drop_all_children`, which
  niso never calls. Patch 10 does what `9f7949b6` did, for every path that
  frees a node.
- Style sheets in tree order belong to loading HTML and CSS, a later step of
  niso's, and will come with it if they are still needed.

### Known cost that is not fixed

`remove_and_drop_node_with` takes a node out of its DOM parent's `children`
with a pass over that list, on upstream as here. Removing n children of one
parent one by one is still O(n²): 20,000 children take about 105 ms in a release
build, both here and on upstream.

Moving a node walks everything under it (patch 11), to find the boxes whose
layout parent is outside it. In a release build that is about 40 ns a node: 0.04
ms for a subtree of 1,000 nodes and 0.7 ms for 16,000, against 0.1 ms and 1.4 ms
for the layout that follows the move. Moving one leaf takes under 1 µs.

A subtree moved into a `display: none` element and back is laid out again from
scratch when shown, since nothing under a hidden element keeps its box (patch
24): about 4 ms for 1,000 nodes and 68 ms for 16,000, where a move between two
shown blocks takes 0.1 ms and 1.4 ms.

### Known differences that are not fixed

Incremental layout can still differ from a full relayout, in 2 of 20,000
release seeds of the fuzzer (1 of 2,000 debug seeds). The fuzzer counts these
cases and does not fail on them. The smallest one: a `display: flex` div with a
`::before` is laid out, an absolutely positioned div comes into it, an inline,
relatively positioned div with a fixed `::after` comes before it, and the flex
div becomes `display: contents`. The absolutely positioned div is then at
(8, 24) after incremental layout and at (0, 16) after a full relayout. Its
static position, under a `display: contents` element in an inline flow, is
where the two differ, and neither is checked against a browser yet. The case
noted here before (a `position: fixed` container with an inline-block, 52px
against 48px) gives 48px both ways now.

Lens's fuzzer finds the same kind of difference in 6 of 2,000 seeds with an
800×600 viewport (8 with no viewport, 1 of 300 in its wide mode). The same seeds
differ before patches 26 to 30. They are not looked into yet.

blitz builds no anonymous table rows and cells (CSS 2.1 17.2.1). A child of a
`display: table` box that is not a table part (a block, text) gets no box, and
reads as never laid out, where a browser wraps it in an anonymous cell.

An inline element with no content on any line has no fragment, and
`get_client_bounding_rect` answers `None` for it, unless it has a spacer
(patch 58): then its rectangle is its spacers', with no height.

A spacer (patch 58) is a U+FFFC in the text, which the Unicode bidi algorithm
places as a neutral: in an element whose `direction` differs from its text's,
the start spacer goes where that text's direction puts it, not at the
element's start. A spacer is glued to what comes before or after it unless
white space comes between, so a break a browser would allow there for another
reason (after a hyphen, between ideographs) is lost, and `text-transform:
capitalize` sees a spacer as a word boundary.

### Running the tests

```
cargo test -p blitz-dom --no-default-features --features system-fonts \
  --test change_list --test ua_stylesheet --test rects --test hidden
cargo test -p blitz-dom --no-default-features --features system-fonts,floats
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
