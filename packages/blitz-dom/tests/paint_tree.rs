//! Paint lists (`Node::paint_children`) must name the boxes a node lays out, when
//! a document is edited the way niso's layout side edits its copy of the page:
//! with `DocumentMutator` calls only.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

fn document() -> (BaseDocument, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(
            "html, body, div, h2 { display: block }",
        )],
        ..DocumentConfig::default()
    });
    doc.set_viewport(Viewport::new(800, 600, 1.0, ColorScheme::Light));
    let root = doc.root_node().id;
    let html = create(&mut doc, "html");
    append(&mut doc, root, html);
    (doc, html)
}

fn create(doc: &mut BaseDocument, name: &str) -> NodeId {
    doc.mutate().create_element(
        QualName::new(None, ns!(html), LocalName::from(name)),
        Vec::new(),
    )
}

fn append(doc: &mut BaseDocument, parent: NodeId, node: NodeId) {
    doc.mutate().append_children(parent, &[node]);
}

fn set_style(doc: &mut BaseDocument, node: NodeId, style: &str) {
    doc.mutate().set_attribute(
        node,
        QualName::new(None, ns!(), LocalName::from("style")),
        style,
    );
}

fn paint_children(doc: &BaseDocument, node: NodeId) -> Option<Vec<NodeId>> {
    let node = doc.get_node(node).unwrap();
    node.paint_children.borrow().as_ref().map(|c| c.to_vec())
}

/// A box moved into a `display: none` element lets go of its box there, and is
/// built again when it is moved back out. A child it lost while hidden is no
/// longer in its paint list, even though its layout is all zeros (an empty box
/// in a shrink-to-fit one), the same as the layout of a box never laid out, so
/// that layout gives it no damage of its own.
#[test]
fn a_box_shown_again_does_not_paint_a_child_it_lost_while_hidden() {
    let (mut doc, html) = document();
    let wrap = create(&mut doc, "div");
    set_style(&mut doc, wrap, "position: absolute");
    append(&mut doc, html, wrap);
    let hidden = create(&mut doc, "div");
    set_style(&mut doc, hidden, "display: none");
    append(&mut doc, html, hidden);
    let old = create(&mut doc, "div");
    append(&mut doc, wrap, old);
    let h2 = create(&mut doc, "h2");
    append(&mut doc, old, h2);
    doc.resolve(0.0);
    assert_eq!(paint_children(&doc, old), Some(vec![h2]));

    append(&mut doc, hidden, old);
    append(&mut doc, html, h2);
    doc.resolve(0.0);

    append(&mut doc, wrap, old);
    doc.resolve(0.0);
    assert_eq!(paint_children(&doc, old), Some(vec![]));
    assert_eq!(paint_children(&doc, html), Some(vec![hidden, h2, wrap]));
}
