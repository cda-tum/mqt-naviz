use core::str;
#[cfg(not(target_arch = "wasm32"))]
use std::{sync::mpsc::channel, thread};

use eframe::egui_wgpu::CallbackTrait;
use log::error;
use naviz_parser::config::{machine::MachineConfig, visual::VisualConfig};
use naviz_renderer::renderer::Renderer;
use naviz_repository::Repository;
use naviz_state::{config::Config, state::State};
#[cfg(not(target_arch = "wasm32"))]
use naviz_video::VideoExport;

use crate::{
    animator_adapter::{AnimatorAdapter, AnimatorState},
    aspect_panel::AspectPanel,
    canvas::{CanvasContent, EmptyCanvas, WgpuCanvas},
    current_machine::CurrentMachine,
    future_helper::FutureHelper,
    import::{ImportError, ImportOptions},
    menu::{FileType, MenuBar, MenuConfig, MenuEvent},
    util::WEB,
};

/// The main App to draw using [egui]/[eframe]
pub struct App {
    future_helper: FutureHelper,
    menu_bar: MenuBar,
    animator_adapter: AnimatorAdapter,
    machine_repository: Repository,
    style_repository: Repository,
    current_machine: CurrentMachine,
}

#[derive(Default)]
pub struct InitOptions<'a> {
    /// The machine-id to load
    machine: Option<&'a str>,
    /// The style-id to load
    style: Option<&'a str>,
    /// The visualization input to load.
    /// Pass [Some] [ImportOptions] if the content needs to be imported.
    input: Option<(Option<ImportOptions>, &'a [u8])>,
}

impl App {
    /// Create a new instance of the [App]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        RendererAdapter::setup(cc);

        let mut machine_repository = Repository::empty()
            .bundled_machines()
            .expect("Failed to load bundled machines");
        let mut style_repository = Repository::empty()
            .bundled_styles()
            .expect("Failed to load bundled styles");

        // Load user-dirs only on non-web builds as there is no filesystem on web
        if !WEB {
            machine_repository = machine_repository
                .user_dir_machines()
                .expect("Failed to load machines from user dir");
            style_repository = style_repository
                .user_dir_styles()
                .expect("Failed to load styles from user dir");
        }

        let mut app = Self {
            future_helper: FutureHelper::new().expect("Failed to create FutureHelper"),
            menu_bar: MenuBar::new(),
            animator_adapter: AnimatorAdapter::default(),
            machine_repository,
            style_repository,
            current_machine: Default::default(),
        };

        app.update_machines();
        app.update_styles();

        // Load any machine as default (if any machine is available)
        if let Some((id, machine)) = app.machine_repository.try_get_any() {
            app.set_loaded_machine(Some(id.to_string()), machine);
        }
        // Load any style as default (if any style is available)
        if let Some((id, style)) = app.style_repository.try_get_any() {
            app.set_loaded_style(Some(id.to_string()), style);
        }

        app
    }

    /// Create a new instance of the [App] with the specified [InitOptions]
    pub fn new_with_init(cc: &eframe::CreationContext<'_>, init_options: InitOptions<'_>) -> Self {
        let mut app = Self::new(cc);

        if let Some((import_options, data)) = init_options.input {
            match import_options {
                Some(import_options) => app.import(import_options, data).expect("Failed to import"),
                None => app.open(data),
            }
        }

        if let Some(machine_id) = init_options.machine {
            app.set_machine(machine_id);
        }
        if let Some(style_id) = init_options.style {
            app.set_style(style_id);
        }

        app
    }

    /// Import the instructions from `data` using the specified [ImportOptions]
    pub fn import(
        &mut self,
        import_options: ImportOptions,
        data: &[u8],
    ) -> Result<(), ImportError> {
        let instructions = import_options.import(data)?;
        self.animator_adapter.set_instructions(instructions);
        self.update_compatible_machines();
        Ok(())
    }

    /// Open the naviz-instructions from `data`
    pub fn open(&mut self, data: &[u8]) {
        let input =
            naviz_parser::input::lexer::lex(str::from_utf8(data).unwrap()).expect("Failed to lex");
        let input = naviz_parser::input::parser::parse(&input).expect("Failed to parse");
        let input = naviz_parser::input::concrete::Instructions::new(input)
            .expect("Failed to convert to instructions");
        self.animator_adapter.set_instructions(input);
        self.update_compatible_machines();
        self.select_compatible_machine();
    }

    /// Selects any compatible machine for the currently opened machine.
    /// Returns `true` if a compatible machine could be found and was loaded,
    /// or `false` otherwise.
    pub fn select_compatible_machine(&mut self) -> bool {
        if let Some(instructions) = self.animator_adapter.get_instructions() {
            if instructions.directives.targets.is_empty() {
                // No targets specified => no machine is compatible => cannot load any
                return false;
            }

            if self
                .current_machine
                .compatible_with(&instructions.directives.targets)
            {
                // compatible machine already loaded
                return true;
            }

            // Machine is not compatible or not set => load compatible machine

            // Find some compatible machine
            let compatible_machine = instructions
                .directives
                .targets
                .iter()
                .find(|id| self.machine_repository.has(id));
            if let Some(id) = compatible_machine {
                // compatible machine exists => load machine
                self.set_machine(id.clone().as_str());
                return true;
            }

            // failed to find a compatible machine
            return false;
        }
        // No instructions loaded =>cannot set any compatible machine
        false
    }

    /// Sets the machine to the one with the specified `id`
    pub fn set_machine(&mut self, id: impl Into<String>) {
        let id = id.into();
        let machine = self.machine_repository.get(&id).expect("Not found");
        self.set_loaded_machine(Some(id), machine.expect("Failed to load machine"));
    }

    /// Sets the current machine to `machine` with the optional `id`.
    /// If `id` is [None], the machine is assumed to be set manually.
    fn set_loaded_machine(&mut self, id: Option<impl Into<String>>, machine: MachineConfig) {
        let id = id.map(Into::into);
        self.current_machine = id
            .clone()
            .map(CurrentMachine::Id)
            .unwrap_or(CurrentMachine::Manual);
        self.menu_bar.set_selected_machine(id);
        self.animator_adapter.set_machine_config(machine);
    }

    /// Set the current machine to the one specified in `data`.
    pub fn set_machine_manually(&mut self, data: &[u8]) {
        let machine =
            naviz_parser::config::lexer::lex(str::from_utf8(data).unwrap()).expect("Failed to lex");
        let machine = naviz_parser::config::parser::parse(&machine).expect("Failed to parse");
        let machine: naviz_parser::config::generic::Config = machine.into();
        let machine: MachineConfig = machine
            .try_into()
            .expect("Failed to convert to machine-config");
        self.set_loaded_machine(None::<String>, machine);
    }

    /// Sets the style to the one with the specified `id`
    pub fn set_style(&mut self, id: impl Into<String>) {
        let id = id.into();
        let style = self.style_repository.get(&id).expect("Not found");
        self.set_loaded_style(Some(id), style.expect("Failed to load style"));
    }

    /// Sets the current style to `style` with the optional `id`.
    /// If `id` is [None], the style is assumed to be set manually.
    fn set_loaded_style(&mut self, id: Option<impl Into<String>>, style: VisualConfig) {
        self.menu_bar.set_selected_style(id.map(Into::into));
        self.animator_adapter.set_visual_config(style);
    }

    /// Set the current style to the one specified in `data`.
    pub fn set_style_manually(&mut self, data: &[u8]) {
        let visual =
            naviz_parser::config::lexer::lex(str::from_utf8(data).unwrap()).expect("Failed to lex");
        let visual = naviz_parser::config::parser::parse(&visual).expect("Failed to parse");
        let visual: naviz_parser::config::generic::Config = visual.into();
        let visual: VisualConfig = visual
            .try_into()
            .expect("Failed to convert to visual-config");
        self.set_loaded_style(None::<String>, visual);
    }

    /// Update the machines displayed in the menu from the repository
    fn update_machines(&mut self) {
        self.menu_bar.update_machines(
            self.machine_repository
                .list()
                .into_iter()
                .map(|(a, b)| (a.to_owned(), b.to_owned()))
                .collect(),
        );
    }

    /// Update the compatible machines to be the ones specified in the currently loaded instructions
    fn update_compatible_machines(&mut self) {
        self.menu_bar.set_compatible_machines(
            self.animator_adapter
                .get_instructions()
                .map(|x| x.directives.targets.as_slice())
                .unwrap_or(&[]),
        );
    }

    /// Update the styles displayed in the menu from the repository
    fn update_styles(&mut self) {
        self.menu_bar.update_styles(
            self.style_repository
                .list()
                .into_iter()
                .map(|(a, b)| (a.to_owned(), b.to_owned()))
                .collect(),
        );
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if a new file was read
        if let Ok(event) = self.menu_bar.events().try_recv() {
            match event {
                MenuEvent::FileOpen(FileType::Instructions, content) => {
                    self.open(&content);
                }
                MenuEvent::FileImport(options, content) => {
                    self.import(options, &content).expect("Failed to import");
                }
                MenuEvent::FileOpen(FileType::Machine, content) => {
                    self.set_machine_manually(&content);
                }
                MenuEvent::FileOpen(FileType::Style, content) => {
                    self.set_style_manually(&content);
                }
                #[cfg(not(target_arch = "wasm32"))]
                MenuEvent::ExportVideo {
                    target,
                    resolution,
                    fps,
                    progress,
                } => {
                    if let Some(animator) = self.animator_adapter.animator() {
                        let video = VideoExport::new(animator, resolution, fps);
                        let (tx, rx) = channel();
                        self.future_helper.execute_to(video, tx);
                        thread::spawn(move || {
                            let mut video = rx.recv().unwrap();
                            video.export_video(&target, progress);
                        });
                    }
                }
                MenuEvent::SetMachine(id) => {
                    self.set_machine(id);
                }
                MenuEvent::SetStyle(id) => {
                    self.set_style(id);
                }
                #[cfg(not(target_arch = "wasm32"))]
                MenuEvent::ImportMachine(file) => {
                    self.machine_repository
                        .import_machine_to_user_dir(&file)
                        .expect("Failed to import machine");
                    self.update_machines();
                }
                #[cfg(not(target_arch = "wasm32"))]
                MenuEvent::ImportStyle(file) => {
                    self.style_repository
                        .import_style_to_user_dir(&file)
                        .expect("Failed to import style");
                    self.update_styles();
                }
            }
        }

        // Menu
        egui::TopBottomPanel::top("app_menu").show(ctx, |ui| {
            self.menu_bar.draw(
                MenuConfig {
                    export: self.animator_adapter.all_inputs_set(),
                },
                &self.future_helper,
                ctx,
                ui,
            )
        });

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            let padding = ui.style().spacing.item_spacing.y;
            let (_, space) = ui.allocate_space(ui.available_size());
            let panel = AspectPanel {
                space,
                aspect_ratio: 16. / 9.,
                top: 0.,
                bottom: 20. + padding,
                left: 0.,
                right: 0.,
            };
            let animator_state = self.animator_adapter.get();
            panel.draw(
                ui,
                |ui| {
                    if let Some(animator_state) = animator_state {
                        WgpuCanvas::new(RendererAdapter::new(animator_state)).draw(ctx, ui);
                    } else {
                        // Animator is not ready (something missing) => empty canvas
                        WgpuCanvas::new(EmptyCanvas::new()).draw(ctx, ui);
                    }
                },
                |_| {},
                |_| {},
                |ui| {
                    ui.add_space(padding);
                    self.animator_adapter.draw_progress_bar(ui);
                },
                |_| {},
            );
        });
    }
}

/// An adapter from [naviz_renderer] to [CallbackTrait].
///
/// Setup the renderer using [RendererAdapter::setup]
/// before drawing the renderer using the callback implementation.
#[derive(Clone)]
struct RendererAdapter {
    size: (f32, f32),
    /// The animator_state to render
    animator_state: AnimatorState,
}

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
                &wgpu_render_state.queue,
                wgpu_render_state.target_format,
                &Config::example(),
                &State::example(),
                (1920, 1080), // Use some default resolution to create renderer, as the canvas-resolution is not yet known
            ));
    }

    /// Creates a new [RendererAdapter] from the passed [AnimatorState]
    pub fn new(animator_state: AnimatorState) -> Self {
        Self {
            animator_state,
            size: Default::default(),
        }
    }
}

impl CallbackTrait for RendererAdapter {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_descriptor: &eframe::egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut eframe::egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(r) = callback_resources.get_mut::<Renderer>() {
            r.update_viewport(
                device,
                queue,
                (
                    (self.size.0 * screen_descriptor.pixels_per_point) as u32,
                    (self.size.1 * screen_descriptor.pixels_per_point) as u32,
                ),
            );
            self.animator_state
                .update(r, &mut (device, queue), device, queue);
        } else {
            error!("Failed to get renderer");
        }
        Vec::new()
    }

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

impl CanvasContent for RendererAdapter {
    fn background_color(&self) -> egui::Color32 {
        let [r, g, b, a] = self.animator_state.background();
        egui::Color32::from_rgba_unmultiplied(r, g, b, a)
    }

    fn target_size(&mut self, size: (f32, f32)) {
        self.size = size;
    }
}
