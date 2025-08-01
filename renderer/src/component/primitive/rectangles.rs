use naga_oil::compose::Composer;
use wgpu::{Device, RenderPass, TextureFormat};

use crate::{buffer_updater::BufferUpdater, globals::Globals, viewport::Viewport};

use super::lines::{LineSpec, Lines};

#[derive(Clone, Copy, Debug)]
pub struct RectangleSpec {
    // Position of the rectangle
    pub start: [f32; 2],
    // Size of the rectangle
    pub size: [f32; 2],
    /// The color of the line
    pub color: [u8; 4],
    /// The width of the line
    pub width: f32,
    /// The length of a dash-segment (both drawn and non-drawn)
    pub segment_length: f32,
    /// The duty-cycle of a dash-segment (how much of the segment should be drawn)
    pub duty: f32,
}

/// A [Component] which draws one or multiple rectangles to the screen
pub struct Rectangles(Lines);

impl Rectangles {
    pub fn new(
        device: &Device,
        format: TextureFormat,
        globals: &Globals,
        viewport: &Viewport,
        shader_composer: &mut Composer,
        rectangles: impl IntoIterator<Item = RectangleSpec>,
    ) -> Self {
        Self(Lines::new(
            device,
            format,
            globals,
            viewport,
            shader_composer,
            &rectangles_to_lines(rectangles),
        ))
    }

    /// Update this component to have the new `spec`
    pub fn update<U: BufferUpdater>(
        &mut self,
        updater: &mut U,
        spec: impl IntoIterator<Item = RectangleSpec>,
    ) {
        self.0.update(updater, &rectangles_to_lines(spec));
    }

    /// Draws this component
    #[inline]
    pub fn draw(&self, render_pass: &mut RenderPass<'_>) {
        self.0.draw(render_pass);
    }
}

/// Converts a slice of [RectangleSpec]s to a [Vec] of [LineSpec]s
fn rectangles_to_lines(rectangles: impl IntoIterator<Item = RectangleSpec>) -> Vec<LineSpec> {
    rectangles
        .into_iter()
        .flat_map(
            |RectangleSpec {
                 start: [x, y],
                 size: [w, h],
                 color,
                 width,
                 segment_length,
                 duty,
             }| {
                // Offset positions by half line-width to prevent ugly corners
                let delta = width / 2.;
                // +----->
                // |      ^
                // v      |
                //  <-----+
                [
                    LineSpec {
                        start: [x - delta, y],
                        end: [x + w + delta, y],
                        color,
                        width,
                        segment_length,
                        duty,
                    },
                    LineSpec {
                        end: [x + w, y - delta],
                        start: [x + w, y + h + delta],
                        color,
                        width,
                        segment_length,
                        duty,
                    },
                    LineSpec {
                        start: [x + w + delta, y + h],
                        end: [x - delta, y + h],
                        color,
                        width,
                        segment_length,
                        duty,
                    },
                    LineSpec {
                        end: [x, y + h + delta],
                        start: [x, y - delta],
                        color,
                        width,
                        segment_length,
                        duty,
                    },
                ]
            },
        )
        .collect()
}
