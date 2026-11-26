//! What an `input` shows of its own, the label of a button and the editor
//! of a text field, is made in layout and is none of its children, as in
//! Chrome 157: its children are the page's alone, kept through a change of
//! its `type` and not rendered, and an input the page gives no children
//! matches `:empty`, a button with a label included.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeData, NodeId, QualName,
    UserAgentStylesheet, Viewport, ns,
};

const UA_CSS: &str = "html, body, div { display: block } input { display: inline-block } \
                      input:empty { margin-left: 7px }";

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

/// Each input in its own block, so that its left edge is its margin.
fn input(doc: &mut BaseDocument, body: NodeId, attributes: &[(&str, &str)]) -> NodeId {
    let container = create(doc, body, "div", &[]);
    create(doc, container, "input", attributes)
}

fn size(doc: &BaseDocument, id: NodeId) -> (f64, f64) {
    let rect = doc.get_client_bounding_rect(id).unwrap();
    (rect.width, rect.height)
}

/// The text an input shows, as its inline formatting context lays it out.
fn shown(doc: &BaseDocument, id: NodeId) -> String {
    let element = doc.get_node(id).unwrap().element_data().unwrap();
    element
        .inline_layout_data
        .as_deref()
        .map_or(String::new(), |layout| layout.text.clone())
}

/// A text field holding a comment of the page's keeps it through a change of
/// its `type` in the same batch as an edit of the comment, which it once
/// freed, and a button keeps an element of the page's through a change to a
/// field and back. Neither is rendered: the button is the size of one with no
/// children and shows its label alone.
#[test]
fn a_change_of_type_keeps_the_pages_children_of_an_input() {
    let (mut doc, body) = document();
    let field = input(&mut doc, body, &[("type", "text")]);
    let note = doc.mutate().create_comment_node("x");
    doc.mutate().append_children(field, &[note]);
    let fresh = input(&mut doc, body, &[("type", "button"), ("value", "Go")]);
    let button = input(&mut doc, body, &[("type", "button"), ("value", "Go")]);
    let span = doc.mutate().create_element(
        QualName::new(None, ns!(html), LocalName::from("span")),
        Vec::new(),
    );
    let text = doc.mutate().create_text_node("WWWWWWWW");
    doc.mutate().append_children(span, &[text]);
    doc.mutate().append_children(button, &[span]);
    doc.resolve(0.0);
    assert_eq!(size(&doc, button), size(&doc, fresh));
    assert_eq!(shown(&doc, button), "Go");

    let kind = QualName::new(None, ns!(), LocalName::from("type"));
    {
        let mut mutator = doc.mutate();
        mutator.set_attribute(field, kind.clone(), "password");
        mutator.set_node_text(note, "y");
        mutator.set_attribute(button, kind.clone(), "text");
    }
    doc.resolve(0.0);
    doc.mutate().set_attribute(button, kind, "button");
    doc.resolve(0.0);

    let node = doc.get_node(field).unwrap();
    assert_eq!(node.children, [note]);
    assert!(matches!(
        &doc.get_node(note).unwrap().data,
        NodeData::Comment { contents } if contents == "y"
    ));
    assert_eq!(doc.get_node(button).unwrap().children, [span]);
    assert_eq!(size(&doc, button), size(&doc, fresh));
    assert_eq!(shown(&doc, button), "Go");
}

/// An input with no children of the page's matches `:empty`, whether it shows
/// a label, an editor or nothing, and one that became a button too, while one
/// holding an element of the page's does not.
#[test]
fn an_input_with_no_children_of_the_pages_matches_empty() {
    let (mut doc, body) = document();
    let empty = [
        input(&mut doc, body, &[("type", "button"), ("value", "Go")]),
        input(&mut doc, body, &[("type", "submit"), ("value", "Go")]),
        input(&mut doc, body, &[("type", "text"), ("value", "Go")]),
        input(&mut doc, body, &[("type", "checkbox")]),
    ];
    let field = input(&mut doc, body, &[("type", "text")]);
    let held = input(&mut doc, body, &[("type", "button"), ("value", "Go")]);
    let span = doc.mutate().create_element(
        QualName::new(None, ns!(html), LocalName::from("span")),
        Vec::new(),
    );
    doc.mutate().append_children(held, &[span]);
    doc.resolve(0.0);
    let kind = QualName::new(None, ns!(), LocalName::from("type"));
    let value = QualName::new(None, ns!(), LocalName::from("value"));
    doc.mutate().set_attribute(field, kind, "button");
    doc.mutate().set_attribute(field, value, "Go");
    doc.resolve(0.0);
    let left = |id| doc.get_client_bounding_rect(id).unwrap().x;
    for id in empty.into_iter().chain([field]) {
        assert!(doc.get_node(id).unwrap().children.is_empty());
        assert_eq!(left(id), 7.0, "{id}");
    }
    assert_eq!(shown(&doc, field), "Go");
    assert_eq!(left(held), 0.0);
}
