//! A list item's marker on a page edited after layout must be the one a page built
//! fresh gives it. The edits are [DocumentMutator](blitz_dom::DocumentMutator) calls
//! only, as niso's layout side makes them.

use blitz_dom::node::ListItemLayoutPosition;
use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, div, ul { display: block } li { display: list-item }";

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

fn has_marker(doc: &BaseDocument, node: NodeId) -> bool {
    doc.get_node(node)
        .and_then(|node| node.element_data())
        .is_some_and(|element| element.list_item_data.is_some())
}

/// html > span > div > span > li, where the div becomes display: contents. The li then
/// sits under an inline element laid out in html's inline formatting context, and is no
/// longer under a box that construction gives list markers to. Shrunk from seed 2104 of
/// Lens's frames harness.
#[test]
fn a_list_item_whose_parent_joins_an_inline_flow_through_contents_reads_as_on_a_fresh_page() {
    let read = |change_after_layout: bool| {
        let (mut doc, html) = page();
        let outer = add(&mut doc, html, "span");
        let div = add(&mut doc, outer, "div");
        let inner = add(&mut doc, div, "span");
        let li = add(&mut doc, inner, "li");
        if change_after_layout {
            doc.resolve(0.0);
        }
        set_style(&mut doc, div, "display: contents");
        doc.resolve(0.0);
        has_marker(&doc, li)
    };
    assert_eq!(read(true), read(false), "kept and fresh differ");
}

/// html > span > (li, div), where the li is out of flow and the div is removed. The span
/// then holds no block and is laid out in html's inline formatting context.
#[test]
fn an_out_of_flow_list_item_whose_parent_joins_an_inline_flow_reads_as_on_a_fresh_page() {
    let read = |li_style: &str, remove_after_layout: bool| {
        let (mut doc, html) = page();
        let span = add(&mut doc, html, "span");
        let li = add(&mut doc, span, "li");
        set_style(&mut doc, li, li_style);
        let block = add(&mut doc, span, "div");
        if remove_after_layout {
            doc.resolve(0.0);
        }
        doc.mutate().remove_and_drop_node_with(block, &mut |_| {});
        doc.resolve(0.0);
        has_marker(&doc, li)
    };
    let differ: Vec<_> = ["position: absolute", "float: left"]
        .into_iter()
        .filter(|style| read(style, true) != read(style, false))
        .collect();
    assert!(differ.is_empty(), "kept and fresh differ for {differ:?}");
}

/// The width of an outside marker's layout, which is in device pixels.
fn outside_marker_width(doc: &BaseDocument, node: NodeId) -> f32 {
    let element = doc.get_node(node).unwrap().element_data().unwrap();
    match &element.list_item_data.as_deref().unwrap().position {
        ListItemLayoutPosition::Outside(layout) => layout.full_width(),
        ListItemLayoutPosition::Inside => panic!("the marker is inside"),
    }
}

/// A marker outside its list item is shaped in device pixels, so when the scale goes
/// from 1 to 2 for the same 800 by 600 CSS pixels its layout must be twice as wide. The
/// list item holds no text: a scale change builds every inline layout again, and the
/// list around one, but here there is none.
#[test]
fn an_outside_marker_is_laid_out_again_when_the_scale_changes() {
    let (mut doc, html) = page();
    let ul = add(&mut doc, html, "ul");
    let li = add(&mut doc, ul, "li");
    doc.resolve(0.0);
    let at_one = outside_marker_width(&doc, li);

    doc.set_viewport(Viewport::new(1600, 1200, 2.0, ColorScheme::Light));
    doc.resolve(0.0);
    let at_two = outside_marker_width(&doc, li);

    assert!(at_one > 0.0, "the marker has no width at scale 1");
    assert!(
        (at_two - 2.0 * at_one).abs() <= 1.0,
        "at scale 1 the marker is {at_one} device pixels wide, and at scale 2 {at_two}"
    );
}
