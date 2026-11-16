//! The `::before` and `::after` of an inline element inside an inline formatting context
//! are made and laid out, on a page built fresh and on one built again. The edits are
//! [DocumentMutator](blitz_dom::DocumentMutator) calls only, as niso's layout side makes
//! them.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

/// An 800 by 600 page holding only its html element, with `css` as its sheet.
fn page(css: &str) -> (BaseDocument, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(&format!(
            "html, div {{ display: block }} span, b {{ display: inline }} {css}"
        ))],
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

fn add_text(doc: &mut BaseDocument, parent: NodeId, data: &str) {
    let mut mutator = doc.mutate();
    let id = mutator.create_text_node(data);
    mutator.append_children(parent, &[id]);
}

fn height(doc: &BaseDocument, node: NodeId) -> f64 {
    doc.get_client_bounding_rect(node).unwrap().height
}

/// The height of a div holding `x` as text: one line.
fn one_line() -> f64 {
    let (mut doc, html) = page("");
    let div = add(&mut doc, html, "div");
    add_text(&mut doc, div, "x");
    doc.resolve(0.0);
    height(&doc, div)
}

/// html > div > span > b, where the b's `::before` is an "x". The div holds one line.
#[test]
fn the_before_of_an_inline_inside_an_inline_is_laid_out() {
    let (mut doc, html) = page("b::before { content: 'x' }");
    let div = add(&mut doc, html, "div");
    let span = add(&mut doc, div, "span");
    add(&mut doc, span, "b");
    doc.resolve(0.0);
    assert_eq!(height(&doc, div), one_line());
}

/// html > div > span > "t", where the span's `::before` is a block holding "x". The
/// block breaks the span, so the div holds two lines, whether the page is built fresh
/// or the div is built again after a text node comes into it.
#[test]
fn a_block_before_of_an_inline_breaks_its_line_fresh_and_built_again() {
    let read = |built_again: bool| {
        let (mut doc, html) = page("span::before { content: 'x'; display: block }");
        let div = add(&mut doc, html, "div");
        let span = add(&mut doc, div, "span");
        add_text(&mut doc, span, "t");
        doc.resolve(0.0);
        if built_again {
            add_text(&mut doc, div, "");
            doc.resolve(0.0);
        }
        height(&doc, div)
    };
    assert_eq!(
        (read(false), read(true)),
        (2.0 * one_line(), 2.0 * one_line())
    );
}
