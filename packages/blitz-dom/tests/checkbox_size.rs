//! A checkbox or a radio button with its native appearance is 13px along a
//! width or a height that is auto, with no padding and no border, as Chrome's
//! theme makes one: an auto width does not stretch it to a block container,
//! as a replaced element's does not, while a flex or grid container still
//! stretches it. With `appearance: none` it has no size of its own, so only its
//! padding and border take room, and an auto width still does not stretch it
//! to a block container.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div { display: block } \
    input { display: inline-block; box-sizing: border-box; appearance: auto }";

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
fn a_native_checkbox_is_13px_and_an_unstyled_one_has_no_size() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", &[]);
    let body = create(&mut doc, html, "body", &[("style", "margin: 0")]);
    let cases = [
        ("", "checkbox", "", (13.0, 13.0)),
        ("", "RADIO", "", (13.0, 13.0)),
        ("", "checkbox", "width: 20px", (20.0, 13.0)),
        ("", "radio", "height: 30px", (13.0, 30.0)),
        ("", "checkbox", "min-width: 30px", (30.0, 13.0)),
        ("", "radio", "padding: 2px; border: 1px solid", (13.0, 13.0)),
        ("", "checkbox", "padding: 10px", (13.0, 13.0)),
        ("", "checkbox", "display: block", (13.0, 13.0)),
        ("", "radio", "display: block; padding: 10px", (13.0, 13.0)),
        (
            "display: flex; flex-direction: column; width: 200px",
            "checkbox",
            "",
            (200.0, 13.0),
        ),
        (
            "display: grid; width: 200px; height: 100px",
            "radio",
            "",
            (200.0, 100.0),
        ),
        ("", "checkbox", "appearance: none", (0.0, 0.0)),
        ("", "radio", "-webkit-appearance: none", (0.0, 0.0)),
        (
            "",
            "checkbox",
            "appearance: none; display: block",
            (0.0, 0.0),
        ),
        (
            "display: flex; flex-direction: column; width: 200px",
            "radio",
            "appearance: none",
            (200.0, 0.0),
        ),
        ("", "checkbox", "appearance: none; padding: 2px", (4.0, 4.0)),
        ("", "checkbox", "appearance: none; width: 10px", (10.0, 0.0)),
    ];
    let ids: Vec<NodeId> = cases
        .iter()
        .map(|(outer, kind, style, _)| {
            let container = create(&mut doc, body, "div", &[("style", outer)]);
            create(
                &mut doc,
                container,
                "input",
                &[("type", kind), ("style", style)],
            )
        })
        .collect();
    doc.resolve(0.0);
    for (id, (outer, kind, style, size)) in ids.into_iter().zip(cases) {
        let rect = doc.get_client_bounding_rect(id).unwrap();
        assert_eq!(
            (rect.width, rect.height),
            size,
            "{kind} {style:?} in {outer:?}"
        );
    }
}
