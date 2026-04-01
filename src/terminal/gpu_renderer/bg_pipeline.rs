use super::*;
use wgpu::util::DeviceExt;

impl BgResources {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terminal_bg_shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
struct Uniforms {
  viewport_px: vec2<f32>,
  cell_size_px: vec2<f32>,
  origin_px: vec2<f32>,
  _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsIn {
  @location(0) quad_pos: vec2<f32>,
  @location(1) cell_xy: vec2<u32>,
  @location(2) bg_rgba: vec4<f32>,
};

struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(v: VsIn) -> VsOut {
  let cell = vec2<f32>(f32(v.cell_xy.x), f32(v.cell_xy.y));
  let px = u.origin_px + cell * u.cell_size_px + v.quad_pos * u.cell_size_px;
  let ndc_x = (px.x / u.viewport_px.x) * 2.0 - 1.0;
  let ndc_y = 1.0 - (px.y / u.viewport_px.y) * 2.0;
  var o: VsOut;
  o.pos = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
  o.color = v.bg_rgba;
  return o;
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
  return v.color;
}
"#
                .into(),
            ),
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terminal_bg_uniform"),
            size: std::mem::size_of::<BgUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terminal_bg_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terminal_bg_bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terminal_bg_vertices"),
            contents: bytemuck::cast_slice(&[
                BgVertex { pos: [0.0, 0.0] },
                BgVertex { pos: [1.0, 0.0] },
                BgVertex { pos: [1.0, 1.0] },
                BgVertex { pos: [0.0, 1.0] },
            ]),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terminal_bg_indices"),
            contents: bytemuck::cast_slice(&[0u16, 1, 2, 0, 2, 3]),
            usage: wgpu::BufferUsages::INDEX,
        });
        let instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terminal_bg_instances"),
            size: 4,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("terminal_bg_pl"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terminal_bg_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<BgVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<BgInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Uint16x2,
                                offset: 0,
                                shader_location: 1,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Unorm8x4,
                                offset: 8,
                                shader_location: 2,
                            },
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group,
            uniform_buf,
            vertex_buf,
            index_buf,
            index_count: 6,
            instance_buf,
            instance_cap: 0,
        }
    }

    pub(super) fn ensure_instance_capacity(&mut self, device: &wgpu::Device, needed: usize) {
        if needed <= self.instance_cap {
            return;
        }
        let mut cap = self.instance_cap.max(1);
        while cap < needed {
            cap *= 2;
        }
        let new_size = (cap * std::mem::size_of::<BgInstance>()) as u64;
        self.instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terminal_bg_instances"),
            size: new_size.max(4),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.instance_cap = cap;
    }
}
