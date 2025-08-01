use naviz_state::{config::Config, state::State};
use wgpu::{Device, Queue, RenderPass, TextureFormat};

use crate::{
    buffer_updater::BufferUpdater,
    component::{
        atoms::Atoms, legend::Legend, machine::Machine, time::Time, updatable::Updatable,
        ComponentInit,
    },
    globals::Globals,
    layout::Layout,
    shaders::{create_composer, load_default_shaders},
    viewport::ViewportSource,
};

/// The main renderer, which renders the visualization output
pub struct Renderer {
    globals: Globals,

    machine: Machine,
    atoms: Atoms,
    legend: Legend,
    time: Time,
    screen_resolution: (u32, u32),
}

impl Renderer {
    /// Creates a new [Renderer] on the passed [Device] and for the passed [TextureFormat]
    pub fn new(
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
        config: &Config,
        state: &State,
        screen_resolution: (u32, u32),
    ) -> Self {
        let mut composer =
            load_default_shaders(create_composer()).expect("Failed to load default shader modules");

        let globals = Globals::new(device);

        let Layout {
            content,
            legend,
            time,
        } = Layout::new(
            screen_resolution,
            ViewportSource::from_tl_br(config.content_extent.0, config.content_extent.1),
            36.,
            1024.,
            config.time.font.size * 1.2,
        );

        Self {
            machine: Machine::new(ComponentInit {
                device,
                queue,
                format,
                globals: &globals,
                shader_composer: &mut composer,
                config,
                state,
                viewport_projection: content,
                screen_resolution,
            }),
            atoms: Atoms::new(ComponentInit {
                device,
                queue,
                format,
                globals: &globals,
                shader_composer: &mut composer,
                config,
                state,
                viewport_projection: content,
                screen_resolution,
            }),
            legend: Legend::new(ComponentInit {
                device,
                queue,
                format,
                globals: &globals,
                shader_composer: &mut composer,
                config,
                state,
                viewport_projection: legend,
                screen_resolution,
            }),
            time: Time::new(ComponentInit {
                device,
                queue,
                format,
                globals: &globals,
                shader_composer: &mut composer,
                config,
                state,
                viewport_projection: time,
                screen_resolution,
            }),
            globals,
            screen_resolution,
        }
    }

    /// Updates this [Renderer] to resemble the new [State].
    /// See [Updatable::update].
    pub fn update(
        &mut self,
        updater: &mut impl BufferUpdater,
        device: &Device,
        queue: &Queue,
        config: &Config,
        state: &State,
    ) {
        self.machine.update(updater, device, queue, config, state);
        self.atoms.update(updater, device, queue, config, state);
        self.legend.update(updater, device, queue, config, state);
        self.time.update(updater, device, queue, config, state);
    }

    /// Updates this [Renderer] to resemble the new [State] and [Config].
    /// See [Updatable::update_full].
    pub fn update_full(
        &mut self,
        updater: &mut impl BufferUpdater,
        device: &Device,
        queue: &Queue,
        config: &Config,
        state: &State,
    ) {
        let Layout {
            content,
            legend,
            time,
        } = Layout::new(
            self.screen_resolution,
            ViewportSource::from_tl_br(config.content_extent.0, config.content_extent.1),
            36.,
            1024.,
            config.time.font.size * 1.2,
        );

        self.machine
            .update_full(updater, device, queue, config, state, content);
        self.atoms
            .update_full(updater, device, queue, config, state, content);
        self.legend
            .update_full(updater, device, queue, config, state, legend);
        self.time
            .update_full(updater, device, queue, config, state, time);
    }

    /// Updates the viewport resolution of this [Renderer]
    pub fn update_viewport(
        &mut self,
        device: &Device,
        queue: &Queue,
        screen_resolution: (u32, u32),
    ) {
        self.screen_resolution = screen_resolution;
        self.legend
            .update_viewport(device, queue, screen_resolution);
        self.machine
            .update_viewport(device, queue, screen_resolution);
        self.atoms.update_viewport(device, queue, screen_resolution);
        self.time.update_viewport(device, queue, screen_resolution);
    }

    /// Draws the contents of this [Renderer] to the passed [RenderPass]
    pub fn draw(&self, render_pass: &mut RenderPass<'_>) {
        self.rebind(render_pass);

        self.machine.draw::<true>(render_pass, self.rebind_fn());
        self.atoms.draw::<true>(render_pass, self.rebind_fn());
        self.legend.draw::<false>(render_pass, self.rebind_fn()); // No rebind: time does not need globals
        self.time.draw::<false>(render_pass, self.rebind_fn());
    }

    /// A closure which calls [Self::rebind] on `self` with the passed [RenderPass]
    #[inline]
    fn rebind_fn(&self) -> impl Fn(&mut RenderPass) + '_ {
        |r| self.rebind(r)
    }

    /// Rebinds all globals of this renderer
    #[inline]
    fn rebind(&self, render_pass: &mut RenderPass<'_>) {
        self.globals.bind(render_pass);
    }
}
