//! A text field sits in its line on the baseline of its text, as an
//! inline-block sits on its last line's (CSS 2.1, section 10.8.1).

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div { display: block } input { display: inline-block }";

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

/// A field with 16px of padding and a 1px border, in a line of the same
/// font and line height, fills the line: its text and the line share a
/// baseline, so it reaches as far above and below it as the line's strut
/// plus its padding and border. Empty, it has the baseline it would have
/// with text; with text, the same one.
#[test]
fn a_text_field_sits_on_the_baseline_of_its_text() {
    for value in [None, Some("What needs to be done?")] {
        let mut doc = BaseDocument::new(DocumentConfig {
            ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
            viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
            ..DocumentConfig::default()
        });
        let root = doc.root_node().id;
        let html = create(&mut doc, root, "html", "");
        let body = create(&mut doc, html, "body", "margin: 0");
        let font = "font: 24px sans-serif; line-height: 1.4em";
        let line = create(&mut doc, body, "div", font);
        let field = create(
            &mut doc,
            line,
            "input",
            &format!("{font}; padding: 16px; border: 1px solid; margin: 0"),
        );
        if let Some(value) = value {
            let attr = QualName::new(None, ns!(), LocalName::from("value"));
            doc.mutate().set_attribute(field, attr, value);
        }
        doc.resolve(0.0);

        let field = doc.get_client_bounding_rect(field).unwrap();
        let line = doc.get_client_bounding_rect(line).unwrap();
        assert!(
            (field.height - (24.0 * 1.4 + 34.0)).abs() < 0.1,
            "{value:?}"
        );
        assert_eq!((field.y, line.height), (0.0, field.height), "{value:?}");
    }
}
