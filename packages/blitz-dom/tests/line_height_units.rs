//! A line's height is kept in Chrome's layout unit, 1/64 of a pixel, so ten
//! lines are as high as Chrome 157 makes them: a `line-height` length or
//! percentage rounds to the nearest 1/64, and a number rounds the font size to
//! 1/64 and then floors the product. Before, each line kept its height
//! unrounded, so ten lines of `line-height: 1.3` at 14px were 182px where
//! Chrome's are 181.875, and the difference grows with every line.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div { display: block }";

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

#[test]
fn ten_lines_are_as_high_as_chromes_layout_unit_makes_them() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", "");
    let body = create(&mut doc, html, "body", "margin: 0");
    let cases = [
        ("font-size: 14px; line-height: 18.2px", 182.03125),
        ("font-size: 14px; line-height: 1.3", 181.875),
        ("font-size: 13.3333px; line-height: 1.3", 173.125),
        ("font-size: 14.4px; line-height: 1.5", 216.09375),
        ("font-size: 14px; line-height: 130%", 182.03125),
        ("font-size: 14px; line-height: 1.3em", 182.03125),
        ("font-size: 14px; line-height: 18.24px", 182.34375),
    ];
    let mut blocks = Vec::new();
    for (style, chrome) in cases {
        let block = create(&mut doc, body, "div", style);
        for _ in 0..10 {
            let text = doc.mutate().create_text_node("x");
            doc.mutate().append_children(block, &[text]);
            create(&mut doc, block, "br", "");
        }
        blocks.push((style, block, chrome));
    }
    doc.resolve(0.0);
    for (style, block, chrome) in blocks {
        let height = doc.get_client_bounding_rect(block).unwrap().height;
        assert_eq!(height, chrome, "ten lines of {style}");
    }
}
