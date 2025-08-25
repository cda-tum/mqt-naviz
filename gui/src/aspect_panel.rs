use egui::{Pos2, Rect, Ui, UiBuilder, Vec2};

/// A panel that takes up as much space as possible
/// while keeping the aspect ratio of the content
/// and allowing fixed-size components to the sides
pub struct AspectPanel {
    pub space: Rect,
    pub aspect_ratio: f32,
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl AspectPanel {
    /// Draws this panel with the specified UI-closures
    pub fn draw(
        &self,
        ui: &mut Ui,
        content: impl FnOnce(&mut Ui),
        top: impl FnOnce(&mut Ui),
        right: impl FnOnce(&mut Ui),
        bottom: impl FnOnce(&mut Ui),
        left: impl FnOnce(&mut Ui),
    ) {
        let max_width = self.space.width();
        let max_height = self.space.height();

        let content_max_width = max_width - self.left - self.right;
        let content_max_height = max_height - self.top - self.bottom;

        let content_size =
            constrain_to_aspect(content_max_width, content_max_height, self.aspect_ratio);

        let padding_x = max_width - (content_size.x + self.left + self.right);
        let padding_y = max_height - (content_size.y + self.top + self.bottom);

        let content_pos = Pos2 {
            x: padding_x / 2. + self.left,
            y: padding_y / 2. + self.top,
        };

        let content_rect = Rect::from_min_size(content_pos, content_size);
        let top_rect = Rect::from_min_size(
            (content_pos.x, padding_y).into(),
            (content_size.x, self.top).into(),
        );
        let right_rect = Rect::from_min_size(
            (content_pos.x + content_size.x, content_pos.y).into(),
            (self.right, content_size.y).into(),
        );
        let bottom_rect = Rect::from_min_size(
            (content_pos.x, content_pos.y + content_size.y).into(),
            (content_size.x, self.bottom).into(),
        );
        let left_rect = Rect::from_min_size(
            (padding_x, content_pos.y).into(),
            (self.left, content_size.y).into(),
        );

        ui.scope_builder(
            UiBuilder::new().max_rect(content_rect.translate(self.space.min.to_vec2())),
            content,
        );
        ui.scope_builder(
            UiBuilder::new().max_rect(top_rect.translate(self.space.min.to_vec2())),
            top,
        );
        ui.scope_builder(
            UiBuilder::new().max_rect(right_rect.translate(self.space.min.to_vec2())),
            right,
        );
        ui.scope_builder(
            UiBuilder::new().max_rect(bottom_rect.translate(self.space.min.to_vec2())),
            bottom,
        );
        ui.scope_builder(
            UiBuilder::new().max_rect(left_rect.translate(self.space.min.to_vec2())),
            left,
        );
    }
}

/// constrains the passed size to be the passed `aspect`.
/// Will shrink one of the dimensions if needed.
fn constrain_to_aspect(mut w: f32, mut h: f32, aspect: f32) -> Vec2 {
    match (w.is_finite(), h.is_finite()) {
        (true, true) => {
            if w / aspect < h {
                h = w / aspect;
            }
            if h * aspect < w {
                w = h * aspect;
            }
        }
        (false, true) => w = h * aspect,
        (true, false) => h = w / aspect,
        (false, false) => { /* Infinite in both directions => always correct aspect ratio */ }
    }
    Vec2 { x: w, y: h }
}
