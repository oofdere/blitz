//! A style sheet answers the `<link>` that asked for it, and only while that link still waits for
//! that answer: not once the link has left the document, nor once its `href` has asked for
//! another sheet.

use std::sync::{Arc, Mutex};

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};
use blitz_traits::net::{Bytes, NetHandler, NetProvider, Request};

const UA_CSS: &str = "html, body, div { display: block } head, link { display: none }";

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
    /// Answers the first request for `url` not answered yet with this style sheet.
    fn answer(&self, url: &str, css: &'static str) {
        let index = self
            .0
            .lock()
            .unwrap()
            .iter()
            .position(|(asked, _)| asked == url);
        self.answer_at(index.unwrap(), css);
    }

    /// Answers the last request for `url` not answered yet with this style sheet.
    fn answer_last(&self, url: &str, css: &'static str) {
        let index = self
            .0
            .lock()
            .unwrap()
            .iter()
            .rposition(|(asked, _)| asked == url);
        self.answer_at(index.unwrap(), css);
    }

    fn answer_at(&self, index: usize, css: &'static str) {
        let mut held = self.0.lock().unwrap();
        let (url, handler) = held.remove(index);
        drop(held);
        handler.bytes(url, Bytes::from_static(css.as_bytes()));
    }
}

fn attr(name: &str) -> QualName {
    QualName::new(None, ns!(), LocalName::from(name))
}

fn create(doc: &mut BaseDocument, parent: NodeId, name: &str, attrs: &[(&str, &str)]) -> NodeId {
    let id = doc.mutate().create_element(
        QualName::new(None, ns!(html), LocalName::from(name)),
        Vec::new(),
    );
    for (name, value) in attrs {
        doc.mutate().set_attribute(id, attr(name), value);
    }
    doc.mutate().append_children(parent, &[id]);
    id
}

/// A page at https://example.test/ with a head and a body, a div in the body, and what it asks
/// for kept in `held`.
fn page(held: &Arc<Held>) -> (BaseDocument, NodeId, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        base_url: Some("https://example.test/".into()),
        net_provider: Some(held.clone()),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", &[]);
    let head = create(&mut doc, html, "head", &[]);
    let body = create(&mut doc, html, "body", &[("style", "margin: 0")]);
    let div = create(&mut doc, body, "div", &[("style", "height: 10px")]);
    (doc, head, div)
}

fn width(doc: &mut BaseDocument, node: NodeId) -> f64 {
    doc.handle_messages();
    doc.resolve(0.0);
    doc.get_client_bounding_rect(node).unwrap().width
}

#[test]
fn an_answer_to_a_link_that_has_left_is_passed_over() {
    let held = Arc::new(Held::default());
    let (mut doc, head, div) = page(&held);
    let link = create(
        &mut doc,
        head,
        "link",
        &[("rel", "stylesheet"), ("href", "a.css")],
    );
    doc.mutate().remove_and_drop_node(link);
    held.answer("https://example.test/a.css", "div { width: 10px }");
    assert_eq!(width(&mut doc, div), 800.0);
    assert!(!doc.has_pending_critical_resources());
}

#[test]
fn an_answer_to_a_link_that_has_asked_for_another_sheet_is_passed_over() {
    let held = Arc::new(Held::default());
    let (mut doc, head, div) = page(&held);
    let link = create(
        &mut doc,
        head,
        "link",
        &[("rel", "stylesheet"), ("href", "a.css")],
    );
    doc.mutate().set_attribute(link, attr("href"), "b.css");
    held.answer("https://example.test/b.css", "div { width: 20px }");
    assert_eq!(width(&mut doc, div), 20.0);
    held.answer("https://example.test/a.css", "div { width: 10px }");
    assert_eq!(width(&mut doc, div), 20.0);
}

#[test]
fn an_answer_to_a_link_that_has_been_moved_is_passed_over_for_the_one_it_asks_for_again() {
    let held = Arc::new(Held::default());
    let (mut doc, head, div) = page(&held);
    let link = create(
        &mut doc,
        head,
        "link",
        &[("rel", "stylesheet"), ("href", "a.css")],
    );
    doc.mutate().remove_node(link);
    doc.mutate().append_children(head, &[link]);
    held.answer_last("https://example.test/a.css", "div { width: 30px }");
    assert_eq!(width(&mut doc, div), 30.0);
    held.answer("https://example.test/a.css", "div { width: 10px }");
    assert_eq!(width(&mut doc, div), 30.0);
}
