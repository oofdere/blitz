//! A point is hit-tested as Chrome 157 tests it: taken down to a 64th of a CSS pixel and
//! tested as the device pixel that runs right and down from it. A box is found from one
//! device pixel before its left and top edges up to, and not at, its right and bottom ones;
//! of two boxes that device pixel reaches, the one painted later is found; and an overflow
//! clip lets a box through where the device pixel reaches inside the clip. Measured in
//! Chrome 157 at device scales 1 and 2 with elementFromPoint.

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

/// An 800 by 600 CSS pixel page at `scale` whose body has no margin: the document, its
/// root element and its body.
fn blank(scale: f32) -> (BaseDocument, NodeId, NodeId) {
    let size = |css: f32| (css * scale) as u32;
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(
            size(800.0),
            size(600.0),
            scale,
            ColorScheme::Light,
        )),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = element(&mut doc, root, "html", "");
    let body = element(&mut doc, html, "body", "margin: 0");
    (doc, html, body)
}

fn absolute(doc: &mut BaseDocument, parent: NodeId, style: &str) -> NodeId {
    element(doc, parent, "div", &format!("position: absolute; {style}"))
}

/// A box at left 10.3 is found from 9.3125 at scale 1 and from 9.8125 at scale 2, a 64th
/// later than the point whose device pixel would end at its left edge, up to 60.28125,
/// and not at 60.3125, past its right edge. Top and bottom are the same, and a node's own
/// hit test takes the point down to a 64th as the document's does.
#[test]
fn a_box_is_found_from_a_device_pixel_before_its_left_edge_to_its_right_edge() {
    for (scale, miss, hit) in [(1.0, 9.296875, 9.3125), (2.0, 9.796875, 9.8125)] {
        let (mut doc, html, body) = blank(scale);
        let target = absolute(
            &mut doc,
            body,
            "left: 10.3px; top: 10.3px; width: 50px; height: 20px",
        );
        doc.resolve(0.0);
        let at = |x: f32, y: f32| doc.element_from_point(x, y);
        assert_eq!(at(miss, 20.0), Some(html), "x {miss} at scale {scale}");
        assert_eq!(at(hit, 20.0), Some(target), "x {hit} at scale {scale}");
        assert_eq!(
            at(9.31, 20.0),
            Some(html),
            "9.31 is 9.296875, at scale {scale}"
        );
        assert_eq!(at(60.28125, 20.0), Some(target), "at scale {scale}");
        assert_eq!(at(60.3125, 20.0), Some(html), "at scale {scale}");
        assert_eq!(at(30.0, miss), Some(html), "y {miss} at scale {scale}");
        assert_eq!(at(30.0, hit), Some(target), "y {hit} at scale {scale}");
        assert_eq!(at(30.0, 30.3125), Some(html), "at scale {scale}");
        let node_hit = |x: f32| {
            doc.root_element()
                .hit(x, 20.0, f64::from(scale))
                .map(|hit| hit.node_id)
        };
        assert_ne!(node_hit(9.31), Some(target), "at scale {scale}");
        assert_eq!(node_hit(hit), Some(target), "at scale {scale}");
    }
}

/// A box with no width, or no height, is never found, though the device pixel at the point
/// reaches across its edge: a box 0 wide at left 10.3 and one 0 high at top 10.3 are not
/// found from 9.3125 to 10.3, at scale 1 or 2, and the root element is. What such a box
/// holds is still found.
#[test]
fn a_box_with_no_area_is_never_found() {
    for scale in [1.0, 2.0] {
        let (mut doc, html, body) = blank(scale);
        absolute(
            &mut doc,
            body,
            "left: 10.3px; top: 10px; width: 0; height: 20px; background: red",
        );
        let flat = absolute(
            &mut doc,
            body,
            "left: 100px; top: 10.3px; width: 50px; height: 0",
        );
        let held = element(&mut doc, flat, "div", "width: 10px; height: 10px");
        doc.resolve(0.0);
        let at = |x: f32, y: f32| doc.element_from_point(x, y);
        for point in [9.3125, 9.4, 9.5, 10.0, 10.2, 10.29, 10.3] {
            assert_eq!(at(point, 20.0), Some(html), "x {point} at scale {scale}");
            assert_eq!(at(130.0, point), Some(html), "y {point} at scale {scale}");
        }
        assert_eq!(at(105.0, 15.0), Some(held), "at scale {scale}");
    }
}

/// Of two boxes that meet at 50.25, the point 49.5 finds the second, painted later, at
/// scale 1, whose device pixel reaches to 50.5, though the point is in the first; at scale
/// 2 its device pixel ends at 50, and it finds the first. With the first above the second
/// by `z-index`, the first is found wherever the device pixel reaches it.
#[test]
fn of_two_boxes_the_device_pixel_reaches_the_topmost_is_found() {
    for (scale, at_49_5) in [(1.0, 1), (2.0, 0)] {
        let (mut doc, _, body) = blank(scale);
        let a = absolute(
            &mut doc,
            body,
            "left: 0; top: 0; width: 50.25px; height: 20px",
        );
        let b = absolute(
            &mut doc,
            body,
            "left: 50.25px; top: 0; width: 50px; height: 20px",
        );
        let c = absolute(
            &mut doc,
            body,
            "left: 0; top: 40px; width: 50.25px; height: 20px; z-index: 1",
        );
        let d = absolute(
            &mut doc,
            body,
            "left: 50.25px; top: 40px; width: 50px; height: 20px",
        );
        doc.resolve(0.0);
        assert_eq!(
            doc.element_from_point(49.5, 10.0),
            Some([a, b][at_49_5]),
            "scale {scale}"
        );
        assert_eq!(doc.element_from_point(49.8, 10.0), Some(b), "scale {scale}");
        assert_eq!(doc.element_from_point(49.8, 50.0), Some(c), "scale {scale}");
        assert_eq!(
            doc.element_from_point(50.25, 50.0),
            Some(d),
            "scale {scale}"
        );
    }
}

/// A box clipped by its parent's `overflow: hidden` is found where the device pixel at
/// the point reaches inside the clip: from 9.3125 before a clip at 10.3, and not from the
/// clip's right edge, 40.3, on.
#[test]
fn a_clip_lets_a_box_through_where_the_device_pixel_reaches_inside_it() {
    let (mut doc, html, body) = blank(1.0);
    let clip = absolute(
        &mut doc,
        body,
        "left: 10.3px; top: 0; width: 30px; height: 20px; overflow: hidden",
    );
    let inner = absolute(
        &mut doc,
        clip,
        "left: -10px; top: 0; width: 100px; height: 20px",
    );
    doc.resolve(0.0);
    assert_eq!(doc.element_from_point(9.296875, 10.0), Some(html));
    assert_eq!(doc.element_from_point(9.4, 10.0), Some(inner));
    assert_eq!(doc.element_from_point(40.28125, 10.0), Some(inner));
    assert_eq!(doc.element_from_point(40.3125, 10.0), Some(html));
}

/// Under `transform: scale(2)` the device pixel is still one device pixel of the
/// viewport: a box drawn from 100 is found from 99.1 at scale 1 and from 99.9 at scale 2,
/// not from 99.5.
#[test]
fn under_a_transform_the_device_pixel_is_the_viewports() {
    for (scale, at_99_5) in [(1.0, true), (2.0, false)] {
        let (mut doc, html, body) = blank(scale);
        let t = absolute(
            &mut doc,
            body,
            "left: 100px; top: 0; width: 20px; height: 20px; transform: scale(2); transform-origin: 0 0",
        );
        doc.resolve(0.0);
        let found = doc.element_from_point(99.5, 10.0);
        assert_eq!(found, Some(if at_99_5 { t } else { html }), "scale {scale}");
        assert_eq!(doc.element_from_point(99.9, 10.0), Some(t), "scale {scale}");
        assert_eq!(
            doc.element_from_point(140.0, 10.0),
            Some(html),
            "scale {scale}"
        );
    }
}
