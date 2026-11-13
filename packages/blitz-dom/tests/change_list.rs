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
use std::time::{Duration, Instant};

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

/// Every out-of-flow box a containing block in the document lists is in the slab,
/// names that block as its containing block, and is in no other block's list.
fn assert_hoisted_lists_consistent(doc: &BaseDocument) {
    let mut listed_by = std::collections::HashMap::new();
    for (container, node) in doc.tree().iter() {
        if !node.flags.is_in_document() {
            continue;
        }
        for &child in node.hoisted_children.borrow().iter() {
            let Some(child_node) = doc.get_node(child) else {
                panic!("{container:?} hoists {child:?}, which has been freed");
            };
            assert_eq!(
                child_node.oof_containing_block.get(),
                Some(container),
                "{container:?} hoists {child:?}, whose containing block is another box"
            );
            if let Some(other) = listed_by.insert(child, container) {
                panic!("{child:?} is in the hoisted lists of {other:?} and {container:?}");
            }
        }
    }
}

/// Every box a container in the document lists points back at it, and is in the slab.
fn assert_layout_lists_consistent(doc: &BaseDocument) {
    for (container, node) in doc.tree().iter() {
        if !node.flags.is_in_document() {
            continue;
        }
        for &child in node.layout_children.borrow().iter().flatten() {
            let Some(child_node) = doc.get_node(child) else {
                panic!("{container:?} lists {child:?}, which has been freed");
            };
            assert_eq!(
                child_node.layout_parent.get(),
                Some(container),
                "{container:?} lists {child:?}, whose layout parent is another box"
            );
        }
    }
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

/// A box claimed by a second containing block leaves the first one's list, so
/// the first never hands layout the box once it is freed.
#[test]
fn a_box_reclaimed_by_another_containing_block_leaves_the_first_ones_list() {
    let (mut doc, body) = page(FUZZ_CSS);
    let n1 = div(&mut doc, body, "db pk", "t");
    let n3 = div(&mut doc, n1, "db pk af bs tr z ov", "t");
    let n5 = div(&mut doc, n3, "db pk af z ov", "t");
    doc.resolve(0.0);
    append(&mut doc, n1, n5);
    set_class(&mut doc, n1, "dt pk bf bs tr z");
    doc.resolve(0.0);
    set_class(&mut doc, n1, "db  af bs tr z ov");
    doc.resolve(0.0);
    set_class(&mut doc, n5, "di pa bs z ov");
    doc.resolve(0.0);
}

/// An inline root that loses a child as it moves into a display: none box is
/// built again once that box is shown, and not laid out with the inline box
/// that was freed.
#[test]
fn an_inline_root_changed_as_it_is_hidden_is_rebuilt_when_shown() {
    let (mut doc, body) = page(HOISTING);
    let root = div(&mut doc, body, "", "x");
    let gone = create(&mut doc, "span");
    set_class(&mut doc, gone, "ib");
    let y = doc.mutate().create_text_node("y");
    append(&mut doc, gone, y);
    append(&mut doc, root, gone);
    let z = doc.mutate().create_text_node("z");
    append(&mut doc, root, z);
    let outer = div(&mut doc, body, "hidden", "");
    doc.resolve(0.0);
    remove(&mut doc, gone);
    append(&mut doc, outer, root);
    doc.resolve(0.0);
    set_class(&mut doc, outer, "");
    doc.resolve(0.0);
}

/// The same for a block inside a table, which the table builder skips.
#[test]
fn an_inline_root_changed_while_a_table_skips_it_is_rebuilt_after() {
    let (mut doc, body) = page(HOISTING);
    let outer = div(&mut doc, body, "", "");
    let root = div(&mut doc, outer, "", "x");
    let gone = create(&mut doc, "span");
    set_class(&mut doc, gone, "ib");
    let y = doc.mutate().create_text_node("y");
    append(&mut doc, gone, y);
    append(&mut doc, root, gone);
    doc.resolve(0.0);
    set_class(&mut doc, outer, "table");
    doc.resolve(0.0);
    remove(&mut doc, gone);
    doc.resolve(0.0);
    set_class(&mut doc, outer, "");
    doc.resolve(0.0);
}

/// An element made inline after it held a block, laid out, then left with
/// inline content only, has no box of its own: it lists no layout children.
#[test]
fn an_inline_element_that_held_a_block_lets_go_of_its_layout_children() {
    let (mut doc, body) =
        page(".di { display: inline } .bf::before { content: \"b\"; position: absolute }");
    let inline = div(&mut doc, body, "", "t");
    let block = div(&mut doc, inline, "", "t");
    set_class(&mut doc, inline, "di bf");
    doc.resolve(0.0);
    remove(&mut doc, block);
    doc.resolve(0.0);
    assert_layout_lists_consistent(&doc);
    assert!(
        doc.get_node(inline)
            .unwrap()
            .layout_children
            .borrow()
            .is_none()
    );
}

/// The same when the block moves out instead: the anonymous blocks the
/// element made around its content while it held the block are freed.
#[test]
fn an_inline_element_whose_block_moved_out_frees_its_anonymous_blocks() {
    let (mut doc, body) = page(".di { display: inline }");
    let inline = div(&mut doc, body, "", "t");
    let block = div(&mut doc, inline, "", "t");
    set_class(&mut doc, inline, "di");
    doc.resolve(0.0);
    append(&mut doc, body, block);
    doc.resolve(0.0);
    assert_layout_lists_consistent(&doc);
    assert!(doc.get_node(inline).unwrap().anonymous_blocks.is_empty());
}

/// A box that moves out of a display: none subtree leaves the list of the
/// container it was in, which construction does not reach while it is hidden.
#[test]
fn a_box_moved_out_of_a_hidden_subtree_leaves_its_old_containers_list() {
    let (mut doc, body) = page(".hidden { display: none }");
    let hidden = div(&mut doc, body, "", "t");
    let container = div(&mut doc, hidden, "", "t");
    let moving = div(&mut doc, container, "", "t");
    doc.resolve(0.0);
    set_class(&mut doc, hidden, "hidden");
    doc.resolve(0.0);
    let target = div(&mut doc, body, "", "t");
    append(&mut doc, target, moving);
    doc.resolve(0.0);
    assert_layout_lists_consistent(&doc);
}

const FUZZ_CSS: &str = ".db{display:block}.di{display:inline}.dt{display:table}.dib{display:inline-block}.pa{position:absolute}.pk{position:sticky}.bf::before{content:\"b\";position:absolute}.af::after{content:\"a\";position:fixed}.bs::before{content:\"s\"}.tr{transform:translateX(1px)}.z{z-index:1}.ov{overflow:auto}.dn{display:none}.pr{position:relative}.pf{position:fixed}.fl{display:flex}.dc{display:contents}";

const FUZZ_CLASSES: [&str; 17] = [
    "db", "di", "dt", "dib", "dn", "pa", "pr", "pf", "pk", "bf", "af", "bs", "tr", "z", "ov", "fl",
    "dc",
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
    /// After the incremental layout, a layout list or a hoisted list names a box
    /// it should not.
    Broken(String),
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
    let lists = catch_unwind(AssertUnwindSafe(|| {
        assert_layout_lists_consistent(&doc);
        assert_hoisted_lists_consistent(&doc);
    }));
    if lists.is_err() {
        return Seed::Broken(LAST_PANIC.with(|p| p.borrow().clone()));
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
    let (mut clean, mut stale, mut panicked, mut broken) = (0, Vec::new(), Vec::new(), Vec::new());
    for seed in first..first + seeds {
        match fuzz_seed(seed, ops) {
            Seed::Clean => clean += 1,
            Seed::Stale => stale.push(seed),
            Seed::Panicked(why) => panicked.push((seed, why)),
            Seed::Broken(why) => broken.push((seed, why)),
        }
    }
    let _ = std::panic::take_hook();
    println!(
        "change list fuzz, seeds {first}..{}: {clean} clean, {} stale, {} panicked, {} broken",
        first + seeds,
        stale.len(),
        panicked.len(),
        broken.len()
    );
    for seed in stale.iter().take(10) {
        println!("  stale seed {seed}");
    }
    for (seed, why) in panicked.iter().take(20) {
        println!("  panicked seed {seed}: {why}");
    }
    for (seed, why) in broken.iter().take(20) {
        println!("  broken seed {seed}: {why}");
    }
    assert!(panicked.is_empty() && broken.is_empty());
}

/// A block that becomes display: contents gives its children to its parent's
/// box, and no list still names them.
#[test]
fn a_block_that_becomes_contents_lets_go_of_its_children() {
    let (mut doc, body) = page(".dc { display: contents }");
    let outer = div(&mut doc, body, "", "t");
    div(&mut doc, outer, "", "t");
    doc.resolve(0.0);
    set_class(&mut doc, outer, "dc");
    doc.resolve(0.0);
    assert_layout_lists_consistent(&doc);
}

/// The same, then a child of the display: contents element moves elsewhere.
#[test]
fn a_child_of_a_contents_element_moves_out() {
    let (mut doc, body) = page(".dc { display: contents }");
    let outer = div(&mut doc, body, "", "t");
    let inner = div(&mut doc, outer, "", "t");
    doc.resolve(0.0);
    set_class(&mut doc, outer, "dc");
    doc.resolve(0.0);
    let target = div(&mut doc, body, "", "t");
    append(&mut doc, target, inner);
    doc.resolve(0.0);
    assert_layout_lists_consistent(&doc);
}

/// display: contents inside display: contents.
#[test]
fn contents_inside_contents() {
    let (mut doc, body) = page(".dc { display: contents }");
    let outer = div(&mut doc, body, "", "t");
    let middle = div(&mut doc, outer, "", "t");
    div(&mut doc, middle, "", "t");
    doc.resolve(0.0);
    set_class(&mut doc, middle, "dc");
    doc.resolve(0.0);
    set_class(&mut doc, outer, "dc");
    doc.resolve(0.0);
    assert_layout_lists_consistent(&doc);
}

/// An absolutely positioned box whose containing block becomes display: contents
/// is claimed by the next containing block up, and leaves the hoisted list of the
/// first one, which is not laid out again: a box is in one hoisted list at most,
/// the list of the block its back-pointer names.
#[test]
fn a_box_whose_containing_block_becomes_contents_is_in_one_hoisted_list() {
    let (mut doc, body) = page(".pa { position: absolute } .dc { display: contents }");
    let outer = div(&mut doc, body, "pa", "t");
    div(&mut doc, outer, "pa", "t");
    doc.resolve(0.0);
    set_class(&mut doc, outer, "dc");
    doc.resolve(0.0);
    assert_hoisted_lists_consistent(&doc);
}

/// The elements `DocumentMutator` queues for its flush: form controls, `<style>` and
/// `<title>`.
const QUEUED_FOR_FLUSH: [&str; 9] = [
    "input", "button", "select", "textarea", "fieldset", "output", "object", "style", "title",
];

/// The tags for which `edit` panics, each given a fresh page and an element holding
/// a text node, not yet in the document.
fn tags_that_panic(edit: impl Fn(&mut BaseDocument, NodeId, NodeId)) -> Vec<&'static str> {
    QUEUED_FOR_FLUSH
        .into_iter()
        .filter(|tag| {
            catch_unwind(AssertUnwindSafe(|| {
                let (mut doc, body) = page("");
                let element = create(&mut doc, tag);
                let text = doc.mutate().create_text_node("div { color: red }");
                append(&mut doc, element, text);
                edit(&mut doc, body, element);
                doc.resolve(0.0);
            }))
            .is_err()
        })
        .collect()
}

/// niso applies a task's changes with one mutator, so an element can come in and go
/// out before the mutator flushes.
#[test]
fn an_element_put_in_and_taken_out_with_one_mutator() {
    let panicked = tags_that_panic(|doc, body, element| {
        let mut mutator = doc.mutate();
        mutator.append_children(body, &[element]);
        mutator.remove_and_drop_node_with(element, &mut |_| {});
    });
    assert!(panicked.is_empty(), "panicked for {panicked:?}");
}

#[test]
fn an_element_put_in_and_its_parent_taken_out_with_one_mutator() {
    let panicked = tags_that_panic(|doc, body, element| {
        let parent = create(doc, "div");
        let mut mutator = doc.mutate();
        mutator.append_children(body, &[parent]);
        mutator.append_children(parent, &[element]);
        mutator.remove_and_drop_node_with(parent, &mut |_| {});
    });
    assert!(panicked.is_empty(), "panicked for {panicked:?}");
}

#[test]
fn an_element_whose_text_changed_taken_out_with_one_mutator() {
    let panicked = tags_that_panic(|doc, body, element| {
        append(doc, body, element);
        let text = doc.get_node(element).unwrap().children[0];
        let mut mutator = doc.mutate();
        mutator.set_node_text(text, "p { color: blue }");
        mutator.remove_and_drop_node_with(element, &mut |_| {});
    });
    assert!(panicked.is_empty(), "panicked for {panicked:?}");
}

/// An inline element sits in an inline flow next to a block, so construction wraps
/// the flow in an anonymous block, and the element's layout parent is that block. Its
/// container then becomes `display: none` or `display: table`, which frees the
/// anonymous block, and the element is read.
#[test]
fn a_read_after_its_anonymous_block_is_freed() {
    let mut panicked = Vec::new();
    for display in ["none", "table"] {
        let read = || {
            let (mut doc, body) = page("");
            let container = div(&mut doc, body, "", "");
            let inline = create(&mut doc, "span");
            let text = doc.mutate().create_text_node("t");
            append(&mut doc, inline, text);
            append(&mut doc, container, inline);
            let block = create(&mut doc, "div");
            append(&mut doc, container, block);
            doc.resolve(0.0);
            doc.mutate()
                .set_attribute(container, attr("style"), &format!("display: {display}"));
            doc.resolve(0.0);
            let _ = doc.get_client_bounding_rect(inline);
        };
        if catch_unwind(AssertUnwindSafe(read)).is_err() {
            panicked.push(display);
        }
    }
    assert!(panicked.is_empty(), "panicked for {panicked:?}");
}

/// A table builds no box for a child that is not a table part, so a block in a table
/// has none. Under it, an inline element whose flow held another one is read after
/// that one is removed, and again once the table is a block again.
#[test]
fn a_read_under_a_child_the_table_leaves_out() {
    let (mut doc, body) = page("");
    let container = div(&mut doc, body, "", "");
    let block = div(&mut doc, container, "", "");
    let outer = create(&mut doc, "span");
    let text = doc.mutate().create_text_node("tail ");
    append(&mut doc, outer, text);
    append(&mut doc, block, outer);
    let inner = create(&mut doc, "span");
    let inner_text = doc.mutate().create_text_node("hello world");
    append(&mut doc, inner, inner_text);
    doc.mutate().insert_nodes_before(text, &[inner]);
    div(&mut doc, block, "", "");
    doc.resolve(0.0);
    doc.mutate()
        .set_attribute(container, attr("style"), "display: table");
    remove(&mut doc, inner);
    doc.resolve(0.0);
    let _ = doc.get_client_bounding_rect(outer);
    doc.mutate().clear_attribute(container, attr("style"));
    doc.resolve(0.0);
    let _ = doc.get_client_bounding_rect(outer);
    assert_layout_lists_consistent(&doc);
}

/// A replaced element is laid out as a leaf whatever its `display`, even with children.
#[test]
fn a_replaced_element_with_a_child_laid_out_as_any_display() {
    let mut panicked = Vec::new();
    for tag in ["img", "canvas", "video", "embed", "iframe"] {
        for display in [
            "block",
            "table",
            "inline-table",
            "flex",
            "grid",
            "list-item",
        ] {
            let failed = catch_unwind(AssertUnwindSafe(|| {
                let (mut doc, body) = page("");
                let element = create(&mut doc, tag);
                doc.mutate()
                    .set_attribute(element, attr("style"), &format!("display: {display}"));
                div(&mut doc, element, "", "t");
                append(&mut doc, body, element);
                doc.resolve(0.0);
            }))
            .is_err();
            if failed {
                panicked.push(format!("{tag} {display}"));
            }
        }
    }
    assert!(panicked.is_empty(), "panicked for {panicked:?}");
}

/// A `display: contents` element with a `::before` sits in a flex container, so its
/// pseudo-element and its text are boxes in the container's list. The container is
/// hidden, so construction skips it, and the element moves out and gets a new child.
#[test]
fn a_contents_element_moved_out_of_a_hidden_container_takes_its_boxes() {
    let (mut doc, body) = page(FUZZ_CSS);
    let hidden = div(&mut doc, body, "dn", "t");
    let flex = div(&mut doc, body, "", "t");
    let contents = div(&mut doc, flex, "bs dc", "t");
    let flex_text = doc.get_node(flex).unwrap().children[0];
    doc.mutate().insert_nodes_before(flex_text, &[contents]);
    doc.resolve(0.0);
    set_class(&mut doc, flex, "fl");
    doc.resolve(0.0);
    append(&mut doc, hidden, flex);
    append(&mut doc, body, contents);
    let contents_text = doc.get_node(contents).unwrap().children[0];
    let child = create(&mut doc, "div");
    doc.mutate().insert_nodes_before(contents_text, &[child]);
    doc.resolve(0.0);
    assert_layout_lists_consistent(&doc);
}

/// An inline element holds an inline-block, which is a box in the list of the block
/// whose inline flow they are in. The block is hidden, and the inline element moves out.
#[test]
fn an_inline_element_moved_out_of_a_hidden_container_takes_its_boxes() {
    let (mut doc, body) = page(FUZZ_CSS);
    let hidden = div(&mut doc, body, "dn", "t");
    let block = div(&mut doc, body, "", "t");
    let inline = div(&mut doc, block, "di", "t");
    div(&mut doc, inline, "dib", "t");
    doc.resolve(0.0);
    append(&mut doc, hidden, block);
    append(&mut doc, body, inline);
    doc.resolve(0.0);
    assert_layout_lists_consistent(&doc);
}

/// Remove `count` absolutely positioned boxes one by one through one mutator, as niso
/// does, and lay out again. Each box sits in its own wrapper of `wrapper_class` inside one
/// containing block: a `display: contents` wrapper leaves every box in the containing
/// block's two layout lists, a relatively positioned one gives each box lists of its own.
fn time_removing_boxes(count: usize, wrapper_class: &str) -> Duration {
    let (mut doc, body) = page(HOISTING);
    let cb = create(&mut doc, "div");
    set_class(&mut doc, cb, "cb");
    append(&mut doc, body, cb);
    let mut wrappers = Vec::new();
    let mut boxes = Vec::new();
    for _ in 0..count {
        let wrapper = create(&mut doc, "div");
        set_class(&mut doc, wrapper, wrapper_class);
        let abs = create(&mut doc, "div");
        set_class(&mut doc, abs, "abs");
        append(&mut doc, wrapper, abs);
        wrappers.push(wrapper);
        boxes.push(abs);
    }
    doc.mutate().append_children(cb, &wrappers);
    doc.resolve(0.0);

    let start = Instant::now();
    let mut mutator = doc.mutate();
    for abs in boxes {
        mutator.remove_and_drop_node_with(abs, &mut |_| {});
    }
    drop(mutator);
    doc.resolve(0.0);
    start.elapsed()
}

#[test]
fn removing_many_boxes_of_one_container_stays_linear() {
    let count = 20_000;
    let shared = time_removing_boxes(count, "dc");
    let own = time_removing_boxes(count, "cb");
    assert!(
        shared < own * 3,
        "removing {count} boxes from one container's lists took {shared:?}, from lists of their own {own:?}"
    );
}
