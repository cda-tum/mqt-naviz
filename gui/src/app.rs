use eframe::egui_wgpu::CallbackTrait;
use log::error;
use naviz_renderer::renderer::Renderer;

use crate::{canvas::WgpuCanvas, future_helper::FutureHelper, menu::MenuBar};

/// The main App to draw using [egui]/[eframe]
pub struct App {
    future_helper: FutureHelper,
    menu_bar: MenuBar,
    /// The contents of the currently opened file
    file_contents: Vec<u8>,
}

impl App {
    /// Create a new instance of the [App]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        RendererAdapter::setup(cc);

        Self {
            future_helper: FutureHelper::new().expect("Failed to create FutureHelper"),
            menu_bar: MenuBar::new(),
            file_contents: Vec::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if a new file was read
        if let Ok(c) = self.menu_bar.file_open_channel().try_recv() {
            self.file_contents = c;
        }

        // Menu
        egui::TopBottomPanel::top("app_menu")
            .show(ctx, |ui| self.menu_bar.draw(&self.future_helper, ctx, ui));

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!(
                "File: {:?}{}",
                &self.file_contents[..32.min(self.file_contents.len())],
                if self.file_contents.len() > 32 {
                    " (truncated)"
                } else {
                    ""
                }
            ));

            WgpuCanvas::new(RendererAdapter()).draw(ctx, ui);
        });
    }
}

/// An adapter from [naviz_renderer] to [CallbackTrait].
///
/// Setup the renderer using [RendererAdapter::setup]
/// before drawing the renderer using the callback implementation.
#[derive(Clone)]
struct RendererAdapter();

impl RendererAdapter {
    /// Creates a [Renderer] and stores it in the egui [RenderState][eframe::egui_wgpu::RenderState].
    /// This created renderer will later be rendered from [RendererAdapter::paint].
    ///
    /// The renderer is stored in the renderer state
    /// in order for the graphics pipeline to have the same lifetime as the egui render pass.
    /// See [this section from the egui demo][https://github.com/emilk/egui/blob/0.28.1/crates/egui_demo_app/src/apps/custom3d_wgpu.rs#L83-L85]
    pub fn setup(cc: &eframe::CreationContext<'_>) {
        let wgpu_render_state = cc
            .wgpu_render_state
            .as_ref()
            .expect("No wgpu render state found");

        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert(Renderer::new(
                &wgpu_render_state.device,
                wgpu_render_state.target_format,
            ));
    }
}

impl CallbackTrait for RendererAdapter {
    fn paint<'a>(
        &'a self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'a>,
        callback_resources: &'a eframe::egui_wgpu::CallbackResources,
    ) {
        if let Some(r) = callback_resources.get::<Renderer>() {
            r.draw(render_pass);
        } else {
            error!("Failed to get renderer");
        }
    }
}
