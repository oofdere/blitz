//! The area the viewport scrolls over: the document's scrollable overflow, from the
//! viewport's scroll origin.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div { display: block }";

fn element(doc: &mut BaseDocument, parent: NodeId, name: &str, style: &str) -> NodeId {
    let name = QualName::new(None, ns!(html), LocalName::from(name));
    let id = doc.mutate().create_element(name, Vec::new());
    let style_attr = QualName::new(None, ns!(), LocalName::from("style"));
    doc.mutate().set_attribute(id, style_attr, style);
    doc.mutate().append_children(parent, &[id]);
    id
}

/// An 800 by 600 page with this style on the root element, whose body has no margin and
/// holds one div with `child`'s style.
fn page(html: &str, child: &str) -> BaseDocument {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = element(&mut doc, root, "html", html);
    let body = element(&mut doc, html, "body", "margin: 0");
    element(&mut doc, body, "div", child);
    doc.resolve(0.0);
    doc
}

/// How far the viewport scrolls each way: the least and the most offset.
fn range(doc: &mut BaseDocument) -> ((f64, f64), (f64, f64)) {
    doc.scroll_viewport_by(100_000.0, 100_000.0);
    let least = doc.viewport_scroll();
    doc.scroll_viewport_by(-200_000.0, -200_000.0);
    let most = doc.viewport_scroll();
    ((least.x, least.y), (most.x, most.y))
}

#[test]
fn the_viewport_scrolls_over_what_overflows_it() {
    let mut doc = page("", "width: 1000px; height: 1000px");
    assert_eq!(
        doc.viewport_scroll_area(),
        kurbo::Rect::new(0.0, 0.0, 1000.0, 1000.0)
    );
    assert_eq!(range(&mut doc), ((0.0, 0.0), (200.0, 400.0)));
}

/// The root element's margin box counts: its margins move it and add to it.
#[test]
fn the_root_elements_margins_count() {
    let mut doc = page("margin-top: 300px; margin-bottom: 50px", "height: 1000px");
    assert_eq!(
        doc.viewport_scroll_area(),
        kurbo::Rect::new(0.0, 0.0, 800.0, 1350.0)
    );
    assert_eq!(range(&mut doc), ((0.0, 0.0), (0.0, 750.0)));
}

/// An absolute box the initial containing block holds counts, though it is outside the
/// root element's box.
#[test]
fn an_absolute_box_the_initial_containing_block_holds_counts() {
    let mut doc = page(
        "",
        "position: absolute; top: 2000px; width: 10px; height: 10px",
    );
    assert_eq!(
        doc.viewport_scroll_area(),
        kurbo::Rect::new(0.0, 0.0, 800.0, 2010.0)
    );
    assert_eq!(range(&mut doc), ((0.0, 0.0), (0.0, 1410.0)));
}

/// A right-to-left page scrolls from the right: what overflows to the left can be
/// scrolled to, with negative offsets, and what overflows to the right cannot.
#[test]
fn a_right_to_left_page_scrolls_to_the_left() {
    let mut doc = page("direction: rtl", "width: 1200px; height: 10px");
    assert_eq!(
        doc.viewport_scroll_area(),
        kurbo::Rect::new(-400.0, 0.0, 800.0, 600.0)
    );
    assert_eq!(range(&mut doc), ((-400.0, 0.0), (0.0, 0.0)));
    let mut doc = page(
        "direction: rtl",
        "position: absolute; left: 900px; width: 10px; height: 10px",
    );
    assert_eq!(range(&mut doc), ((0.0, 0.0), (0.0, 0.0)));
}

/// A fixed box does not scroll with the page, so it adds nothing to what the page scrolls
/// over.
#[test]
fn a_fixed_box_does_not_count() {
    let mut doc = page(
        "",
        "position: fixed; top: 2000px; width: 10px; height: 10px",
    );
    assert_eq!(
        doc.viewport_scroll_area(),
        kurbo::Rect::new(0.0, 0.0, 800.0, 600.0)
    );
    assert_eq!(range(&mut doc), ((0.0, 0.0), (0.0, 0.0)));
}
