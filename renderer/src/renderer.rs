use naviz_state::{config::Config, state::State};
use wgpu::{Device, Queue, RenderPass, TextureFormat};

use crate::{
    buffer_updater::BufferUpdater,
    component::{
        atoms::Atoms,
        drawable::{Drawable, Hidable},
        legend::Legend,
        machine::Machine,
        time::Time,
        updatable::Updatable,
        ComponentInit,
    },
    globals::Globals,
    layout::Layout,
    shaders::{create_composer, load_default_shaders},
    viewport::{ViewportProjection, ViewportSource},
};

/// The main renderer, which renders the visualization output
pub struct Renderer {
    globals: Globals,

    machine: Machine,
    atoms: Atoms,
    legend: Hidable<Legend>,
    time: Hidable<Time>,
    screen_resolution: (u32, u32),
    /// Whether to force the [content-only-layout][Layout::new_content_only].
    /// Independent of the selected style.
    force_zen: bool,
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
        } = get_layout(config, screen_resolution, false);

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
            legend: Hidable::new(Legend::new(ComponentInit {
                device,
                queue,
                format,
                globals: &globals,
                shader_composer: &mut composer,
                config,
                state,
                viewport_projection: legend.unwrap_or(ViewportProjection::identity()),
                screen_resolution,
            }))
            .with_visibility(legend.is_some()),
            time: Hidable::new(Time::new(ComponentInit {
                device,
                queue,
                format,
                globals: &globals,
                shader_composer: &mut composer,
                config,
                state,
                viewport_projection: time.unwrap_or(ViewportProjection::identity()),
                screen_resolution,
            }))
            .with_visibility(time.is_some()),
            globals,
            screen_resolution,
            force_zen: false,
        }
    }

    /// Whether to force the [content-only-layout][Layout::new_content_only].
    /// Independent of the selected style.
    pub fn set_force_zen(&mut self, force_zen: bool) {
        self.force_zen = force_zen;
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
        } = get_layout(config, self.screen_resolution, self.force_zen);

        self.machine
            .update_full(updater, device, queue, config, state, content);
        self.atoms
            .update_full(updater, device, queue, config, state, content);
        self.legend.update_full(
            updater,
            device,
            queue,
            config,
            state,
            legend.unwrap_or(ViewportProjection::identity()),
        );
        self.legend.set_visible(legend.is_some());
        self.time.update_full(
            updater,
            device,
            queue,
            config,
            state,
            time.unwrap_or(ViewportProjection::identity()),
        );
        self.time.set_visible(time.is_some());
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

/// Gets the [Layout] to use based on the passed [Config].
/// Will detect which [Layout] to use based on which parts should be displayed in the [Config].
/// If `force_content_only` is `true`, will always use [Layout::new_content_only].
fn get_layout(config: &Config, screen_resolution: (u32, u32), force_content_only: bool) -> Layout {
    const CONTENT_PADDING_Y: f32 = 36.;
    const LEGEND_HEIGHT: f32 = 1024.;

    // content source
    let content = ViewportSource::from_tl_br(config.content_extent.0, config.content_extent.1);

    if force_content_only || (!config.display_time() && !config.display_sidebar()) {
        // no time and no sidebar
        Layout::new_content_only(screen_resolution, content, CONTENT_PADDING_Y)
    } else {
        // default layout
        Layout::new_full(
            screen_resolution,
            content,
            CONTENT_PADDING_Y,
            LEGEND_HEIGHT,
            config.time.font.size * 1.2,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn example_layout_has_all_sections() {
        let config = Config::example();

        let layout = get_layout(&config, (1920, 1080), false);

        assert!(
            layout.legend.is_some(),
            "Example config should produce a layout with space for the legend"
        );
        assert!(
            layout.time.is_some(),
            "Example config should produce a layout with space for the time"
        );
    }

    #[test]
    fn example_layout_display_none_only_content() {
        let mut config = Config::example();
        config.legend.entries = Vec::new(); // No legend
        config.time.display = false; // No time

        let layout = get_layout(&config, (1920, 1080), false);

        assert!(
            layout.legend.is_none(),
            "Example config without legend and time should not allocates space for legend"
        );
        assert!(
            layout.time.is_none(),
            "Example config without legend and time should not allocates space for time"
        );
    }
}
