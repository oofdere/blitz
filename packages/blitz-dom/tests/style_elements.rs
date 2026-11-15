//! A `<style>` element's sheet on a page edited after layout must be the one a page
//! built fresh gives it. The edits are [DocumentMutator](blitz_dom::DocumentMutator)
//! calls only, as niso's layout side makes them.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, div { display: block } style { display: none }";

/// An 800 by 600 page holding only its html element.
fn page() -> (BaseDocument, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        ..DocumentConfig::default()
    });
    doc.set_viewport(Viewport::new(800, 600, 1.0, ColorScheme::Light));
    let root = doc.root_node().id;
    let html = add(&mut doc, root, "html");
    (doc, html)
}

fn add(doc: &mut BaseDocument, parent: NodeId, name: &str) -> NodeId {
    let mut mutator = doc.mutate();
    let id = mutator.create_element(
        QualName::new(None, ns!(html), LocalName::from(name)),
        Vec::new(),
    );
    mutator.append_children(parent, &[id]);
    id
}

fn add_text(doc: &mut BaseDocument, parent: NodeId, data: &str) {
    let mut mutator = doc.mutate();
    let id = mutator.create_text_node(data);
    mutator.append_children(parent, &[id]);
}

/// The width the sheet gives a div.
fn width(doc: &BaseDocument, node: NodeId) -> f64 {
    doc.get_client_bounding_rect(node).unwrap().width
}

/// A style element holds a span and, after it, the text of its sheet, and the span
/// holds an "x". The span's text is not the style element's own text, so the sheet is
/// `div { width: 37px }`, whether the "x" was there when the page was built or came
/// after a layout.
#[test]
fn text_under_an_element_in_a_style_element_is_not_part_of_its_sheet() {
    let read = |x_after_layout: bool| {
        let (mut doc, html) = page();
        let style = add(&mut doc, html, "style");
        let inside = add(&mut doc, style, "span");
        if !x_after_layout {
            add_text(&mut doc, inside, "x");
        }
        add_text(&mut doc, style, "div { width: 37px }");
        let div = add(&mut doc, html, "div");
        doc.resolve(0.0);
        if x_after_layout {
            add_text(&mut doc, inside, "x");
            doc.resolve(0.0);
        }
        width(&doc, div)
    };
    assert_eq!(
        (read(true), read(false)),
        (37.0, 37.0),
        "kept and fresh widths"
    );
}
