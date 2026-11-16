//! getBoundingClientRect inside vertical writing modes: a box placed by a vertical one is laid
//! out in that box's axes, and reads give it back in physical ones, mirrored for `*-rl`.
#![cfg(feature = "writing-mode")]

use blitz_dom::{
    BaseDocument, BoundingRect, ColorScheme, DocumentConfig, LocalName, NodeId, QualName,
    UserAgentStylesheet, Viewport, ns,
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

/// An 800 by 600 page whose body has no margin and holds a box with `outer`'s style, which
/// holds one with `inner`'s: their ids.
fn page(outer: &str, inner: &str) -> (BaseDocument, NodeId, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = element(&mut doc, root, "html", "");
    let body = element(&mut doc, html, "body", "margin: 0");
    let outer = element(&mut doc, body, "div", outer);
    let inner = element(&mut doc, outer, "div", inner);
    doc.resolve(0.0);
    (doc, outer, inner)
}

fn rect(doc: &BaseDocument, id: NodeId) -> (f64, f64, f64, f64) {
    let BoundingRect {
        x,
        y,
        width,
        height,
    } = doc.get_client_bounding_rect(id).unwrap();
    (x, y, width, height)
}

#[test]
fn a_box_in_a_vertical_rl_box_reads_from_the_right() {
    let (doc, outer, inner) = page(
        "writing-mode: vertical-rl; width: 200px; height: 100px",
        "width: 50px; height: 20px",
    );
    assert_eq!(rect(&doc, outer), (0.0, 0.0, 200.0, 100.0));
    assert_eq!(rect(&doc, inner), (150.0, 0.0, 50.0, 20.0));
}

#[test]
fn a_box_in_a_vertical_lr_box_reads_its_own_width_and_height() {
    let (doc, _, inner) = page(
        "writing-mode: vertical-lr; width: 200px; height: 100px",
        "width: 50px; height: 20px",
    );
    assert_eq!(rect(&doc, inner), (0.0, 0.0, 50.0, 20.0));
}

#[test]
fn a_box_overflowing_a_vertical_rl_scroller_reads_past_its_left_edge_and_scrolls_back() {
    let scroller = "overflow: auto; width: 100px; height: 100px; writing-mode: vertical-rl";
    let (mut doc, outer, inner) = page(scroller, "width: 400px; height: 10px");
    assert_eq!(rect(&doc, inner), (-300.0, 0.0, 400.0, 10.0));
    doc.scroll_to(outer, -100.0, 0.0, Default::default());
    assert_eq!(doc.get_node(outer).unwrap().scroll_offset().x, -100.0);
    assert_eq!(rect(&doc, inner), (-200.0, 0.0, 400.0, 10.0));
}
