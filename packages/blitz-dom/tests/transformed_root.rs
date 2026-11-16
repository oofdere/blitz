//! A root element with a transform is the containing block of its fixed-position
//! descendants (CSS Transforms 1 §2), so they are placed against its padding box and
//! scroll with the page instead of staying put in the viewport.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div { display: block }";
const TRANSFORMED: &str = "transform: translateX(10px)";

fn element(doc: &mut BaseDocument, parent: NodeId, name: &str, style: &str) -> NodeId {
    let name = QualName::new(None, ns!(html), LocalName::from(name));
    let id = doc.mutate().create_element(name, Vec::new());
    let style_attr = QualName::new(None, ns!(), LocalName::from("style"));
    doc.mutate().set_attribute(id, style_attr, style);
    doc.mutate().append_children(parent, &[id]);
    id
}

/// An 800 by 600 page with this style on the root element, whose body has no margin and
/// holds a div 2000px tall, then one fixed box for each style in `fixed`.
fn page(html: &str, fixed: &[&str]) -> (BaseDocument, Vec<NodeId>) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = element(&mut doc, root, "html", html);
    let body = element(&mut doc, html, "body", "margin: 0");
    element(&mut doc, body, "div", "height: 2000px");
    let fixed = fixed
        .iter()
        .map(|style| {
            let style = format!("position: fixed; left: 0; width: 50px; height: 50px; {style}");
            element(&mut doc, body, "div", &style)
        })
        .collect();
    doc.resolve(0.0);
    (doc, fixed)
}

fn top_left(doc: &BaseDocument, id: NodeId) -> (f32, f32) {
    let rect = doc.get_client_bounding_rect(id).unwrap();
    (rect.x as f32, rect.y as f32)
}

#[test]
fn the_viewport_holds_the_fixed_boxes_of_a_root_without_a_transform() {
    let (mut doc, fixed) = page("", &["bottom: 0"]);
    assert_eq!(top_left(&doc, fixed[0]), (0.0, 550.0));
    doc.scroll_viewport_by(0.0, -500.0);
    assert_eq!(doc.viewport_scroll().y, 500.0);
    assert_eq!(top_left(&doc, fixed[0]), (0.0, 550.0));
}

#[test]
fn a_transformed_root_moves_its_fixed_boxes_with_the_page() {
    let (mut doc, fixed) = page(TRANSFORMED, &["bottom: 0"]);
    assert_eq!(top_left(&doc, fixed[0]), (10.0, 1950.0));
    doc.scroll_viewport_by(0.0, -500.0);
    assert_eq!(doc.viewport_scroll().y, 500.0);
    assert_eq!(top_left(&doc, fixed[0]), (10.0, 1450.0));
}

/// The box `hit` finds at this point of the page.
fn hit(doc: &BaseDocument, x: f32, y: f32) -> Option<NodeId> {
    doc.hit(x, y).map(|hit| hit.node_id)
}

#[test]
fn a_hit_finds_the_fixed_boxes_of_a_root_without_a_transform_where_the_viewport_took_them() {
    let (mut doc, fixed) = page("", &["top: 100px", "top: 200px; z-index: 1"]);
    doc.scroll_viewport_by(0.0, -500.0);
    assert_eq!(hit(&doc, 30.0, 625.0), Some(fixed[0]));
    assert_eq!(hit(&doc, 30.0, 725.0), Some(fixed[1]));
}

#[test]
fn a_hit_finds_the_fixed_boxes_of_a_transformed_root_where_the_page_took_them() {
    let (mut doc, fixed) = page(TRANSFORMED, &["top: 100px", "top: 200px; z-index: 1"]);
    doc.scroll_viewport_by(0.0, -500.0);
    assert_eq!(hit(&doc, 30.0, 125.0), Some(fixed[0]));
    assert_eq!(hit(&doc, 30.0, 225.0), Some(fixed[1]));
    assert_ne!(hit(&doc, 30.0, 625.0), Some(fixed[0]));
}

#[test]
fn the_fixed_boxes_of_a_transformed_root_are_in_the_page_it_scrolls() {
    let (doc, _) = page(TRANSFORMED, &["top: 3000px"]);
    assert_eq!(doc.viewport_scroll_area().y1, 3050.0);
    let (doc, _) = page("", &["top: 3000px"]);
    assert_eq!(doc.viewport_scroll_area().y1, 2000.0);
}
