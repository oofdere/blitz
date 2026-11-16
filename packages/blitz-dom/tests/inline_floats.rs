//! A float in an inline formatting context is laid out by inline layout, which
//! places it in the block formatting context. Its scrollable overflow is the one
//! its own layout gave, as for a float in a block container. The documents are
//! built with niso's DocumentMutator calls only.
#![cfg(feature = "floats")]

use blitz_dom::{
    BaseDocument, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet, ns,
};
use blitz_traits::shell::{ColorScheme, Viewport};

const SCROLLER: &str = "float: right; overflow: scroll; width: 30px; height: 10px";

fn document() -> BaseDocument {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse("html, div { display: block }")],
        ..DocumentConfig::default()
    });
    doc.set_viewport(Viewport::new(800, 600, 1.0, ColorScheme::Light));
    doc
}

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

/// html > `parent` > a floated scroll container 30 by 10 holding a block 300 by
/// 150. Gives the scroll container's scrollable overflow, as right and bottom.
fn overflow(parent: &str) -> (f32, f32) {
    let mut doc = document();
    let root = doc.root_node().id;
    let html = element(&mut doc, root, "html", "");
    let parent = element(&mut doc, html, parent, "");
    let scroller = element(&mut doc, parent, "div", SCROLLER);
    element(&mut doc, scroller, "div", "width: 300px; height: 150px");
    doc.resolve(0.0);
    let overflow = doc
        .get_node(scroller)
        .unwrap()
        .final_layout()
        .scrollable_overflow_rect;
    (overflow.right(), overflow.bottom())
}

#[test]
fn a_float_in_an_inline_flow_keeps_its_scrollable_overflow() {
    assert_eq!(overflow("div"), (300.0, 150.0), "in a block container");
    assert_eq!(overflow("span"), (300.0, 150.0), "in an inline flow");
}

/// Laid out once in a block container and then moved into an inline flow, the
/// float's scrollable overflow is its new layout's, not the one it had.
#[test]
fn a_float_moved_into_an_inline_flow_has_its_new_overflow() {
    let mut doc = document();
    let root = doc.root_node().id;
    let html = element(&mut doc, root, "html", "");
    let div = element(&mut doc, html, "div", "");
    let span = element(&mut doc, html, "span", "");
    let scroller = element(&mut doc, div, "div", SCROLLER);
    let content = element(&mut doc, scroller, "div", "width: 620px; height: 240px");
    doc.resolve(0.0);
    doc.mutate().append_children(span, &[scroller]);
    doc.mutate().set_attribute(
        content,
        QualName::new(None, ns!(), LocalName::from("style")),
        "width: 300px; height: 150px",
    );
    doc.resolve(0.0);
    let overflow = doc
        .get_node(scroller)
        .unwrap()
        .final_layout()
        .scrollable_overflow_rect;
    assert_eq!((overflow.right(), overflow.bottom()), (300.0, 150.0));
}
