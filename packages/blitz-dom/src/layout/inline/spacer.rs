//! Spacers: the boxes that hold an inline element's margin, border and padding open in its line.
//!
//! Parley has no edge spacing on a span, so a non-atomic inline element whose inline-start or
//! inline-end margin, border or padding is not zero gets an in-flow inline box of that width at
//! that edge: one for the margin, outermost, and one for the border and padding. They are zero
//! high, so they hold the width open without changing the line's height. Start and end follow the
//! element's `direction`, so in a right-to-left element the start spacer holds its right edges.

use blitz_traits::node_id::NodeId;
use parley::{
    InlineBox, InlineBoxKind, LineBreakContext, LineBreakOverrideFn, TreeBuilder,
    WhiteSpaceCollapse,
};
use taffy::{CoreStyle as _, ResolveOrZero as _};

use crate::Node;
use crate::layout::resolve_calc_value;
use crate::node::TextBrush;

const SPACER: u64 = 1 << 63;
const END: u64 = 1 << 62;
const MARGIN: u64 = 1 << 61;
const FLAGS: u64 = SPACER | END | MARGIN;

/// The spacer an inline box id stands for, if it stands for one rather than for an atomic inline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InlineSpacer {
    /// The inline element whose edge it holds open.
    pub node: NodeId,
    /// Whether it is at the element's inline end rather than its start.
    pub end: bool,
    /// Whether it holds the margin rather than the border and padding.
    pub margin: bool,
}

impl InlineSpacer {
    /// The spacer an inline box's id names, or `None` for the id of an atomic inline.
    pub fn from_id(id: u64) -> Option<InlineSpacer> {
        (id & SPACER != 0).then(|| InlineSpacer {
            node: NodeId::from_u64(id & !FLAGS),
            end: id & END != 0,
            margin: id & MARGIN != 0,
        })
    }

    fn id(self) -> u64 {
        let id = self.node.as_u64();
        debug_assert_eq!(id & FLAGS, 0, "a node id with a spacer's bits");
        id | SPACER | if self.end { END } else { 0 } | if self.margin { MARGIN } else { 0 }
    }

    /// Its width in CSS pixels, with percentages of `basis`, the containing block's width.
    pub(crate) fn width(self, node: &Node, basis: Option<f32>) -> f32 {
        width(node, self.end, self.margin, basis)
    }
}

/// The width of one of an element's spacers: its margin, or its border and padding, on the side
/// that is its inline start or end.
fn width(node: &Node, end: bool, margin: bool, basis: Option<f32>) -> f32 {
    let style = node.layout_style();
    let left = (style.direction() == taffy::Direction::Ltr) != end;
    let size = taffy::Size {
        width: basis,
        height: basis,
    };
    let side = |rect: taffy::Rect<f32>| if left { rect.left } else { rect.right };
    if margin {
        side(style.margin().resolve_or_zero(size, resolve_calc_value))
    } else {
        side(style.border().resolve_or_zero(size, resolve_calc_value))
            + side(style.padding().resolve_or_zero(size, resolve_calc_value))
    }
}

/// What the spacers just pushed ask of the text that comes next.
///
/// A spacer is a U+FFFC in the text, which a line may break on either side of, and which white
/// space does not collapse across. So a word joiner keeps each spacer to its element's content,
/// and to the content outside it unless white space stands between them, and white space after a
/// spacer that followed white space is dropped, as it collapses with that white space.
#[derive(Default)]
pub(crate) struct Edges {
    /// Whether white space came before the spacers just pushed.
    after_space: bool,
    /// Whether end spacers were just pushed, which the next text is joined to unless it starts
    /// with white space.
    join_next: bool,
}

impl Edges {
    /// The text node's `text`, about to be pushed, as the spacers before it leave it: without
    /// the collapsible white space at its start if white space came before them, and joined to
    /// them if they end an element and it starts with no white space.
    pub(crate) fn text<'a>(
        &mut self,
        builder: &mut TreeBuilder<TextBrush>,
        text: &'a str,
        collapse: WhiteSpaceCollapse,
    ) -> &'a str {
        let collapsible: &[char] = match collapse {
            WhiteSpaceCollapse::Collapse => &[' ', '\t', '\n', '\r'],
            WhiteSpaceCollapse::PreserveBreaks => &[' ', '\t'],
            _ => &[],
        };
        let text = if self.after_space {
            text.trim_start_matches(collapsible)
        } else {
            text
        };
        if text.is_empty() {
            return text;
        }
        if self.join_next && !text.starts_with(char::is_whitespace) {
            builder.push_text(WORD_JOINER);
        }
        *self = Edges::default();
        text
    }

    /// Forgets the spacers: an atomic inline or a forced break comes next, which a line may break
    /// beside anyway.
    pub(crate) fn clear(&mut self) {
        *self = Edges::default();
    }
}

const WORD_JOINER: &str = "\u{2060}";

/// The line breaks of a text with word joiners in it: none beside a word joiner, as UAX #14 has it,
/// even beside an in-flow box, which Parley otherwise puts an opportunity on each side of.
pub(crate) static JOINED: &LineBreakOverrideFn = &(|context: LineBreakContext| -> Option<bool> {
    (context.before == '\u{2060}' || context.after == '\u{2060}').then_some(false)
});

/// Which of an element's spacers at one edge are pushed: its margin's, and its border's and
/// padding's. A spacer that is zero whatever the containing block's width is left out.
#[derive(Clone, Copy)]
pub(crate) struct Spacers {
    margin: bool,
    edge: bool,
}

impl Spacers {
    fn of(node: &Node, end: bool) -> Option<Spacers> {
        let pushed = |margin| {
            [1.0, 1000.0]
                .into_iter()
                .any(|basis| width(node, end, margin, Some(basis)) != 0.0)
        };
        let spacers = Spacers {
            margin: pushed(true),
            edge: pushed(false),
        };
        (spacers.margin || spacers.edge).then_some(spacers)
    }
}

/// Starts an inline element, before its span: joins it to what comes before unless white space
/// comes between them, and pushes its start margin's spacer. Returns its start spacers, for
/// [`start_inside`].
pub(crate) fn start_outside(
    builder: &mut TreeBuilder<TextBrush>,
    edges: &mut Edges,
    node: &Node,
) -> Option<Spacers> {
    let spacers = Spacers::of(node, false)?;
    edges.boundary(builder, false);
    if spacers.margin {
        push(builder, node, false, true);
        builder.push_text(WORD_JOINER);
    }
    Some(spacers)
}

/// Goes on starting an inline element, in its span: pushes its start border's and padding's
/// spacer, joined to its content.
pub(crate) fn start_inside(
    builder: &mut TreeBuilder<TextBrush>,
    node: &Node,
    spacers: Option<Spacers>,
) {
    if spacers.is_some_and(|spacers| spacers.edge) {
        push(builder, node, false, false);
        builder.push_text(WORD_JOINER);
    }
}

/// Ends an inline element, in its span: pushes its end border's and padding's spacer, joined to
/// its content unless that ends with white space. Returns its end spacers, for [`end_outside`].
pub(crate) fn end_inside(
    builder: &mut TreeBuilder<TextBrush>,
    edges: &mut Edges,
    node: &Node,
) -> Option<Spacers> {
    let spacers = Spacers::of(node, true)?;
    edges.boundary(builder, true);
    if spacers.edge {
        push(builder, node, true, false);
    }
    Some(spacers)
}

/// Goes on ending an inline element, after its span: pushes its end margin's spacer.
pub(crate) fn end_outside(
    builder: &mut TreeBuilder<TextBrush>,
    node: &Node,
    spacers: Option<Spacers>,
) {
    let Some(spacers) = spacers.filter(|spacers| spacers.margin) else {
        return;
    };
    if spacers.edge {
        builder.push_text(WORD_JOINER);
    }
    push(builder, node, true, true);
}

impl Edges {
    /// Notes whether white space comes before the spacers about to be pushed, and joins them to
    /// what comes before if none does.
    fn boundary(&mut self, builder: &mut TreeBuilder<TextBrush>, end: bool) {
        let text = builder.text();
        let space_before = builder.has_pending_whitespace()
            || text.ends_with(char::is_whitespace)
            || (self.after_space && text.ends_with(['\u{FFFC}', '\u{2060}']));
        if !space_before && !text.is_empty() {
            builder.push_text(WORD_JOINER);
        }
        self.after_space = space_before;
        self.join_next = end;
    }
}

fn push(builder: &mut TreeBuilder<TextBrush>, node: &Node, end: bool, margin: bool) {
    builder.push_inline_box(InlineBox {
        id: InlineSpacer {
            node: node.id,
            end,
            margin,
        }
        .id(),
        kind: InlineBoxKind::InFlow,
        index: 0,
        width: 0.0,
        height: 0.0,
        baseline: None,
        vertical_align: Default::default(),
    });
}
