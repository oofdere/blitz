//! A box's cached layout results are written in the axes of the box that placed it, so
//! they are not used once that box's writing mode has changed: a box that was orthogonal
//! to its parent and no longer is lays out as on a page that had the new writing mode
//! from the start.
#![cfg(feature = "writing-mode")]

use blitz_dom::{BaseDocument, DocumentConfig, LocalName, NodeId, QualName, ns};
use blitz_traits::shell::{ColorScheme, Viewport};

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

/// An 800 by 600 page whose root element gets `writing-mode: sideways-rl`, laid out once
/// before that when `early`. In a table cell, a block holds a line of text and a
/// `vertical-rl` box, orthogonal to the block until the root turns vertical. Gives that
/// box's size.
fn vertical_box(early: bool) -> (f32, f32) {
    let mut doc = BaseDocument::new(DocumentConfig::default());
    doc.set_viewport(Viewport::new(800, 600, 1.0, ColorScheme::Light));
    let document = doc.root_node().id;
    let html = element(&mut doc, document, "html", "");
    let table = element(&mut doc, html, "table", "");
    let row = element(&mut doc, table, "tr", "");
    let cell = element(&mut doc, row, "td", "");
    let block = element(&mut doc, cell, "div", "");
    let span = element(&mut doc, block, "span", "");
    let text = doc.mutate().create_text_node("wwwwwwwwww");
    doc.mutate().append_children(span, &[text]);
    let vertical = element(&mut doc, block, "div", "writing-mode: vertical-rl");
    if early {
        doc.resolve(0.0);
    }
    doc.mutate().set_attribute(
        html,
        QualName::new(None, ns!(), LocalName::from("style")),
        "writing-mode: sideways-rl",
    );
    doc.resolve(0.0);
    let size = doc.get_node(vertical).unwrap().final_layout().size;
    (size.width, size.height)
}

#[test]
fn a_box_that_stops_being_orthogonal_is_laid_out_again() {
    assert_eq!(vertical_box(true), vertical_box(false));
}
