//! Text in a `display: contents` element keeps that element's `white-space`, as text takes its
//! parent's style, also where blitz lifts it out into an anonymous block of the container: in a
//! flex or a grid container, or in a block container beside a block. Chrome 157 draws `a  bc` in
//! a `display: contents` span with `white-space: pre-wrap` in a flex container with both its
//! spaces. Before, the lifted text collapsed as the anonymous block does, `a bc`.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div { display: block } span { display: inline }";

fn element(doc: &mut BaseDocument, parent: NodeId, name: &str, style: &str) -> NodeId {
    let id = doc.mutate().create_element(
        QualName::new(None, ns!(html), LocalName::from(name)),
        Vec::new(),
    );
    let style_attr = QualName::new(None, ns!(), LocalName::from("style"));
    doc.mutate().set_attribute(id, style_attr, style);
    doc.mutate().append_children(parent, &[id]);
    id
}

/// The width of a container styled `container`, as wide as its content, that holds `a  bc` in a
/// span styled `span`, and an empty block after it when `beside_block`.
fn width(container: &str, beside_block: bool, span: &str) -> f64 {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = element(&mut doc, root, "html", "");
    let body = element(&mut doc, html, "body", "margin: 0");
    let div = element(&mut doc, body, "div", container);
    let span = element(&mut doc, div, "span", span);
    let text = doc.mutate().create_text_node("a  bc");
    doc.mutate().append_children(span, &[text]);
    if beside_block {
        element(&mut doc, div, "div", "");
    }
    doc.resolve(0.0);
    doc.get_client_bounding_rect(div).unwrap().width
}

#[test]
fn lifted_text_keeps_its_parents_white_space() {
    for (container, beside_block) in [
        ("display: inline-flex", false),
        ("display: inline-grid", false),
        ("display: inline-block", true),
    ] {
        let kept = width(container, beside_block, "white-space: pre-wrap");
        let collapsed = width(container, beside_block, "");
        assert!(kept > collapsed, "{container}: {kept} against {collapsed}");
        assert_eq!(
            width(
                container,
                beside_block,
                "display: contents; white-space: pre-wrap"
            ),
            kept,
            "{container}"
        );
        assert_eq!(
            width(container, beside_block, "display: contents"),
            collapsed,
            "{container}"
        );
    }
}
