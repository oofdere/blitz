use blitz_traits::node_id::NodeId;
use markup5ever::local_name;
use parley::{
    FontContext, InlineBox, InlineBoxKind, LayoutContext, TreeBuilder, WhiteSpaceCollapse,
};
use style::{
    computed_values::position::T as PositionProperty,
    values::{
        computed::{Display, Float},
        specified::box_::{DisplayInside, DisplayOutside},
    },
};
use stylo_taffy::CopyStyleGetters;

use super::spacer;
use crate::{
    Node, NodeData,
    layout::list::BULLET_FONT_FAMILY,
    layout::replaced::is_inline_box_element,
    layout::text_transform::{CaseTransform, TextTransformer},
    node::{ListItemLayout, ListItemLayoutPosition, Marker, TextBrush, TextLayout},
    stylo_to_parley,
};

pub(crate) fn build_inline_layout_into(
    nodes: &crate::NodeTree,
    layout_ctx: &mut LayoutContext<TextBrush>,
    font_ctx: &mut FontContext,
    text_layout: &mut TextLayout,
    scale: f32,
    inline_context_root_node_id: NodeId,
) {
    // Get the inline context's root node's text styles
    let root_node = &nodes[inline_context_root_node_id];
    let root_node_style = root_node.primary_styles().or_else(|| {
        root_node
            .parent
            .and_then(|parent_id| nodes[parent_id].primary_styles())
    });

    let parley_style = root_node_style
        .as_ref()
        .map(|s| stylo_to_parley::style(inline_context_root_node_id, s))
        .unwrap_or_else(|| parley::TextStyle {
            white_space_collapse: WhiteSpaceCollapse::Collapse,
            ..Default::default()
        });

    // Create a parley tree builder
    let mut builder = layout_ctx.tree_builder(font_ctx, scale, true, &parley_style);
    if let Some(style) = root_node_style.as_deref() {
        builder.set_base_direction(stylo_to_parley::base_direction(
            style.clone_direction(),
            style.clone_unicode_bidi(),
        ));
    }

    let text_transform = root_node_style
        .as_deref()
        .map(|s| CaseTransform::from_style(s))
        .unwrap_or(CaseTransform::NONE);

    // Render position-inside list items
    if let Some(ListItemLayout {
        marker,
        position: ListItemLayoutPosition::Inside,
        ..
    }) = root_node
        .element_data()
        .and_then(|el| el.list_item_data.as_deref())
    {
        match marker {
            // Bullet glyphs live in the bundled bullet font. The position-outside
            // path already asks for it; without the same span here a marker like
            // disclosure-closed (U+25B8) falls back to the element's own font and
            // renders as a missing glyph.
            Marker::Char(char) => {
                let mut marker_style = parley_style.clone();
                marker_style.font_family = BULLET_FONT_FAMILY.into();
                builder.push_style_span(marker_style);
                builder.push_text(&format!("{char} "));
                builder.pop_style_span();
            }
            Marker::String(str) => builder.push_text(str),
        }
    };
    // The marker is a separate box, so words in the content don't continue from it.
    let mut text_transformer = TextTransformer::default();
    text_transformer.word_break(&builder);
    text_transformer.edges = spacer::Edges::new(last_text(nodes, root_node));

    if let Some(before_id) = root_node.before() {
        build_inline_layout_recursive(
            &mut builder,
            &mut text_transformer,
            nodes,
            before_id,
            &text_transform,
        );
    }
    for child_id in shown_children(root_node).iter().copied() {
        // Text lifted out of a `display: contents` element into this anonymous block keeps
        // that element's white space, as text takes its parent's style.
        let child = &nodes[child_id];
        let lifted = match child.data {
            NodeData::Text(_) if child.parent != Some(inline_context_root_node_id) => child
                .parent
                .and_then(|parent| nodes[parent].primary_styles()),
            _ => None,
        };
        let whitespace = lifted.map(|s| {
            [
                parley::StyleProperty::WhiteSpaceCollapse(stylo_to_parley::white_space_collapse(
                    s.clone_white_space_collapse(),
                )),
                parley::StyleProperty::TextWrapMode(stylo_to_parley::text_wrap_mode(
                    s.clone_text_wrap_mode(),
                )),
            ]
        });
        if let Some(styles) = &whitespace {
            builder.push_style_modification_span(styles);
        }
        build_inline_layout_recursive(
            &mut builder,
            &mut text_transformer,
            nodes,
            child_id,
            &text_transform,
        );
        if whitespace.is_some() {
            builder.pop_style_span();
        }
    }
    if let Some(after_id) = root_node.after() {
        build_inline_layout_recursive(
            &mut builder,
            &mut text_transformer,
            nodes,
            after_id,
            &text_transform,
        );
    }

    if builder.text().contains('\u{2060}') {
        builder.set_line_break_override(Some(spacer::JOINED));
    }
    text_layout.text = builder.build_into(&mut text_layout.layout);
    text_layout.holds_laid_out_lines = false;
    return;

    fn build_inline_layout_recursive(
        builder: &mut TreeBuilder<TextBrush>,
        text_transformer: &mut TextTransformer,
        nodes: &crate::NodeTree,
        node_id: NodeId,
        parent_text_transform: &CaseTransform,
    ) {
        let node = &nodes[node_id];

        let style = node.primary_styles();
        let style = style.as_ref();

        let text_transform = style
            .map(|s| CaseTransform::from_style(s))
            .unwrap_or(CaseTransform::NONE);

        match &node.data {
            NodeData::Element(element_data) | NodeData::AnonymousBlock(element_data) => {
                // if the input type is hidden, hide it
                if *element_data.name.local == *"input" {
                    if let Some("hidden") = element_data.attr(local_name!("type")) {
                        return;
                    }
                }

                let display = node.display_style().unwrap_or(Display::inline());
                let position = style
                    .map(|s| s.clone_position())
                    .unwrap_or(PositionProperty::Static);
                let float = style.map(|s| s.clone_float()).unwrap_or(Float::None);
                let box_kind = if position.is_absolutely_positioned() {
                    InlineBoxKind::OutOfFlow
                } else if float.is_floating() {
                    InlineBoxKind::CustomOutOfFlow
                } else {
                    InlineBoxKind::InFlow
                };

                match (display.outside(), display.inside()) {
                    (DisplayOutside::None, DisplayInside::None) => {
                        // node.remove_damage(CONSTRUCT_DESCENDENT | CONSTRUCT_FC | CONSTRUCT_BOX);
                    }
                    (DisplayOutside::None, DisplayInside::Contents) => {
                        let whitespace = style.map(|s| {
                            [
                                parley::StyleProperty::WhiteSpaceCollapse(
                                    stylo_to_parley::white_space_collapse(
                                        s.clone_white_space_collapse(),
                                    ),
                                ),
                                parley::StyleProperty::TextWrapMode(
                                    stylo_to_parley::text_wrap_mode(s.clone_text_wrap_mode()),
                                ),
                            ]
                        });
                        builder.push_style_modification_span(
                            whitespace
                                .as_ref()
                                .map_or(&[][..], |styles| styles.as_slice()),
                        );
                        for child_id in node
                            .before()
                            .into_iter()
                            .chain(node.children.iter().copied())
                            .chain(node.after())
                        {
                            // node.remove_damage(CONSTRUCT_DESCENDENT | CONSTRUCT_FC | CONSTRUCT_BOX);
                            build_inline_layout_recursive(
                                builder,
                                text_transformer,
                                nodes,
                                child_id,
                                &text_transform,
                            );
                        }
                        builder.pop_style_span();
                    }
                    (DisplayOutside::Inline, DisplayInside::Flow) => {
                        let tag_name = &element_data.name.local;

                        if is_inline_box_element(element_data) {
                            if box_kind == InlineBoxKind::InFlow {
                                text_transformer.word_break(builder);
                                text_transformer.edges.clear();
                            }
                            builder.push_inline_box(InlineBox {
                                id: node_id.as_u64(),
                                kind: box_kind,
                                // Overridden by push_inline_box method
                                index: 0,
                                // Width and height are set during layout
                                width: 0.0,
                                height: 0.0,
                                baseline: None,
                                vertical_align: node
                                    .primary_styles()
                                    .map(|s| stylo_to_parley::vertical_align(&s))
                                    .unwrap_or_default(),
                            });
                        } else if *tag_name == local_name!("br") {
                            // node.remove_damage(CONSTRUCT_DESCENDENT | CONSTRUCT_FC | CONSTRUCT_BOX);
                            // TODO: update span id for br spans
                            builder.push_style_modification_span(&[
                                parley::StyleProperty::WhiteSpaceCollapse(
                                    WhiteSpaceCollapse::Preserve,
                                ),
                            ]);
                            builder.push_text("\n");
                            builder.pop_style_span();
                            text_transformer.word_break(builder);
                            text_transformer.edges.clear();
                        } else {
                            // node.remove_damage(CONSTRUCT_DESCENDENT | CONSTRUCT_FC | CONSTRUCT_BOX);
                            let style = node
                                .primary_styles()
                                .map(|s| stylo_to_parley::style(node.id, &s))
                                .unwrap_or_else(|| parley::TextStyle {
                                    white_space_collapse: WhiteSpaceCollapse::Collapse,
                                    ..Default::default()
                                });

                            let start =
                                spacer::start_outside(builder, &mut text_transformer.edges, node);
                            builder.push_style_span(style);
                            spacer::start_inside(builder, node, start);

                            if let Some(before_id) = node.before() {
                                build_inline_layout_recursive(
                                    builder,
                                    text_transformer,
                                    nodes,
                                    before_id,
                                    &text_transform,
                                );
                            }

                            for child_id in node.children.iter().copied() {
                                build_inline_layout_recursive(
                                    builder,
                                    text_transformer,
                                    nodes,
                                    child_id,
                                    &text_transform,
                                );
                            }
                            if let Some(after_id) = node.after() {
                                build_inline_layout_recursive(
                                    builder,
                                    text_transformer,
                                    nodes,
                                    after_id,
                                    &text_transform,
                                );
                            }

                            let end =
                                spacer::end_inside(builder, &mut text_transformer.edges, node);
                            builder.pop_style_span();
                            spacer::end_outside(builder, node, end);
                        }
                    }
                    // Inline box
                    (_, _) => {
                        if box_kind == InlineBoxKind::InFlow {
                            text_transformer.word_break(builder);
                            text_transformer.edges.clear();
                        }
                        builder.push_inline_box(InlineBox {
                            id: node_id.as_u64(),
                            kind: box_kind,
                            // Overridden by push_inline_box method
                            index: 0,
                            // Width and height are set during layout
                            width: 0.0,
                            height: 0.0,
                            baseline: None,
                            vertical_align: node
                                .primary_styles()
                                .map(|s| stylo_to_parley::vertical_align(&s))
                                .unwrap_or_default(),
                        });
                    }
                };
            }
            NodeData::Text(data) => {
                // node.remove_damage(CONSTRUCT_DESCENDENT | CONSTRUCT_FC | CONSTRUCT_BOX);
                // dbg!(&data.content);

                let collapse = node
                    .parent
                    .and_then(|parent| nodes[parent].primary_styles())
                    .map(|s| stylo_to_parley::white_space_collapse(s.clone_white_space_collapse()))
                    .unwrap_or(WhiteSpaceCollapse::Collapse);
                let mut edges = std::mem::take(&mut text_transformer.edges);
                let text =
                    text_transformer.transform(&data.content, parent_text_transform, builder);
                let text = edges.text(builder, node_id, text, collapse);
                builder.push_text(text);
                text_transformer.edges = edges;
            }
            NodeData::Comment { .. } => {
                // node.remove_damage(CONSTRUCT_DESCENDENT | CONSTRUCT_FC | CONSTRUCT_BOX);
            }
            NodeData::Document(_) => unreachable!(),
        }
    }
}

/// The text node that ends the content of the inline formatting context `root` lays out, if a
/// text node does rather than an atomic inline or a forced break. Inline elements and their
/// edges, `display: contents`, boxes taken out of flow, and text that is all white space that
/// collapses end nothing.
fn last_text(nodes: &crate::NodeTree, root: &Node) -> Option<NodeId> {
    match last_content(nodes, root)? {
        Last::Text(id) => Some(id),
        Last::Other => None,
    }
}

/// The children of `node` laid out in the inline formatting context it is the root of:
/// none of an `input`'s, which are the page's and not rendered, its label being its
/// `::before` (`build_button_input` in [`crate::layout::construct`]).
fn shown_children(node: &Node) -> &[NodeId] {
    match node
        .element_data()
        .is_some_and(|el| el.name.local == local_name!("input"))
    {
        true => &[],
        false => &node.children[..],
    }
}

/// What ends the content of an inline formatting context.
enum Last {
    Text(NodeId),
    Other,
}

/// What ends the content in `node`, the root or an inline element or `display: contents`
/// in it, its own and its pseudo-elements', in the order [`build_inline_layout_into`] pushes it.
fn last_content(nodes: &crate::NodeTree, node: &Node) -> Option<Last> {
    node.after()
        .into_iter()
        .chain(shown_children(node).iter().rev().copied())
        .chain(node.before())
        .find_map(|id| {
            let child = &nodes[id];
            match &child.data {
                NodeData::Text(data) => {
                    let collapse = child
                        .parent
                        .and_then(|parent| nodes[parent].primary_styles())
                        .map(|s| {
                            stylo_to_parley::white_space_collapse(s.clone_white_space_collapse())
                        })
                        .unwrap_or(WhiteSpaceCollapse::Collapse);
                    let collapsible = spacer::collapsible(collapse);
                    data.content
                        .contains(|c| !collapsible.contains(&c))
                        .then_some(Last::Text(id))
                }
                NodeData::Element(element) | NodeData::AnonymousBlock(element) => {
                    let hidden = *element.name.local == *"input"
                        && element.attr(local_name!("type")) == Some("hidden");
                    let style = child.primary_styles();
                    let out_of_flow = style.as_ref().is_some_and(|s| {
                        s.clone_position().is_absolutely_positioned()
                            || s.clone_float().is_floating()
                    });
                    let display = child.display_style().unwrap_or(Display::inline());
                    match (display.outside(), display.inside()) {
                        _ if hidden || out_of_flow => None,
                        (DisplayOutside::None, DisplayInside::None) => None,
                        (DisplayOutside::None, DisplayInside::Contents) => {
                            last_content(nodes, child)
                        }
                        (DisplayOutside::Inline, DisplayInside::Flow)
                            if !is_inline_box_element(element)
                                && element.name.local != local_name!("br") =>
                        {
                            last_content(nodes, child)
                        }
                        _ => Some(Last::Other),
                    }
                }
                _ => None,
            }
        })
}
