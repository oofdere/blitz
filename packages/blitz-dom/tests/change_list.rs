//! Layout must never reach a freed box, and incremental layout must agree with
//! a full relayout, when a document is edited the way niso's layout side edits
//! its copy of the page.
//!
//! niso keeps the DOM on its own side and sends the layout side a list of
//! changes. Each change is one of these calls on [DocumentMutator], and nothing
//! else: a node comes into the document (create_element or create_text_node,
//! then append_children or insert_nodes_before), a node in the document moves
//! (append_children or insert_nodes_before on a connected node), a node leaves
//! the document with everything under it (remove_and_drop_node_with), an
//! attribute is set or removed, or a text node's data is set. The tests below
//! use those calls only.

use std::cell::RefCell;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::panic::{AssertUnwindSafe, catch_unwind};

use blitz_dom::{
    BaseDocument, DocumentConfig, LocalName, NodeId, QualName, UserAgentStylesheet, ns,
};

fn attr(name: &str) -> QualName {
    QualName::new(None, ns!(), LocalName::from(name))
}

fn create(doc: &mut BaseDocument, name: &str) -> NodeId {
    let name = QualName::new(None, ns!(html), LocalName::from(name));
    doc.mutate().create_element(name, Vec::new())
}

fn append(doc: &mut BaseDocument, parent: NodeId, node: NodeId) {
    doc.mutate().append_children(parent, &[node]);
}

fn set_class(doc: &mut BaseDocument, node: NodeId, value: &str) {
    doc.mutate().set_attribute(node, attr("class"), value);
}

fn remove(doc: &mut BaseDocument, node: NodeId) {
    doc.mutate().remove_and_drop_node_with(node, &mut |_| {});
}

/// The user-agent sheet the tests give their documents, so that they do not
/// depend on the one blitz-dom carries behind its `default-ua-stylesheet` feature.
const UA_CSS: &str = "html, body, div, p { display: block } head, style { display: none } body { margin: 8px } p { margin: 1em 0 } table { display: table } tr { display: table-row }";

/// A document with a style sheet and an empty body, laid out once.
fn page(css: &str) -> (BaseDocument, NodeId) {
    let mut doc = BaseDocument::new(DocumentConfig {
        ua_stylesheets: vec![UserAgentStylesheet::parse(UA_CSS)],
        ..DocumentConfig::default()
    });
    let root = doc.root_node().id;
    let html = create(&mut doc, "html");
    append(&mut doc, root, html);
    let head = create(&mut doc, "head");
    append(&mut doc, html, head);
    let style = create(&mut doc, "style");
    let text = doc.mutate().create_text_node(css);
    append(&mut doc, style, text);
    append(&mut doc, head, style);
    let body = create(&mut doc, "body");
    append(&mut doc, html, body);
    doc.resolve(0.0);
    (doc, body)
}

/// A div with these classes and a text node, put last in parent.
fn div(doc: &mut BaseDocument, parent: NodeId, classes: &str, text: &str) -> NodeId {
    let id = create(doc, "div");
    set_class(doc, id, classes);
    let text = doc.mutate().create_text_node(text);
    append(doc, id, text);
    append(doc, parent, id);
    id
}

const HOISTING: &str = ".cb { position: relative } .cb.before::before { content: \"\"; position: absolute } .abs { position: absolute } .hidden { display: none } .ib { display: inline-block } .table { display: table }";

/// outer > cb (with an absolutely positioned ::before) > (box, absolutely positioned; flow, a paragraph).
fn hoisting() -> (BaseDocument, [NodeId; 5]) {
    let (mut doc, body) = page(HOISTING);
    let outer = div(&mut doc, body, "", "");
    let cb = div(&mut doc, outer, "cb before", "");
    let b = div(&mut doc, cb, "abs", "x");
    let flow = create(&mut doc, "p");
    let y = doc.mutate().create_text_node("y");
    append(&mut doc, flow, y);
    append(&mut doc, cb, flow);
    doc.resolve(0.0);
    (doc, [body, outer, cb, b, flow])
}

#[test]
fn a_hidden_containing_block_lets_go_of_its_hoisted_boxes() {
    let (mut doc, [_, _, cb, b, _]) = hoisting();
    assert_eq!(doc.get_node(cb).unwrap().hoisted_children.borrow().len(), 2);
    set_class(&mut doc, cb, "cb before hidden");
    doc.resolve(0.0);
    assert!(
        doc.get_node(cb)
            .unwrap()
            .hoisted_children
            .borrow()
            .is_empty()
    );
    assert_eq!(doc.get_node(b).unwrap().oof_containing_block.get(), None);
}

#[test]
fn a_pseudo_element_dropped_while_its_containing_block_is_hidden() {
    let (mut doc, [_, _, cb, _, _]) = hoisting();
    set_class(&mut doc, cb, "cb before hidden");
    doc.resolve(0.0);
    set_class(&mut doc, cb, "cb hidden");
    doc.resolve(0.0);
}

#[test]
fn a_box_removed_while_its_containing_block_is_hidden() {
    let (mut doc, [_, _, cb, b, _]) = hoisting();
    set_class(&mut doc, cb, "cb before hidden");
    doc.resolve(0.0);
    remove(&mut doc, b);
    doc.resolve(0.0);
}

#[test]
fn a_box_removed_inside_a_hidden_ancestor() {
    let (mut doc, [_, outer, _, b, _]) = hoisting();
    set_class(&mut doc, outer, "hidden");
    doc.resolve(0.0);
    remove(&mut doc, b);
    doc.resolve(0.0);
}

#[test]
fn an_in_flow_child_removed_inside_a_hidden_ancestor() {
    let (mut doc, [_, outer, _, _, flow]) = hoisting();
    set_class(&mut doc, outer, "hidden");
    doc.resolve(0.0);
    remove(&mut doc, flow);
    doc.resolve(0.0);
}

#[test]
fn a_containing_block_removed_after_its_box_moved_out() {
    let (mut doc, [body, _, cb, b, _]) = hoisting();
    append(&mut doc, body, b);
    remove(&mut doc, cb);
    assert!(
        doc.get_node(b)
            .unwrap()
            .containing_block()
            .is_none_or(|id| doc.get_node(id).is_some())
    );
    doc.resolve(0.0);
}

const FUZZ_CSS: &str = ".db{display:block}.di{display:inline}.dt{display:table}.dib{display:inline-block}.pa{position:absolute}.pk{position:sticky}.bf::before{content:\"b\";position:absolute}.af::after{content:\"a\";position:fixed}.bs::before{content:\"s\"}.tr{transform:translateX(1px)}.z{z-index:1}.ov{overflow:auto}.dn{display:none}.pr{position:relative}.pf{position:fixed}.fl{display:flex}";

const FUZZ_CLASSES: [&str; 16] = [
    "db", "di", "dt", "dib", "dn", "pa", "pr", "pf", "pk", "bf", "af", "bs", "tr", "z", "ov", "fl",
];

thread_local! {
    static LAST_PANIC: RefCell<String> = const { RefCell::new(String::new()) };
}

fn is_inclusive_ancestor(doc: &BaseDocument, ancestor: NodeId, mut node: NodeId) -> bool {
    loop {
        if node == ancestor {
            return true;
        }
        match doc.get_node(node).and_then(|n| n.parent) {
            Some(parent) => node = parent,
            None => return false,
        }
    }
}

/// The layout of every box that is shown: in the document, with no
/// display: none inclusive ancestor.
fn shown_digest(doc: &BaseDocument, nodes: &[NodeId]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for &id in nodes {
        let node = doc.get_node(id).unwrap();
        let mut up = Some(id);
        let mut shown = node.flags.is_in_document();
        while let Some(a) = up.filter(|_| shown) {
            let a = doc.get_node(a).unwrap();
            shown = !a
                .primary_styles()
                .is_some_and(|s| s.get_box().display.is_none());
            up = a.parent;
        }
        shown.hash(&mut hasher);
        if shown {
            let l = node.final_layout();
            for v in [l.location.x, l.location.y, l.size.width, l.size.height] {
                v.to_bits().hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}

enum Seed {
    Clean,
    /// Layout of the edited tree differs from a full relayout of it.
    Stale,
    Panicked(String),
}

/// One seed: ops random edits of the kinds niso makes, with layouts between
/// some of them, then a comparison with a full relayout.
fn fuzz_seed(seed: u64, ops: u32) -> Seed {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut next = |n: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % n as u64) as usize
    };
    let (mut doc, body) = page(FUZZ_CSS);
    let mut nodes: Vec<NodeId> = Vec::new();
    let panicked = || Seed::Panicked(LAST_PANIC.with(|p| p.borrow().clone()));
    for _ in 0..ops {
        let classes = FUZZ_CLASSES
            .iter()
            .filter(|_| next(3) == 0)
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        let pick = |i: usize| nodes.get(i % (nodes.len() + 1)).copied().unwrap_or(body);
        let (a, b) = (pick(next(1 << 16)), pick(next(1 << 16)));
        let place = next(3);
        let op = next(10);
        let then_layout = next(4) == 0;
        let step = catch_unwind(AssertUnwindSafe(|| {
            let before = |doc: &BaseDocument, parent: NodeId| {
                let children = &doc.get_node(parent).unwrap().children;
                (place > 0 && !children.is_empty()).then(|| children[place % children.len()])
            };
            match op {
                0 | 1 => {
                    let id = create(&mut doc, "div");
                    set_class(&mut doc, id, &classes);
                    let text = doc.mutate().create_text_node("t");
                    append(&mut doc, id, text);
                    match before(&doc, a) {
                        Some(anchor) => doc.mutate().insert_nodes_before(anchor, &[id]),
                        None => append(&mut doc, a, id),
                    }
                    Some(id)
                }
                2 | 3 if a != body => {
                    set_class(&mut doc, a, &classes);
                    None
                }
                4 | 5 if a != body && !is_inclusive_ancestor(&doc, a, b) => {
                    match before(&doc, b).filter(|&anchor| anchor != a) {
                        Some(anchor) => doc.mutate().insert_nodes_before(anchor, &[a]),
                        None => append(&mut doc, b, a),
                    }
                    None
                }
                6 | 7 if a != body => {
                    remove(&mut doc, a);
                    None
                }
                8 => {
                    let text = doc
                        .get_node(a)
                        .unwrap()
                        .children
                        .iter()
                        .copied()
                        .find(|&c| doc.get_node(c).unwrap().is_text_node());
                    if let Some(text) = text {
                        let data = "t".repeat(1 + place);
                        doc.mutate().set_node_text(text, &data);
                    }
                    None
                }
                _ => None,
            }
        }));
        match step {
            Ok(Some(created)) => nodes.push(created),
            Ok(None) => {}
            Err(_) => return panicked(),
        }
        nodes.retain(|&id| doc.get_node(id).is_some());
        if (op == 9 || then_layout) && catch_unwind(AssertUnwindSafe(|| doc.resolve(0.0))).is_err()
        {
            return panicked();
        }
    }
    if catch_unwind(AssertUnwindSafe(|| doc.resolve(0.0))).is_err() {
        return panicked();
    }
    let digest = shown_digest(&doc, &nodes);
    doc.set_incremental_layout(false);
    if catch_unwind(AssertUnwindSafe(|| doc.resolve(0.0))).is_err() {
        return panicked();
    }
    if shown_digest(&doc, &nodes) == digest {
        Seed::Clean
    } else {
        Seed::Stale
    }
}

/// Random edits of the kinds niso makes, laid out between some of them. Counts
/// the seeds that panic and those whose incremental layout differs from a full
/// relayout, and lists the first of each. A panic fails it. A difference does not
/// yet: upstream lays a shrink-to-fit box out differently after an edit in some
/// cases (the README on this branch has the smallest one), and this branch does
/// not change that:
///
///   CHANGE_LIST_FUZZ_SEEDS=20000 cargo test --release -p blitz-dom \
///     --test change_list -- --ignored --nocapture change_list_fuzz
#[test]
#[ignore = "a fuzzer; run in release with --ignored --nocapture"]
fn change_list_fuzz() {
    let var = |name: &str, default: u64| {
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let first = var("CHANGE_LIST_FUZZ_FIRST", 0);
    let seeds = var("CHANGE_LIST_FUZZ_SEEDS", 1000);
    let ops = var("CHANGE_LIST_FUZZ_OPS", 80) as u32;
    std::panic::set_hook(Box::new(|info| {
        let location = info.location().map(|l| l.to_string()).unwrap_or_default();
        let message = info.payload_as_str().unwrap_or("");
        LAST_PANIC.with(|p| *p.borrow_mut() = format!("{location}: {message}"));
    }));
    let (mut clean, mut stale, mut panicked) = (0, Vec::new(), Vec::new());
    for seed in first..first + seeds {
        match fuzz_seed(seed, ops) {
            Seed::Clean => clean += 1,
            Seed::Stale => stale.push(seed),
            Seed::Panicked(why) => panicked.push((seed, why)),
        }
    }
    let _ = std::panic::take_hook();
    println!(
        "change list fuzz, seeds {first}..{}: {clean} clean, {} stale, {} panicked",
        first + seeds,
        stale.len(),
        panicked.len()
    );
    for seed in stale.iter().take(10) {
        println!("  stale seed {seed}");
    }
    for (seed, why) in panicked.iter().take(20) {
        println!("  panicked seed {seed}: {why}");
    }
    assert!(panicked.is_empty());
}
