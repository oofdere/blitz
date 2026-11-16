//! Collapsible spaces at the end of a line are removed (CSS Text 3, section
//! 4.1.2), so the box of the inline element they end is as wide as its text
//! without them.

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

fn text(doc: &mut BaseDocument, parent: NodeId, text: &str) {
    let id = doc.mutate().create_text_node(text);
    doc.mutate().append_children(parent, &[id]);
}

fn width(doc: &BaseDocument, node: NodeId) -> f64 {
    doc.get_client_bounding_rect(node).unwrap().width
}

/// Two spans in a line 100px wide, each "xxxx yyyy" in a 20px font, wrap
/// after their space: the one whose spaces collapse is as wide as "xxxx" on
/// its first line, and the one in `white-space: pre-wrap` keeps its space.
#[test]
fn a_space_at_the_end_of_a_line_is_in_no_box_when_it_collapses() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", "");
    let body = create(&mut doc, html, "body", "margin: 0");
    let style = "font: 20px monospace; width: 100px";
    let collapsing = create(&mut doc, body, "div", style);
    let span = create(&mut doc, collapsing, "span", "");
    text(&mut doc, span, "xxxx yyyy");
    let alone = create(&mut doc, body, "div", style);
    let word = create(&mut doc, alone, "span", "");
    text(&mut doc, word, "xxxx");
    let preserving = create(
        &mut doc,
        body,
        "div",
        &format!("{style}; white-space: pre-wrap"),
    );
    let kept = create(&mut doc, preserving, "span", "");
    text(&mut doc, kept, "xxxx yyyy");
    doc.resolve(0.0);

    let word = width(&doc, word);
    assert!(word > 0.0);
    assert_eq!(width(&doc, span), word);
    assert!(width(&doc, kept) > word, "{} {word}", width(&doc, kept));
}
