//! A minimal rotary knob widget bound to a `nice_plug` parameter. `nice-plug-egui` only ships
//! slider widgets, so this fills the gap for a more typical plugin look.

use nice_plug::context::gui::ParamSetter;
use nice_plug::params::Param;

/// Angle (radians) where the knob's travel starts, measured with egui's y-down convention
/// (0 = pointing right, increasing clockwise). 135 degrees puts the start at the lower-left.
const ANGLE_START: f32 = std::f32::consts::PI * 0.75;
/// 270 degrees of total travel, ending at the lower-right.
const ANGLE_SWEEP: f32 = std::f32::consts::PI * 1.5;

pub struct Knob<'a, P: Param> {
    param: &'a P,
    setter: &'a ParamSetter<'a>,
    diameter: f32,
}

impl<'a, P: Param> Knob<'a, P> {
    pub fn for_param(param: &'a P, setter: &'a ParamSetter<'a>) -> Self {
        Self {
            param,
            setter,
            diameter: 48.0,
        }
    }
}

impl<P: Param> egui::Widget for Knob<'_, P> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = egui::Vec2::splat(self.diameter);
        let (rect, response) =
            ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

        if response.drag_started() {
            self.setter.begin_set_parameter(self.param);
        }
        if response.dragged() {
            // Dragging up increases the value; shift for fine control.
            let delta = -response.drag_delta().y;
            let speed = if ui.input(|i| i.modifiers.shift) {
                0.0015
            } else {
                0.005
            };
            let current = self.param.unmodulated_normalized_value();
            let new_value = (current + delta * speed).clamp(0.0, 1.0);
            self.setter.set_parameter_normalized(self.param, new_value);
        }
        if response.drag_stopped() {
            self.setter.end_set_parameter(self.param);
        }
        if response.double_clicked() {
            self.setter.begin_set_parameter(self.param);
            self.setter
                .set_parameter_normalized(self.param, self.param.default_normalized_value());
            self.setter.end_set_parameter(self.param);
        }

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);
            let painter = ui.painter();
            let center = rect.center();
            let radius = rect.width() * 0.5;

            painter.circle(
                center,
                radius,
                ui.visuals().widgets.inactive.bg_fill,
                visuals.fg_stroke,
            );

            let normalized = self.param.unmodulated_normalized_value();
            let angle = ANGLE_START + normalized * ANGLE_SWEEP;
            let indicator_end =
                center + egui::Vec2::new(angle.cos(), angle.sin()) * (radius * 0.85);
            painter.line_segment(
                [center, indicator_end],
                egui::Stroke::new(2.0, visuals.fg_stroke.color),
            );
        }

        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, self.param.name()));
        response.on_hover_text(self.param.to_string())
    }
}
