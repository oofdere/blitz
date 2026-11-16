//! A non-atomic inline element's box is as tall as the content area of its
//! text (CSS 2.1, section 10.6.1): an atomic inline inside it widens it but
//! does not make it taller.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div { display: block }";

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

fn rect(doc: &BaseDocument, node: NodeId) -> (f64, f64, f64, f64) {
    let r = doc.get_client_bounding_rect(node).unwrap();
    (r.x, r.y, r.width, r.height)
}

/// A span holding only a 50px by 40px inline-block, beside a span of text
/// in the same font, on a line 60px high: the first is as tall as the
/// second and where it is, and as wide as the inline-block.
#[test]
fn an_atomic_inline_widens_its_inline_parent_but_does_not_make_it_taller() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", "");
    let body = create(&mut doc, html, "body", "margin: 0");
    let line = create(&mut doc, body, "div", "font: 12px/60px sans-serif");
    let holder = create(&mut doc, line, "span", "");
    let block = create(
        &mut doc,
        holder,
        "span",
        "display: inline-block; width: 50px; height: 40px; vertical-align: bottom",
    );
    let text = create(&mut doc, line, "span", "");
    let x = doc.mutate().create_text_node("x");
    doc.mutate().append_children(text, &[x]);
    doc.resolve(0.0);

    let (holder_x, holder_y, holder_width, holder_height) = rect(&doc, holder);
    let (_, text_y, _, text_height) = rect(&doc, text);
    let (block_x, _, block_width, block_height) = rect(&doc, block);
    assert_eq!(block_height, 40.0);
    assert_eq!((holder_x, holder_width), (block_x, block_width));
    assert!(text_height < 40.0, "{text_height}");
    assert_eq!((holder_y, holder_height), (text_y, text_height));
}
