//! An element is `:empty` when it has no children but comments and text
//! nodes with no data, as Selectors 3 and Chrome 157 have it: a text node
//! holding a space counts. A change of that restyles what is matched through
//! it, its later siblings included.

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

/// A div whose span is moved out becomes `:empty`, and the div after it,
/// matched through `div:empty + p`, takes that style, and gives it up when a
/// text node is put back in.
#[test]
fn a_change_of_emptiness_restyles_later_siblings() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(
            "html, body, div, p { display: block } p { width: 20px; height: 10px } \
             div:empty + p { width: 50px }",
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
    let div = element(&mut doc, body, "div");
    let span = element(&mut doc, div, "span");
    let next = element(&mut doc, body, "p");
    doc.resolve(0.0);
    let width = |doc: &BaseDocument| doc.get_client_bounding_rect(next).unwrap().width;
    assert_eq!(width(&doc), 20.0);
    doc.mutate().append_children(body, &[span]);
    doc.resolve(0.0);
    assert_eq!(width(&doc), 50.0);
    let text = doc.mutate().create_text_node("x");
    doc.mutate().append_children(div, &[text]);
    doc.resolve(0.0);
    assert_eq!(width(&doc), 20.0);
}

/// A div holding an empty text node is `:empty`, and so is matched with the
/// div after it, as Chrome 157 measures them: 50 and 70 wide. Once the data
/// is `x`, by a change or by an append, both are 20, and once it is empty
/// again they are 50 and 70 again.
#[test]
fn a_change_of_text_between_empty_and_not_restyles_the_parent_and_later_siblings() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(
            "html, body, div { display: block } div { width: 20px; height: 10px } \
             div:empty { width: 50px } div:empty + div { width: 70px }",
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
    let first = element(&mut doc, body, "div");
    let text = doc.mutate().create_text_node("");
    doc.mutate().append_children(first, &[text]);
    let second = element(&mut doc, body, "div");
    let held = doc.mutate().create_text_node("y");
    doc.mutate().append_children(second, &[held]);
    let widths = |doc: &mut BaseDocument| {
        doc.resolve(0.0);
        let width = |id| doc.get_client_bounding_rect(id).unwrap().width;
        (width(first), width(second))
    };
    assert_eq!(widths(&mut doc), (50.0, 70.0));
    doc.mutate().set_node_text(text, "x");
    assert_eq!(widths(&mut doc), (20.0, 20.0));
    doc.mutate().set_node_text(text, "");
    assert_eq!(widths(&mut doc), (50.0, 70.0));
    doc.mutate().append_text_to_node(text, "x").unwrap();
    assert_eq!(widths(&mut doc), (20.0, 20.0));
}
