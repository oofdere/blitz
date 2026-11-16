//! A table's grid is built in the table's writing mode, so a table whose writing mode
//! changes after it was laid out builds its grid again: it lays out as a page that had
//! that writing mode from the start.
#![cfg(feature = "writing-mode")]

use blitz_dom::{
    BaseDocument, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet, ns,
};
use blitz_traits::shell::{ColorScheme, Viewport};

/// The rules the tests need, so that they run with or without blitz's own sheet.
const UA_CSS: &str = "html, body, div { display: block } table { display: table } \
                      tr { display: table-row } td { display: table-cell }";

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

fn text(doc: &mut BaseDocument, parent: NodeId, data: &str) {
    let mut mutator = doc.mutate();
    let id = mutator.create_text_node(data);
    mutator.append_children(parent, &[id]);
}

/// Builds an 800 by 600 page with `build`, laid out once first when `early`, then sets
/// `style` on the element `build` gives back. Gives every element's box, in tree order.
fn boxes(
    early: bool,
    style: &str,
    build: impl Fn(&mut BaseDocument, NodeId) -> NodeId,
) -> Vec<(f32, f32, f32, f32)> {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        ..DocumentConfig::default()
    });
    doc.set_viewport(Viewport::new(800, 600, 1.0, ColorScheme::Light));
    let document = doc.root_node().id;
    let changed = build(&mut doc, document);
    if early {
        doc.resolve(0.0);
    }
    doc.mutate().set_attribute(
        changed,
        QualName::new(None, ns!(), LocalName::from("style")),
        style,
    );
    doc.resolve(0.0);
    let mut boxes = Vec::new();
    let mut next = vec![document];
    while let Some(id) = next.pop() {
        let node = doc.get_node(id).unwrap();
        next.extend(node.children.iter().rev().copied());
        if node.data.downcast_element().is_some() {
            let layout = node.final_layout();
            boxes.push((
                layout.location.x,
                layout.location.y,
                layout.size.width,
                layout.size.height,
            ));
        }
    }
    boxes
}

/// A table under a root element that turns vertical: its `height: 100%` is now its inline
/// size, the viewport's height.
#[test]
fn a_table_follows_the_root_element_turning_vertical() {
    let build = |doc: &mut BaseDocument, document: NodeId| {
        let html = element(doc, document, "html", "");
        let table = element(doc, html, "table", "height: 100%");
        text(doc, table, "x");
        html
    };
    let laid_out_before = boxes(true, "writing-mode: sideways-rl", build);
    assert_eq!(
        laid_out_before,
        boxes(false, "writing-mode: sideways-rl", build)
    );
    assert_eq!(laid_out_before[1].3, 600.0);
}

/// A table with a cell, under a box that turns vertical.
#[test]
fn a_table_follows_a_box_turning_vertical() {
    let build = |doc: &mut BaseDocument, document: NodeId| {
        let html = element(doc, document, "html", "");
        let body = element(doc, html, "body", "");
        let holder = element(doc, body, "div", "");
        let table = element(doc, holder, "table", "height: 100%");
        let row = element(doc, table, "tr", "");
        let cell = element(doc, row, "td", "width: 30px");
        text(doc, cell, "x");
        holder
    };
    assert_eq!(
        boxes(true, "writing-mode: vertical-rl", build),
        boxes(false, "writing-mode: vertical-rl", build)
    );
}
