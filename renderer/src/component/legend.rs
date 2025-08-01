use naviz_state::{
    config::{Config, LegendConfig, LegendEntry, LegendSection},
    state::State,
};
use wgpu::{Device, Queue, RenderPass};

use crate::{
    buffer_updater::BufferUpdater,
    viewport::{Viewport, ViewportProjection},
};

use super::{
    primitive::{
        circles::{CircleSpec, Circles},
        text::{Alignment, HAlignment, Text, TextSpec, VAlignment},
    },
    updatable::Updatable,
    ComponentInit,
};

/// A component to draw the legend:
/// - A heading per block
/// - Entries, with an optional colored circle
pub struct Legend {
    viewport: Viewport,
    text: Text,
    colors: Circles,
}

impl Legend {
    pub fn new(
        ComponentInit {
            device,
            queue,
            format,
            globals,
            shader_composer,
            config,
            state: _,
            viewport_projection,
            screen_resolution,
        }: ComponentInit,
    ) -> Self {
        let LegendSpec { text, colors } = get_specs(config, viewport_projection);
        let viewport = Viewport::new(viewport_projection, device);

        Self {
            text: Text::new(device, queue, format, text, screen_resolution),
            colors: Circles::new(device, format, globals, &viewport, shader_composer, &colors),
            viewport,
        }
    }

    /// Updates the viewport resolution of this [Legend]
    pub fn update_viewport(
        &mut self,
        device: &Device,
        queue: &Queue,
        screen_resolution: (u32, u32),
    ) {
        self.text
            .update_viewport((device, queue), screen_resolution);
    }

    /// Draws this [Legend].
    ///
    /// May overwrite bind groups.
    /// If `REBIND` is `true`, will call the passed `rebind`-function to rebind groups.
    pub fn draw<const REBIND: bool>(
        &self,
        render_pass: &mut RenderPass<'_>,
        rebind: impl Fn(&mut RenderPass),
    ) {
        self.viewport.bind(render_pass);
        self.colors.draw(render_pass);
        self.text.draw::<REBIND>(render_pass, rebind);
    }
}

impl Updatable for Legend {
    fn update(
        &mut self,
        _updater: &mut impl BufferUpdater,
        _device: &Device,
        _queue: &Queue,
        _config: &Config,
        _state: &State,
    ) {
        // Nothing depends on state
    }

    fn update_full(
        &mut self,
        updater: &mut impl BufferUpdater,
        device: &Device,
        queue: &Queue,
        config: &Config,
        _state: &State,
        viewport_projection: ViewportProjection,
    ) {
        self.viewport.update(updater, viewport_projection);
        let LegendSpec { text, colors } = get_specs(config, viewport_projection);
        self.text.update((device, queue), text);
        self.colors.update(updater, &colors);
    }
}

#[derive(Clone, Debug)]
struct LegendSpec<'a, TextIterator: IntoIterator<Item = (&'a str, (f32, f32), Alignment)>> {
    /// The legend text to draw
    text: TextSpec<'a, TextIterator>,
    /// The circles representing the colors to the left of the text
    colors: Vec<CircleSpec>,
}

/// Gets the specs for [Legend] from the passed [State] and [Config].
fn get_specs(
    config: &Config,
    viewport_projection: ViewportProjection,
) -> LegendSpec<'_, impl IntoIterator<Item = (&'_ str, (f32, f32), Alignment)>> {
    let LegendConfig {
        font,
        heading_skip,
        entry_skip,
        color_circle_radius,
        color_padding,
        entries,
    } = &config.legend;

    // Layout the texts.
    // Will always lay out heading, then all entries, each separated by `entry_skip`.
    // After the block, additional space will be skipped so that the distance to the next
    // heading will be `heading_skip`.
    let num_texts = entries.iter().map(|e| e.entries.len()).sum::<usize>() + entries.len();
    let num_colors = entries
        .iter()
        .map(|e| e.entries.iter().filter(|e| e.color.is_some()).count())
        .sum::<usize>();
    let mut colors = Vec::with_capacity(num_colors);
    let mut texts = Vec::with_capacity(num_texts);
    let mut y = *heading_skip; // Start with margin from top
    for LegendSection {
        name: heading,
        entries,
    } in entries.iter()
    {
        // heading
        texts.push((
            heading.as_str(),
            (0., y),
            Alignment(HAlignment::Left, VAlignment::Center),
        ));
        y += entry_skip;

        // entries
        for LegendEntry { text, color } in entries.iter() {
            // text
            texts.push((
                text.as_str(),
                (2. * color_circle_radius + color_padding, y),
                Alignment(HAlignment::Left, VAlignment::Center),
            ));

            // colored circle
            if let Some(color) = color {
                colors.push(CircleSpec {
                    center: [*color_circle_radius, y],
                    radius: *color_circle_radius,
                    radius_inner: 0.,
                    color: *color,
                });
            }
            y += entry_skip;
        }

        // end: increase skip to heading_skip
        y += heading_skip - entry_skip;
    }

    let text = TextSpec {
        viewport_projection,
        font_size: font.size,
        font_family: &font.family,
        texts,
        color: font.color,
    };

    LegendSpec { text, colors }
}
