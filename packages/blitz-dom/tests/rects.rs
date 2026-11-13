//! `getBoundingClientRect` as CSSOM View defines it, for the cases where blitz's layout
//! box alone is not the answer.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div { display: block } span { display: inline }";

fn create(doc: &mut BaseDocument, name: &str) -> NodeId {
    let name = QualName::new(None, ns!(html), LocalName::from(name));
    doc.mutate().create_element(name, Vec::new())
}

/// An element with this style and, optionally, a text node, put last in `parent`.
fn element(doc: &mut BaseDocument, parent: NodeId, name: &str, style: &str, text: &str) -> NodeId {
    let id = create(doc, name);
    let style_attr = QualName::new(None, ns!(), LocalName::from("style"));
    doc.mutate().set_attribute(id, style_attr, style);
    if !text.is_empty() {
        let text = doc.mutate().create_text_node(text);
        doc.mutate().append_children(id, &[text]);
    }
    doc.mutate().append_children(parent, &[id]);
    id
}

/// An 800 by 600 page whose body has no margin.
fn page() -> (BaseDocument, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, "html");
    doc.mutate().append_children(root, &[html]);
    let body = element(&mut doc, html, "body", "margin: 0", "");
    (doc, body)
}

fn rect(doc: &BaseDocument, node: NodeId) -> (f64, f64, f64, f64) {
    let r = doc.get_client_bounding_rect(node).unwrap();
    (r.x, r.y, r.width, r.height)
}

#[test]
fn a_block_is_as_wide_as_the_viewport() {
    let (mut doc, body) = page();
    let node = element(&mut doc, body, "div", "height: 10px", "");
    doc.resolve(0.0);
    assert_eq!(rect(&doc, node), (0.0, 0.0, 800.0, 10.0));
}

#[test]
fn a_translated_box() {
    let (mut doc, body) = page();
    let node = element(
        &mut doc,
        body,
        "div",
        "width: 100px; height: 50px; transform: translate(10px, 20px)",
        "",
    );
    doc.resolve(0.0);
    assert_eq!(rect(&doc, node), (10.0, 20.0, 100.0, 50.0));
}

#[test]
fn a_scaled_box() {
    let (mut doc, body) = page();
    let node = element(
        &mut doc,
        body,
        "div",
        "width: 100px; height: 50px; transform: scale(2)",
        "",
    );
    doc.resolve(0.0);
    assert_eq!(rect(&doc, node), (-50.0, -25.0, 200.0, 100.0));
}

#[test]
fn a_rotated_box() {
    let (mut doc, body) = page();
    let node = element(
        &mut doc,
        body,
        "div",
        "width: 100px; height: 50px; transform: rotate(90deg)",
        "",
    );
    doc.resolve(0.0);
    assert_eq!(rect(&doc, node), (25.0, -25.0, 50.0, 100.0));
}

#[test]
fn the_child_of_a_translated_box() {
    let (mut doc, body) = page();
    let parent = element(
        &mut doc,
        body,
        "div",
        "transform: translate(10px, 20px)",
        "",
    );
    let node = element(&mut doc, parent, "div", "width: 30px; height: 10px", "");
    doc.resolve(0.0);
    assert_eq!(rect(&doc, node), (10.0, 20.0, 30.0, 10.0));
}

#[test]
fn an_inline_element_in_a_translated_box() {
    let (mut doc, body) = page();
    let parent = element(
        &mut doc,
        body,
        "div",
        "transform: translate(10px, 20px)",
        "",
    );
    let node = element(&mut doc, parent, "span", "", "abc");
    doc.resolve(0.0);
    let untransformed = {
        let (mut doc, body) = page();
        let parent = element(&mut doc, body, "div", "", "");
        let node = element(&mut doc, parent, "span", "", "abc");
        doc.resolve(0.0);
        rect(&doc, node)
    };
    let (x, y, width, height) = untransformed;
    assert_eq!(rect(&doc, node), (x + 10.0, y + 20.0, width, height));
}

/// An inline element's box is its content area, which `line-height` does not change:
/// with a 40px line, the box is as tall as with a 10px one, and the half-leading puts
/// it in the middle of the line.
#[test]
fn an_inline_element_is_as_tall_as_its_content_area() {
    let read = |line_height: &str| {
        let (mut doc, body) = page();
        let parent = element(
            &mut doc,
            body,
            "div",
            &format!("line-height: {line_height}"),
            "",
        );
        let node = element(&mut doc, parent, "span", "", "abc");
        doc.resolve(0.0);
        rect(&doc, node)
    };
    let (_, y40, _, height40) = read("40px");
    let (_, _, _, height10) = read("10px");
    assert_eq!(height40, height10);
    assert!(height40 < 40.0);
    assert!(
        (y40 - (40.0 - height40) / 2.0).abs() <= 1.0,
        "y {y40}, height {height40}"
    );
}
