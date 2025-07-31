use glam::Mat4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferUsages, Device, RenderPass,
    ShaderStages,
};

use crate::buffer_updater::BufferUpdater;

/// The specs/data of a viewport
///
/// Will map coordinates from `source` into `target`
///
/// Can be converted into a projection-matrix using [`Into<Mat4>`].
#[derive(Clone, Copy, Debug)]
pub struct ViewportProjection {
    pub source: ViewportSource,
    pub target: ViewportTarget,
}

/// The source-coordinates of the viewport.
/// Will be from `(x, y)` in the top-left to `(width, height)` in the bottom-right.
#[derive(Clone, Copy, Debug)]
pub struct ViewportSource {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// The target coordinates of the viewport.
/// Coordinates are in [wgpu] coordinate-space.
#[derive(Clone, Copy, Debug)]
pub struct ViewportTarget {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl ViewportProjection {
    /// An identity-[ViewportProjection].
    /// Transforms nothing and fills the whole viewport.
    pub fn identity() -> Self {
        Self {
            source: ViewportSource {
                x: -1.,
                y: -1.,
                width: 2.,
                height: 2.,
            },
            target: ViewportTarget {
                x: -1.,
                y: -1.,
                width: 2.,
                height: 2.,
            },
        }
    }
}

impl From<ViewportProjection> for Mat4 {
    fn from(ViewportProjection { source, target }: ViewportProjection) -> Self {
        // Content -> Between
        let from_content = glam::Mat4::orthographic_rh(
            source.left(),
            source.right(),
            source.bottom(),
            source.top(),
            -1.,
            1.,
        );
        // Between -> Viewport
        // Created by inverting the projection Viewport -> Between
        // Can always be inverted, as it is an orthographic projection matrix
        let to_viewport = glam::Mat4::orthographic_rh(
            target.x,
            target.x + target.width,
            target.y,
            target.y + target.height,
            -1.,
            1.,
        )
        .inverse();
        // Full: Content -> Between -> Viewport
        to_viewport * from_content
    }
}

impl Default for ViewportTarget {
    fn default() -> Self {
        // Default: Fill whole viewport
        Self {
            x: -1.,
            y: -1.,
            width: 2.,
            height: 2.,
        }
    }
}

/// A viewport, which holds all uniform buffers unique to a viewport,
/// such as the [ViewportProjection]-spec.
///
/// Will bind to group `1`.
pub struct Viewport {
    bind_group: BindGroup,
    bind_group_layout: BindGroupLayout,
    projection_matrix: Buffer,
}

impl Viewport {
    /// Creates a new viewport.
    /// Allocates uniform buffers in a bind group.
    pub fn new(projection: ViewportProjection, device: &Device) -> Self {
        let matrix: Mat4 = projection.into();

        let projection_matrix = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[matrix]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: None,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: projection_matrix.as_entire_binding(),
            }],
            label: None,
        });

        Self {
            bind_group,
            bind_group_layout,
            projection_matrix,
        }
    }

    /// Updates this [Viewport]
    pub fn update(&mut self, updater: &mut impl BufferUpdater, projection: ViewportProjection) {
        let matrix: Mat4 = projection.into();

        updater.update(
            &mut self.projection_matrix,
            &[matrix],
            None,
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
        );
    }

    /// Binds this [Viewport] to group `1`
    pub fn bind(&self, render_pass: &mut RenderPass<'_>) {
        render_pass.set_bind_group(1, &self.bind_group, &[]);
    }

    /// The bind group layout for this [Viewport]
    pub fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }
}

impl ViewportSource {
    /// Creates a [ViewportSource] from a given start-point and size.
    pub fn from_point_size(from: (f32, f32), size: (f32, f32)) -> Self {
        Self {
            x: from.0,
            y: from.1,
            width: size.0,
            height: size.1,
        }
    }

    /// Creates a [ViewportSource] from a given top-left and bottom-right point
    pub fn from_tl_br(tl: (f32, f32), br: (f32, f32)) -> Self {
        Self {
            x: tl.0,
            y: tl.1,
            width: br.0 - tl.0,
            height: br.1 - tl.1,
        }
    }

    /// The left edge (minimum `x`) of this [ViewportSource]
    pub fn left(&self) -> f32 {
        self.x
    }

    /// The right edge (maximum `x`) of this [ViewportSource]
    pub fn right(&self) -> f32 {
        self.width + self.x
    }

    /// The top edge (minimum `y`) of this [ViewportSource]
    pub fn top(&self) -> f32 {
        self.y
    }

    /// The bottom edge (maximum `y`) of this [ViewportSource]
    pub fn bottom(&self) -> f32 {
        self.height + self.y
    }
}
