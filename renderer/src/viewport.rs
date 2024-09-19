use glam::Mat4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferBindingType, BufferUsages, Device, RenderPass,
    ShaderStages,
};

/// The specs/data of a viewport
///
/// Will map coordinates from `source` into `target`
///
/// Can be converted into a projection-matrix using [`Into<Mat4>`].
#[derive(Clone, Copy)]
pub struct ViewportProjection {
    pub source: ViewportSource,
    pub target: ViewportTarget,
}

/// The source-coordinates of the viewport.
/// Will be from `(0, 0)` in the top-left to `(width, height)` in the bottom-right.
#[derive(Clone, Copy)]
pub struct ViewportSource {
    pub width: f32,
    pub height: f32,
}

/// The target coordinates of the viewport.
/// Coordinates are in [wgpu] coordinate-space.
#[derive(Clone, Copy)]
pub struct ViewportTarget {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl From<ViewportProjection> for Mat4 {
    fn from(ViewportProjection { source, target }: ViewportProjection) -> Self {
        // Content -> Between
        let from_content =
            glam::Mat4::orthographic_rh(0., source.width, source.height, 0., -1., 1.);
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
        }
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
