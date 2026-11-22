//! A checkbox or a radio button whose width or height is auto is 13px that
//! way, its border box, while its `appearance` is not `none`, as Chrome's
//! theme makes one. With `appearance: none` it has no size of its own, so
//! only its padding and border take room.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body { display: block } input { display: inline-block; box-sizing: border-box; appearance: auto }";

fn create(doc: &mut BaseDocument, parent: NodeId, kind: &str, style: &str) -> NodeId {
    let id = doc.mutate().create_element(
        QualName::new(None, ns!(html), LocalName::from("input")),
        Vec::new(),
    );
    for (name, value) in [("type", kind), ("style", style)] {
        let attr = QualName::new(None, ns!(), LocalName::from(name));
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
    let html = doc.mutate().create_element(
        QualName::new(None, ns!(html), LocalName::from("html")),
        Vec::new(),
    );
    doc.mutate().append_children(root, &[html]);
    let cases = [
        ("checkbox", "", (13.0, 13.0)),
        ("RADIO", "", (13.0, 13.0)),
        ("checkbox", "width: 20px", (20.0, 13.0)),
        ("radio", "padding: 2px; border: 1px solid", (13.0, 13.0)),
        ("checkbox", "padding: 8px", (16.0, 16.0)),
        ("checkbox", "appearance: none", (0.0, 0.0)),
        ("radio", "-webkit-appearance: none", (0.0, 0.0)),
        ("checkbox", "appearance: none; padding: 2px", (4.0, 4.0)),
        ("checkbox", "appearance: none; width: 10px", (10.0, 0.0)),
    ];
    let ids: Vec<NodeId> = cases
        .iter()
        .map(|(kind, style, _)| create(&mut doc, html, kind, style))
        .collect();
    doc.resolve(0.0);
    for (id, (kind, style, size)) in ids.into_iter().zip(cases) {
        let rect = doc.get_client_bounding_rect(id).unwrap();
        assert_eq!((rect.width, rect.height), size, "{kind} {style:?}");
    }
}
