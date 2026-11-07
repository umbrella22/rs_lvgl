//! Interactive presentation, axes, point cursors and bounded canonical history.
use super::*;
use core::num::NonZeroU32;

pub(super) fn history() -> ChartHistory {
    let mut history = ChartHistory::new(1, NonZeroU32::new(12).unwrap(), ChartSampleKind::Y);
    for value in [20, 33, 27, 56, 43, 70, 62, 79, 68, 84, 74, 92] {
        history.push_y(value).unwrap();
    }
    history
}
fn cursor(index: u32) -> ChartCursor {
    ChartCursor::new(
        1,
        BLUE,
        ChartCursorDirection::Both,
        ChartCursorTarget::Point { series: 1, index },
    )
}
fn plot(spec: ChartSpec, key: &'static str, height: i16) -> View<StudioAction> {
    chart(spec)
        .key(Key::Named(key))
        .width(percent(100))
        .height(px(height))
        .padding(10)
        .border(Color::rgb(0, 0, 0), 0)
        .background_opacity(0)
        .on_click(StudioAction::ChartNext)
}
pub(super) fn view(app: &Studio, theme: &Theme) -> View<StudioAction> {
    let kind = [
        ChartType::Line,
        ChartType::Bar,
        ChartType::Stacked,
        ChartType::Curve,
        ChartType::Scatter,
    ][app.chart_form];
    let data = if kind == ChartType::Scatter {
        ChartData::xy([(0, 20), (20, 70), (40, 35), (60, 85), (80, 55), (100, 90)])
    } else {
        ChartData::values([20, 70, 35, 85, 55, 90])
    };
    let mut description = ChartSpec::new(
        kind,
        ChartAxes::new(ChartRange::new(0, 100), ChartRange::new(0, 100)),
        vec![ChartSeries::new(1, ORANGE, data)],
    );
    let secondary = if kind == ChartType::Scatter {
        ChartData::xy([
            (1000, 100),
            (1200, 400),
            (1400, 180),
            (1600, 600),
            (1800, 300),
            (2000, 700),
        ])
    } else {
        ChartData::values([10, 40, 18, 60, 30, 70])
    };
    description.axes.secondary_x = ChartRange::new(1000, 2000);
    description.axes.secondary_y = ChartRange::new(0, 1000);
    description
        .series
        .push(ChartSeries::new(2, TEAL, secondary).axes(
            ChartAxis::Secondary,
            if kind == ChartType::Stacked {
                ChartAxis::Primary
            } else {
                ChartAxis::Secondary
            },
        ));
    description.cursors.push(cursor(app.chart_point % 6));
    let presentation = card(
        "Choose a presentation",
        "Click the plot to move its point cursor.",
        theme,
    )
    .height(px(260))
    .child(
        row([
            text(&format!("{kind:?} / point {}", app.chart_point % 6), theme),
            action_button("Next form", StudioAction::ChartForm, theme),
        ])
        .gap(16)
        .align_items(ItemAlign::Center),
    )
    .child(plot(description, "advanced-chart", 100));
    let mut description = ChartSpec::new(
        ChartType::Line,
        ChartAxes::new(ChartRange::new(0, 100), ChartRange::new(0, 100)),
        vec![ChartSeries::new(
            1,
            TEAL,
            ChartData::History(app.chart_history.snapshot()),
        )],
    );
    description.update_mode = if app.chart_circular {
        ChartUpdateMode::Circular
    } else {
        ChartUpdateMode::Shift
    };
    description.cursors.push(cursor(app.chart_point % 12));
    let history = card(
        "A bounded signal",
        "Add samples, skip a batch or start a new epoch.",
        theme,
    )
    .height(px(300))
    .child(
        row([
            action_button("Add sample", StudioAction::ChartAppend(1), theme),
            action_button("Burst 16", StudioAction::ChartAppend(16), theme),
            action_button("Clear history", StudioAction::ChartClear, theme),
            text("Circular", theme),
            switch(app.chart_circular).on_toggle(StudioAction::ChartCircular),
        ])
        .gap(10)
        .align_items(ItemAlign::Center),
    )
    .child(text(
        &format!(
            "epoch {} / {} retained / next {}",
            app.chart_history.epoch(),
            app.chart_history.len(),
            app.chart_history.next_sequence()
        ),
        theme,
    ))
    .child(plot(description, "history-chart", 110));
    studio_column().child(presentation).child(history)
}
