//! Interactive controlled editing and independent Flex flow options.
use super::*;

pub(super) fn view(app: &Studio, theme: &Theme) -> View<StudioAction> {
    let range = card(
        "Select an interval",
        "Move either endpoint. Hold model rejects edits.",
        theme,
    )
    .width(percent(49))
    .height(px(234))
    .child(styled_label(
        &format!("{} - {}", app.range.lower, app.range.upper),
        ORANGE,
        Font::montserrat_28(),
    ))
    .child(
        column([
            typed::range_slider(0, 100, binding(app.range, StudioAction::Range))
                .key(Key::Named("depth-range"))
                .width(percent(100))
                .height(px(12))
                .focusable(true)
                .into_view(),
        ])
        .width(percent(100))
        .padding(14),
    )
    .child(
        row([
            text("Hold model", theme),
            switch(app.hold).on_toggle(StudioAction::Hold),
        ])
        .gap(12)
        .align_items(ItemAlign::Center),
    );
    let flow = card(
        "Flexible tracks",
        "Wrap and reverse preserve the same items.",
        theme,
    )
    .width(percent(49))
    .height(px(234))
    .child(
        row((1..=6).map(|id| {
            label(format!("{id:02}"))
                .key(Key::Id(id))
                .width(px(96))
                .height(px(28))
                .background(theme.background)
                .text_color(theme.text)
                .padding(5)
        }))
        .width(percent(100))
        .height(px(78))
        .gap(6)
        .flex_wrap(app.wrap)
        .flex_reverse(app.reverse),
    )
    .child(
        row([
            text("Wrap", theme),
            switch(app.wrap).on_toggle(StudioAction::Wrap),
            text("Reverse", theme),
            switch(app.reverse).on_toggle(StudioAction::Reverse),
        ])
        .gap(10)
        .align_items(ItemAlign::Center),
    );
    let input = card(
        "Editing rules",
        "Input accepts up to six digits. Restore text loads a full model value.",
        theme,
    )
    .height(px(300))
    .child(
        row([
            typed::textarea(binding(&app.depth_text, |s: &str| {
                StudioAction::DepthText(s.into())
            }))
            .key(Key::Named("depth-text"))
            .font(Font::cjk_16())
            .accepted_chars("0123456789")
            .input_max_length(core::num::NonZeroU32::new(6))
            .keyboard_id(90)
            .width(percent(65))
            .height(px(44))
            .focusable(true)
            .into_view(),
            action_button("Restore text", StudioAction::RestoreText, theme),
        ])
        .align_items(ItemAlign::Center),
    )
    .child(keyboard(90).width(percent(100)).height(px(138)));
    studio_column()
        .child(row([range, flow]).width(percent(100)).gap(18))
        .child(input)
}
