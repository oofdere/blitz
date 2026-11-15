//! A page whose viewport changes scale after layout must lay out as a page built fresh at that
//! scale. The edits are [DocumentMutator](blitz_dom::DocumentMutator) calls only, as niso's
//! layout side makes them.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, div { display: block }";

/// A page holding only its html element, laid out for a viewport of 800 by 600 device pixels at
/// scale 1.
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

/// A div with a 1px border, laid out at scale 1 and then at scale 1.5. A border width is snapped
/// to whole device pixels when it is computed, so at scale 1.5 each border is one device pixel,
/// 2/3 of a CSS pixel, and the div is 4/3 CSS pixels tall. Shrunk from seed 7 of Lens's frames
/// harness in its mode that changes the viewport between tasks.
#[test]
fn a_border_reads_as_on_a_fresh_page_after_the_scale_changes() {
    let height = |laid_out_at_scale_1: bool| {
        let (mut doc, html) = page();
        let div = add(&mut doc, html, "div");
        set_style(&mut doc, div, "border: 1px solid");
        if laid_out_at_scale_1 {
            doc.resolve(0.0);
        }
        doc.set_viewport(Viewport::new(1200, 900, 1.5, ColorScheme::Light));
        doc.resolve(0.0);
        doc.get_node(div).unwrap().unrounded_layout().size.height
    };
    let fresh = height(false);
    assert!(
        (fresh - 4.0 / 3.0).abs() < 1e-4,
        "a fresh page gives {fresh}"
    );
    assert_eq!(height(true), fresh);
}
