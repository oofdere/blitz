//! A box in a table that is no table part sits in an anonymous cell of its
//! own (CSS 2.1, section 17.2.1): it takes a column, and the table is as
//! wide as all of them.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div, ul, li { display: block }";

fn create(doc: &mut BaseDocument, parent: NodeId, name: &str, style: &str) -> NodeId {
    let id = doc.mutate().create_element(
        QualName::new(None, ns!(html), LocalName::from(name)),
        Vec::new(),
    );
    let attr = QualName::new(None, ns!(), LocalName::from("style"));
    doc.mutate().set_attribute(id, attr, style);
    doc.mutate().append_children(parent, &[id]);
    id
}

fn rect(doc: &BaseDocument, node: NodeId) -> (f64, f64, f64, f64) {
    let r = doc.get_client_bounding_rect(node).unwrap();
    (r.x, r.y, r.width, r.height)
}

/// A table at the top left with a 10px square cell, a box styled
/// `middle`, and a cell 10px wide and 5px high, in one row.
fn table_around(middle: &str) -> (BaseDocument, [NodeId; 4]) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", "");
    let body = create(&mut doc, html, "body", "margin: 0");
    let ul = create(
        &mut doc,
        body,
        "ul",
        "display: table; padding: 0; margin: 0; border-spacing: 0",
    );
    let a = create(
        &mut doc,
        ul,
        "li",
        "display: table-cell; height: 10px; width: 10px",
    );
    let b = create(&mut doc, ul, "li", middle);
    let c = create(
        &mut doc,
        ul,
        "li",
        "display: table-cell; height: 5px; width: 10px",
    );
    doc.resolve(0.0);
    (doc, [ul, a, b, c])
}

/// A box in an anonymous cell whose style changes after the table was laid out is laid out with
/// its new style.
#[test]
fn a_block_in_a_table_restyled_later_takes_its_new_style() {
    let (mut doc, [ul, a, b, c]) = table_around("height: 5px; width: 10px");
    let style = QualName::new(None, ns!(), LocalName::from("style"));
    let restyled = "height: 5px; width: 10px; border: 2px solid; margin-left: 3px";
    doc.mutate().set_attribute(b, style, restyled);
    doc.resolve(0.0);
    assert_eq!(rect(&doc, ul), (0.0, 0.0, 37.0, 10.0));
    assert_eq!(rect(&doc, a), (0.0, 0.0, 10.0, 10.0));
    assert_eq!(rect(&doc, b), (13.0, 0.0, 14.0, 9.0));
    assert_eq!(rect(&doc, c).0, 27.0);
    let fresh = table_around(restyled);
    assert_eq!(rect(&doc, b), rect(&fresh.0, fresh.1[2]));
}

#[test]
fn a_block_in_a_table_sits_in_a_cell_of_its_own() {
    let (doc, [ul, a, b, c]) = table_around("height: 5px; width: 10px");
    assert_eq!(rect(&doc, ul), (0.0, 0.0, 30.0, 10.0));
    assert_eq!(rect(&doc, a), (0.0, 0.0, 10.0, 10.0));
    assert_eq!(rect(&doc, b), (10.0, 0.0, 10.0, 5.0));
    assert_eq!(rect(&doc, c).0, 20.0);
}

#[test]
fn a_table_in_a_table_sits_in_a_cell_of_its_own() {
    let (doc, [ul, a, b, c]) = table_around("display: table; height: 10px; width: 10px");
    assert_eq!(rect(&doc, ul), (0.0, 0.0, 30.0, 10.0));
    assert_eq!(rect(&doc, a), (0.0, 0.0, 10.0, 10.0));
    assert_eq!(rect(&doc, b), (10.0, 0.0, 10.0, 10.0));
    assert_eq!(rect(&doc, c).0, 20.0);
}
