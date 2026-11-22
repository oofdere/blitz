//! `elementFromPoint` takes viewport coordinates, so what it finds moves with the page's
//! scroll, and a fixed box stays where the viewport holds it.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div { display: block }";

fn element(doc: &mut BaseDocument, parent: NodeId, name: &str, style: &str) -> NodeId {
    let name = QualName::new(None, ns!(html), LocalName::from(name));
    let id = doc.mutate().create_element(name, Vec::new());
    let style_attr = QualName::new(None, ns!(), LocalName::from("style"));
    doc.mutate().set_attribute(id, style_attr, style);
    doc.mutate().append_children(parent, &[id]);
    id
}

/// An 800 by 600 page whose body has no margin and holds two blocks 1000px tall, one under
/// the other, and a fixed box at the top left: their ids.
fn page() -> (BaseDocument, [NodeId; 3]) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = element(&mut doc, root, "html", "");
    let body = element(&mut doc, html, "body", "margin: 0");
    let first = element(&mut doc, body, "div", "height: 1000px");
    let second = element(&mut doc, body, "div", "height: 1000px");
    let fixed = element(
        &mut doc,
        body,
        "div",
        "position: fixed; top: 0; left: 0; width: 50px; height: 50px",
    );
    doc.resolve(0.0);
    (doc, [first, second, fixed])
}

#[test]
fn a_point_finds_what_the_page_scrolled_under_it() {
    let (mut doc, [first, second, _]) = page();
    assert_eq!(doc.element_from_point(100.0, 100.0), Some(first));
    doc.scroll_viewport_by(0.0, -1500.0);
    assert_eq!(doc.viewport_scroll().y, 1400.0);
    assert_eq!(doc.element_from_point(100.0, 100.0), Some(second));
    assert_eq!(doc.elements_from_point(100.0, 100.0)[0], second);
}

#[test]
fn a_point_finds_a_fixed_box_where_the_viewport_holds_it() {
    let (mut doc, [_, _, fixed]) = page();
    doc.scroll_viewport_by(0.0, -1500.0);
    assert_eq!(doc.element_from_point(25.0, 25.0), Some(fixed));
}

/// An 800 by 600 page whose body has no margin: the document, its root element and its
/// body.
fn blank() -> (BaseDocument, NodeId, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = element(&mut doc, root, "html", "");
    let body = element(&mut doc, html, "body", "margin: 0");
    (doc, html, body)
}

/// A box positioned absolutely in a positioned parent that clips its overflow is cut by the
/// parent's clip, though the parent does not paint it: past the clip the point finds what
/// is behind, and inside it the box.
#[test]
fn a_clip_cuts_a_positioned_box_it_contains() {
    let (mut doc, html, body) = blank();
    let clip = element(
        &mut doc,
        body,
        "div",
        "position: relative; overflow: hidden; width: 50px; height: 50px",
    );
    let child = element(
        &mut doc,
        clip,
        "div",
        "position: absolute; left: 0; top: 0; width: 200px; height: 200px",
    );
    doc.resolve(0.0);
    assert_eq!(doc.element_from_point(25.0, 25.0), Some(child));
    assert_eq!(doc.element_from_point(100.0, 100.0), Some(html));
}

/// A clip only cuts the boxes whose containing blocks it is among: a box positioned
/// absolutely against an ancestor above the clip is found past it.
#[test]
fn a_clip_leaves_a_box_contained_above_it() {
    let (mut doc, _html, body) = blank();
    let holder = element(&mut doc, body, "div", "position: relative");
    let clip = element(
        &mut doc,
        holder,
        "div",
        "overflow: hidden; width: 50px; height: 50px",
    );
    let child = element(
        &mut doc,
        clip,
        "div",
        "position: absolute; left: 0; top: 0; width: 200px; height: 200px",
    );
    doc.resolve(0.0);
    assert_eq!(doc.element_from_point(100.0, 100.0), Some(child));
}

/// A hidden box is not hit, but a descendant that is visible again is.
#[test]
fn a_visible_child_of_a_hidden_box_is_hit() {
    let (mut doc, _html, body) = blank();
    let hidden = element(
        &mut doc,
        body,
        "div",
        "visibility: hidden; width: 100px; height: 100px",
    );
    let child = element(
        &mut doc,
        hidden,
        "div",
        "visibility: visible; width: 50px; height: 50px",
    );
    doc.resolve(0.0);
    assert_eq!(doc.element_from_point(25.0, 25.0), Some(child));
    assert_eq!(doc.element_from_point(75.0, 75.0), Some(body));
}

/// A child in flow that overflows a box with overflow: hidden or clip is cut at the
/// box's padding box too: past it the point finds what is behind.
#[test]
fn a_clip_cuts_a_child_in_flow() {
    for overflow in ["hidden", "clip"] {
        let (mut doc, _html, body) = blank();
        let clip = element(
            &mut doc,
            body,
            "div",
            &format!("overflow: {overflow}; width: 50px; height: 50px"),
        );
        let child = element(&mut doc, clip, "div", "width: 100px; height: 20px");
        doc.resolve(0.0);
        assert_eq!(
            doc.element_from_point(25.0, 10.0),
            Some(child),
            "{overflow}"
        );
        assert_eq!(doc.element_from_point(75.0, 10.0), Some(body), "{overflow}");
    }
}

/// A scroll container under a transform is scrolled inside its own box: the transform is
/// undone first and the scroll offset added after. A box 100px square with `scale(2)`
/// about its origin, scrolled 50px down past a first child 50px high, shows the second
/// child at its top, and the point (10, 10) finds it, as in Chrome 157.
#[test]
fn a_scaled_scroll_container_finds_what_it_scrolled_under_the_point() {
    let (mut doc, _html, body) = blank();
    let scroller = element(
        &mut doc,
        body,
        "div",
        "width: 100px; height: 100px; overflow: hidden; transform: scale(2); \
         transform-origin: 0 0",
    );
    element(&mut doc, scroller, "div", "height: 50px");
    let second = element(&mut doc, scroller, "div", "height: 250px");
    doc.resolve(0.0);
    doc.get_node_mut(scroller).unwrap().scroll_offset_mut().y = 50.0;
    assert_eq!(doc.element_from_point(10.0, 10.0), Some(second));
    assert_eq!(doc.element_from_point(10.0, 150.0), Some(second));
}

/// A box with `overflow: clip` cuts what it holds at the box `overflow-clip-margin` names,
/// grown by its length, as Chrome 157 paints it and finds what is under a point in it: a
/// `content-box` clip cuts inside the padding box. `overflow: hidden` makes a scroll
/// container, which the margin does not move.
#[test]
fn a_clip_margin_moves_where_a_clip_cuts() {
    for (style, inside, outside) in [
        ("overflow-clip-margin: 20px", [60.0, 69.0], 75.0),
        (
            "overflow-clip-margin: content-box 5px; padding: 20px",
            [16.0, 74.0],
            76.0,
        ),
        (
            "overflow-clip-margin: border-box; border: 10px solid",
            [61.0, 69.0],
            71.0,
        ),
        (
            "overflow-clip-margin: border-box 5px; border: 10px solid",
            [74.0, 74.0],
            76.0,
        ),
    ] {
        let (mut doc, _html, body) = blank();
        let clip = element(
            &mut doc,
            body,
            "div",
            &format!("overflow: clip; width: 50px; height: 50px; {style}"),
        );
        let child = element(
            &mut doc,
            clip,
            "div",
            "width: 200px; height: 20px; margin-left: -20px",
        );
        doc.resolve(0.0);
        let y = doc.get_node(child).unwrap().final_layout().location.y + 5.0;
        for x in inside {
            assert_eq!(doc.element_from_point(x, y), Some(child), "{style} at {x}");
        }
        assert_ne!(doc.element_from_point(outside, y), Some(child), "{style}");
    }
    let (mut doc, _html, body) = blank();
    let clip = element(
        &mut doc,
        body,
        "div",
        "overflow: hidden; overflow-clip-margin: 20px; width: 50px; height: 50px",
    );
    let child = element(&mut doc, clip, "div", "width: 100px; height: 20px");
    doc.resolve(0.0);
    assert_ne!(doc.element_from_point(55.0, 10.0), Some(child));
}
