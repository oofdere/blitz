//! A progress bar lays out none of its children, as a browser renders it
//! from a shadow tree of its own: one with fallback text or a block in it is
//! as big as an empty one and sits in its line where an empty one does, its
//! children have no box, and they are still in the document for selectors,
//! so `:empty` matches only a bar with no children but comments.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, p, div { display: block } \
    progress { display: inline-block; box-sizing: border-box; height: 1em; width: 10em; \
    vertical-align: -0.2em } \
    progress:empty { min-width: 200px }";

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

fn text(doc: &mut BaseDocument, parent: NodeId, data: &str) {
    let id = doc.mutate().create_text_node(data);
    doc.mutate().append_children(parent, &[id]);
}

#[test]
fn a_progress_bar_lays_out_none_of_its_children() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", "");
    let body = create(&mut doc, html, "body", "margin: 0; font-size: 16px");
    let line = create(&mut doc, body, "p", "margin: 0");
    text(&mut doc, line, "Empty ");
    let empty = create(&mut doc, line, "progress", "");
    text(&mut doc, line, " with text ");
    let with_text = create(&mut doc, line, "progress", "");
    text(&mut doc, with_text, "fallback text ");
    let bold = create(&mut doc, with_text, "b", "");
    text(&mut doc, bold, "bold");
    text(&mut doc, line, " with a block ");
    let with_block = create(&mut doc, line, "progress", "");
    let block = create(&mut doc, with_block, "div", "height: 40px");
    text(&mut doc, block, "x");
    doc.resolve(0.0);
    let rect = |doc: &BaseDocument, id| doc.get_client_bounding_rect(id).unwrap();
    assert_eq!(
        (rect(&doc, empty).width, rect(&doc, empty).height),
        (200.0, 16.0)
    );
    for bar in [with_text, with_block] {
        assert_eq!(
            (rect(&doc, bar).width, rect(&doc, bar).height),
            (160.0, 16.0)
        );
        assert_eq!(rect(&doc, bar).y, rect(&doc, empty).y);
        assert!(rect(&doc, bar).x > rect(&doc, empty).x, "one line");
    }
    for child in [bold, block] {
        assert!(
            doc.get_client_bounding_rect(child)
                .is_none_or(|rect| rect.width == 0.0 && rect.height == 0.0),
            "{child}"
        );
    }
}
