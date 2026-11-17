//! An inline element's margin, border and padding take up width in its line, at its inline start
//! and end, as CSS 2's 10.3.1 and 9.4.2 have it. The tests compare each element with the same
//! element without them, so they hold in any font.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div { display: block } span { display: inline }";

fn create(doc: &mut BaseDocument, name: &str) -> NodeId {
    let name = QualName::new(None, ns!(html), LocalName::from(name));
    doc.mutate().create_element(name, Vec::new())
}

/// An element with this style and, optionally, a text node, put last in `parent`.
fn element(doc: &mut BaseDocument, parent: NodeId, name: &str, style: &str, text: &str) -> NodeId {
    let id = create(doc, name);
    let style_attr = QualName::new(None, ns!(), LocalName::from("style"));
    doc.mutate().set_attribute(id, style_attr, style);
    if !text.is_empty() {
        let text = doc.mutate().create_text_node(text);
        doc.mutate().append_children(id, &[text]);
    }
    doc.mutate().append_children(parent, &[id]);
    id
}

fn text(doc: &mut BaseDocument, parent: NodeId, data: &str) {
    let text = doc.mutate().create_text_node(data);
    doc.mutate().append_children(parent, &[text]);
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
}

fn rect(doc: &BaseDocument, node: NodeId) -> Rect {
    let r = doc.get_client_bounding_rect(node).unwrap();
    Rect {
        x: r.x,
        y: r.y,
        width: r.width,
    }
}

/// The rectangles of a line `a<span style>b</span>c`, in a block with this style: the span's, the
/// `b`'s and the `c`'s, each in a span of its own.
fn line(block: &str, style: &str) -> [Rect; 3] {
    line_of(block, style, ["aaa", "bbb", "ccc"])
}

/// The same, with these three words.
fn line_of(block: &str, style: &str, words: [&str; 3]) -> [Rect; 3] {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, "html");
    doc.mutate().append_children(root, &[html]);
    let body = element(&mut doc, html, "body", "margin: 0", "");
    let div = element(&mut doc, body, "div", block, "");
    text(&mut doc, div, words[0]);
    let span = element(&mut doc, div, "span", style, "");
    let inner = element(&mut doc, span, "span", "", words[1]);
    let after = element(&mut doc, div, "span", "", words[2]);
    doc.resolve(0.0);
    [rect(&doc, span), rect(&doc, inner), rect(&doc, after)]
}

/// The rectangles of `line` with the span styled `from` and laid out, then styled `to` and laid
/// out again.
fn restyled(from: &str, to: &str) -> [Rect; 3] {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, "html");
    doc.mutate().append_children(root, &[html]);
    let body = element(&mut doc, html, "body", "margin: 0", "");
    let div = element(&mut doc, body, "div", "", "");
    text(&mut doc, div, "aaa");
    let span = element(&mut doc, div, "span", from, "");
    let inner = element(&mut doc, span, "span", "", "bbb");
    let after = element(&mut doc, div, "span", "", "ccc");
    doc.resolve(0.0);
    let style = QualName::new(None, ns!(), LocalName::from("style"));
    doc.mutate().set_attribute(span, style, to);
    doc.resolve(0.0);
    [rect(&doc, span), rect(&doc, inner), rect(&doc, after)]
}

/// How far each of the three rectangles of `line` moves, and how much wider the span is, with
/// `style` than without it.
fn moved(block: &str, style: &str) -> ([f64; 3], f64) {
    moved_in(block, style, ["aaa", "bbb", "ccc"])
}

/// The same, with these three words. The line stays: the boxes of the inner span and the one
/// after do not move up or down. The span's own box grows by its border and padding above.
fn moved_in(block: &str, style: &str, words: [&str; 3]) -> ([f64; 3], f64) {
    let plain = line_of(block, "", words);
    let styled = line_of(block, style, words);
    for (plain, styled) in plain.iter().zip(&styled).skip(1) {
        assert_eq!(plain.y, styled.y, "{style} keeps the line");
    }
    (
        [0, 1, 2].map(|i| styled[i].x - plain[i].x),
        styled[0].width - plain[0].width,
    )
}

#[test]
fn padding_takes_width_inside_the_element() {
    assert_eq!(moved("", "padding-left: 7px"), ([0.0, 7.0, 7.0], 7.0));
    assert_eq!(moved("", "padding-right: 7px"), ([0.0, 0.0, 7.0], 7.0));
}

#[test]
fn margin_takes_width_outside_the_element() {
    assert_eq!(moved("", "margin-left: 3px"), ([3.0, 3.0, 3.0], 0.0));
    assert_eq!(moved("", "margin-right: 3px"), ([0.0, 0.0, 3.0], 0.0));
}

#[test]
fn a_border_takes_its_width() {
    assert_eq!(
        moved("", "border-left: 1px solid; border-right: 2px solid"),
        ([0.0, 1.0, 3.0], 3.0)
    );
}

#[test]
fn margin_border_and_padding_add_up_on_both_sides() {
    assert_eq!(
        moved("", "margin: 0 3px; border: 1px solid; padding: 0 7px"),
        ([3.0, 11.0, 22.0], 16.0)
    );
}

#[test]
fn a_percentage_is_of_the_containing_blocks_width() {
    assert_eq!(
        moved("width: 200px", "padding-left: 5%"),
        ([0.0, 10.0, 10.0], 10.0)
    );
}

/// Right-to-left words: the line runs from the right, so what comes later is further left.
const HEBREW: [&str; 3] = [
    "\u{5d0}\u{5d0}\u{5d0}",
    "\u{5d1}\u{5d1}\u{5d1}",
    "\u{5d2}\u{5d2}\u{5d2}",
];

#[test]
fn in_a_right_to_left_element_the_start_is_the_right() {
    let rtl = "direction: rtl";
    let start = moved_in(rtl, "padding-right: 7px", HEBREW);
    assert_eq!(start, ([-7.0, -7.0, -7.0], 7.0));
    let end = moved_in(rtl, "padding-left: 7px", HEBREW);
    assert_eq!(end, ([-7.0, 0.0, -7.0], 7.0));
    let margin = moved_in(rtl, "margin-right: 3px", HEBREW);
    assert_eq!(margin, ([-3.0, -3.0, -3.0], 0.0));
}

#[test]
fn an_element_with_nothing_in_it_is_as_wide_as_its_edges() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, "html");
    doc.mutate().append_children(root, &[html]);
    let body = element(&mut doc, html, "body", "margin: 0", "");
    let div = element(&mut doc, body, "div", "", "");
    let empty = element(
        &mut doc,
        div,
        "span",
        "padding: 0 5px; margin-left: 2px",
        "",
    );
    let after = element(&mut doc, div, "span", "", "ccc");
    doc.resolve(0.0);
    assert_eq!((rect(&doc, empty).x, rect(&doc, empty).width), (2.0, 10.0));
    assert_eq!(rect(&doc, after).x, 12.0);
}

#[test]
fn the_edges_are_no_place_to_break_a_line() {
    let [span, inner, after] = line("width: 0", "padding: 0 7px; margin: 0 3px");
    let [plain_span, ..] = line("width: 0", "");
    assert_eq!(span.y, plain_span.y);
    assert_eq!(inner.y, span.y);
    assert_eq!(after.y, span.y);
}

#[test]
fn white_space_collapses_across_the_edges() {
    let start = moved_in("", "padding-left: 7px", ["aaa ", " bbb", "ccc"]);
    assert_eq!(start, ([0.0, 7.0, 7.0], 7.0));
    let end = moved_in("", "padding-right: 7px", ["aaa", "bbb ", " ccc"]);
    assert_eq!(end, ([0.0, 0.0, 7.0], 7.0));
}

#[test]
fn a_line_still_breaks_at_white_space_beside_an_edge() {
    let before = moved_in("width: 0", "padding-left: 7px", ["aaa ", "bbb", " ccc"]);
    assert_eq!(before, ([0.0, 7.0, 0.0], 7.0));
    let after = moved_in("width: 0", "padding-right: 7px", ["aaa ", "bbb", " ccc"]);
    assert_eq!(after, ([0.0, 0.0, 0.0], 7.0));
}

#[test]
fn a_negative_margin_pulls_what_follows_back() {
    assert_eq!(moved("", "margin-left: -3px"), ([-3.0, -3.0, -3.0], 0.0));
}

#[test]
fn the_selected_text_is_only_the_words() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, "html");
    doc.mutate().append_children(root, &[html]);
    let body = element(&mut doc, html, "body", "margin: 0", "");
    let div = element(&mut doc, body, "div", "", "");
    text(&mut doc, div, "aaa");
    let style = "margin: 0 3px; border: 1px solid; padding: 0 7px";
    let span = element(&mut doc, div, "span", style, "");
    element(&mut doc, span, "span", style, "bbb");
    text(&mut doc, div, "ccc");
    doc.resolve(0.0);
    let length = doc
        .get_node(div)
        .unwrap()
        .element_data()
        .unwrap()
        .inline_layout_data
        .as_ref()
        .unwrap()
        .text
        .len();
    doc.set_text_selection(div, 0, div, length);
    assert_eq!(doc.get_selected_text().as_deref(), Some("aaabbbccc"));
}

/// An element whose edges change after it was laid out takes the width it would have had with
/// them from the start, and gives it back when they go.
#[test]
fn edges_set_or_taken_away_later_take_their_width() {
    let edges = "margin: 0 3px; border: 2px solid; padding: 0 7px";
    assert_eq!(restyled("", edges), line("", edges));
    assert_eq!(restyled(edges, ""), line("", ""));
    let wider = "margin: 0 5px; border: 2px solid; padding: 0 9px";
    assert_eq!(restyled(edges, wider), line("", wider));
}

/// The rectangles of two elements styled `style` that hold these two words and fill a line,
/// each in a plain span of its own when `nested`, as a list's items hold its links.
fn ends(style: &str, nested: bool, words: [&str; 2]) -> [Rect; 2] {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, "html");
    doc.mutate().append_children(root, &[html]);
    let body = element(&mut doc, html, "body", "margin: 0", "");
    let div = element(&mut doc, body, "div", "", "");
    let spans = words.map(|word| {
        let parent = match nested {
            true => element(&mut doc, div, "span", "", ""),
            false => div,
        };
        element(&mut doc, parent, "span", style, word)
    });
    doc.resolve(0.0);
    spans.map(|span| rect(&doc, span))
}

/// Collapsible spaces at the start and the end of a line are removed (CSS Text 3, 4.1.2), also
/// inside an element whose edges stand between them and the line's edge: the elements of
/// `" aaa "` and `" bbb "` are where and as wide as those of `"aaa "` and `"bbb"`, and the
/// space between the words stays.
#[test]
fn white_space_at_the_start_and_end_of_a_line_goes_inside_the_edges() {
    for style in [
        "padding: 0 7px",
        "margin: 0 3px",
        "border: 1px solid",
        "margin: 3px; padding: 3px 7px; border: 1px solid transparent",
    ] {
        for nested in [false, true] {
            assert_eq!(
                ends(style, nested, [" aaa ", " bbb "]),
                ends(style, nested, ["aaa ", "bbb"]),
                "{style}, nested {nested}"
            );
        }
    }
}

/// Where the word of a list item whose marker is inside it starts: the item holds `space` and
/// then the word in a span.
fn word_after_marker(list_style: &str, space: &str) -> f64 {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, "html");
    doc.mutate().append_children(root, &[html]);
    let body = element(&mut doc, html, "body", "margin: 0", "");
    let item = element(
        &mut doc,
        body,
        "div",
        &format!("display: list-item; list-style: inside {list_style}"),
        "",
    );
    if !space.is_empty() {
        text(&mut doc, item, space);
    }
    let word = element(&mut doc, item, "span", "", "xx");
    doc.resolve(0.0);
    rect(&doc, word).x
}

/// White space at the start of a list item's content stays after a marker inside it that is a
/// string, which ends in no space, and collapses with the space a bullet ends in.
#[test]
fn white_space_after_an_inside_marker_collapses_only_with_its_space() {
    assert!(
        word_after_marker("\"-\"", " ") > word_after_marker("\"-\"", ""),
        "after a string"
    );
    assert_eq!(
        word_after_marker("disc", " "),
        word_after_marker("disc", ""),
        "after a bullet"
    );
}

/// The rectangles of the elements `build` puts in a block, in the order it gives them.
fn built(build: impl Fn(&mut BaseDocument, NodeId) -> Vec<NodeId>) -> Vec<Rect> {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, "html");
    doc.mutate().append_children(root, &[html]);
    let body = element(&mut doc, html, "body", "margin: 0", "");
    let div = element(&mut doc, body, "div", "", "");
    let elements = build(&mut doc, div);
    doc.resolve(0.0);
    elements.into_iter().map(|node| rect(&doc, node)).collect()
}

/// White space that comes after the text that ends the content, between elements' edges or in
/// `display: contents`, is at the end of the line too, and goes.
#[test]
fn white_space_after_the_last_text_goes_too() {
    let padded = "padding: 0 7px";
    let spaced = built(|doc, div| {
        let first = element(doc, div, "span", padded, "aaa ");
        text(doc, div, " ");
        let second = element(doc, div, "span", padded, " ");
        vec![first, second]
    });
    let bare = built(|doc, div| {
        let first = element(doc, div, "span", padded, "aaa");
        let second = element(doc, div, "span", padded, "");
        vec![first, second]
    });
    assert_eq!(spaced, bare, "between edges");
    let spaced = built(|doc, div| {
        let first = element(doc, div, "span", padded, "aaa ");
        element(doc, div, "span", "display: contents", " ");
        vec![first]
    });
    let bare = built(|doc, div| vec![element(doc, div, "span", padded, "aaa")]);
    assert_eq!(spaced, bare, "in display: contents");
}
