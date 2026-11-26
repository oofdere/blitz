//! An element is `:empty` when it has no children but comments and text
//! nodes with no data, as Selectors 3 and Chrome 157 have it: a text node
//! holding a space counts.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

#[test]
fn a_comment_or_empty_text_leaves_an_element_empty() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(
            "html, body, div { display: block } div { width: 20px; height: 10px } \
             div:empty { width: 50px }",
        )],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let element = |doc: &mut BaseDocument, parent: NodeId, name: &str| {
        let id = doc.mutate().create_element(
            QualName::new(None, ns!(html), LocalName::from(name)),
            Vec::new(),
        );
        doc.mutate().append_children(parent, &[id]);
        id
    };
    let html = element(&mut doc, root, "html");
    let body = element(&mut doc, html, "body");
    let cases: Vec<(NodeId, f64)> = [
        (None, 50.0),
        (Some(""), 50.0),
        (Some(" "), 20.0),
        (Some("x"), 20.0),
    ]
    .into_iter()
    .map(|(text, width)| {
        let div = element(&mut doc, body, "div");
        let child = match text {
            Some(text) => doc.mutate().create_text_node(text),
            None => doc.mutate().create_comment_node("x"),
        };
        doc.mutate().append_children(div, &[child]);
        (div, width)
    })
    .collect();
    doc.resolve(0.0);
    for (index, (div, width)) in cases.into_iter().enumerate() {
        let rect = doc.get_client_bounding_rect(div).unwrap();
        assert_eq!(rect.width, width, "case {index}");
    }
}
