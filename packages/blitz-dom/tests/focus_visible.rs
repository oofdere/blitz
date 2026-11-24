//! `:focus-visible` matches an element whose state has the focus ring, which
//! focusing an element gives it, and stops matching when the ring is taken
//! from its state while it keeps the focus, as an embedder that follows a
//! browser's heuristic for when to show the ring does.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};
use style_dom::ElementState;

const UA_CSS: &str = "html, body, div { display: block } \
    div { width: 20px; height: 10px } \
    div:focus { height: 30px } \
    div:focus-visible { width: 50px }";

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
fn focus_visible_matches_the_focus_ring_state() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", &[]);
    let body = create(&mut doc, html, "body", &[("style", "margin: 0")]);
    let div = create(&mut doc, body, "div", &[("tabindex", "0")]);
    let size = |doc: &mut BaseDocument| {
        doc.resolve(0.0);
        let rect = doc.get_client_bounding_rect(div).unwrap();
        (rect.width, rect.height)
    };
    assert_eq!(size(&mut doc), (20.0, 10.0), "not focused");
    doc.set_focus_to(div);
    assert_eq!(size(&mut doc), (50.0, 30.0), "focused, with the ring");
    doc.snapshot_node_and(div, ElementState::FOCUSRING, |node| {
        node.element_data_mut()
            .unwrap()
            .element_state
            .remove(ElementState::FOCUSRING);
        node.mark_ancestors_dirty();
    });
    assert_eq!(size(&mut doc), (20.0, 30.0), "focused, without the ring");
    doc.clear_focus();
    assert_eq!(size(&mut doc), (20.0, 10.0), "not focused again");
}
