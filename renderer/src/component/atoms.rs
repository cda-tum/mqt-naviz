use naviz_state::{
    config::{AtomsConfig, Config},
    state::{AtomState, State},
};
use wgpu::{Device, Queue, RenderPass};

use crate::{
    buffer_updater::BufferUpdater,
    viewport::{Viewport, ViewportProjection},
};

use super::{
    primitive::{
        circles::{CircleSpec, Circles},
        lines::{LineSpec, Lines},
        text::{Alignment, HAlignment, Text, TextSpec, VAlignment},
    },
    updatable::Updatable,
    ComponentInit,
};

/// A component to draw atoms:
/// - Circle representing atom
/// - Shuttle lines
/// - Label
pub struct Atoms {
    viewport: Viewport,
    atoms: Circles,
    shuttles: Lines,
    labels: Text,
    viewport_projection: ViewportProjection,
}

impl Atoms {
    pub fn new(
        ComponentInit {
            device,
            queue,
            format,
            globals,
            shader_composer,
            config,
            state,
            viewport_projection,
            screen_resolution,
        }: ComponentInit,
    ) -> Self {
        let (atom_circles, shuttles, labels) = get_specs(config, state, viewport_projection);
        let viewport = Viewport::new(viewport_projection, device);

        Self {
            atoms: Circles::new(
                device,
                format,
                globals,
                &viewport,
                shader_composer,
                &atom_circles,
            ),
            shuttles: Lines::new(
                device,
                format,
                globals,
                &viewport,
                shader_composer,
                &shuttles,
            ),
            labels: Text::new(device, queue, format, labels, screen_resolution),
            viewport,
            viewport_projection,
        }
    }

    /// Updates the viewport resolution of these [Atoms]
    pub fn update_viewport(
        &mut self,
        device: &Device,
        queue: &Queue,
        screen_resolution: (u32, u32),
    ) {
        self.labels
            .update_viewport((device, queue), screen_resolution);
    }

    /// Draws these [Atoms].
    ///
    /// May overwrite bind groups.
    /// If `REBIND` is `true`, will call the passed `rebind`-function to rebind groups.
    pub fn draw<'a, const REBIND: bool>(
        &'a self,
        render_pass: &mut RenderPass<'a>,
        rebind: impl Fn(&mut RenderPass),
    ) {
        self.viewport.bind(render_pass);
        self.shuttles.draw(render_pass);
        self.atoms.draw(render_pass);
        self.labels.draw::<REBIND>(render_pass, rebind);
    }
}

impl Updatable for Atoms {
    fn update(
        &mut self,
        updater: &mut impl BufferUpdater,
        device: &Device,
        queue: &Queue,
        config: &Config,
        state: &State,
    ) {
        let (atom_circles, shuttles, labels) = get_specs(config, state, self.viewport_projection);
        self.atoms.update(updater, &atom_circles);
        self.shuttles.update(updater, &shuttles);
        self.labels.update((device, queue), labels);
    }

    fn update_full(
        &mut self,
        updater: &mut impl BufferUpdater,
        device: &Device,
        queue: &Queue,
        config: &Config,
        state: &State,
        viewport_projection: ViewportProjection,
    ) {
        self.viewport_projection = viewport_projection;
        self.viewport.update(updater, viewport_projection);
        self.update(updater, device, queue, config, state);
    }
}

/// Gets the specs for [Atoms] from the passed [State] and [Config].
fn get_specs<'a>(
    config: &'a Config,
    state: &'a State,
    viewport_projection: ViewportProjection,
) -> (
    Vec<CircleSpec>,
    Vec<LineSpec>,
    TextSpec<'a, impl IntoIterator<Item = (&'a str, (f32, f32), Alignment)>>,
) {
    let atoms = &state.atoms;
    let AtomsConfig { shuttle, label } = &config.atoms;

    // The circles for the atoms
    let atom_circles: Vec<_> = atoms
        .iter()
        .map(
            |AtomState {
                 position,
                 size,
                 color,
                 shuttle: _,
                 label: _,
             }| CircleSpec {
                center: (*position).into(),
                radius: *size,
                color: *color,
                radius_inner: 0.,
            },
        )
        .collect();

    // The shuttle lines
    let shuttles: Vec<_> = atoms
        .iter()
        .filter(|s| s.shuttle)
        .flat_map(
            |AtomState {
                 position: (x, y),
                 size: _,
                 color: _,
                 shuttle: _,
                 label: _,
             }| {
                [
                    LineSpec {
                        start: [*x, viewport_projection.source.top()],
                        end: [*x, viewport_projection.source.bottom()],
                        color: shuttle.color,
                        width: shuttle.width,
                        segment_length: shuttle.segment_length,
                        duty: shuttle.duty,
                    },
                    LineSpec {
                        start: [viewport_projection.source.left(), *y],
                        end: [viewport_projection.source.right(), *y],
                        color: shuttle.color,
                        width: shuttle.width,
                        segment_length: shuttle.segment_length,
                        duty: shuttle.duty,
                    },
                ]
            },
        )
        .collect();

    // The labels
    let labels: Vec<_> = atoms
        .iter()
        .map(
            |AtomState {
                 position: (x, y),
                 size: _,
                 color: _,
                 shuttle: _,
                 label,
             }| {
                (
                    label.as_str(),
                    (*x, *y),
                    Alignment(HAlignment::Center, VAlignment::Center),
                )
            },
        )
        .collect();

    (
        atom_circles,
        shuttles,
        TextSpec {
            viewport_projection,
            font_size: label.size,
            font_family: &label.family,
            texts: labels,
            color: label.color,
        },
    )
}
