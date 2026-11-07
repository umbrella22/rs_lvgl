#![no_std]
extern crate alloc;
use rs_lvgl_ui::prelude::*;
pub fn description() -> ChartSpec {
    let mut history = ChartHistory::new(
        1,
        core::num::NonZeroU32::new(8).unwrap(),
        ChartSampleKind::Y,
    );
    let _ = history.push_y(30);
    let mut spec = ChartSpec::new(
        ChartType::Curve,
        ChartAxes::new(ChartRange::new(0, 100), ChartRange::new(-20, 100)),
        alloc::vec![ChartSeries::new(
            1,
            Color::rgb(200, 80, 30),
            ChartData::History(history.snapshot())
        )
        .axes(ChartAxis::Primary, ChartAxis::Secondary)],
    );
    spec.cursors.push(ChartCursor::new(
        1,
        Color::rgb(30, 200, 80),
        ChartCursorDirection::Both,
        ChartCursorTarget::Point {
            series: 1,
            index: 7,
        },
    ));
    spec
}
pub fn root() -> View<()> {
    rs_lvgl_ui::chart(description())
}
pub fn typed_and_map() -> View<u8> {
    typed::chart(description())
        .on_click(())
        .map(|()| 0)
        .padding(2)
        .into_view()
}
pub fn xy() -> View<()> {
    chart(ChartSpec::new(
        ChartType::Scatter,
        ChartAxes::new(ChartRange::new(0, 100), ChartRange::new(0, 100)),
        alloc::vec![ChartSeries::new(
            2,
            Color::rgb(20, 180, 80),
            ChartData::xy([(1, 2), (3, 4)])
        )],
    ))
}
