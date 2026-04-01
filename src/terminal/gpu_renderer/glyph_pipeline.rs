use super::*;
use wgpu::util::DeviceExt;
use crate::terminal::glyph_atlas::MAX_ATLAS_PAGES;

impl GlyphResources {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        // Default to nearest to keep thin strokes/punctuation crisp at small cell sizes.
        // You can force linear by setting: RUST_SSH_GPU_GLYPH_NEAREST=0
        let use_nearest = match std::env::var("RUST_SSH_GPU_GLYPH_NEAREST")
            .ok()
            .as_deref()
            .map(str::trim)
        {
            Some("0" | "false" | "FALSE" | "False" | "no" | "NO" | "No") => false,
            Some("1" | "true" | "TRUE" | "True" | "yes" | "YES" | "Yes") => true,
            _ => true,
        };
        let atlas_w = ATLAS_PX;
        let atlas_h = ATLAS_PX;

        let atlas_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("terminal_ttf_atlas_r8"),
            size: wgpu::Extent3d {
                width: atlas_w,
                height: atlas_h,
                depth_or_array_layers: MAX_ATLAS_PAGES,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let atlas_view = atlas_tex.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: if use_nearest {
                wgpu::FilterMode::Nearest
            } else {
                wgpu::FilterMode::Linear
            },
            min_filter: if use_nearest {
                wgpu::FilterMode::Nearest
            } else {
                wgpu::FilterMode::Linear
            },
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terminal_glyph_shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
struct Uniforms {
  viewport_px: vec2<f32>,
  cell_size_px: vec2<f32>,
  origin_px: vec2<f32>,
  atlas_w: f32,
  atlas_h: f32,
  slot_px: f32,
  atlas_cols: f32,
  slots_per_page: f32,
  glyph_uv_crop: f32,
  glyph_alpha_boost: f32,
  _pad0: f32,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

@group(1) @binding(0) var atlas_tex: texture_2d_array<f32>;
@group(1) @binding(1) var atlas_samp: sampler;

struct VsIn {
  @location(0) quad_pos: vec2<f32>,
  @location(1) cell_xy: vec2<u32>,
  @location(2) glyph_id: u32,
  @location(3) fg_rgba: vec4<f32>,
  @location(4) flags: u32,
  @location(5) uv_rect: vec4<u32>,
  @location(6) dst_rect: vec4<u32>,
  @location(7) atlas_page: u32,
};

struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
  @location(1) color: vec4<f32>,
  @location(2) flags: u32,
  @location(3) qy: f32,
  @location(4) page: i32,
};

@vertex
fn vs_main(v: VsIn) -> VsOut {
  let cell = vec2<f32>(f32(v.cell_xy.x), f32(v.cell_xy.y));
  let dst = vec4<f32>(
    f32(v.dst_rect.x) / 65535.0,
    f32(v.dst_rect.y) / 65535.0,
    f32(v.dst_rect.z) / 65535.0,
    f32(v.dst_rect.w) / 65535.0
  );
  let dst_lo = min(vec2<f32>(dst.x, dst.y), vec2<f32>(dst.z, dst.w));
  let dst_hi = max(vec2<f32>(dst.x, dst.y), vec2<f32>(dst.z, dst.w));
  let local = mix(dst_lo, dst_hi, v.quad_pos);
  let px = u.origin_px + cell * u.cell_size_px + local * u.cell_size_px;
  let ndc_x = (px.x / u.viewport_px.x) * 2.0 - 1.0;
  let ndc_y = 1.0 - (px.y / u.viewport_px.y) * 2.0;

  let gid = v.glyph_id;
  let spp = u32(u.slots_per_page);
  // Slot 0 means "no glyph"; real atlas tiles are addressed by (slot - 1).
  let tile = select(0u, gid - 1u, gid > 0u);
  let page = tile / spp;
  let in_page = tile - page * spp;
  let cols_u = max(1u, u32(u.atlas_cols));
  let sp = u.slot_px;
  // Use integer math for slot coordinates to avoid float precision artifacts
  // that can manifest as sampling the wrong atlas tile (looks like garbled text).
  let gx = f32(in_page % cols_u);
  let gy = f32(in_page / cols_u);
  // Sample strictly inside slot bounds to avoid linear-filter bleeding
  // from neighboring atlas slots (shows up as overlapping/corrupted glyphs).
  let uv0 = vec2<f32>(gx * sp / u.atlas_w, gy * sp / u.atlas_h);
  let slot_uv = vec2<f32>(sp / u.atlas_w, sp / u.atlas_h);
  let crop = vec4<f32>(
    f32(v.uv_rect.x) / 65535.0,
    f32(v.uv_rect.y) / 65535.0,
    f32(v.uv_rect.z) / 65535.0,
    f32(v.uv_rect.w) / 65535.0
  );
  let crop_lo = min(vec2<f32>(crop.x, crop.y), vec2<f32>(crop.z, crop.w));
  let crop_hi = max(vec2<f32>(crop.x, crop.y), vec2<f32>(crop.z, crop.w));
  let inset = vec2<f32>(0.5 / u.atlas_w, 0.5 / u.atlas_h);
  var uv_lo = uv0 + slot_uv * crop_lo + inset;
  var uv_hi = uv0 + slot_uv * crop_hi - inset;
  uv_hi = max(uv_hi, uv_lo + vec2<f32>(1.0 / u.atlas_w, 1.0 / u.atlas_h));
  if (u.glyph_uv_crop > 0.0) {
    // Shrink sampling area inside the slot; helpful for A/B diagnosing
    // edge bleeding or slot-boundary artifacts.
    let c = (uv_lo + uv_hi) * 0.5;
    let half = (uv_hi - uv_lo) * (0.5 * (1.0 - u.glyph_uv_crop));
    uv_lo = c - half;
    uv_hi = c + half;
  }
  let uv = mix(uv_lo, uv_hi, v.quad_pos);

  var o: VsOut;
  o.pos = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
  // Pack page in uv.y sign? keep separate in flags bits higher.
  // We can't add a new varying cheaply here; reuse high bits of flags for page is too small.
  // Instead, encode page into the fractional part of uv.x? Not safe.
  // Solution: store page in flags' upper 16 bits in Rust.
  // Here we assume v.flags already contains page in bits 16..31.
  o.uv = uv;
  o.color = v.fg_rgba;
  o.flags = v.flags;
  o.qy = v.quad_pos.y;
  o.page = i32(v.atlas_page);
  return o;
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
  let page = v.page;
  var a = textureSample(atlas_tex, atlas_samp, v.uv, page).r;
  // Improve continuity for thin strokes under linear filtering:
  // lift mid-alpha coverage slightly so glyphs look less "broken".
  a = pow(clamp(a, 0.0, 1.0), 0.85);
  if ((v.flags & 1u) != 0u) {
    let du = 1.4 / u.atlas_w;
    let dv = 0.5 / u.atlas_h;
    let a1 = textureSample(atlas_tex, atlas_samp, v.uv + vec2<f32>(du, 0.0), page).r;
    let a2 = textureSample(atlas_tex, atlas_samp, v.uv + vec2<f32>(0.0, dv), page).r;
    a = max(a, max(a1, a2));
  }
  if ((v.flags & 2u) != 0u && v.qy > 0.88) {
    a = 1.0;
  }
  // Strikethrough: draw a 1-2px bar near the vertical center.
  if ((v.flags & 8u) != 0u && v.qy > 0.50 && v.qy < 0.60) {
    a = 1.0;
  }
  a = clamp(a * u.glyph_alpha_boost, 0.0, 1.0);
  // Dim (faint): scale alpha down.
  if ((v.flags & 4u) != 0u) {
    a = a * 0.65;
  }
  return vec4<f32>(v.color.rgb, a);
}
"#
                .into(),
            ),
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terminal_glyph_uniform"),
            size: std::mem::size_of::<GlyphUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terminal_glyph_bgl0"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terminal_glyph_bg0"),
            layout: &bgl0,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });
        let bgl1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terminal_glyph_bgl1"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind_group1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terminal_glyph_bg1"),
            layout: &bgl1,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terminal_glyph_vertices"),
            contents: bytemuck::cast_slice(&[
                BgVertex { pos: [0.0, 0.0] },
                BgVertex { pos: [1.0, 0.0] },
                BgVertex { pos: [1.0, 1.0] },
                BgVertex { pos: [0.0, 1.0] },
            ]),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terminal_glyph_indices"),
            contents: bytemuck::cast_slice(&[0u16, 1, 2, 0, 2, 3]),
            usage: wgpu::BufferUsages::INDEX,
        });
        let instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terminal_glyph_instances"),
            size: 4,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("terminal_glyph_pl"),
            bind_group_layouts: &[&bgl0, &bgl1],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terminal_glyph_pipeline"),
            layout: Some(&pl),
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
                        array_stride: std::mem::size_of::<GlyphInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Uint16x2,
                                offset: 0,
                                shader_location: 1,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Uint16,
                                offset: 4,
                                shader_location: 2,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Uint16,
                                offset: 6,
                                shader_location: 7,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Uint16,
                                offset: 8,
                                shader_location: 4,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Unorm8x4,
                                offset: 12,
                                shader_location: 3,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Uint16x4,
                                offset: 16,
                                shader_location: 5,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Uint16x4,
                                offset: 24,
                                shader_location: 6,
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
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
            bind_group0,
            uniform_buf,
            vertex_buf,
            index_buf,
            index_count: 6,
            instance_buf,
            instance_cap: 0,
            atlas_tex,
            bind_group1,
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
        let new_size = (cap * std::mem::size_of::<GlyphInstance>()) as u64;
        self.instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terminal_glyph_instances"),
            size: new_size.max(4),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.instance_cap = cap;
    }
}
