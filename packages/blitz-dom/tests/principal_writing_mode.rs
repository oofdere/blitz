//! The document's principal writing mode (CSS Writing Modes 4, section 8): the root
//! element's box and the initial containing block use the `writing-mode` and
//! `direction` of the body, when the root element is an `html` element with a `body`
//! child that is displayed, and the viewport scrolls from the origin it gives.
//! Containment on either turns that off (CSS Containment 2, section 3).

use blitz_dom::{
    BaseDocument, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet, ns,
};
use blitz_traits::shell::{ColorScheme, Viewport};

fn element(doc: &mut BaseDocument, parent: NodeId, name: &str, style: &str) -> NodeId {
    let mut mutator = doc.mutate();
    let id = mutator.create_element(
        QualName::new(None, ns!(html), LocalName::from(name)),
        Vec::new(),
    );
    mutator.set_attribute(
        id,
        QualName::new(None, ns!(), LocalName::from("style")),
        style,
    );
    mutator.append_children(parent, &[id]);
    id
}

/// An 800 by 600 page: a root element named `root` with the style `html`, and under
/// it a `body` element with each of the styles in `bodies`. Gives the root element
/// and the bodies.
fn page(root: &str, html: &str, bodies: &[&str]) -> (BaseDocument, NodeId, Vec<NodeId>) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(
            "html, body, div { display: block }",
        )],
        ..DocumentConfig::default()
    });
    doc.set_viewport(Viewport::new(800, 600, 1.0, ColorScheme::Light));
    let document = doc.root_node().id;
    let html = element(&mut doc, document, root, html);
    let bodies = bodies
        .iter()
        .map(|style| element(&mut doc, html, "body", style))
        .collect();
    doc.resolve(0.0);
    (doc, html, bodies)
}

fn x(doc: &BaseDocument, id: NodeId) -> f32 {
    doc.get_node(id).unwrap().final_layout().location.x
}

/// The initial containing block takes the body's `direction`: a root element narrower
/// than the viewport sits on its right.
#[test]
fn the_initial_containing_block_takes_the_bodys_direction() {
    let (doc, html, _) = page("html", "width: 300px", &["direction: rtl"]);
    assert_eq!(x(&doc, html), 500.0);
    let (doc, html, _) = page("html", "width: 300px; direction: rtl", &["direction: ltr"]);
    assert_eq!(
        x(&doc, html),
        0.0,
        "and its `ltr` over the root element's `rtl`"
    );
}

/// The root element's box takes the body's `direction`: a body narrower than it sits on
/// its right.
#[test]
fn the_root_elements_box_takes_the_bodys_direction() {
    let (doc, _, bodies) = page("html", "", &["direction: rtl; width: 300px"]);
    assert_eq!(x(&doc, bodies[0]), 500.0);
}

/// The viewport scrolls from the right on a page whose body is right to left.
#[test]
fn the_viewport_scrolls_from_the_bodys_start() {
    let (mut doc, _, bodies) = page("html", "", &["direction: rtl"]);
    element(&mut doc, bodies[0], "div", "width: 2000px; height: 10px");
    doc.resolve(0.0);
    let area = doc.viewport_scroll_area();
    assert_eq!((area.x0, area.x1), (-1200.0, 800.0));
}

/// A body that is not displayed gives nothing, and the first that is gives it. A root
/// element that is not an `html` element takes nothing from a `body` child.
#[test]
fn only_a_displayed_body_of_an_html_element_gives_it() {
    let (doc, html, _) = page("html", "width: 300px", &["direction: rtl; display: none"]);
    assert_eq!(x(&doc, html), 0.0, "not displayed");
    let (doc, html, _) = page(
        "html",
        "width: 300px",
        &["direction: ltr; display: none", "direction: rtl"],
    );
    assert_eq!(x(&doc, html), 500.0, "the first that is displayed");
    let (doc, root, _) = page("div", "width: 300px", &["direction: rtl"]);
    assert_eq!(x(&doc, root), 0.0, "not an html element");
}

/// Containment on the root element or on the body turns it off.
#[test]
fn containment_turns_it_off() {
    let (doc, html, _) = page("html", "width: 300px; contain: paint", &["direction: rtl"]);
    assert_eq!(x(&doc, html), 0.0, "on the root element");
    let (doc, html, _) = page("html", "width: 300px", &["direction: rtl; contain: paint"]);
    assert_eq!(x(&doc, html), 0.0, "on the body");
}

/// A page whose body changes its `direction` after layout is laid out as one built
/// with it.
#[test]
fn a_body_that_changes_direction_moves_the_root_element() {
    let (mut doc, html, bodies) = page("html", "width: 300px", &["direction: ltr"]);
    assert_eq!(x(&doc, html), 0.0);
    doc.mutate().set_attribute(
        bodies[0],
        QualName::new(None, ns!(), LocalName::from("style")),
        "direction: rtl",
    );
    doc.resolve(0.0);
    assert_eq!(x(&doc, html), 500.0);
}
