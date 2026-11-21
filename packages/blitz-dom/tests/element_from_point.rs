//! `elementFromPoint` takes viewport coordinates, so what it finds moves with the page's
//! scroll, and a fixed box stays where the viewport holds it.

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

/// An 800 by 600 page whose body has no margin and holds two blocks 1000px tall, one under
/// the other, and a fixed box at the top left: their ids.
fn page() -> (BaseDocument, [NodeId; 3]) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = element(&mut doc, root, "html", "");
    let body = element(&mut doc, html, "body", "margin: 0");
    let first = element(&mut doc, body, "div", "height: 1000px");
    let second = element(&mut doc, body, "div", "height: 1000px");
    let fixed = element(
        &mut doc,
        body,
        "div",
        "position: fixed; top: 0; left: 0; width: 50px; height: 50px",
    );
    doc.resolve(0.0);
    (doc, [first, second, fixed])
}

#[test]
fn a_point_finds_what_the_page_scrolled_under_it() {
    let (mut doc, [first, second, _]) = page();
    assert_eq!(doc.element_from_point(100.0, 100.0), Some(first));
    doc.scroll_viewport_by(0.0, -1500.0);
    assert_eq!(doc.viewport_scroll().y, 1400.0);
    assert_eq!(doc.element_from_point(100.0, 100.0), Some(second));
    assert_eq!(doc.elements_from_point(100.0, 100.0)[0], second);
}

#[test]
fn a_point_finds_a_fixed_box_where_the_viewport_holds_it() {
    let (mut doc, [_, _, fixed]) = page();
    doc.scroll_viewport_by(0.0, -1500.0);
    assert_eq!(doc.element_from_point(25.0, 25.0), Some(fixed));
}

/// An 800 by 600 page whose body has no margin: the document, its root element and its
/// body.
fn blank() -> (BaseDocument, NodeId, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = element(&mut doc, root, "html", "");
    let body = element(&mut doc, html, "body", "margin: 0");
    (doc, html, body)
}

/// A box positioned absolutely in a positioned parent that clips its overflow is cut by the
/// parent's clip, though the parent does not paint it: past the clip the point finds what
/// is behind, and inside it the box.
#[test]
fn a_clip_cuts_a_positioned_box_it_contains() {
    let (mut doc, html, body) = blank();
    let clip = element(
        &mut doc,
        body,
        "div",
        "position: relative; overflow: hidden; width: 50px; height: 50px",
    );
    let child = element(
        &mut doc,
        clip,
        "div",
        "position: absolute; left: 0; top: 0; width: 200px; height: 200px",
    );
    doc.resolve(0.0);
    assert_eq!(doc.element_from_point(25.0, 25.0), Some(child));
    assert_eq!(doc.element_from_point(100.0, 100.0), Some(html));
}

/// A clip only cuts the boxes whose containing blocks it is among: a box positioned
/// absolutely against an ancestor above the clip is found past it.
#[test]
fn a_clip_leaves_a_box_contained_above_it() {
    let (mut doc, _html, body) = blank();
    let holder = element(&mut doc, body, "div", "position: relative");
    let clip = element(
        &mut doc,
        holder,
        "div",
        "overflow: hidden; width: 50px; height: 50px",
    );
    let child = element(
        &mut doc,
        clip,
        "div",
        "position: absolute; left: 0; top: 0; width: 200px; height: 200px",
    );
    doc.resolve(0.0);
    assert_eq!(doc.element_from_point(100.0, 100.0), Some(child));
}

/// A hidden box is not hit, but a descendant that is visible again is.
#[test]
fn a_visible_child_of_a_hidden_box_is_hit() {
    let (mut doc, _html, body) = blank();
    let hidden = element(
        &mut doc,
        body,
        "div",
        "visibility: hidden; width: 100px; height: 100px",
    );
    let child = element(
        &mut doc,
        hidden,
        "div",
        "visibility: visible; width: 50px; height: 50px",
    );
    doc.resolve(0.0);
    assert_eq!(doc.element_from_point(25.0, 25.0), Some(child));
    assert_eq!(doc.element_from_point(75.0, 75.0), Some(body));
}
