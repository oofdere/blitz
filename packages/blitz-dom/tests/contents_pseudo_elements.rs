//! The pseudo-elements of a `display: contents` element take part in its
//! container's formatting context. A container laid out again must read as on a page
//! built fresh, and both as in a browser. The edits are
//! [DocumentMutator](blitz_dom::DocumentMutator) calls only, as niso's layout side makes
//! them.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, div { display: block } .contents { display: contents } .inline::before { content: \"s\" } .block::before { content: \"s\"; display: block } .fixed { position: fixed }";

/// An 800 by 600 page holding only its html element.
fn page() -> (BaseDocument, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        ..DocumentConfig::default()
    });
    doc.set_viewport(Viewport::new(800, 600, 1.0, ColorScheme::Light));
    let root = doc.root_node().id;
    let html = add(&mut doc, root, "html", "");
    (doc, html)
}

fn add(doc: &mut BaseDocument, parent: NodeId, name: &str, class: &str) -> NodeId {
    let mut mutator = doc.mutate();
    let id = mutator.create_element(
        QualName::new(None, ns!(html), LocalName::from(name)),
        Vec::new(),
    );
    if !class.is_empty() {
        mutator.set_attribute(
            id,
            QualName::new(None, ns!(), LocalName::from("class")),
            class,
        );
    }
    mutator.append_children(parent, &[id]);
    id
}

fn size(doc: &BaseDocument, node: NodeId) -> (f32, f32) {
    let size = doc.get_node(node).unwrap().final_layout().size;
    (size.width, size.height)
}

/// html > div > div.contents.inline: the div holds one line, the inline ::before "s" of
/// the display: contents element. Put back in the div after layout, the display:
/// contents element makes the div be built again. Shrunk from seed 1658 of Lens's
/// fuzzer.
#[test]
fn an_inline_before_of_a_contents_child_stays_in_a_rebuilt_block() {
    let read = |rebuild: bool| {
        let (mut doc, html) = page();
        let div = add(&mut doc, html, "div", "");
        let contents = add(&mut doc, div, "div", "contents inline");
        doc.resolve(0.0);
        if rebuild {
            doc.mutate().append_children(div, &[contents]);
            doc.resolve(0.0);
        }
        size(&doc, div)
    };
    let fresh = read(false);
    assert_eq!(read(true), fresh);
    assert!(fresh.1 > 0.0, "the div has no line: {fresh:?}");
}

/// html > div > div.fixed > div.contents.inline: the fixed box shrinks to the width of
/// the ::before "s". Put back in the fixed box after layout, the display: contents
/// element makes it be built again. Shrunk from seed 1137 of Lens's fuzzer.
#[test]
fn an_inline_before_of_a_contents_child_sizes_a_rebuilt_fixed_box() {
    let read = |rebuild: bool| {
        let (mut doc, html) = page();
        let outer = add(&mut doc, html, "div", "");
        let fixed = add(&mut doc, outer, "div", "fixed");
        let contents = add(&mut doc, fixed, "div", "contents inline");
        doc.resolve(0.0);
        if rebuild {
            doc.mutate().append_children(fixed, &[contents]);
            doc.resolve(0.0);
        }
        size(&doc, fixed)
    };
    let fresh = read(false);
    assert_eq!(read(true), fresh);
    assert!(
        fresh.0 > 0.0 && fresh.1 > 0.0,
        "the fixed box is empty: {fresh:?}"
    );
}

/// html > div > [div.contents.block, "t"]: the block ::before "s" of the display:
/// contents element is a block of its own, so the div holds two lines, one above the
/// other, on a fresh page as after the div is built again.
#[test]
fn a_block_before_of_a_contents_child_breaks_the_line_on_a_fresh_page() {
    let read = |rebuild: bool| {
        let (mut doc, html) = page();
        let div = add(&mut doc, html, "div", "");
        let contents = add(&mut doc, div, "div", "contents block");
        let text = doc.mutate().create_text_node("t");
        doc.mutate().append_children(div, &[text]);
        doc.resolve(0.0);
        if rebuild {
            doc.mutate().insert_nodes_before(text, &[contents]);
            doc.resolve(0.0);
        }
        size(&doc, div)
    };
    let (fresh, rebuilt) = (read(false), read(true));
    assert_eq!(rebuilt, fresh);
    let line = {
        let (mut doc, html) = page();
        let div = add(&mut doc, html, "div", "");
        let text = doc.mutate().create_text_node("t");
        doc.mutate().append_children(div, &[text]);
        doc.resolve(0.0);
        size(&doc, div).1
    };
    assert_eq!(
        fresh.1,
        2.0 * line,
        "the div does not hold two lines: {fresh:?}"
    );
}
