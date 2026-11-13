//! `getBoundingClientRect` as CSSOM View defines it, for the cases where blitz's layout
//! box alone is not the answer.

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

/// An 800 by 600 page whose body has no margin.
fn page() -> (BaseDocument, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, "html");
    doc.mutate().append_children(root, &[html]);
    let body = element(&mut doc, html, "body", "margin: 0", "");
    (doc, body)
}

fn rect(doc: &BaseDocument, node: NodeId) -> (f64, f64, f64, f64) {
    let r = doc.get_client_bounding_rect(node).unwrap();
    (r.x, r.y, r.width, r.height)
}

#[test]
fn a_block_is_as_wide_as_the_viewport() {
    let (mut doc, body) = page();
    let node = element(&mut doc, body, "div", "height: 10px", "");
    doc.resolve(0.0);
    assert_eq!(rect(&doc, node), (0.0, 0.0, 800.0, 10.0));
}
