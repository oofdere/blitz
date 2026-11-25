//! A canvas is a replaced element whose natural size is its width and height
//! attributes, read with HTML's rules for parsing non-negative integers, and
//! 300 by 150 for an attribute that is missing or an error. Its natural ratio
//! is that size's unless a side of it is 0. Each size is what Chrome 157 lays
//! the same canvas out at.

use blitz_dom::{
    BaseDocument, ColorScheme, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet,
    Viewport, ns,
};

const UA_CSS: &str = "html, body, div { display: block }";

/// A canvas's attributes, and the size Chrome 157 lays it out at.
type Case = (&'static [(&'static str, &'static str)], (f64, f64));

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
fn a_canvas_is_sized_by_its_width_and_height_attributes() {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, root, "html", &[]);
    let body = create(&mut doc, html, "body", &[("style", "margin: 0")]);
    let cases: &[Case] = &[
        (&[], (300.0, 150.0)),
        (&[("width", "100"), ("height", "50")], (100.0, 50.0)),
        (&[("width", "100")], (100.0, 150.0)),
        (&[("height", "50")], (300.0, 50.0)),
        (&[("width", "10.5"), ("height", "20.9")], (10.0, 20.0)),
        (&[("width", " 40"), ("height", "\t30")], (40.0, 30.0)),
        (&[("width", "\n7"), ("height", "\x0C8")], (7.0, 8.0)),
        (&[("width", "50%"), ("height", "25%")], (50.0, 25.0)),
        (&[("width", "50px"), ("height", "20px")], (50.0, 20.0)),
        (&[("width", "+40"), ("height", "+30")], (40.0, 30.0)),
        (&[("width", "1e2"), ("height", "2e1")], (1.0, 2.0)),
        (&[("width", "0x10"), ("height", "010")], (0.0, 10.0)),
        (&[("width", "-0"), ("height", "-0")], (0.0, 0.0)),
        (&[("width", "-5"), ("height", "-1")], (300.0, 150.0)),
        (&[("width", "abc"), ("height", "")], (300.0, 150.0)),
        (&[("width", "  "), ("height", "+-4")], (300.0, 150.0)),
        (
            &[("width", "\u{a0}50"), ("height", "\x0B9")],
            (300.0, 150.0),
        ),
        (&[("width", "99999999999"), ("height", "50")], (300.0, 50.0)),
        (&[("width", "2147483648"), ("height", "3")], (300.0, 3.0)),
        (&[("width", "0"), ("height", "0")], (0.0, 0.0)),
        (
            &[
                ("width", "100"),
                ("height", "50"),
                ("style", "width: 200px"),
            ],
            (200.0, 100.0),
        ),
        (&[("style", "height: 30px")], (60.0, 30.0)),
        (
            &[("width", "0"), ("height", "0"), ("style", "width: 50px")],
            (50.0, 0.0),
        ),
        (
            &[("width", "0"), ("height", "50"), ("style", "width: 50px")],
            (50.0, 50.0),
        ),
        (
            &[
                ("width", "400"),
                ("height", "200"),
                ("style", "max-width: 120px"),
            ],
            (120.0, 60.0),
        ),
    ];
    let ids: Vec<NodeId> = cases
        .iter()
        .map(|(attributes, _)| {
            let container = create(&mut doc, body, "div", &[("style", "width: 600px")]);
            create(&mut doc, container, "canvas", attributes)
        })
        .collect();
    doc.resolve(0.0);
    for (id, (attributes, size)) in ids.into_iter().zip(cases) {
        let rect = doc.get_client_bounding_rect(id).unwrap();
        assert_eq!((rect.width, rect.height), *size, "{attributes:?}");
    }
}
