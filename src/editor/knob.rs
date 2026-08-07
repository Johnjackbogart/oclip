//! A minimal rotary knob widget. `nice-plug-egui` only ships slider widgets, so this fills the
//! gap for a more typical plugin look.
//!
//! The widget itself is generic over [`KnobValue`] rather than being tied directly to
//! `nice_plug`'s `Param`/`ParamSetter` types. That indirection exists so the exact same widget
//! code (including its drag/click handling, which drives the same baseview window/view machinery
//! implicated in the macOS GUI crash) can be exercised from `examples/editor_preview.rs` — a
//! standalone window with no host or real params — as well as from the real plugin editor.

use nice_plug::context::gui::ParamSetter;
use nice_plug::params::Param;

/// Angle (radians) where the knob's travel starts, measured with egui's y-down convention
/// (0 = pointing right, increasing clockwise). 135 degrees puts the start at the lower-left.
const ANGLE_START: f32 = std::f32::consts::PI * 0.75;
/// 270 degrees of total travel, ending at the lower-right.
const ANGLE_SWEEP: f32 = std::f32::consts::PI * 1.5;
/// Number of segments used to approximate the value arc as a polyline.
const ARC_SEGMENTS: usize = 48;

/// Everything [`Knob`] needs from whatever it's bound to: a real `nice_plug` parameter in the
/// plugin editor, or a plain mock value in the standalone preview harness.
pub trait KnobValue {
    /// Accessible name, e.g. "Gain".
    fn name(&self) -> String;
    /// Current normalized `[0, 1]` value.
    fn normalized(&self) -> f32;
    /// Normalized `[0, 1]` default value, used by double-click-to-reset.
    fn default_normalized(&self) -> f32;
    /// Human-readable current value with unit, e.g. "-3.00 dB", shown on hover.
    fn display(&self) -> String;

    fn begin_set(&mut self);
    fn set_normalized(&mut self, value: f32);
    fn end_set(&mut self);
}

/// Binds [`KnobValue`] to a real `nice_plug` parameter and its setter.
pub struct ParamKnobValue<'a, P: Param> {
    param: &'a P,
    setter: &'a ParamSetter<'a>,
}

impl<P: Param> KnobValue for ParamKnobValue<'_, P> {
    fn name(&self) -> String {
        self.param.name().to_string()
    }

    fn normalized(&self) -> f32 {
        self.param.unmodulated_normalized_value()
    }

    fn default_normalized(&self) -> f32 {
        self.param.default_normalized_value()
    }

    fn display(&self) -> String {
        self.param.to_string()
    }

    fn begin_set(&mut self) {
        self.setter.begin_set_parameter(self.param);
    }

    fn set_normalized(&mut self, value: f32) {
        self.setter.set_parameter_normalized(self.param, value);
    }

    fn end_set(&mut self) {
        self.setter.end_set_parameter(self.param);
    }
}

pub struct Knob<V> {
    value: V,
    diameter: f32,
}

impl<V: KnobValue> Knob<V> {
    pub fn new(value: V) -> Self {
        Self {
            value,
            diameter: 48.0,
        }
    }
}

impl<'a, P: Param> Knob<ParamKnobValue<'a, P>> {
    pub fn for_param(param: &'a P, setter: &'a ParamSetter<'a>) -> Self {
        Knob::new(ParamKnobValue { param, setter })
    }
}

impl<V: KnobValue> egui::Widget for Knob<V> {
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = egui::Vec2::splat(self.diameter);
        let (rect, response) =
            ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

        if response.drag_started() {
            self.value.begin_set();
        }
        if response.dragged() {
            // Dragging up increases the value; shift for fine control.
            let delta = -response.drag_delta().y;
            let speed = if ui.input(|i| i.modifiers.shift) {
                0.0015
            } else {
                0.005
            };
            let new_value = (self.value.normalized() + delta * speed).clamp(0.0, 1.0);
            self.value.set_normalized(new_value);
        }
        if response.drag_stopped() {
            self.value.end_set();
        }
        if response.double_clicked() {
            self.value.begin_set();
            self.value.set_normalized(self.value.default_normalized());
            self.value.end_set();
        }

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);
            let painter = ui.painter();
            let center = rect.center();
            let radius = rect.width() * 0.5;
            let normalized = self.value.normalized();

            painter.circle(
                center,
                radius * 0.72,
                ui.visuals().widgets.inactive.bg_fill,
                egui::Stroke::NONE,
            );

            let track_stroke = egui::Stroke::new(3.0, ui.visuals().widgets.inactive.bg_fill);
            painter.add(egui::Shape::line(
                arc_points(center, radius, ANGLE_START, ANGLE_START + ANGLE_SWEEP),
                track_stroke,
            ));

            let value_stroke = egui::Stroke::new(3.0, visuals.fg_stroke.color);
            painter.add(egui::Shape::line(
                arc_points(center, radius, ANGLE_START, ANGLE_START + normalized * ANGLE_SWEEP),
                value_stroke,
            ));

            let angle = ANGLE_START + normalized * ANGLE_SWEEP;
            let indicator_end = center + egui::Vec2::new(angle.cos(), angle.sin()) * (radius * 0.62);
            painter.line_segment(
                [center, indicator_end],
                egui::Stroke::new(2.0, ui.visuals().widgets.inactive.bg_fill),
            );
        }

        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, self.value.name())
        });
        response.on_hover_text(self.value.display())
    }
}

/// Points along a circular arc from `start_angle` to `end_angle`, suitable for
/// [`egui::Shape::line`] — egui's `Painter` has no native arc-stroke primitive.
fn arc_points(center: egui::Pos2, radius: f32, start_angle: f32, end_angle: f32) -> Vec<egui::Pos2> {
    (0..=ARC_SEGMENTS)
        .map(|i| {
            let t = i as f32 / ARC_SEGMENTS as f32;
            let angle = start_angle + (end_angle - start_angle) * t;
            center + egui::Vec2::new(angle.cos(), angle.sin()) * radius
        })
        .collect()
}
