use std::marker::PhantomData;

use naga_oil::compose::Composer;
use naviz_state::{config::Config, state::State};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BlendComponent, BlendFactor, BlendState, Buffer, BufferAddress,
    BufferUsages, ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState,
    PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, TextureFormat, VertexAttribute, VertexBufferLayout, VertexState,
    VertexStepMode,
};

use crate::{
    buffer_updater::BufferUpdater,
    component::drawable::Drawable,
    globals::Globals,
    shaders::compile_shader,
    viewport::{Viewport, ViewportProjection},
};

pub mod atoms;
pub mod drawable;
pub mod legend;
pub mod machine;
pub mod primitive;
pub mod time;
pub mod updatable;

/// Data used to initialize a component
pub struct ComponentInit<'a> {
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub format: TextureFormat,
    pub globals: &'a Globals,
    pub shader_composer: &'a mut Composer,
    pub config: &'a Config,
    pub state: &'a State,
    pub viewport_projection: ViewportProjection,
    pub screen_resolution: (u32, u32),
}

/// The spec of a [Component].
#[derive(Clone, Copy, Debug)]
pub struct ComponentSpec<'a, Spec: bytemuck::NoUninit> {
    /// The specifications for the instances
    specs: &'a [Spec],
    /// The vertex attributes
    attributes: &'a [VertexAttribute],
    /// The shader source code.
    /// Will be compiled using the passed [Composer].
    shader_source: &'static str,
    /// The path to the shader
    shader_path: &'static str,
    /// Optional uniform buffer group.
    /// Will be bound at group `2`.
    uniform: Option<(&'a [BindGroupLayoutEntry], &'a [BindGroupEntry<'a>])>,
}

/// A drawable component.
/// Groups together common setup.
///
/// Assumes a [Component] generates its vertices on  the GPU
/// and takes an instance buffer.
///
/// Binds [Globals] and [Viewport] to group `0` and `1`.
/// Allows binding a local uniform to group `2`.
pub struct Component<Spec: bytemuck::NoUninit> {
    render_pipeline: RenderPipeline,
    instance_buffer: Buffer,
    instance_count: u32,
    bind_group: BindGroup,
    phantom: PhantomData<Spec>,
}

impl<Spec: bytemuck::NoUninit> Component<Spec> {
    /// Creates a new [Component]
    pub fn new(
        device: &Device,
        format: TextureFormat,
        globals: &Globals,
        viewport: &Viewport,
        shader_composer: &mut Composer,
        ComponentSpec {
            specs,
            attributes,
            shader_source,
            shader_path,
            uniform,
        }: ComponentSpec<Spec>,
    ) -> Self {
        let instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: bytemuck::cast_slice(specs),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let instance_buffer_layout = VertexBufferLayout {
            array_stride: size_of::<Spec>() as BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes,
        };

        let shader = compile_shader(
            device,
            shader_composer,
            shader_source,
            shader_path,
            Default::default(),
        )
        .unwrap_or_else(|_| panic!("Failed to load shader: {shader_path}"));

        let uniform = uniform.unwrap_or_default();

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: uniform.0,
            label: Some("uniform buffer group layout"),
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: uniform.1,
            label: Some("uniform buffer group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                globals.bind_group_layout(),
                viewport.bind_group_layout(),
                &bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[instance_buffer_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: BlendComponent::OVER,
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            render_pipeline,
            instance_buffer,
            instance_count: specs.len() as u32,
            bind_group,
            phantom: PhantomData,
        }
    }

    /// Update this component to have the new `spec`
    pub fn update<U: BufferUpdater>(&mut self, updater: &mut U, spec: &[Spec]) {
        updater.update(
            &mut self.instance_buffer,
            spec,
            Some("instance buffer"),
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
        );
        self.instance_count = spec.len() as u32;
    }

    /// Draws this component
    pub fn draw(&self, render_pass: &mut RenderPass<'_>) {
        if self.instance_count == 0 {
            // nothing to render
            return;
        }
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        render_pass.set_bind_group(2, &self.bind_group, &[]);
        render_pass.draw(0..6, 0..self.instance_count);
    }
}

impl<Spec: bytemuck::NoUninit> Drawable for Component<Spec> {
    #[inline]
    fn draw<const REBIND: bool>(
        &self,
        render_pass: &mut RenderPass<'_>,
        _rebind: impl Fn(&mut RenderPass),
    ) {
        // no bindings are changed, therefore we never need to rebind
        self.draw(render_pass);
    }
}
