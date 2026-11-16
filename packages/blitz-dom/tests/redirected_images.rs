//! An image answers the URL that was asked for, even when the answer comes from another URL that
//! the request was redirected to: the elements wait on the URL they asked for.

use std::sync::{Arc, Mutex};

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};
use blitz_traits::net::{Bytes, NetHandler, NetProvider, Request};

const UA_CSS: &str = "html, body, div { display: block } img { display: inline-block }";

/// An SVG image 3 pixels wide and 2 high, which blitz decodes with its `svg` feature.
const SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="3" height="2"></svg>"#;

/// Keeps every request, with the handler its answer goes to.
#[derive(Default)]
struct Held(Mutex<Vec<(String, Box<dyn NetHandler>)>>);

impl NetProvider for Held {
    fn fetch(&self, _doc_id: usize, request: Request, handler: Box<dyn NetHandler>) {
        self.0
            .lock()
            .unwrap()
            .push((request.url.to_string(), handler));
    }
}

fn create(doc: &mut BaseDocument, parent: NodeId, name: &str, attrs: &[(&str, &str)]) -> NodeId {
    let id = doc.mutate().create_element(
        QualName::new(None, ns!(html), LocalName::from(name)),
        Vec::new(),
    );
    for (name, value) in attrs {
        let name = QualName::new(None, ns!(), LocalName::from(*name));
        doc.mutate().set_attribute(id, name, value);
    }
    doc.mutate().append_children(parent, &[id]);
    id
}

#[cfg(feature = "svg")]
#[test]
fn a_redirected_image_reaches_the_elements_that_asked_for_it() {
    let held = Arc::new(Held::default());
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        base_url: Some("https://example.test/".into()),
        net_provider: Some(held.clone()),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", &[]);
    let body = create(&mut doc, html, "body", &[("style", "margin: 0")]);
    let img = create(&mut doc, body, "img", &[("src", "a.png")]);
    doc.resolve(0.0);
    let held = std::mem::take(&mut *held.0.lock().unwrap());
    let urls: Vec<&str> = held.iter().map(|(url, _)| url.as_str()).collect();
    assert_eq!(urls, ["https://example.test/a.png"]);
    for (_, handler) in held {
        handler.bytes(
            "https://cdn.example.test/a.png".into(),
            Bytes::from_static(SVG),
        );
    }
    doc.handle_messages();
    doc.resolve(0.0);
    let rect = doc.get_client_bounding_rect(img).unwrap();
    assert_eq!((rect.width, rect.height), (3.0, 2.0));
}
