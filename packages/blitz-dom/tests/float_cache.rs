//! Incremental layout must answer as a page built fresh does when a box that
//! sits in its parent's block formatting context, and holds a float, is not
//! laid out again. The documents are built with niso's DocumentMutator calls
//! only.

use blitz_dom::{
    BaseDocument, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet, ns,
};
use blitz_traits::shell::{ColorScheme, Viewport};

fn document() -> BaseDocument {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse("html, div { display: block }")],
        ..DocumentConfig::default()
    });
    doc.set_viewport(Viewport::new(800, 600, 1.0, ColorScheme::Light));
    doc
}

fn element(doc: &mut BaseDocument, parent: NodeId, name: &str, style: Option<&str>) -> NodeId {
    let mut mutator = doc.mutate();
    let id = mutator.create_element(
        QualName::new(None, ns!(html), LocalName::from(name)),
        Vec::new(),
    );
    if let Some(style) = style {
        mutator.set_attribute(
            id,
            QualName::new(None, ns!(), LocalName::from("style")),
            style,
        );
    }
    mutator.append_children(parent, &[id]);
    id
}

fn height(doc: &BaseDocument, id: NodeId) -> f64 {
    doc.get_client_bounding_rect(id).unwrap().height
}

/// html > div > div > float, laid out; then a div goes in before the inner
/// div. The inner div is not changed, so its layout comes from the cache, and
/// the float in it must still count towards the height of html, the root of
/// the block formatting context, as it does in a page built fresh.
#[test]
fn a_cached_box_still_places_its_floats() {
    let float = "float: left; width: 10px; height: 40px";

    let mut kept = document();
    let root = kept.root_node().id;
    let html = element(&mut kept, root, "html", None);
    let outer = element(&mut kept, html, "div", None);
    let inner = element(&mut kept, outer, "div", None);
    element(&mut kept, inner, "div", Some(float));
    kept.resolve(0.0);
    assert_eq!(height(&kept, html), 40.0);
    let added = kept.mutate().create_element(
        QualName::new(None, ns!(html), LocalName::from("div")),
        Vec::new(),
    );
    kept.mutate().insert_nodes_before(inner, &[added]);
    kept.resolve(0.0);

    let mut fresh = document();
    let root = fresh.root_node().id;
    let fresh_html = element(&mut fresh, root, "html", None);
    let outer = element(&mut fresh, fresh_html, "div", None);
    element(&mut fresh, outer, "div", None);
    let inner = element(&mut fresh, outer, "div", None);
    element(&mut fresh, inner, "div", Some(float));
    fresh.resolve(0.0);

    assert_eq!(height(&fresh, fresh_html), 40.0);
    assert_eq!(height(&kept, html), height(&fresh, fresh_html));
}
