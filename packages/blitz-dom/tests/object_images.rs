//! An `<object>` whose data is an image is a replaced element showing it, and its children are
//! not rendered. Until the image has come, and if what came is no image, it is an ordinary
//! element and its children are its fallback, as Acid2's eyes have it.

use std::sync::{Arc, Mutex};

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};
use blitz_traits::net::{Bytes, NetHandler, NetProvider, Request};

const UA_CSS: &str = "html, body, div { display: block } object { display: inline }";

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

impl Held {
    fn answer(&self, url: &str, bytes: &'static [u8]) {
        let mut held = self.0.lock().unwrap();
        let index = held.iter().position(|(asked, _)| asked == url).unwrap();
        let (url, handler) = held.remove(index);
        drop(held);
        handler.bytes(url, Bytes::from_static(bytes));
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

fn text(doc: &mut BaseDocument, parent: NodeId, data: &str) -> NodeId {
    let text = doc.mutate().create_text_node(data);
    doc.mutate().append_children(parent, &[text]);
    text
}

/// A page whose body holds a div, and what it asks for kept in `held`.
fn page(held: &Arc<Held>) -> (BaseDocument, NodeId) {
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
    let div = create(&mut doc, body, "div", &[]);
    (doc, div)
}

fn size(doc: &mut BaseDocument, node: NodeId) -> (f64, f64) {
    doc.handle_messages();
    doc.resolve(0.0);
    let rect = doc.get_client_bounding_rect(node).unwrap();
    (rect.width, rect.height)
}

#[cfg(feature = "svg")]
#[test]
fn an_object_whose_data_is_an_image_shows_it_and_not_its_children() {
    let held = Arc::new(Held::default());
    let (mut doc, div) = page(&held);
    let object = create(&mut doc, div, "object", &[("data", "a.svg")]);
    let fallback = create(&mut doc, object, "span", &[]);
    text(&mut doc, fallback, "fallback");
    let (width, _) = size(&mut doc, object);
    assert!(width > 0.0, "the fallback shows until the image comes");
    held.answer("https://example.test/a.svg", SVG);
    assert_eq!(size(&mut doc, object), (3.0, 2.0));
    assert!(
        doc.get_client_bounding_rect(fallback)
            .is_none_or(|rect| rect.width == 0.0)
    );
}

#[cfg(feature = "svg")]
#[test]
fn an_object_with_an_image_takes_its_width_and_height() {
    let held = Arc::new(Held::default());
    let (mut doc, div) = page(&held);
    let style = "width: 30px; height: 20px; padding: 0 5px; border-right: 1px solid";
    let object = create(
        &mut doc,
        div,
        "object",
        &[("data", "a.svg"), ("style", style)],
    );
    held.answer("https://example.test/a.svg", SVG);
    assert_eq!(size(&mut doc, object), (41.0, 20.0));
}

#[test]
fn an_object_whose_data_is_no_image_shows_its_children() {
    let held = Arc::new(Held::default());
    let (mut doc, div) = page(&held);
    let outer = create(
        &mut doc,
        div,
        "object",
        &[("data", "a.bin"), ("style", "width: 300px")],
    );
    let fallback = create(&mut doc, outer, "span", &[]);
    text(&mut doc, fallback, "fallback");
    let before = size(&mut doc, fallback);
    held.answer("https://example.test/a.bin", b"ERROR");
    assert_eq!(size(&mut doc, fallback), before);
    assert!(before.0 > 0.0);
    assert_ne!(
        size(&mut doc, outer).0,
        300.0,
        "an inline object takes no width"
    );
}

#[cfg(feature = "svg")]
#[test]
fn an_image_in_the_fallback_of_an_object_that_is_none_shows() {
    let held = Arc::new(Held::default());
    let (mut doc, div) = page(&held);
    let outer = create(&mut doc, div, "object", &[("data", "a.bin")]);
    let inner = create(&mut doc, outer, "object", &[("data", "b.svg")]);
    text(&mut doc, inner, "ERROR");
    held.answer("https://example.test/a.bin", b"ERROR");
    held.answer("https://example.test/b.svg", SVG);
    assert_eq!(size(&mut doc, inner), (3.0, 2.0));
}
