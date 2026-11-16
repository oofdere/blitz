//! An `@import` whose URL resolves against nothing is left out of its sheet, as if it were
//! refused, and asks for nothing, and the rest of the sheet still applies: on a page with no base
//! URL yet, and on a page whose own URL is a `data:` URL, which nothing resolves against.

use std::sync::{Arc, Mutex};

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};
use blitz_traits::net::{NetHandler, NetProvider, Request};

const UA_CSS: &str = "html, body, div { display: block } head, style { display: none }";

/// Keeps the URL of every request.
#[derive(Default)]
struct Asked(Mutex<Vec<String>>);

impl NetProvider for Asked {
    fn fetch(&self, _doc_id: usize, request: Request, _handler: Box<dyn NetHandler>) {
        self.0.lock().unwrap().push(request.url.to_string());
    }
}

fn create(doc: &mut BaseDocument, parent: NodeId, name: &str, style: &str) -> NodeId {
    let id = doc.mutate().create_element(
        QualName::new(None, ns!(html), LocalName::from(name)),
        Vec::new(),
    );
    if !style.is_empty() {
        let attr = QualName::new(None, ns!(), LocalName::from("style"));
        doc.mutate().set_attribute(id, attr, style);
    }
    doc.mutate().append_children(parent, &[id]);
    id
}

/// The width of a div on a page with this base URL and a `<style>` that imports `x.css` and
/// makes divs 10px wide, and what the page asked for.
fn imported(base_url: Option<&str>) -> (f64, Vec<String>) {
    let asked = Arc::new(Asked::default());
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        base_url: base_url.map(Into::into),
        net_provider: Some(asked.clone()),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", "");
    let head = create(&mut doc, html, "head", "");
    let body = create(&mut doc, html, "body", "margin: 0");
    let div = create(&mut doc, body, "div", "height: 10px");
    let style = create(&mut doc, head, "style", "");
    let sheet = doc
        .mutate()
        .create_text_node("@import 'x.css'; div { width: 10px }");
    doc.mutate().append_children(style, &[sheet]);
    doc.resolve(0.0);
    let width = doc.get_client_bounding_rect(div).unwrap().width;
    let asked = asked.0.lock().unwrap().clone();
    (width, asked)
}

#[test]
fn an_import_on_a_page_with_no_base_url_yet_is_left_out() {
    assert_eq!(imported(None), (10.0, Vec::new()));
}

#[test]
fn an_import_on_a_data_url_page_is_left_out() {
    let page = "data:text/html,<p>hello</p>";
    assert_eq!(imported(Some(page)), (10.0, Vec::new()));
}

#[test]
fn an_import_that_resolves_is_asked_for() {
    let (width, asked) = imported(Some("https://example.test/dir/page.html"));
    assert_eq!(width, 10.0);
    assert_eq!(asked, ["https://example.test/dir/x.css"]);
}
