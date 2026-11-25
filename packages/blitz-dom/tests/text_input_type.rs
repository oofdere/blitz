//! An `input` is a text field, built with an editor and sized as one,
//! when HTML's `type` attribute puts it in a state edited as a line of
//! text: `text`, `password`, `email`, `number`, `search`, `tel` or `url`,
//! read without regard to ASCII case. A missing or empty `type`, or one HTML
//! has no state for, is `text`, as in Chrome 157. Any other state is not.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div { display: block } input { display: inline-block }";

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

/// Each field is as wide as a field with no `type`, 300 in a line of 600,
/// and as high as its line, 20.
#[test]
fn an_input_is_a_text_field_in_the_states_edited_as_text() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", &[]);
    let body = create(&mut doc, html, "body", &[("style", "margin: 0")]);
    let text = [
        None,
        Some("text"),
        Some("TEXT"),
        Some("Password"),
        Some("email"),
        Some("number"),
        Some("NUMBER"),
        Some("search"),
        Some("tel"),
        Some("url"),
        Some(""),
        Some("bogus"),
        Some(" text"),
    ];
    let other = [
        "checkbox",
        "RADIO",
        "hidden",
        "file",
        "submit",
        "image",
        "reset",
        "button",
        "color",
        "date",
        "datetime-local",
        "month",
        "time",
        "week",
        "range",
    ];
    let mut add = |kind: Option<&str>| {
        let container = create(&mut doc, body, "div", &[("style", "width: 600px")]);
        let style = ("style", "line-height: 20px");
        match kind {
            Some(kind) => create(&mut doc, container, "input", &[("type", kind), style]),
            None => create(&mut doc, container, "input", &[style]),
        }
    };
    let fields: Vec<NodeId> = text.iter().map(|kind| add(*kind)).collect();
    let others: Vec<NodeId> = other.iter().map(|kind| add(Some(kind))).collect();
    doc.resolve(0.0);
    for (id, kind) in fields.into_iter().zip(text) {
        let element = doc.get_node(id).unwrap().element_data().unwrap();
        assert!(element.text_input_data().is_some(), "{kind:?}");
        let rect = doc.get_client_bounding_rect(id).unwrap();
        assert_eq!((rect.width, rect.height), (300.0, 20.0), "{kind:?}");
    }
    for (id, kind) in others.into_iter().zip(other) {
        let element = doc.get_node(id).unwrap().element_data().unwrap();
        assert!(element.text_input_data().is_none(), "{kind}");
    }
}
