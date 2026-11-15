//! A node that layout no longer reaches must read as on a page built fresh, where it
//! was never laid out. The edits are [DocumentMutator](blitz_dom::DocumentMutator)
//! calls only, as niso's layout side makes them. Seeds 278 and 1944 of niso's copy
//! harness come down to these.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str =
    "html, div { display: block } span { display: inline } template { display: none }";

/// An 800 by 600 page holding only its html element.
fn page() -> (BaseDocument, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        ..DocumentConfig::default()
    });
    doc.set_viewport(Viewport::new(800, 600, 1.0, ColorScheme::Light));
    let root = doc.root_node().id;
    let html = add(&mut doc, root, "html");
    (doc, html)
}

fn add(doc: &mut BaseDocument, parent: NodeId, name: &str) -> NodeId {
    let mut mutator = doc.mutate();
    let id = mutator.create_element(
        QualName::new(None, ns!(html), LocalName::from(name)),
        Vec::new(),
    );
    mutator.append_children(parent, &[id]);
    id
}

fn set_style(doc: &mut BaseDocument, node: NodeId, value: &str) {
    doc.mutate().set_attribute(
        node,
        QualName::new(None, ns!(), LocalName::from("style")),
        value,
    );
}

/// A transformed div is laid out, then left out of layout by `leave`. The rect read
/// from it must be the one read when the page is built fresh with the change made.
fn kept_and_fresh(
    transform: &str,
    leave: fn(&mut BaseDocument, NodeId, NodeId) -> (),
) -> (String, String) {
    let read = |change_after_layout: bool| {
        let (mut doc, html) = page();
        let outer = add(&mut doc, html, "div");
        let moved = add(&mut doc, outer, "div");
        set_style(&mut doc, moved, transform);
        if change_after_layout {
            doc.resolve(0.0);
        }
        leave(&mut doc, outer, moved);
        doc.resolve(0.0);
        format!("{:?}", doc.get_client_bounding_rect(moved))
    };
    (read(true), read(false))
}

/// The div is the only child of a span that becomes a table, which leaves out a child
/// it has no row or cell for.
#[test]
fn a_transformed_box_left_out_of_a_table_reads_as_on_a_fresh_page() {
    let (kept, fresh) = kept_and_fresh("transform: scale(2)", |doc, outer, moved| {
        let span = add(doc, outer, "span");
        doc.mutate().append_children(span, &[moved]);
        set_style(doc, span, "display: table");
    });
    assert_eq!(kept, fresh, "kept and fresh differ");
}

/// The div moves into a canvas, whose children are not rendered.
#[test]
fn a_transformed_box_moved_into_a_canvas_reads_as_on_a_fresh_page() {
    let (kept, fresh) = kept_and_fresh("transform: scale(2)", |doc, outer, moved| {
        let canvas = add(doc, outer, "canvas");
        doc.mutate().append_children(canvas, &[moved]);
    });
    assert_eq!(kept, fresh, "kept and fresh differ");
}

/// The div moves into a template, which is display: none.
#[test]
fn a_transformed_box_moved_into_a_hidden_element_reads_as_on_a_fresh_page() {
    let (kept, fresh) = kept_and_fresh("transform: translate(10px, 20px)", |doc, outer, moved| {
        let template = add(doc, outer, "template");
        doc.mutate().append_children(template, &[moved]);
    });
    assert_eq!(kept, fresh, "kept and fresh differ");
}
