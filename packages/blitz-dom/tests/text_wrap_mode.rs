//! A change to whether text may wrap must reach the lines already laid out. The edits
//! are [DocumentMutator](blitz_dom::DocumentMutator) calls only, as niso's layout side
//! makes them. Seeds 96, 612, 1221 and 1822 of Lens's fuzzer come down to these.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, div { display: block }";

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

/// A 10px wide div holds a run of words, inside an outer div. The style of the div
/// chosen by `styled` becomes `white-space: nowrap`. The height read from the inner
/// div must be the one read when the page is built fresh with that style.
fn kept_and_fresh(styled: fn(NodeId, NodeId) -> NodeId) -> (f32, f32) {
    let read = |change_after_layout: bool| {
        let (mut doc, html) = page();
        let outer = add(&mut doc, html, "div");
        let inner = add(&mut doc, outer, "div");
        set_style(&mut doc, inner, "width: 10px");
        let text = doc.mutate().create_text_node("lead and trail");
        doc.mutate().append_children(inner, &[text]);
        if change_after_layout {
            doc.resolve(0.0);
        }
        let node = styled(outer, inner);
        let style = if node == inner {
            "width: 10px; white-space: nowrap"
        } else {
            "white-space: nowrap"
        };
        set_style(&mut doc, node, style);
        doc.resolve(0.0);
        doc.get_node(inner).unwrap().final_layout().size.height
    };
    (read(true), read(false))
}

/// The div that holds the text stops wrapping it, so its words share one line.
#[test]
fn text_that_stops_wrapping_is_laid_out_on_one_line() {
    let (kept, fresh) = kept_and_fresh(|_, inner| inner);
    assert_eq!(kept, fresh, "kept and fresh differ");
}

/// The outer div stops wrapping text, and the div that holds the text inherits that.
#[test]
fn text_whose_parent_stops_wrapping_is_laid_out_on_one_line() {
    let (kept, fresh) = kept_and_fresh(|outer, _| outer);
    assert_eq!(kept, fresh, "kept and fresh differ");
}
