//! A table cell is as tall as its row: its `height` is a minimum for the row,
//! and a shorter cell stretches to the row's height (CSS 2.1 section 17.5.3).

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

/// The heights of a one-row table's cells, styled `cells`, and the table's.
fn heights(cells: &[&str]) -> (Vec<f64>, f64) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "");
    let body = create(&mut doc, html, "margin: 0");
    let table = create(&mut doc, body, "display: table; border-spacing: 0");
    let row = create(&mut doc, table, "display: table-row");
    let ids: Vec<NodeId> = cells
        .iter()
        .map(|style| {
            create(
                &mut doc,
                row,
                &format!("display: table-cell; width: 10px; {style}"),
            )
        })
        .collect();
    doc.resolve(0.0);
    let height = |id| doc.get_client_bounding_rect(id).unwrap().height;
    (ids.into_iter().map(height).collect(), height(table))
}

#[test]
fn a_shorter_cell_stretches_to_its_row() {
    assert_eq!(
        heights(&["height: 10px", "height: 5px", ""]),
        (vec![10.0, 10.0, 10.0], 10.0)
    );
}

#[test]
fn the_tallest_height_of_a_row_wins() {
    assert_eq!(
        heights(&["height: 3px", "height: 5px"]),
        (vec![5.0, 5.0], 5.0)
    );
}

#[test]
fn a_cell_taller_than_its_height_keeps_its_content() {
    assert_eq!(
        heights(&["height: 5px", "height: 2px; padding-top: 8px"]),
        (vec![10.0, 10.0], 10.0)
    );
}
