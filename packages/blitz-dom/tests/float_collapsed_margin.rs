//! A float's exclusion is where the float is when a top margin collapses
//! through its block: the lines beside it are narrowed, and the lines that are
//! not are not. The expected numbers are Chrome's.
#![cfg(feature = "floats")]

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div, section { display: block }";

fn doc() -> BaseDocument {
    BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    })
}

fn create(doc: &mut BaseDocument, parent: NodeId, name: &str, style: &str) -> NodeId {
    let id = doc.mutate().create_element(
        QualName::new(None, ns!(html), LocalName::from(name)),
        Vec::new(),
    );
    let attr = QualName::new(None, ns!(), LocalName::from("style"));
    doc.mutate().set_attribute(id, attr, style);
    doc.mutate().append_children(parent, &[id]);
    id
}

fn text(doc: &mut BaseDocument, parent: NodeId) -> NodeId {
    let span = create(doc, parent, "span", "");
    let x = doc.mutate().create_text_node("x");
    doc.mutate().append_children(span, &[x]);
    span
}

/// A section 100px down, its margin collapsed through the body, holds a
/// 50px float 20px high. A line 100px below the float is not beside it:
/// right-aligned, it ends at the right edge.
#[test]
fn a_float_under_a_collapsed_margin_narrows_only_the_lines_beside_it() {
    let mut doc = doc();
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", "margin: 0");
    let body = create(&mut doc, html, "body", "margin: 0");
    let section = create(&mut doc, body, "section", "margin-top: 100px; height: 20px");
    let float = create(
        &mut doc,
        section,
        "div",
        "float: right; width: 50px; height: 20px",
    );
    let below = create(
        &mut doc,
        body,
        "div",
        "margin-top: 80px; font: 10px/20px sans-serif; text-align: right",
    );
    let span = text(&mut doc, below);
    doc.resolve(0.0);

    let float = doc.get_client_bounding_rect(float).unwrap();
    assert_eq!((float.x, float.y), (750.0, 100.0));
    let below = doc.get_client_bounding_rect(below).unwrap();
    assert_eq!(below.y, 200.0);
    let span = doc.get_client_bounding_rect(span).unwrap();
    assert!((span.x + span.width - 800.0).abs() < 0.01, "{span:?}");
}

/// A float first in the body moves down with the body when a later child's
/// 100px margin collapses through the body's top, so a line at the
/// section's top is beside it.
#[test]
#[ignore = "taffy's known fault: a float placed before a margin that collapses through its parent stays where the parent was in the float space (oofdere/taffy NISO.md)"]
fn a_float_moves_with_its_parents_collapsed_margin() {
    let mut doc = doc();
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", "margin: 0");
    let body = create(&mut doc, html, "body", "margin: 0");
    let float = create(
        &mut doc,
        body,
        "div",
        "float: right; width: 50px; height: 50px",
    );
    let section = create(
        &mut doc,
        body,
        "section",
        "margin-top: 100px; font: 10px/20px sans-serif; text-align: right",
    );
    let span = text(&mut doc, section);
    doc.resolve(0.0);
    let float = doc.get_client_bounding_rect(float).unwrap();
    let span = doc.get_client_bounding_rect(span).unwrap();
    assert_eq!(float.y, 100.0);
    assert!((span.x + span.width - 750.0).abs() < 0.01, "{span:?}");
}

/// A float in a flow-root before a block whose child's 100px margin
/// collapses through it stays at the top, so the line 100px down is past it.
#[test]
fn a_float_before_a_collapsed_margin_in_a_flow_root_stays_at_its_top() {
    let mut doc = doc();
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", "margin: 0");
    let body = create(&mut doc, html, "body", "margin: 0");
    let flow = create(&mut doc, body, "div", "display: flow-root");
    let float = create(
        &mut doc,
        flow,
        "div",
        "float: left; width: 300px; height: 50px",
    );
    let wrapper = create(&mut doc, flow, "div", "");
    let section = create(
        &mut doc,
        wrapper,
        "section",
        "margin-top: 100px; font: 10px/20px sans-serif",
    );
    let span = text(&mut doc, section);
    doc.resolve(0.0);
    let float = doc.get_client_bounding_rect(float).unwrap();
    let span = doc.get_client_bounding_rect(span).unwrap();
    assert_eq!(float.y, 0.0);
    assert!(span.x.abs() < 0.01, "{span:?}");
}
