use eframe::egui_wgpu::{Callback, CallbackTrait};
use egui::{Color32, Context, Ui};

/// A canvas that allows drawing using OpenGL.
/// The content to draw must implement [CanvasContent] and be set in [WgpuCanvas::new].
pub struct WgpuCanvas<C: CanvasContent + 'static> {
    content: C,
}

impl<C: CanvasContent + 'static> WgpuCanvas<C> {
    /// Create a new [WgpuCanvas] that renders the specified content.
    pub fn new(content: C) -> Self {
        Self { content }
    }

    /// Draws this canvas.
    /// Takes remaining space of parent.
    /// Also requests a repaint immediately.
    pub fn draw(&mut self, ctx: &Context, ui: &mut Ui) {
        egui::Frame::canvas(ui.style())
            .fill(self.content.background_color())
            .show(ui, |ui| {
                let available = ui.available_size();
                let (_, rect) = ui.allocate_space(available);
                self.content.target_size(rect.size().into());
                ui.painter()
                    .add(Callback::new_paint_callback(rect, self.content.clone()));

                ctx.request_repaint();
            });
    }
}

pub trait CanvasContent: CallbackTrait + Clone {
    fn background_color(&self) -> Color32;
    fn target_size(&mut self, size: (f32, f32));
}

/// An empty canvas.
///
/// Draws nothing
#[derive(Clone, Copy)]
pub struct EmptyCanvas {}

#[allow(dead_code)]
impl EmptyCanvas {
    pub fn new() -> Self {
        Self {}
    }
}

impl CallbackTrait for EmptyCanvas {
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        _render_pass: &mut eframe::wgpu::RenderPass<'static>,
        _callback_resources: &eframe::egui_wgpu::CallbackResources,
    ) {
    }
}

impl CanvasContent for EmptyCanvas {
    fn background_color(&self) -> Color32 {
        Default::default()
    }

    fn target_size(&mut self, _size: (f32, f32)) {
        // Does not need target size
    }
}
