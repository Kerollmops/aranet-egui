#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::{f32, io, ops::RangeInclusive, path::Path};

use eframe::egui::{self, Color32, Response, ScrollArea, TextWrapMode, Vec2, Vec2b};
use egui_plot::{GridMark, Line, Plot, PlotPoint, PlotPoints};
use jiff::{Unit, fmt::strtime, tz::TimeZone};

const PLOT_WIDTH: f32 = 450.0;
const PLOT_HEIGHT: f32 = 120.0;
const PLOT_LINK_AXIS_NAME: &'static str = "linked";
const PLOT_SPACE: f32 = 8.0;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // .with_resizable(false)
            .with_inner_size([480.0, 720.0]),
        ..Default::default()
    };

    // Our application state:
    let mut linked_axes_demo = LinkedAxesDemo::default();
    linked_axes_demo.refresh().unwrap();

    eframe::run_simple_native("Aranet4", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Aranet4");
            if ui.button("Refresh").clicked() {
                linked_axes_demo.refresh().unwrap();
            }
            linked_axes_demo.ui(ui);
        });
    })
}

#[derive(Default)]
struct LinkedAxesDemo {
    records: Vec<Record>,
}

impl LinkedAxesDemo {
    fn refresh(&mut self) -> io::Result<()> {
        let path = Path::new("../aranet2sonnerie/measurements.son");
        let db = sonnerie::DatabaseReader::new(&path)?;
        let reader = db.get("aranet4");
        self.records = reader
            .into_iter()
            .map(Record::try_from)
            .collect::<io::Result<_>>()?;
        Ok(())
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Response {
        ScrollArea::horizontal()
            .show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                self.plot_temperature(ui);
                ui.add_space(PLOT_SPACE);
                self.plot_pressure(ui);
                ui.add_space(PLOT_SPACE);
                self.plot_co2(ui);
                ui.add_space(PLOT_SPACE);
                self.plot_humidity(ui);
                ui.add_space(PLOT_SPACE);
                self.plot_battery(ui)
            })
            .inner
    }

    fn plot_generic(
        &self,
        ui: &mut egui::Ui,
        title: &str,
        line_color: Color32,
        label_formatter: impl Fn(&str, &PlotPoint) -> String,
        y_axis_formatter: impl Fn(GridMark, &RangeInclusive<f64>) -> String,
        x_axis_formatter: impl Fn(GridMark, &RangeInclusive<f64>) -> String,
        show_y_axis: bool,
        record_point_extractor: impl Fn(&Record) -> f64,
    ) -> Response {
        let link_group_id = ui.id().with(PLOT_LINK_AXIS_NAME);

        let response = Plot::new(title)
            .width(PLOT_WIDTH)
            .height(PLOT_HEIGHT)
            .link_axis(link_group_id, Vec2b::new(true, false))
            .show_axes(Vec2b::new(show_y_axis, true))
            .link_cursor(link_group_id, Vec2b::new(true, false))
            .label_formatter(label_formatter)
            .x_axis_formatter(x_axis_formatter)
            .y_axis_min_width(60.0)
            .y_axis_formatter(y_axis_formatter)
            .show(ui, |plot_ui| {
                let data = self
                    .records
                    .iter()
                    .map(|record| [record.timestamp_millis(), record_point_extractor(record)])
                    .collect();
                let line = Line::new(title, PlotPoints::new(data))
                    .fill(-100.0)
                    .color(line_color);
                plot_ui.line(line);
            })
            .response;

        let plot_rect = response.rect;
        let label = egui::Label::new(
            egui::RichText::new(title)
                .size(14.0)
                .color(Color32::BLACK)
                .strong(),
        );
        let (_, galley, label_response) = label.layout_in_ui(ui);
        let mut bg_ui = ui.new_child(egui::UiBuilder::new());
        let external_rect = egui::Rect::from_two_pos(
            plot_rect.left_top() + Vec2::splat(10.),
            plot_rect.left_top()
                + Vec2::new(
                    label_response.rect.width() + 15.,
                    label_response.rect.height() + 14.,
                ),
        );
        if !ui.rect_contains_pointer(external_rect.scale_from_center(1.5)) {
            bg_ui.put(external_rect, |ui: &mut egui::Ui| {
                ui.painter().rect_filled(
                    ui.max_rect(),
                    egui::CornerRadius::default().at_least(2),
                    egui::Color32::from_white_alpha(0xD9),
                );
                ui.painter().add(egui::epaint::TextShape::new(
                    plot_rect.left_top() + Vec2::splat(12.0),
                    galley,
                    ui.style().visuals.text_color(),
                ));

                label_response
            });
        }

        response
    }

    fn plot_temperature(&self, ui: &mut egui::Ui) -> Response {
        self.plot_generic(
            ui,
            "Temperature (°C)",
            Color32::GREEN,
            |_name, point| {
                let nanosecond = (point.x * 1_000_000.0) as i128;
                let timestamp = jiff::Timestamp::from_nanosecond(nanosecond).unwrap();
                let timestamp = timestamp.round((Unit::Minute, 1)).unwrap_or(timestamp);
                let datetime = timestamp.to_zoned(TimeZone::UTC);
                let formatted = strtime::format("%a, %-d %b %Y %T", &datetime).unwrap();
                format!("temperature: {:.1}°C\nOn: {formatted}", point.y)
            },
            |mark, _range| format!("{:.0}°C", mark.value),
            |mark, _range| {
                let nanosecond = (mark.value * 1_000_000.0) as i128;
                let timestamp = jiff::Timestamp::from_nanosecond(nanosecond).unwrap();
                format!("{timestamp:#}")
            },
            false,
            |record| record.celsius as f64,
        )
    }

    fn plot_pressure(&self, ui: &mut egui::Ui) -> Response {
        self.plot_generic(
            ui,
            "Pressure (hPa)",
            Color32::BLUE,
            |_name, point| {
                let nanosecond = (point.x * 1_000_000.0) as i128;
                let timestamp = jiff::Timestamp::from_nanosecond(nanosecond).unwrap();
                let timestamp = timestamp.round((Unit::Minute, 1)).unwrap_or(timestamp);
                let datetime = timestamp.to_zoned(TimeZone::UTC);
                let formatted = strtime::format("%a, %-d %b %Y %T", &datetime).unwrap();
                format!("pressure: {:.1} hPa\nOn: {formatted}", point.y)
            },
            |mark, _range| format!("{:.0} hPa", mark.value),
            |mark, _range| {
                let nanosecond = (mark.value * 1_000_000.0) as i128;
                let timestamp = jiff::Timestamp::from_nanosecond(nanosecond).unwrap();
                format!("{timestamp:#}")
            },
            false,
            |record| record.pressure as f64,
        )
    }

    fn plot_co2(&self, ui: &mut egui::Ui) -> Response {
        self.plot_generic(
            ui,
            "CO₂ (ppm)",
            Color32::WHITE,
            |_name, point| {
                let nanosecond = (point.x * 1_000_000.0) as i128;
                let timestamp = jiff::Timestamp::from_nanosecond(nanosecond).unwrap();
                let timestamp = timestamp.round((Unit::Minute, 1)).unwrap_or(timestamp);
                let datetime = timestamp.to_zoned(TimeZone::UTC);
                let formatted = strtime::format("%a, %-d %b %Y %T", &datetime).unwrap();
                format!("CO₂: {:.1} ppm\nOn: {formatted}", point.y)
            },
            |mark, _range| format!("{:.0} ppm", mark.value),
            |mark, _range| {
                let nanosecond = (mark.value * 1_000_000.0) as i128;
                let timestamp = jiff::Timestamp::from_nanosecond(nanosecond).unwrap();
                format!("{timestamp:#}")
            },
            false,
            |record| record.co2_level as f64,
        )
    }

    fn plot_humidity(&self, ui: &mut egui::Ui) -> Response {
        self.plot_generic(
            ui,
            "Humidity (%)",
            Color32::CYAN,
            |_name, point| {
                let nanosecond = (point.x * 1_000_000.0) as i128;
                let timestamp = jiff::Timestamp::from_nanosecond(nanosecond).unwrap();
                let timestamp = timestamp.round((Unit::Minute, 1)).unwrap_or(timestamp);
                let datetime = timestamp.to_zoned(TimeZone::UTC);
                let formatted = strtime::format("%a, %-d %b %Y %T", &datetime).unwrap();
                format!("Humidity: {:.1} %\nOn: {formatted}", point.y)
            },
            |mark, _range| format!("{:.0} %", mark.value),
            |mark, _range| {
                let nanosecond = (mark.value * 1_000_000.0) as i128;
                let timestamp = jiff::Timestamp::from_nanosecond(nanosecond).unwrap();
                format!("{timestamp:#}")
            },
            false,
            |record| record.humidity as f64,
        )
    }

    fn plot_battery(&self, ui: &mut egui::Ui) -> Response {
        self.plot_generic(
            ui,
            "Battery (%)",
            Color32::YELLOW,
            |_name, point| {
                let nanosecond = (point.x * 1_000_000.0) as i128;
                let timestamp = jiff::Timestamp::from_nanosecond(nanosecond).unwrap();
                let timestamp = timestamp.round((Unit::Minute, 1)).unwrap_or(timestamp);
                let datetime = timestamp.to_zoned(TimeZone::UTC);
                let formatted = strtime::format("%a, %-d %b %Y %T", &datetime).unwrap();
                format!("Battery: {:.1} %\nOn: {formatted}", point.y)
            },
            |mark, _range| format!("{:.0} %", mark.value),
            |mark, _range| {
                let nanosecond = (mark.value * 1_000_000.0) as i128;
                let timestamp = jiff::Timestamp::from_nanosecond(nanosecond).unwrap();
                format!("{timestamp:#}")
            },
            true,
            |record| record.battery as f64,
        )
    }
}

struct Record {
    timestamp_nanos: u64,
    celsius: f32,
    pressure: f32,
    co2_level: u32,
    humidity: u32,
    battery: u32,
}

impl Record {
    fn timestamp_millis(&self) -> f64 {
        self.timestamp_nanos as f64 / 1_000_000.0
    }
}

impl TryFrom<sonnerie::Record> for Record {
    type Error = io::Error;

    fn try_from(value: sonnerie::Record) -> Result<Self, Self::Error> {
        Ok(Record {
            timestamp_nanos: value.timestamp_nanos(),
            celsius: value.get_checked::<f32>(0).unwrap(),
            pressure: value.get_checked::<f32>(1).unwrap(),
            co2_level: value.get_checked::<u32>(2).unwrap(),
            humidity: value.get_checked::<u32>(3).unwrap(),
            battery: value.get_checked::<u32>(4).unwrap(),
        })
    }
}
