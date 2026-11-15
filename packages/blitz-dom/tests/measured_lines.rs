//! The lines of a box laid out again, with its box taken from the layout cache, must be the
//! ones a page built fresh breaks. The edits are [DocumentMutator](blitz_dom::DocumentMutator)
//! calls only, as niso's layout side makes them.

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

fn add_text(doc: &mut BaseDocument, parent: NodeId, text: &str) {
    let mut mutator = doc.mutate();
    let id = mutator.create_text_node(text);
    mutator.append_children(parent, &[id]);
}

fn set_style(doc: &mut BaseDocument, node: NodeId, value: &str) {
    doc.mutate().set_attribute(
        node,
        QualName::new(None, ns!(), LocalName::from("style")),
        value,
    );
}

/// How many bytes of a box's text each of its lines holds, in order.
fn lines(doc: &BaseDocument, node: NodeId) -> Vec<usize> {
    let text = doc
        .get_node(node)
        .and_then(|node| node.element_data())
        .and_then(|element| element.inline_layout_data.as_deref())
        .expect("the box lays out text");
    text.layout
        .lines()
        .map(|line| line.text_range().len())
        .collect()
}

/// html > div > (absolutely positioned column flexbox > short words, one long word). The flexbox
/// is as wide as the long word, and so is the item with the short words, which fit on one line.
/// The div's width changes, and the flexbox measures the item at the div's new width, which
/// breaks its words onto two lines. The item is then as wide as before, so its layout comes from
/// the cache, and its lines must be the one line it was laid out with, not the two lines it was
/// measured with. Shrunk from seed 59 of Lens's frames harness.
#[test]
fn a_box_measured_narrower_and_laid_out_from_the_cache_keeps_its_lines() {
    let read = |change_after_layout: bool| {
        let (mut doc, html) = page();
        let div = add(&mut doc, html, "div");
        set_style(&mut doc, div, "position: relative; width: 180px");
        let flex = add(&mut doc, div, "div");
        set_style(
            &mut doc,
            flex,
            "position: absolute; display: flex; flex-direction: column",
        );
        let short = add(&mut doc, flex, "div");
        add_text(&mut doc, short, "a b c d e f g h i j k l m n o p q r s t");
        let long = add(&mut doc, flex, "div");
        add_text(&mut doc, long, &"w".repeat(53));
        if change_after_layout {
            doc.resolve(0.0);
        }
        set_style(&mut doc, div, "position: relative; width: 200px");
        doc.resolve(0.0);
        let width = doc.get_node(short).unwrap().unrounded_layout().size.width;
        (width, lines(&doc, short))
    };
    let fresh = read(false);
    assert_eq!(fresh.1, vec![39]);
    assert_eq!(read(true), fresh);
}
