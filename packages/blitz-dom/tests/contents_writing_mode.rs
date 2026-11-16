//! An inline box whose writing mode differs from its layout parent's is made an inline-block
//! (CSS Writing Modes 3, section 3.1). Below a `display: contents` element that parent is
//! further up, so the box is styled again when that parent's writing mode changes, also when
//! the `display: contents` element's own style stays the same.
#![cfg(feature = "writing-mode")]

use blitz_dom::{BaseDocument, DocumentConfig, LocalName, NodeId, QualName, ns};
use blitz_traits::shell::{ColorScheme, Viewport};
use style::values::specified::Display;

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

fn display(doc: &BaseDocument, id: NodeId) -> Display {
    doc.get_node(id)
        .unwrap()
        .primary_styles()
        .unwrap()
        .get_box()
        .display
}

#[test]
fn a_box_below_display_contents_follows_its_layout_parents_writing_mode() {
    let mut doc = BaseDocument::new(DocumentConfig::default());
    doc.set_viewport(Viewport::new(800, 600, 1.0, ColorScheme::Light));
    let document = doc.root_node().id;
    let html = element(&mut doc, document, "html", "");
    let holder = element(&mut doc, html, "div", "writing-mode: vertical-rl");
    let contents = element(
        &mut doc,
        holder,
        "span",
        "display: contents; writing-mode: vertical-rl",
    );
    let inline = element(&mut doc, contents, "i", "");
    doc.resolve(0.0);
    assert_eq!(display(&doc, inline), Display::Inline);

    doc.mutate().set_attribute(
        holder,
        QualName::new(None, ns!(), LocalName::from("style")),
        "",
    );
    doc.resolve(0.0);
    assert_eq!(display(&doc, inline), Display::InlineBlock);

    doc.mutate().set_attribute(
        holder,
        QualName::new(None, ns!(), LocalName::from("style")),
        "writing-mode: vertical-rl",
    );
    doc.resolve(0.0);
    assert_eq!(display(&doc, inline), Display::Inline, "and back");
}
