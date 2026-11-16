//! A fixed box with `z-index: auto` inside a positioned box is painted in tree
//! order among that box's positioned descendants (CSS 2.1 Appendix E, step 8),
//! though its containing block is the viewport: after the ones before it and
//! under the ones after it, as Acid2's hidden scalp line needs.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div { display: block }";

fn create(doc: &mut BaseDocument, parent: NodeId, style: &str) -> NodeId {
    let id = doc.mutate().create_element(
        QualName::new(None, ns!(html), LocalName::from("div")),
        Vec::new(),
    );
    let attr = QualName::new(None, ns!(), LocalName::from("style"));
    doc.mutate().set_attribute(id, attr, style);
    doc.mutate().append_children(parent, &[id]);
    id
}

/// A relatively positioned box at (10, 20) holding an absolutely positioned
/// box, a fixed one and another absolutely positioned one, 50px squares at
/// (10, 20), (30, 40) and (50, 60) in the viewport, each overlapping the next.
/// `wrap` styles a static box around the fixed one, or none.
fn page(wrap: Option<&str>) -> (BaseDocument, [NodeId; 4]) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "");
    let body = create(&mut doc, html, "margin: 0; height: 2000px");
    let picture = create(
        &mut doc,
        body,
        "position: relative; margin: 20px 0 0 10px; width: 100px; height: 100px",
    );
    let square = "width: 50px; height: 50px";
    let before = create(
        &mut doc,
        picture,
        &format!("position: absolute; top: 0; left: 0; {square}"),
    );
    let holder = match wrap {
        Some(style) => create(&mut doc, picture, style),
        None => picture,
    };
    let fixed = create(
        &mut doc,
        holder,
        &format!("position: fixed; top: 40px; left: 30px; {square}"),
    );
    let after = create(
        &mut doc,
        picture,
        &format!("position: absolute; top: 40px; left: 40px; {square}"),
    );
    doc.resolve(0.0);
    (doc, [picture, before, fixed, after])
}

fn top(doc: &BaseDocument, x: f32, y: f32) -> Option<NodeId> {
    doc.element_from_point(x, y)
}

#[test]
fn a_fixed_box_is_painted_in_tree_order_among_positioned_boxes() {
    let (doc, [picture, before, fixed, after]) = page(None);
    let painted: Option<Vec<NodeId>> = doc
        .get_node(picture)
        .unwrap()
        .paint_children
        .borrow()
        .as_ref()
        .map(|painted| painted.to_vec());
    assert_eq!(painted, Some(vec![before, fixed, after]));
    let rect = doc.get_client_bounding_rect(fixed).unwrap();
    assert_eq!((rect.x, rect.y), (30.0, 40.0));
    assert_eq!(top(&doc, 35.0, 45.0), Some(fixed), "over the box before it");
    assert_eq!(top(&doc, 55.0, 65.0), Some(after), "under the box after it");
}

#[test]
fn a_fixed_box_in_a_static_box_is_painted_in_tree_order_too() {
    let (doc, [_, before, fixed, after]) = page(Some(""));
    assert_eq!(top(&doc, 35.0, 45.0), Some(fixed), "over the box before it");
    assert_eq!(top(&doc, 55.0, 65.0), Some(after), "under the box after it");
    assert_ne!(top(&doc, 15.0, 25.0), Some(fixed));
    assert_eq!(top(&doc, 15.0, 25.0), Some(before));
}

#[test]
fn a_fixed_box_in_a_clipping_box_is_left_to_the_viewport() {
    let (doc, [_, _, fixed, _]) = page(Some("overflow: hidden; height: 0"));
    assert_eq!(top(&doc, 55.0, 65.0), Some(fixed), "over everything");
}

#[test]
fn a_fixed_box_painted_in_tree_order_stays_put_when_the_page_scrolls() {
    let (mut doc, [_, before, fixed, after]) = page(Some(""));
    doc.set_viewport_scroll(blitz_dom::Point { x: 0.0, y: 10.0 });
    doc.resolve(0.0);
    assert_eq!(top(&doc, 35.0, 45.0), Some(fixed), "over the box before it");
    assert_eq!(top(&doc, 55.0, 65.0), Some(after), "under the box after it");
    assert_eq!(
        top(&doc, 35.0, 85.0),
        Some(fixed),
        "where it was before the scroll"
    );
    assert_eq!(
        top(&doc, 15.0, 15.0),
        Some(before),
        "the box before it scrolled"
    );
}

/// A fixed box two static boxes down, with the outer of the two styled
/// `outer`, and the outer box's paint list.
fn nested(outer: &str) -> (BaseDocument, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "");
    let body = create(&mut doc, html, "margin: 0");
    let outer_box = create(&mut doc, body, outer);
    let middle = create(&mut doc, outer_box, "");
    create(
        &mut doc,
        middle,
        "position: fixed; top: 40px; left: 30px; width: 50px; height: 50px",
    );
    doc.resolve(0.0);
    (doc, outer_box)
}

fn painted(doc: &BaseDocument, node: NodeId) -> Option<Vec<NodeId>> {
    doc.get_node(node)
        .unwrap()
        .paint_children
        .borrow()
        .as_ref()
        .map(|painted| painted.to_vec())
}

/// A box that becomes positioned after a layout paints the fixed box inside
/// it in tree order, as it does when it was positioned from the start, though
/// the static box between them did not change.
#[test]
fn a_box_positioned_later_paints_the_fixed_box_inside_it() {
    let (mut doc, outer) = nested("");
    let attr = QualName::new(None, ns!(), LocalName::from("style"));
    doc.mutate()
        .set_attribute(outer, attr, "position: relative");
    doc.resolve(0.0);
    let (fresh, fresh_outer) = nested("position: relative");
    assert_eq!(painted(&doc, outer), painted(&fresh, fresh_outer));
    assert_eq!(painted(&doc, outer).map(|painted| painted.len()), Some(2));
}
