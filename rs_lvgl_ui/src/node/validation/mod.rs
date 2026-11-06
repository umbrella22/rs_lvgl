//! Descriptor validation and diagnostic traversal before native mutation.

mod kinds;
use super::{Key, Node, Size, WidgetKind};
use crate::{Style, UiError};
use alloc::vec::Vec;
use kinds::validate_kind;

pub(crate) fn validate<M>(
    node: &Node<M>,
    diagnostics: &crate::diagnostics::Diagnostics,
) -> Result<(), UiError> {
    validate_node(node, &mut Vec::new(), diagnostics, 0, false)
}

fn validate_node<M>(
    node: &Node<M>,
    keyboard_ids: &mut Vec<u64>,
    diagnostics: &crate::diagnostics::Diagnostics,
    child_index: usize,
    inside_grid: bool,
) -> Result<(), UiError> {
    #[cfg(all(feature = "diagnostics", not(feature = "strict-properties")))]
    use crate::diagnostics::DiagnosticEvent;
    use crate::diagnostics::{DiagnosticStage, Property};
    diagnostics.node(
        node.key,
        child_index,
        crate::widgets::tag(&node.kind),
        DiagnosticStage::Validation,
        Property::Value,
        || {
            #[cfg(any(feature = "diagnostics", feature = "strict-properties"))]
            if let Some(property) = node.invalid_property {
                #[cfg(feature = "strict-properties")]
                {
                    diagnostics.report(
                        UiError::InvalidProperty,
                        property,
                        DiagnosticStage::Validation,
                    );
                    return Err(UiError::InvalidProperty);
                }
                #[cfg(not(feature = "strict-properties"))]
                diagnostics.emit(DiagnosticEvent::IgnoredProperty(property));
            }
            let local = (|| -> Result<(), (UiError, Property)> {
                if let Some(offset) = node.interaction.offset {
                    let max = rs_lvgl_sys::LV_COORD_MAX as i32;
                    if !(-max..=max).contains(&offset.x) || !(-max..=max).contains(&offset.y) {
                        return Err((UiError::InvalidProperty, Property::Value));
                    }
                }
                #[cfg(feature = "input-gridnav")]
                validate_navigation(node, inside_grid)
                    .map_err(|e| (e, Property::GridNavigation))?;
                crate::animation::validate(&node.animations)
                    .map_err(|e| (e, Property::Animation))?;
                validate_siblings(&node.children).map_err(|e| (e, Property::Children))?;
                if node
                    .static_text
                    .is_some_and(|text| text.as_bytes().contains(&0))
                {
                    return Err((UiError::InvalidProperty, Property::Text));
                }
                if node.font_resource.is_some() {
                    crate::resources::validate_support().map_err(|e| (e, Property::Font))?;
                    if matches!(node.kind, WidgetKind::Custom(_)) {
                        return Err((UiError::InvalidProperty, Property::Font));
                    }
                }
                #[cfg(feature = "widget-image")]
                if node.image_resource.is_some() {
                    crate::resources::validate_support().map_err(|e| (e, Property::Source))?;
                }
                validate_kind(node).map_err(|e| (e, validation_property(node)))?;
                validate_style(&node.style).map_err(|e| (e, Property::Style))?;
                for (_, style) in &node.part_styles {
                    validate_style(style).map_err(|e| (e, Property::Style))?;
                }
                if let Some(id) = node.keyboard_id {
                    if keyboard_ids.contains(&id) {
                        return Err((UiError::InvalidProperty, Property::KeyboardId));
                    }
                    keyboard_ids.push(id);
                }
                Ok(())
            })();
            if let Err((error, property)) = local {
                diagnostics.report(error, property, DiagnosticStage::Validation);
                return Err(error);
            }
            #[cfg(feature = "input-gridnav")]
            let inside_grid = inside_grid || node.grid_navigation.is_some();
            for (index, child) in node.children.iter().enumerate() {
                validate_node(child, keyboard_ids, diagnostics, index, inside_grid)?;
            }
            Ok(())
        },
    )
}

fn validation_property<M>(node: &Node<M>) -> crate::Property {
    use crate::Property;
    if node.handlers.text_changed.is_some() && !is_textarea(&node.kind) {
        return Property::Text;
    }
    if node.keyboard_id.is_some() && !is_textarea(&node.kind) {
        return Property::KeyboardId;
    }
    if !node.children.is_empty() && !is_container(&node.kind) {
        return Property::Children;
    }
    match &node.kind {
        WidgetKind::Label { .. } | WidgetKind::Button { .. } => Property::Text,
        #[cfg(feature = "widget-textarea")]
        WidgetKind::Textarea { placeholder, .. } if placeholder.as_bytes().contains(&0) => {
            Property::Placeholder
        }
        #[cfg(feature = "widget-textarea")]
        WidgetKind::Textarea { .. } => Property::Text,
        #[cfg(feature = "widget-image")]
        WidgetKind::Image { scale, .. } if scale.validate_image().is_err() => Property::Scale,
        #[cfg(feature = "widget-image")]
        WidgetKind::Image { .. } => Property::Source,
        _ => Property::Value,
    }
}

fn validate_siblings<M>(children: &[Node<M>]) -> Result<(), UiError> {
    let mut named: Vec<&'static str> = Vec::new();
    let mut ids: Vec<u64> = Vec::new();
    for child in children {
        match child.key {
            Key::Auto => {}
            Key::Named(name) => {
                if named.contains(&name) {
                    return Err(UiError::DuplicateKey);
                }
                named.push(name);
            }
            Key::Id(id) => {
                if ids.contains(&id) {
                    return Err(UiError::DuplicateKey);
                }
                ids.push(id);
            }
        }
    }
    Ok(())
}

fn is_textarea(kind: &WidgetKind) -> bool {
    #[cfg(feature = "widget-textarea")]
    {
        matches!(kind, WidgetKind::Textarea { .. })
    }
    #[cfg(not(feature = "widget-textarea"))]
    {
        let _ = kind;
        false
    }
}

fn is_container(kind: &WidgetKind) -> bool {
    match kind {
        WidgetKind::Container | WidgetKind::Row | WidgetKind::Column | WidgetKind::Grid { .. } => {
            true
        }
        #[cfg(feature = "widget-tabview")]
        WidgetKind::Tabview { .. } => true,
        #[cfg(feature = "widget-tileview")]
        WidgetKind::Tileview { .. } => true,
        #[cfg(feature = "widget-msgbox")]
        WidgetKind::MessageBox { .. } => true,
        _ => false,
    }
}

fn validate_style(style: &Style) -> Result<(), UiError> {
    style.extended.validate()?;
    if let Some(transition) = style.transition {
        transition.validate()?;
    }
    if style.border_width < 0 || style.radius < 0 {
        return Err(UiError::InvalidProperty);
    }
    for size in [
        style.width,
        style.height,
        style.max_width,
        style.max_height,
        style.min_width,
        style.min_height,
    ] {
        if let Size::Px(value) = size {
            if value < 0 {
                return Err(UiError::InvalidProperty);
            }
        }
    }
    Ok(())
}

#[cfg(feature = "input-gridnav")]
fn validate_navigation<M>(node: &Node<M>, inside_grid: bool) -> Result<(), UiError> {
    if inside_grid && (node.grid_navigation.is_some() || node.modal) {
        return Err(UiError::InvalidProperty);
    }
    if node.grid_navigation.is_some()
        && !matches!(
            node.kind,
            WidgetKind::Container | WidgetKind::Row | WidgetKind::Column | WidgetKind::Grid { .. }
        )
    {
        return Err(UiError::InvalidProperty);
    }
    // The IME's focus target is a private native child, not the direct grid child.
    #[cfg(feature = "widget-ime-pinyin")]
    if inside_grid && matches!(node.kind, WidgetKind::PinyinKeyboard { .. }) {
        return Err(UiError::InvalidProperty);
    }
    Ok(())
}
