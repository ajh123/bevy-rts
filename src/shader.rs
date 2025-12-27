use wgpu::util::DeviceExt;
use std::marker::PhantomData;

/// Trait for types that can be used as uniform data in shaders
pub trait UniformData: bytemuck::Pod + bytemuck::Zeroable + Clone + Copy {}

/// Configuration for creating a shader
pub struct ShaderConfig<'a> {
    /// The WGSL shader source code
    pub shader_source: &'a str,
    /// Label for the shader module
    pub shader_label: Option<&'a str>,
    /// Vertex shader entry point
    pub vertex_entry_point: &'a str,
    /// Fragment shader entry point
    pub fragment_entry_point: &'a str,
    /// Vertex buffer layouts
    pub vertex_buffer_layouts: Vec<wgpu::VertexBufferLayout<'a>>,
    /// Primitive state
    pub primitive: wgpu::PrimitiveState,
    /// Multisample state
    pub multisample: wgpu::MultisampleState,
    /// Depth stencil state
    pub depth_stencil: Option<wgpu::DepthStencilState>,
    /// Color target states
    pub color_targets: Vec<Option<wgpu::ColorTargetState>>,
    /// Bind group layout entries
    pub bind_group_layout_entries: Vec<wgpu::BindGroupLayoutEntry>,
}

impl<'a> Default for ShaderConfig<'a> {
    fn default() -> Self {
        Self {
            shader_source: "",
            shader_label: None,
            vertex_entry_point: "vs_main",
            fragment_entry_point: "fs_main",
            vertex_buffer_layouts: Vec::new(),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            depth_stencil: None,
            color_targets: Vec::new(),
            bind_group_layout_entries: Vec::new(),
        }
    }
}

/// A generic shader manager that handles shader module, pipeline, and uniform buffers
pub struct Shader<U: UniformData> {
    _shader_module: wgpu::ShaderModule,
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    _bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    _phantom: PhantomData<U>,
}

impl<U: UniformData> Shader<U> {
    /// Creates a new shader with the given configuration and initial uniform data
    pub fn new(
        device: &wgpu::Device,
        config: ShaderConfig,
        initial_uniforms: &U,
        additional_bind_group_resources: &[wgpu::BindGroupEntry],
    ) -> Self {
        // Create shader module
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: config.shader_label,
            source: wgpu::ShaderSource::Wgsl(config.shader_source.into()),
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[*initial_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shader Bind Group Layout"),
            entries: &config.bind_group_layout_entries,
        });

        // Create bind group
        let mut bind_group_entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }];
        bind_group_entries.extend_from_slice(additional_bind_group_resources);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shader Bind Group"),
            layout: &bind_group_layout,
            entries: &bind_group_entries,
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some(config.vertex_entry_point),
                buffers: &config.vertex_buffer_layouts,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some(config.fragment_entry_point),
                targets: &config.color_targets,
                compilation_options: Default::default(),
            }),
            primitive: config.primitive,
            depth_stencil: config.depth_stencil,
            multisample: config.multisample,
            cache: None,
            multiview_mask: None,
        });

        Self {
            _shader_module: shader_module,
            pipeline,
            uniform_buffer,
            _bind_group_layout: bind_group_layout,
            bind_group,
            _phantom: PhantomData,
        }
    }

    /// Updates the uniform data
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &U) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    /// Returns a reference to the render pipeline
    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    /// Returns a reference to the bind group
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Returns a reference to the uniform buffer
    pub fn uniform_buffer(&self) -> &wgpu::Buffer {
        &self.uniform_buffer
    }
}