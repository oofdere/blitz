//! An inline box whose writing mode differs from its layout parent's is made an inline-block
//! (CSS Writing Modes 3, section 3.1). Below a `display: contents` element that parent is
//! further up, so the box is styled again when that parent's writing mode changes, also when
//! the `display: contents` element's own style stays the same.
#![cfg(feature = "writing-mode")]

use blitz_dom::{
    BaseDocument, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet, ns,
};
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

/// A holder that leaves `vertical-rl` and comes back, over `depth` nested `display: contents`
/// elements in `vertical-rl` around an inline element.
fn follows_through(depth: usize) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse("html, div { display: block }")],
        ..DocumentConfig::default()
    });
    doc.set_viewport(Viewport::new(800, 600, 1.0, ColorScheme::Light));
    let document = doc.root_node().id;
    let html = element(&mut doc, document, "html", "");
    let holder = element(&mut doc, html, "div", "writing-mode: vertical-rl");
    let mut parent = holder;
    for _ in 0..depth {
        parent = element(
            &mut doc,
            parent,
            "span",
            "display: contents; writing-mode: vertical-rl",
        );
    }
    let inline = element(&mut doc, parent, "i", "");
    doc.resolve(0.0);
    assert_eq!(display(&doc, inline), Display::Inline, "depth {depth}");

    doc.mutate().set_attribute(
        holder,
        QualName::new(None, ns!(), LocalName::from("style")),
        "",
    );
    doc.resolve(0.0);
    assert_eq!(display(&doc, inline), Display::InlineBlock, "depth {depth}");

    doc.mutate().set_attribute(
        holder,
        QualName::new(None, ns!(), LocalName::from("style")),
        "writing-mode: vertical-rl",
    );
    doc.resolve(0.0);
    assert_eq!(
        display(&doc, inline),
        Display::Inline,
        "depth {depth}, and back"
    );
}

#[test]
fn a_box_below_display_contents_follows_its_layout_parents_writing_mode() {
    follows_through(1);
}

#[test]
fn a_box_below_nested_display_contents_follows_its_layout_parents_writing_mode() {
    follows_through(2);
}

/// The `::before` of the innermost of two nested `display: contents` elements in `vertical-rl`
/// follows a holder that leaves `vertical-rl` and comes back.
#[test]
fn a_pseudo_element_of_nested_display_contents_follows_its_layout_parents_writing_mode() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(
            "html, div { display: block } .b::before { content: \"b\" }",
        )],
        ..DocumentConfig::default()
    });
    doc.set_viewport(Viewport::new(800, 600, 1.0, ColorScheme::Light));
    let document = doc.root_node().id;
    let html = element(&mut doc, document, "html", "");
    let holder = element(&mut doc, html, "div", "writing-mode: vertical-rl");
    let outer = element(
        &mut doc,
        holder,
        "span",
        "display: contents; writing-mode: vertical-rl",
    );
    let inner = element(
        &mut doc,
        outer,
        "span",
        "display: contents; writing-mode: vertical-rl",
    );
    doc.mutate().set_attribute(
        inner,
        QualName::new(None, ns!(), LocalName::from("class")),
        "b",
    );
    doc.resolve(0.0);
    let before = |doc: &BaseDocument| display(doc, doc.get_node(inner).unwrap().before().unwrap());
    assert_eq!(before(&doc), Display::Inline);

    let style = QualName::new(None, ns!(), LocalName::from("style"));
    doc.mutate().set_attribute(holder, style.clone(), "");
    doc.resolve(0.0);
    assert_eq!(before(&doc), Display::InlineBlock);

    doc.mutate()
        .set_attribute(holder, style, "writing-mode: vertical-rl");
    doc.resolve(0.0);
    assert_eq!(before(&doc), Display::Inline, "and back");
}
