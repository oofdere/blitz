//! The box a native control's own toolkit gives it, which the embedder sets
//! with `BaseDocument::set_native_box`, takes the place of the user agent's
//! table: the control is that size where the page's `width` or `height` is
//! `auto`, and what it holds goes inside the toolkit's insets, while a size
//! the page sets still wins, as in Chrome.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NativeBox, NodeId, QualName,
    UserAgentStylesheet, Viewport, ns,
};

const UA_CSS: &str = "html, body, div { display: block } \
                      input, button { display: inline-block; appearance: auto } \
                      input:not([type=checkbox]), button { padding: 1px 6px; border: 2px solid }";

fn document() -> (BaseDocument, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", &[]);
    let body = create(&mut doc, html, "body", &[("style", "margin: 0")]);
    (doc, body)
}

fn create(
    doc: &mut BaseDocument,
    parent: NodeId,
    name: &str,
    attributes: &[(&str, &str)],
) -> NodeId {
    let id = doc.mutate().create_element(
        QualName::new(None, ns!(html), LocalName::from(name)),
        Vec::new(),
    );
    for (name, value) in attributes {
        let attr = QualName::new(None, ns!(), LocalName::from(*name));
        doc.mutate().set_attribute(id, attr, value);
    }
    doc.mutate().append_children(parent, &[id]);
    id
}

/// Each control in its own block, so that its left edge is the block's.
fn control(
    doc: &mut BaseDocument,
    body: NodeId,
    name: &str,
    attributes: &[(&str, &str)],
) -> NodeId {
    let container = create(doc, body, "div", &[]);
    create(doc, container, name, attributes)
}

fn rect(doc: &BaseDocument, id: NodeId) -> (f64, f64, f64, f64) {
    let rect = doc.get_client_bounding_rect(id).unwrap();
    (rect.x, rect.y, rect.width, rect.height)
}

fn native(width: f32, height: f32, [left, right, top, bottom]: [f32; 4]) -> Option<NativeBox> {
    Some(NativeBox {
        size: taffy::Size { width, height },
        insets: taffy::Rect {
            left,
            right,
            top,
            bottom,
        },
    })
}

/// A checkbox the toolkit makes 44 by 44 is that size in place of Chrome's
/// 13, and taking the box away gives it back Chrome's.
#[test]
fn a_native_checkbox_is_the_size_its_toolkit_gives_it() {
    let (mut doc, body) = document();
    let checkbox = control(&mut doc, body, "input", &[("type", "checkbox")]);
    doc.resolve(0.0);
    assert_eq!(rect(&doc, checkbox).2, 13.0);
    doc.set_native_box(checkbox, native(44.0, 44.0, [0.0; 4]));
    doc.resolve(0.0);
    assert_eq!(rect(&doc, checkbox).2, 44.0);
    assert_eq!(rect(&doc, checkbox).3, 44.0);
    doc.set_native_box(checkbox, None);
    doc.resolve(0.0);
    assert_eq!(rect(&doc, checkbox).2, 13.0);
}

/// The page's `width` and `height` win where it sets them: a checkbox the
/// page makes 20px wide is 20 wide and the toolkit's 44 high, and one it
/// makes 20 by 30 is 20 by 30.
#[test]
fn the_pages_size_wins_over_the_toolkits() {
    let (mut doc, body) = document();
    let wide = control(
        &mut doc,
        body,
        "input",
        &[("type", "checkbox"), ("style", "width: 20px")],
    );
    let both = control(
        &mut doc,
        body,
        "input",
        &[("type", "checkbox"), ("style", "width: 20px; height: 30px")],
    );
    doc.resolve(0.0);
    for id in [wide, both] {
        doc.set_native_box(id, native(44.0, 44.0, [0.0; 4]));
    }
    doc.resolve(0.0);
    assert_eq!((rect(&doc, wide).2, rect(&doc, wide).3), (20.0, 44.0));
    assert_eq!((rect(&doc, both).2, rect(&doc, both).3), (20.0, 30.0));
}

/// A text field the toolkit makes 200 by 36 with insets of 8 across and 6
/// down is that size, whatever the user agent's padding and border, and a
/// button's label is laid out inside the toolkit's insets: one 120 by 44
/// with insets of 16, 16, 12 and 12 puts a label 20 high, which fills the
/// content box it leaves, at 16, 12.
#[test]
fn a_field_and_a_buttons_label_take_the_toolkits_insets() {
    let (mut doc, body) = document();
    let field = control(&mut doc, body, "input", &[]);
    let button = control(&mut doc, body, "button", &[]);
    let label = create(
        &mut doc,
        button,
        "div",
        &[("style", "width: 30px; height: 20px")],
    );
    doc.resolve(0.0);
    doc.set_native_box(field, native(200.0, 36.0, [8.0, 8.0, 6.0, 6.0]));
    doc.set_native_box(button, native(120.0, 44.0, [16.0, 16.0, 12.0, 12.0]));
    doc.resolve(0.0);
    assert_eq!((rect(&doc, field).2, rect(&doc, field).3), (200.0, 36.0));
    let (x, y, width, height) = rect(&doc, button);
    assert_eq!((width, height), (120.0, 44.0));
    let inner = rect(&doc, label);
    assert_eq!((inner.0 - x, inner.1 - y), (16.0, 12.0));
}
