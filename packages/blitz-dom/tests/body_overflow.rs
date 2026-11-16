//! The body's `overflow` goes to the viewport when the root element's is `visible`
//! (CSS Overflow 3, section 3.3), and the body's box then uses `visible`.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, ScrollBehavior,
    UserAgentStylesheet, Viewport, ns,
};
use style::values::computed::Overflow;

const UA_CSS: &str = "html, body, div { display: block }";

fn element(doc: &mut BaseDocument, parent: NodeId, name: &str, style: &str) -> NodeId {
    let name = QualName::new(None, ns!(html), LocalName::from(name));
    let id = doc.mutate().create_element(name, Vec::new());
    restyle(doc, id, style);
    doc.mutate().append_children(parent, &[id]);
    id
}

fn restyle(doc: &mut BaseDocument, id: NodeId, style: &str) {
    let style_attr = QualName::new(None, ns!(), LocalName::from("style"));
    doc.mutate().set_attribute(id, style_attr, style);
}

/// An 800 by 600 page: the root element and the body with these styles, and a 2000px
/// tall div in the body. Gives the root element and the body.
fn page(html: &str, body: &str) -> (BaseDocument, NodeId, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = element(&mut doc, root, "html", html);
    let body = element(&mut doc, html, "body", body);
    element(&mut doc, body, "div", "height: 2000px");
    doc.resolve(0.0);
    (doc, html, body)
}

/// How far a user scrolls the viewport down, and the body, from as far up as they go.
fn scrolled(doc: &mut BaseDocument, body: NodeId) -> (f64, f64) {
    doc.scroll_viewport_by(0.0, -100_000.0);
    doc.scroll_by(body, 0.0, 100_000.0, ScrollBehavior::Instant);
    (
        doc.viewport_scroll().y,
        doc.get_node(body).unwrap().scroll_offset().y,
    )
}

#[test]
fn the_viewport_takes_the_bodys_overflow() {
    let (mut doc, _, body) = page("", "margin: 0; height: 100px; overflow: auto");
    assert_eq!(doc.viewport_overflow(), (Overflow::Auto, Overflow::Auto));
    let node = doc.get_node(body).unwrap();
    assert!(node.gives_overflow_to_viewport());
    assert_eq!(
        node.used_overflow(),
        Some((Overflow::Visible, Overflow::Visible))
    );
    assert_eq!(node.final_layout().size.height, 100.0);
    assert_eq!(
        scrolled(&mut doc, body),
        (1400.0, 0.0),
        "the page scrolls, the body does not"
    );
}

#[test]
fn a_body_hidden_stops_the_user_scrolling_the_page() {
    let (mut doc, _, body) = page("", "margin: 0; overflow: hidden");
    assert_eq!(
        doc.viewport_overflow(),
        (Overflow::Hidden, Overflow::Hidden)
    );
    assert_eq!(scrolled(&mut doc, body), (0.0, 0.0));
}

/// When the root element's overflow is not `visible`, it is the viewport's, and the body
/// keeps its own: it scrolls by itself.
#[test]
fn the_body_keeps_its_overflow_when_the_root_element_has_one() {
    let (mut doc, _, body) = page(
        "overflow: hidden",
        "margin: 0; height: 100px; overflow: auto",
    );
    assert_eq!(
        doc.viewport_overflow(),
        (Overflow::Hidden, Overflow::Hidden)
    );
    assert!(!doc.get_node(body).unwrap().gives_overflow_to_viewport());
    assert_eq!(scrolled(&mut doc, body), (0.0, 1900.0));
}

/// Restyling the root element moves the body's overflow between it and the viewport.
#[test]
fn restyling_the_root_element_moves_the_bodys_overflow() {
    let (mut doc, html, body) = page("", "margin: 0; height: 100px; overflow: auto");
    restyle(&mut doc, html, "overflow: hidden");
    doc.resolve(0.0);
    assert_eq!(scrolled(&mut doc, body), (0.0, 1900.0));
    doc.scroll_by(body, 0.0, -100_000.0, ScrollBehavior::Instant);
    restyle(&mut doc, html, "");
    doc.resolve(0.0);
    assert_eq!(scrolled(&mut doc, body), (1400.0, 0.0));
}

/// The root element's own `overflow` goes to the viewport, and its box uses `visible`.
#[test]
fn the_root_element_gives_its_overflow_to_the_viewport() {
    let (doc, html, _) = page("overflow: hidden", "margin: 0");
    assert_eq!(
        doc.viewport_overflow(),
        (Overflow::Hidden, Overflow::Hidden)
    );
    let node = doc.get_node(html).unwrap();
    assert!(node.gives_overflow_to_viewport());
    assert_eq!(
        node.used_overflow(),
        Some((Overflow::Visible, Overflow::Visible))
    );
}

/// `clip` on the viewport is `hidden`: the user cannot scroll the page, and code can.
#[test]
fn clip_on_the_viewport_is_hidden() {
    let (mut doc, html, body) = page("overflow: clip", "margin: 0");
    assert_eq!(scrolled(&mut doc, body), (0.0, 0.0));
    doc.scroll_to(html, 0.0, 500.0, ScrollBehavior::Instant);
    assert_eq!(doc.viewport_scroll().y, 500.0);
}
