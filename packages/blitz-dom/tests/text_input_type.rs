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

/// An input shown as a text field whose `type` changes to a state not edited
/// as text drops its editor and is built as what it is now: a checkbox for
/// `CHECKBOX`, in any case, and no text field for `date`, `button` or `range`.
/// Changed back to `text`, it is a text field again.
#[test]
fn an_input_whose_type_leaves_the_text_states_drops_its_editor() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", &[]);
    let body = create(&mut doc, html, "body", &[("style", "margin: 0")]);
    let kinds = ["CHECKBOX", "Radio", "date", "button", "range"];
    let inputs: Vec<NodeId> = kinds
        .iter()
        .map(|_| create(&mut doc, body, "input", &[("type", "text")]))
        .collect();
    doc.resolve(0.0);
    let editor = |doc: &BaseDocument, id: NodeId| {
        let element = doc.get_node(id).unwrap().element_data().unwrap();
        element.text_input_data().is_some()
    };
    assert!(inputs.iter().all(|id| editor(&doc, *id)));
    let kind = QualName::new(None, ns!(), LocalName::from("type"));
    for (id, value) in inputs.iter().zip(kinds) {
        doc.mutate().set_attribute(*id, kind.clone(), value);
    }
    doc.resolve(0.0);
    for (id, value) in inputs.iter().zip(kinds) {
        assert!(!editor(&doc, *id), "{value}");
        let element = doc.get_node(*id).unwrap().element_data().unwrap();
        let toggle = value.eq_ignore_ascii_case("checkbox") || value.eq_ignore_ascii_case("radio");
        assert_eq!(
            element.checkbox_input_checked().is_some(),
            toggle,
            "{value}"
        );
    }
    for id in &inputs {
        doc.mutate().set_attribute(*id, kind.clone(), "text");
    }
    doc.resolve(0.0);
    assert!(inputs.iter().all(|id| editor(&doc, *id)));
}

/// What was typed in a text field comes back when its `type` goes to another
/// state and back to `text`, as in Chrome 157. A checkbox, a radio button,
/// a button or a hidden input writes it into its `value` attribute, as
/// HTML's steps for leaving the value mode have it, and one in the value
/// mode, as a date or a range, keeps it. An empty value is not written, so a
/// field emptied over `def` comes back as `def`, and with nothing typed a
/// checkbox writes nothing and the field comes back empty.
#[test]
fn what_was_typed_comes_back_through_a_change_of_type() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", &[]);
    let body = create(&mut doc, html, "body", &[("style", "margin: 0")]);
    // To, typed, the attribute, the attribute in the other type, and the
    // value back in text.
    let rows = [
        ("CHECKBOX", Some("abc"), Some("def"), Some("abc"), "abc"),
        ("Radio", Some("abc"), Some("def"), Some("abc"), "abc"),
        ("CHECKBOX", Some("abc"), None, Some("abc"), "abc"),
        ("button", Some("abc"), Some("def"), Some("abc"), "abc"),
        ("SUBMIT", Some("abc"), None, Some("abc"), "abc"),
        ("hidden", Some("abc"), Some("def"), Some("abc"), "abc"),
        ("CHECKBOX", Some(""), Some("def"), Some("def"), "def"),
        ("button", Some(""), Some("def"), Some("def"), "def"),
        ("date", Some("2024-01-02"), None, None, "2024-01-02"),
        ("range", Some("37"), None, None, "37"),
        ("number", Some("12"), None, None, "12"),
        ("checkbox", None, None, None, ""),
    ];
    let inputs: Vec<NodeId> = rows
        .iter()
        .map(|(_, _, attribute, ..)| match attribute {
            Some(value) => create(
                &mut doc,
                body,
                "input",
                &[("type", "text"), ("value", value)],
            ),
            None => create(&mut doc, body, "input", &[("type", "text")]),
        })
        .collect();
    doc.resolve(0.0);
    for (id, (_, typed, ..)) in inputs.iter().zip(rows) {
        if let Some(typed) = typed {
            let element = doc.get_node_mut(*id).unwrap().element_data_mut().unwrap();
            element
                .text_input_data_mut()
                .unwrap()
                .editor
                .set_text(typed);
        }
    }
    let kind = QualName::new(None, ns!(), LocalName::from("type"));
    for (id, (to, ..)) in inputs.iter().zip(rows) {
        doc.mutate().set_attribute(*id, kind.clone(), to);
    }
    doc.resolve(0.0);
    for (id, (to, _, _, written, _)) in inputs.iter().zip(rows) {
        let element = doc.get_node(*id).unwrap().element_data().unwrap();
        let value = element.attr(LocalName::from("value"));
        assert_eq!(value, written, "the value attribute of {to}");
    }
    for id in &inputs {
        doc.mutate().set_attribute(*id, kind.clone(), "text");
    }
    doc.resolve(0.0);
    for (id, (to, _, _, _, back)) in inputs.iter().zip(rows) {
        let element = doc.get_node(*id).unwrap().element_data().unwrap();
        let text = element
            .text_input_data()
            .unwrap()
            .editor
            .raw_text()
            .to_string();
        assert_eq!(text, back, "back from {to}");
    }
}

/// A `value` attribute set or removed while the input is a date or a range
/// is what the field shows back in `text`, as in Chrome 157 when nothing was
/// typed: the value kept from the editor gives way to it, as the editor's own
/// text does to a change of the attribute.
#[test]
fn a_value_set_while_the_type_is_another_comes_back() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", &[]);
    let body = create(&mut doc, html, "body", &[("style", "margin: 0")]);
    // To, the attribute, the attribute set in the other type, or removed, and
    // the value back in text.
    let rows = [
        ("date", "2024-01-01", Some("2024-02-02"), "2024-02-02"),
        ("date", "2024-01-01", None, ""),
        ("range", "10", Some("20"), "20"),
    ];
    let inputs: Vec<NodeId> = rows
        .iter()
        .map(|(_, value, ..)| {
            create(
                &mut doc,
                body,
                "input",
                &[("type", "text"), ("value", value)],
            )
        })
        .collect();
    doc.resolve(0.0);
    let kind = QualName::new(None, ns!(), LocalName::from("type"));
    let value = QualName::new(None, ns!(), LocalName::from("value"));
    for (id, (to, ..)) in inputs.iter().zip(rows) {
        doc.mutate().set_attribute(*id, kind.clone(), to);
    }
    doc.resolve(0.0);
    for (id, (_, _, set, _)) in inputs.iter().zip(rows) {
        match set {
            Some(set) => doc.mutate().set_attribute(*id, value.clone(), set),
            None => doc.mutate().clear_attribute(*id, value.clone()),
        }
    }
    doc.resolve(0.0);
    for id in &inputs {
        doc.mutate().set_attribute(*id, kind.clone(), "text");
    }
    doc.resolve(0.0);
    for (id, (to, _, set, back)) in inputs.iter().zip(rows) {
        let element = doc.get_node(*id).unwrap().element_data().unwrap();
        let text = element
            .text_input_data()
            .unwrap()
            .editor
            .raw_text()
            .to_string();
        assert_eq!(text, back, "{to} with {set:?}");
    }
}

/// An input whose `type` is a button state in any case shows its `value` as
/// a text child and is laid out as a lowercase `submit` is.
#[test]
fn a_button_input_is_one_in_any_case() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", &[]);
    let body = create(&mut doc, html, "body", &[("style", "margin: 0")]);
    let kinds = ["submit", "SUBMIT", "Button", "RESET"];
    let inputs: Vec<NodeId> = kinds
        .iter()
        .map(|kind| {
            let container = create(&mut doc, body, "div", &[]);
            create(
                &mut doc,
                container,
                "input",
                &[("type", kind), ("value", "Go")],
            )
        })
        .collect();
    doc.resolve(0.0);
    let submit = doc.get_client_bounding_rect(inputs[0]).unwrap();
    for (id, kind) in inputs.iter().zip(kinds) {
        let node = doc.get_node(*id).unwrap();
        assert_eq!(node.children.len(), 1, "{kind}");
        assert_eq!(node.text_content(), "Go", "{kind}");
        let rect = doc.get_client_bounding_rect(*id).unwrap();
        assert_eq!(
            (rect.width, rect.height),
            (submit.width, submit.height),
            "{kind}"
        );
    }
}
