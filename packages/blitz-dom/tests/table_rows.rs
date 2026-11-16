//! A table's rows and row groups have boxes to read: a row over its row
//! track, from the first column to the last, and a row group over its rows
//! (CSS 2.1, section 17.5), inside the table's border spacing as browsers
//! have them.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body { display: block } table { display: table } \
    tbody { display: table-row-group } tr { display: table-row } td { display: table-cell }";

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

/// A table at (10, 20) with a 2px border and 4px border spacing, holding a
/// row group of two rows of two cells 30px wide and 20px high.
#[test]
fn rows_and_row_groups_span_their_cells() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", "");
    let body = create(&mut doc, html, "body", "margin: 0");
    let table = create(
        &mut doc,
        body,
        "table",
        "margin: 20px 0 0 10px; border: 2px solid; border-spacing: 4px",
    );
    let group = create(&mut doc, table, "tbody", "");
    let rows = [(); 2].map(|()| {
        let row = create(&mut doc, group, "tr", "");
        for _ in 0..2 {
            create(&mut doc, row, "td", "width: 30px; height: 20px; padding: 0");
        }
        row
    });
    doc.resolve(0.0);
    assert_eq!(rect(&doc, table), (10.0, 20.0, 76.0, 56.0));
    assert_eq!(rect(&doc, rows[0]), (16.0, 26.0, 64.0, 20.0));
    assert_eq!(rect(&doc, rows[1]), (16.0, 50.0, 64.0, 20.0));
    assert_eq!(rect(&doc, group), (16.0, 26.0, 64.0, 44.0));
}

/// A row with no cells still spans the table's columns, at no height.
#[test]
fn an_empty_row_spans_the_columns_at_no_height() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", "");
    let body = create(&mut doc, html, "body", "margin: 0");
    let table = create(&mut doc, body, "table", "border-spacing: 0");
    let full = create(&mut doc, table, "tr", "");
    create(
        &mut doc,
        full,
        "td",
        "width: 30px; height: 20px; padding: 0",
    );
    let empty = create(&mut doc, table, "tr", "");
    doc.resolve(0.0);
    assert_eq!(rect(&doc, full), (0.0, 0.0, 30.0, 20.0));
    assert_eq!(rect(&doc, empty), (0.0, 20.0, 30.0, 0.0));
}
