//! A text field whose `line-height` is `normal` gives a line of its text the
//! height its font's metrics give, as a line of any other text gets: an
//! `input`'s content box is one such line, and a `textarea`'s one for each
//! row, as Chrome sizes them. Before, they were 1.2 times the font size
//! whatever the font, 24px at 20px where Chrome 157 has 20px in Ahem, 27px in
//! Noto Sans and 24px in DejaVu Sans.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div, input, textarea { display: block } \
    input, textarea { padding: 0; border: 0; margin: 0 }";

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

#[test]
fn a_text_fields_normal_line_is_as_high_as_its_fonts_metrics_give() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", &[]);
    let body = create(&mut doc, html, "body", &[("style", "margin: 0")]);
    let mut cases = Vec::new();
    for size in [20, 13] {
        let font = format!("font: {size}px Bullet");
        let line = create(&mut doc, body, "div", &[("style", &font)]);
        let text = doc.mutate().create_text_node("\u{2022}");
        doc.mutate().append_children(line, &[text]);
        let fields = [
            (
                create(
                    &mut doc,
                    body,
                    "input",
                    &[("style", &font), ("value", "\u{2022}")],
                ),
                1.0,
            ),
            (create(&mut doc, body, "input", &[("style", &font)]), 1.0),
            (
                create(
                    &mut doc,
                    body,
                    "input",
                    &[("style", &font), ("value", "\u{644}")],
                ),
                1.0,
            ),
            (
                create(
                    &mut doc,
                    body,
                    "textarea",
                    &[("style", &font), ("rows", "1")],
                ),
                1.0,
            ),
            (
                create(
                    &mut doc,
                    body,
                    "textarea",
                    &[("style", &font), ("rows", "3")],
                ),
                3.0,
            ),
        ];
        cases.push((size, line, fields));
    }
    doc.resolve(0.0);
    let height = |id: NodeId| doc.get_client_bounding_rect(id).unwrap().height;
    for (size, line, fields) in cases {
        let line = height(line);
        assert_ne!(line, f64::from(size) * 1.2, "the font's line at {size}px");
        for (field, rows) in fields {
            assert_eq!(
                height(field),
                line * rows,
                "a field at {size}px, {rows} rows"
            );
        }
    }
}

/// A text field beside text in a line sits on its text's baseline, as an
/// inline block of the same text and font does and as Chrome 157 aligns it,
/// so the line is as high as either. Before, measuring the field's line left
/// its editor to lay out again, so the field gave no baseline, the line put
/// its bottom edge there, and the text beside it hung below.
#[test]
fn a_text_field_beside_text_sits_on_its_baseline() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", &[]);
    let body = create(&mut doc, html, "body", &[("style", "margin: 0")]);
    let style = (
        "style",
        "display: inline-block; width: 100px; font: 40px Bullet",
    );
    let mut lines = Vec::new();
    for (name, attributes) in [
        ("input", vec![style, ("value", "\u{2022}")]),
        ("div", vec![style]),
    ] {
        let line = create(&mut doc, body, "div", &[("style", "font: 20px Bullet")]);
        let span = create(&mut doc, line, "span", &[]);
        let text = doc.mutate().create_text_node("\u{2022}");
        doc.mutate().append_children(span, &[text]);
        let beside = create(&mut doc, line, name, &attributes);
        if name == "div" {
            let text = doc.mutate().create_text_node("\u{2022}");
            doc.mutate().append_children(beside, &[text]);
        }
        lines.push(line);
    }
    doc.resolve(0.0);
    let height = |id: NodeId| doc.get_client_bounding_rect(id).unwrap().height;
    assert_eq!(height(lines[0]), height(lines[1]));
}
