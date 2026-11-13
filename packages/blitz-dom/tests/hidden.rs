//! The `hidden` attribute hides an element through the user-agent sheet's rules
//! for it (HTML, "Hidden elements"), and not by a mapping of its own, so that
//! `hidden=until-found` and a hidden `embed` keep the boxes the standard gives
//! them.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

fn page(config: DocumentConfig) -> (BaseDocument, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..config
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, "html");
    doc.mutate().append_children(root, &[html]);
    let body = create(&mut doc, "body");
    doc.mutate().append_children(html, &[body]);
    (doc, body)
}

fn create(doc: &mut BaseDocument, name: &str) -> NodeId {
    let name = QualName::new(None, ns!(html), LocalName::from(name));
    doc.mutate().create_element(name, Vec::new())
}

/// An element put last in `parent`, with a `hidden` attribute of this value.
fn hidden(doc: &mut BaseDocument, parent: NodeId, name: &str, value: &str) -> NodeId {
    let id = create(doc, name);
    let attr = QualName::new(None, ns!(), LocalName::from("hidden"));
    doc.mutate().set_attribute(id, attr, value);
    doc.mutate().append_children(parent, &[id]);
    id
}

fn display(doc: &BaseDocument, node: NodeId) -> style::values::specified::Display {
    doc.get_node(node)
        .unwrap()
        .primary_styles()
        .unwrap()
        .get_box()
        .display
}

#[test]
fn the_attribute_alone_hides_nothing() {
    let ua = UserAgentStylesheet::parse("html, body, div { display: block }");
    let (mut doc, body) = page(DocumentConfig {
        ua_stylesheets: vec![ua],
        ..DocumentConfig::default()
    });
    let div = hidden(&mut doc, body, "div", "");
    doc.resolve(0.0);
    assert!(!display(&doc, div).is_none());
}

#[cfg(feature = "default-ua-stylesheet")]
#[test]
fn the_default_sheet_hides_all_but_until_found_and_embed() {
    let (mut doc, body) = page(DocumentConfig::default());
    let div = hidden(&mut doc, body, "div", "");
    let until_found = hidden(&mut doc, body, "div", "Until-Found");
    let embed = hidden(&mut doc, body, "embed", "");
    doc.resolve(0.0);
    assert!(display(&doc, div).is_none());
    assert!(!display(&doc, until_found).is_none());
    assert!(display(&doc, embed).is_inline_flow());
}
