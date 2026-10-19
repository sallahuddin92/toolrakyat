use std::time::Instant;

use crate::render::ir::{
    ColorRgba, FillRule, LineCap, LineJoin, PathVerb, RenderItem, RenderPage, Transform2D,
};
use crate::render::rasterizer::{FallbackFontCache, NativeRasterizer};
use lyon_path::builder::SvgPathBuilder;
use lyon_path::geom::point;
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillRule as LyonFillRule, FillTessellator, StrokeOptions,
    StrokeTessellator, VertexBuffers,
};
use read_fonts::FontRef;
use skrifa::instance::Size;
use skrifa::outline::OutlinePen;
use skrifa::MetadataProvider;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GpuPerformanceMetrics {
    pub upload_duration_ms: f64,
    pub render_duration_ms: f64,
    pub readback_duration_ms: f64,
    pub total_frame_duration_ms: f64,
    pub cpu_render_duration_ms: f64,
    pub gpu_draw_calls: usize,
    pub gpu_paths_rendered: usize,
    pub gpu_glyphs_rendered: usize,
    pub gpu_images_rendered: usize,
    pub fallback_cpu_items: usize,
    pub fallback_reasons: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GpuRenderResult {
    pub rgba_pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub metrics: GpuPerformanceMetrics,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuTextureVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuUniforms {
    screen_size: [f32; 2],
    _padding: [f32; 2],
}

#[derive(Clone, Copy, Debug)]
struct ScissorRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl ScissorRect {
    fn full(w: u32, h: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width: w.max(1),
            height: h.max(1),
        }
    }

    fn intersect(&self, other: &Self) -> Self {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);
        if x2 > x1 && y2 > y1 {
            Self {
                x: x1,
                y: y1,
                width: x2 - x1,
                height: y2 - y1,
            }
        } else {
            Self {
                x: x1,
                y: y1,
                width: 0,
                height: 0,
            }
        }
    }
}

enum GpuDrawCmd {
    SetScissor(ScissorRect),
    DrawVector {
        vbuf: wgpu::Buffer,
        ibuf: wgpu::Buffer,
        index_count: u32,
    },
    DrawTexture {
        bind_group: wgpu::BindGroup,
        vbuf: wgpu::Buffer,
    },
}

pub struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    vector_pipeline: wgpu::RenderPipeline,
    texture_pipeline: wgpu::RenderPipeline,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    adapter_name: String,
    fallback_fonts: FallbackFontCache,
}

impl WgpuRenderer {
    pub fn new() -> Result<Self, String> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            ..Default::default()
        }))
        .map_err(|e| format!("Failed to find suitable GPU adapter: {e:?}"))?;

        let adapter_info = adapter.get_info();
        let adapter_name = format!("{} ({:?})", adapter_info.name, adapter_info.backend);

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("starpdf_wgpu_device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            ..Default::default()
        }))
        .map_err(|e| format!("Failed to request GPU device: {e:?}"))?;

        // 1. Shaders
        let vector_shader_src = r"
            struct Uniforms {
                screen_size: vec2<f32>,
                _padding: vec2<f32>,
            };
            @group(0) @binding(0) var<uniform> uniforms: Uniforms;

            struct VertexInput {
                @location(0) position: vec2<f32>,
                @location(1) color: vec4<f32>,
            };
            struct VertexOutput {
                @builtin(position) clip_position: vec4<f32>,
                @location(0) color: vec4<f32>,
            };

            @vertex
            fn vs_main(in: VertexInput) -> VertexOutput {
                var out: VertexOutput;
                let x = (in.position.x / uniforms.screen_size.x) * 2.0 - 1.0;
                let y = 1.0 - (in.position.y / uniforms.screen_size.y) * 2.0;
                out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
                out.color = in.color;
                return out;
            }

            @fragment
            fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                return in.color;
            }
        ";

        let texture_shader_src = r"
            struct Uniforms {
                screen_size: vec2<f32>,
                _padding: vec2<f32>,
            };
            @group(0) @binding(0) var<uniform> uniforms: Uniforms;

            struct VertexInput {
                @location(0) position: vec2<f32>,
                @location(1) tex_coords: vec2<f32>,
                @location(2) color: vec4<f32>,
            };
            struct VertexOutput {
                @builtin(position) clip_position: vec4<f32>,
                @location(0) tex_coords: vec2<f32>,
                @location(1) color: vec4<f32>,
            };

            @vertex
            fn vs_main(in: VertexInput) -> VertexOutput {
                var out: VertexOutput;
                let x = (in.position.x / uniforms.screen_size.x) * 2.0 - 1.0;
                let y = 1.0 - (in.position.y / uniforms.screen_size.y) * 2.0;
                out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
                out.tex_coords = in.tex_coords;
                out.color = in.color;
                return out;
            }

            @group(1) @binding(0) var t_diffuse: texture_2d<f32>;
            @group(1) @binding(1) var s_diffuse: sampler;

            @fragment
            fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                let tex = textureSample(t_diffuse, s_diffuse, in.tex_coords);
                return tex * in.color;
            }
        ";

        let vector_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vector_shader"),
            source: wgpu::ShaderSource::Wgsl(vector_shader_src.into()),
        });

        let texture_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("texture_shader"),
            source: wgpu::ShaderSource::Wgsl(texture_shader_src.into()),
        });

        // 2. Bind group layouts
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("uniform_bind_group_layout"),
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

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
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

        // 3. Pipelines
        let vector_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("vector_pipeline_layout"),
                bind_group_layouts: &[Some(&uniform_bind_group_layout)],
                immediate_size: 0,
            });

        let texture_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("texture_pipeline_layout"),
                bind_group_layouts: &[
                    Some(&uniform_bind_group_layout),
                    Some(&texture_bind_group_layout),
                ],
                immediate_size: 0,
            });

        let vector_vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };

        let texture_vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuTextureVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 2]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };

        let alpha_blend = Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        });

        let vector_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vector_render_pipeline"),
            layout: Some(&vector_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vector_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(vector_vertex_layout)],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &vector_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: alpha_blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let texture_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("texture_render_pipeline"),
            layout: Some(&texture_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &texture_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(texture_vertex_layout)],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &texture_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: alpha_blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("gpu_linear_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let fallback_fonts = FallbackFontCache::new();

        Ok(Self {
            device,
            queue,
            vector_pipeline,
            texture_pipeline,
            uniform_bind_group_layout,
            texture_bind_group_layout,
            sampler,
            adapter_name,
            fallback_fonts,
        })
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    /// Renders a compiled `RenderPage` with genuine GPU acceleration.
    /// Vector paths and images are rasterized and composited directly via wgpu render pipelines.
    /// Unsupported primitives (e.g. text runs) are honestly classified and layered via CPU fallback.
    pub fn render_page(&self, page: &RenderPage, scale: f32) -> Result<GpuRenderResult, String> {
        let frame_start = Instant::now();
        let mut upload_dur = 0.0;
        let mut cpu_render_dur = 0.0;
        let mut gpu_draw_calls = 0;
        let mut gpu_paths_rendered = 0;
        let mut gpu_glyphs_rendered = 0;
        let mut gpu_images_rendered = 0;
        let mut fallback_cpu_items = 0;
        let mut fallback_reasons = Vec::new();

        let norm_rotate = ((page.rotate % 360) + 360) % 360;
        let (target_width, target_height, page_transform) = match norm_rotate {
            90 => {
                let tw = ((page.height * scale).round() as u32).max(1);
                let th = ((page.width * scale).round() as u32).max(1);
                let tf = Transform2D::new(
                    0.0,
                    scale,
                    scale,
                    0.0,
                    -page.crop_box[1] * scale,
                    -page.crop_box[0] * scale,
                );
                (tw, th, tf)
            }
            180 => {
                let tw = ((page.width * scale).round() as u32).max(1);
                let th = ((page.height * scale).round() as u32).max(1);
                let tf = Transform2D::new(
                    -scale,
                    0.0,
                    0.0,
                    scale,
                    page.crop_box[2] * scale,
                    -page.crop_box[1] * scale,
                );
                (tw, th, tf)
            }
            270 => {
                let tw = ((page.height * scale).round() as u32).max(1);
                let th = ((page.width * scale).round() as u32).max(1);
                let tf = Transform2D::new(
                    0.0,
                    -scale,
                    -scale,
                    0.0,
                    page.crop_box[3] * scale,
                    page.crop_box[2] * scale,
                );
                (tw, th, tf)
            }
            _ => {
                let tw = ((page.width * scale).round() as u32).max(1);
                let th = ((page.height * scale).round() as u32).max(1);
                let tf = Transform2D::new(
                    scale,
                    0.0,
                    0.0,
                    -scale,
                    -page.crop_box[0] * scale,
                    page.crop_box[3] * scale,
                );
                (tw, th, tf)
            }
        };

        // 1. Create render target texture
        let target_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gpu_render_target"),
            size: wgpu::Extent3d {
                width: target_width,
                height: target_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 2. Uniform buffer for screen dimensions
        let uniforms = GpuUniforms {
            screen_size: [target_width as f32, target_height as f32],
            _padding: [0.0, 0.0],
        };
        let uniform_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu_screen_uniforms"),
            size: std::mem::size_of::<GpuUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let t_up0 = Instant::now();
        self.queue
            .write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
        upload_dur += t_up0.elapsed().as_secs_f64() * 1000.0;

        let uniform_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniform_bind_group"),
            layout: &self.uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // 3. Prepare Draw Commands preserving exact sequential IR order
        let mut draw_cmds: Vec<GpuDrawCmd> = Vec::new();
        let mut scissor_stack: Vec<ScissorRect> =
            vec![ScissorRect::full(target_width, target_height)];

        for item in &page.items {
            match item {
                RenderItem::PushClip(clip) => {
                    let combined_tf = clip.transform.multiply(&page_transform);
                    let mut min_x = f32::INFINITY;
                    let mut min_y = f32::INFINITY;
                    let mut max_x = f32::NEG_INFINITY;
                    let mut max_y = f32::NEG_INFINITY;

                    for v in &clip.verbs {
                        match v {
                            PathVerb::MoveTo(x, y) | PathVerb::LineTo(x, y) => {
                                let (px, py) = combined_tf.transform_point(*x, *y);
                                min_x = min_x.min(px);
                                min_y = min_y.min(py);
                                max_x = max_x.max(px);
                                max_y = max_y.max(py);
                            }
                            _ => {}
                        }
                    }

                    if min_x.is_finite()
                        && min_y.is_finite()
                        && max_x.is_finite()
                        && max_y.is_finite()
                    {
                        let sx = (min_x.floor() as i64).clamp(0, target_width as i64) as u32;
                        let sy = (min_y.floor() as i64).clamp(0, target_height as i64) as u32;
                        let ex = (max_x.ceil() as i64).clamp(0, target_width as i64) as u32;
                        let ey = (max_y.ceil() as i64).clamp(0, target_height as i64) as u32;

                        let clip_rect = ScissorRect {
                            x: sx,
                            y: sy,
                            width: ex.saturating_sub(sx),
                            height: ey.saturating_sub(sy),
                        };
                        let current_top = scissor_stack
                            .last()
                            .copied()
                            .unwrap_or(ScissorRect::full(target_width, target_height));
                        let new_scissor = current_top.intersect(&clip_rect);
                        scissor_stack.push(new_scissor);
                        draw_cmds.push(GpuDrawCmd::SetScissor(new_scissor));
                    }
                }
                RenderItem::PopClip => {
                    if scissor_stack.len() > 1 {
                        scissor_stack.pop();
                    }
                    let top = scissor_stack
                        .last()
                        .copied()
                        .unwrap_or(ScissorRect::full(target_width, target_height));
                    draw_cmds.push(GpuDrawCmd::SetScissor(top));
                }
                RenderItem::Path(render_path) => {
                    let combined_tf = render_path.transform.multiply(&page_transform);
                    if let Some(lyon_path) =
                        Self::convert_path_to_screen_lyon(&render_path.verbs, &combined_tf)
                    {
                        let mut geometry: VertexBuffers<GpuVertex, u32> = VertexBuffers::new();

                        // 1. Fill tessellation
                        if let Some(fill) = &render_path.fill {
                            let fill_rule = match fill.rule {
                                FillRule::NonZero => LyonFillRule::NonZero,
                                FillRule::EvenOdd => LyonFillRule::EvenOdd,
                            };
                            let fill_options = FillOptions::default()
                                .with_fill_rule(fill_rule)
                                .with_tolerance(0.25);
                            let [r, g, b, a] = fill.color.to_u8_array();
                            let color = [
                                r as f32 / 255.0,
                                g as f32 / 255.0,
                                b as f32 / 255.0,
                                (a as f32 / 255.0) * render_path.fill_alpha,
                            ];
                            let mut tess = FillTessellator::new();
                            let _ = tess.tessellate_path(
                                &lyon_path,
                                &fill_options,
                                &mut BuffersBuilder::new(
                                    &mut geometry,
                                    |vertex: lyon_tessellation::FillVertex| GpuVertex {
                                        position: [vertex.position().x, vertex.position().y],
                                        color,
                                    },
                                ),
                            );
                        }

                        // 2. Stroke tessellation
                        if let Some(stroke) = &render_path.stroke {
                            let stroke_scale = ((combined_tf.a * combined_tf.a
                                + combined_tf.b * combined_tf.b)
                                .sqrt()
                                + (combined_tf.c * combined_tf.c
                                    + combined_tf.d * combined_tf.d)
                                    .sqrt())
                                * 0.5;
                            let line_width = (stroke.width * stroke_scale).max(0.5);

                            let stroke_options = StrokeOptions::default()
                                .with_line_width(line_width)
                                .with_line_cap(match stroke.cap {
                                    LineCap::Butt => lyon_tessellation::LineCap::Butt,
                                    LineCap::Round => lyon_tessellation::LineCap::Round,
                                    LineCap::Square => lyon_tessellation::LineCap::Square,
                                })
                                .with_line_join(match stroke.join {
                                    LineJoin::Miter => lyon_tessellation::LineJoin::Miter,
                                    LineJoin::Round => lyon_tessellation::LineJoin::Round,
                                    LineJoin::Bevel => lyon_tessellation::LineJoin::Bevel,
                                })
                                .with_miter_limit(stroke.miter_limit)
                                .with_tolerance(0.25);

                            let [r, g, b, a] = stroke.color.to_u8_array();
                            let color = [
                                r as f32 / 255.0,
                                g as f32 / 255.0,
                                b as f32 / 255.0,
                                (a as f32 / 255.0) * render_path.stroke_alpha,
                            ];
                            let mut tess = StrokeTessellator::new();
                            let _ = tess.tessellate_path(
                                &lyon_path,
                                &stroke_options,
                                &mut BuffersBuilder::new(
                                    &mut geometry,
                                    |vertex: lyon_tessellation::StrokeVertex| GpuVertex {
                                        position: [vertex.position().x, vertex.position().y],
                                        color,
                                    },
                                ),
                            );
                        }

                        if !geometry.indices.is_empty() && !geometry.vertices.is_empty() {
                            let t_u = Instant::now();
                            let vbuf = self.device.create_buffer(&wgpu::BufferDescriptor {
                                label: Some("gpu_vector_vbuf"),
                                size: (geometry.vertices.len()
                                    * std::mem::size_of::<GpuVertex>())
                                    as u64,
                                usage: wgpu::BufferUsages::VERTEX
                                    | wgpu::BufferUsages::COPY_DST,
                                mapped_at_creation: false,
                            });
                            self.queue.write_buffer(
                                &vbuf,
                                0,
                                bytemuck::cast_slice(&geometry.vertices),
                            );

                            let ibuf = self.device.create_buffer(&wgpu::BufferDescriptor {
                                label: Some("gpu_vector_ibuf"),
                                size: (geometry.indices.len() * std::mem::size_of::<u32>())
                                    as u64,
                                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                                mapped_at_creation: false,
                            });
                            self.queue.write_buffer(
                                &ibuf,
                                0,
                                bytemuck::cast_slice(&geometry.indices),
                            );
                            upload_dur += t_u.elapsed().as_secs_f64() * 1000.0;

                            draw_cmds.push(GpuDrawCmd::DrawVector {
                                vbuf,
                                ibuf,
                                index_count: geometry.indices.len() as u32,
                            });
                            gpu_draw_calls += 1;
                            gpu_paths_rendered += 1;
                        }
                    }
                }
                RenderItem::Image(render_img) => {
                    let t_u = Instant::now();
                    let img_tex = self.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("gpu_image_texture"),
                        size: wgpu::Extent3d {
                            width: render_img.width,
                            height: render_img.height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING
                            | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    });

                    self.queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &img_tex,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        &render_img.rgba_data,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(render_img.width * 4),
                            rows_per_image: Some(render_img.height),
                        },
                        wgpu::Extent3d {
                            width: render_img.width,
                            height: render_img.height,
                            depth_or_array_layers: 1,
                        },
                    );

                    let img_view = img_tex.create_view(&wgpu::TextureViewDescriptor::default());
                    let img_bind_group =
                        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("gpu_image_bind_group"),
                            layout: &self.texture_bind_group_layout,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: wgpu::BindingResource::TextureView(&img_view),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                                },
                            ],
                        });

                    let sx = 1.0 / render_img.width as f32;
                    let sy = -1.0 / render_img.height as f32;
                    let img_unit_tf = Transform2D::new(sx, 0.0, 0.0, sy, 0.0, 1.0);
                    let final_tf = img_unit_tf
                        .multiply(&render_img.transform)
                        .multiply(&page_transform);

                    let p0 = final_tf.transform_point(0.0, 0.0);
                    let p1 = final_tf.transform_point(render_img.width as f32, 0.0);
                    let p2 = final_tf
                        .transform_point(render_img.width as f32, render_img.height as f32);
                    let p3 = final_tf.transform_point(0.0, render_img.height as f32);

                    let color = [1.0, 1.0, 1.0, render_img.opacity];
                    let img_vertices = [
                        GpuTextureVertex {
                            position: [p0.0, p0.1],
                            tex_coords: [0.0, 0.0],
                            color,
                        },
                        GpuTextureVertex {
                            position: [p3.0, p3.1],
                            tex_coords: [0.0, 1.0],
                            color,
                        },
                        GpuTextureVertex {
                            position: [p2.0, p2.1],
                            tex_coords: [1.0, 1.0],
                            color,
                        },
                        GpuTextureVertex {
                            position: [p0.0, p0.1],
                            tex_coords: [0.0, 0.0],
                            color,
                        },
                        GpuTextureVertex {
                            position: [p2.0, p2.1],
                            tex_coords: [1.0, 1.0],
                            color,
                        },
                        GpuTextureVertex {
                            position: [p1.0, p1.1],
                            tex_coords: [1.0, 0.0],
                            color,
                        },
                    ];

                    let img_vbuf = self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("gpu_image_vbuf"),
                        size: (img_vertices.len() * std::mem::size_of::<GpuTextureVertex>())
                            as u64,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    self.queue
                        .write_buffer(&img_vbuf, 0, bytemuck::cast_slice(&img_vertices));
                    upload_dur += t_u.elapsed().as_secs_f64() * 1000.0;

                    draw_cmds.push(GpuDrawCmd::DrawTexture {
                        bind_group: img_bind_group,
                        vbuf: img_vbuf,
                    });
                    gpu_draw_calls += 1;
                    gpu_images_rendered += 1;
                }
                RenderItem::TextRun(text_run) => {
                    let mut fill_tess = FillTessellator::new();
                    let mut stroke_tess = StrokeTessellator::new();
                    let mut run_mesh: VertexBuffers<GpuVertex, u32> = VertexBuffers::new();
                    let mut run_glyphs_drawn = 0;
                    let mut font_failed = false;

                    let is_all_whitespace = text_run
                        .glyphs
                        .iter()
                        .all(|g| g.text.chars().all(char::is_whitespace));

                    if !is_all_whitespace
                        && text_run.font_size > 0.0
                        && text_run.render_mode != 3
                    {
                        let fcol = text_run.fill_color.unwrap_or(ColorRgba::BLACK);
                        let [fr, fg, fb, fa] = fcol.to_u8_array();
                        let fill_color = [
                            fr as f32 / 255.0,
                            fg as f32 / 255.0,
                            fb as f32 / 255.0,
                            (fa as f32 / 255.0) * text_run.fill_alpha,
                        ];

                        let scol = text_run.stroke_color.unwrap_or(ColorRgba::BLACK);
                        let [sr, sg, sb, sa] = scol.to_u8_array();
                        let stroke_color = [
                            sr as f32 / 255.0,
                            sg as f32 / 255.0,
                            sb as f32 / 255.0,
                            (sa as f32 / 255.0) * text_run.stroke_alpha,
                        ];

                        let should_fill = text_run.render_mode == 0
                            || text_run.render_mode == 2
                            || text_run.render_mode == 4
                            || text_run.render_mode == 6;

                        let should_stroke = text_run.render_mode == 1
                            || text_run.render_mode == 2
                            || text_run.render_mode == 5
                            || text_run.render_mode == 6;

                        for glyph in &text_run.glyphs {
                            let ch = glyph.text.chars().next().unwrap_or(' ');
                            let candidate_bytes = text_run
                                .font_data
                                .as_deref()
                                .or_else(|| self.fallback_fonts.font_for_char(ch));

                            if let Some(bytes) = candidate_bytes {
                                if let Ok(font_ref) = FontRef::from_index(bytes, 0) {
                                    let outlines = font_ref.outline_glyphs();
                                    let charmap = font_ref.charmap();

                                    let gid = glyph
                                        .glyph_id
                                        .map(|g| skrifa::GlyphId::new(g as u32))
                                        .or_else(|| {
                                            char::from_u32(glyph.codepoint)
                                                .and_then(|c| charmap.map(c))
                                        });

                                    if let Some(gid) = gid {
                                        if let Some(outline_glyph) = outlines.get(gid) {
                                            let glyph_tf = Transform2D::new(
                                                1.0, 0.0, 0.0, 1.0, glyph.x, glyph.y,
                                            );
                                            let combined = glyph_tf
                                                .multiply(&text_run.transform)
                                                .multiply(&page_transform);

                                            let mut builder = lyon_path::Path::svg_builder();
                                            let mut pen = LyonPen {
                                                builder: &mut builder,
                                                transform: combined,
                                                has_moved: false,
                                            };
                                            let draw_settings =
                                                skrifa::outline::DrawSettings::unhinted(
                                                    Size::new(text_run.font_size),
                                                    skrifa::instance::LocationRef::default(),
                                                );
                                            if outline_glyph
                                                .draw(draw_settings, &mut pen)
                                                .is_ok()
                                            {
                                                let path = builder.build();
                                                if path.iter().next().is_some() {
                                                    if should_fill {
                                                        let fill_options =
                                                            FillOptions::default()
                                                                .with_fill_rule(
                                                                    LyonFillRule::NonZero,
                                                                )
                                                                .with_tolerance(0.2);
                                                        let _ = fill_tess.tessellate_path(
                                                            &path,
                                                            &fill_options,
                                                            &mut BuffersBuilder::new(
                                                                &mut run_mesh,
                                                                |vertex: lyon_tessellation::FillVertex| {
                                                                    GpuVertex {
                                                                        position: [
                                                                            vertex.position().x,
                                                                            vertex.position().y,
                                                                        ],
                                                                        color: fill_color,
                                                                    }
                                                                },
                                                            ),
                                                        );
                                                    }
                                                    if should_stroke {
                                                        let stroke_scale = ((combined.a
                                                            * combined.a
                                                            + combined.b * combined.b)
                                                            .sqrt()
                                                            + (combined.c * combined.c
                                                                + combined.d * combined.d)
                                                                .sqrt())
                                                            * 0.5;
                                                        let stroke_width = (text_run.font_size
                                                            * 0.05)
                                                            .max(0.5)
                                                            * stroke_scale;
                                                        let stroke_options =
                                                            StrokeOptions::default()
                                                                .with_line_width(stroke_width)
                                                                .with_tolerance(0.2);
                                                        let _ = stroke_tess.tessellate_path(
                                                            &path,
                                                            &stroke_options,
                                                            &mut BuffersBuilder::new(
                                                                &mut run_mesh,
                                                                |vertex: lyon_tessellation::StrokeVertex| {
                                                                    GpuVertex {
                                                                        position: [
                                                                            vertex.position().x,
                                                                            vertex.position().y,
                                                                        ],
                                                                        color: stroke_color,
                                                                    }
                                                                },
                                                            ),
                                                        );
                                                    }
                                                }
                                                run_glyphs_drawn += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if run_glyphs_drawn == 0 && !is_all_whitespace {
                            font_failed = true;
                        }
                    }

                    if !font_failed {
                        if !run_mesh.indices.is_empty() {
                            let t_u = Instant::now();
                            let vbuf = self.device.create_buffer(&wgpu::BufferDescriptor {
                                label: Some("gpu_text_vbuf"),
                                size: (run_mesh.vertices.len()
                                    * std::mem::size_of::<GpuVertex>())
                                    as u64,
                                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                                mapped_at_creation: false,
                            });
                            self.queue.write_buffer(
                                &vbuf,
                                0,
                                bytemuck::cast_slice(&run_mesh.vertices),
                            );

                            let ibuf = self.device.create_buffer(&wgpu::BufferDescriptor {
                                label: Some("gpu_text_ibuf"),
                                size: (run_mesh.indices.len() * std::mem::size_of::<u32>())
                                    as u64,
                                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                                mapped_at_creation: false,
                            });
                            self.queue.write_buffer(
                                &ibuf,
                                0,
                                bytemuck::cast_slice(&run_mesh.indices),
                            );
                            upload_dur += t_u.elapsed().as_secs_f64() * 1000.0;

                            draw_cmds.push(GpuDrawCmd::DrawVector {
                                vbuf,
                                ibuf,
                                index_count: run_mesh.indices.len() as u32,
                            });
                            gpu_draw_calls += 1;
                            gpu_glyphs_rendered += run_glyphs_drawn;
                        }
                    } else {
                        // Scoped CPU fallback for this single text run
                        fallback_cpu_items += 1;
                        if !fallback_reasons.iter().any(|r: &String| r == "TextRun") {
                            fallback_reasons.push("TextRun".to_string());
                        }

                        let t_c0 = Instant::now();
                        let mut sub_page = page.clone();
                        sub_page.items = vec![item.clone()];
                        if let Some(cpu_res) = NativeRasterizer::render_with_background(
                            &sub_page,
                            scale,
                            tiny_skia::Color::TRANSPARENT,
                        ) {
                            cpu_render_dur += t_c0.elapsed().as_secs_f64() * 1000.0;

                            let t_u = Instant::now();
                            let fb_tex = self.device.create_texture(&wgpu::TextureDescriptor {
                                label: Some("gpu_text_fallback_texture"),
                                size: wgpu::Extent3d {
                                    width: target_width,
                                    height: target_height,
                                    depth_or_array_layers: 1,
                                },
                                mip_level_count: 1,
                                sample_count: 1,
                                dimension: wgpu::TextureDimension::D2,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                usage: wgpu::TextureUsages::TEXTURE_BINDING
                                    | wgpu::TextureUsages::COPY_DST,
                                view_formats: &[],
                            });

                            self.queue.write_texture(
                                wgpu::TexelCopyTextureInfo {
                                    texture: &fb_tex,
                                    mip_level: 0,
                                    origin: wgpu::Origin3d::ZERO,
                                    aspect: wgpu::TextureAspect::All,
                                },
                                &cpu_res.rgba_pixels,
                                wgpu::TexelCopyBufferLayout {
                                    offset: 0,
                                    bytes_per_row: Some(target_width * 4),
                                    rows_per_image: Some(target_height),
                                },
                                wgpu::Extent3d {
                                    width: target_width,
                                    height: target_height,
                                    depth_or_array_layers: 1,
                                },
                            );

                            let fb_view =
                                fb_tex.create_view(&wgpu::TextureViewDescriptor::default());
                            let fb_bind_group =
                                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                                    label: Some("gpu_text_fallback_bind_group"),
                                    layout: &self.texture_bind_group_layout,
                                    entries: &[
                                        wgpu::BindGroupEntry {
                                            binding: 0,
                                            resource: wgpu::BindingResource::TextureView(&fb_view),
                                        },
                                        wgpu::BindGroupEntry {
                                            binding: 1,
                                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                                        },
                                    ],
                                });

                            let w = target_width as f32;
                            let h = target_height as f32;
                            let quad_vertices = [
                                GpuTextureVertex {
                                    position: [0.0, 0.0],
                                    tex_coords: [0.0, 0.0],
                                    color: [1.0, 1.0, 1.0, 1.0],
                                },
                                GpuTextureVertex {
                                    position: [0.0, h],
                                    tex_coords: [0.0, 1.0],
                                    color: [1.0, 1.0, 1.0, 1.0],
                                },
                                GpuTextureVertex {
                                    position: [w, h],
                                    tex_coords: [1.0, 1.0],
                                    color: [1.0, 1.0, 1.0, 1.0],
                                },
                                GpuTextureVertex {
                                    position: [0.0, 0.0],
                                    tex_coords: [0.0, 0.0],
                                    color: [1.0, 1.0, 1.0, 1.0],
                                },
                                GpuTextureVertex {
                                    position: [w, h],
                                    tex_coords: [1.0, 1.0],
                                    color: [1.0, 1.0, 1.0, 1.0],
                                },
                                GpuTextureVertex {
                                    position: [w, 0.0],
                                    tex_coords: [1.0, 0.0],
                                    color: [1.0, 1.0, 1.0, 1.0],
                                },
                            ];
                            let quad_vbuf =
                                self.device.create_buffer(&wgpu::BufferDescriptor {
                                    label: Some("gpu_text_fallback_quad_vbuf"),
                                    size: (quad_vertices.len()
                                        * std::mem::size_of::<GpuTextureVertex>())
                                        as u64,
                                    usage: wgpu::BufferUsages::VERTEX
                                        | wgpu::BufferUsages::COPY_DST,
                                    mapped_at_creation: false,
                                });
                            self.queue.write_buffer(
                                &quad_vbuf,
                                0,
                                bytemuck::cast_slice(&quad_vertices),
                            );
                            upload_dur += t_u.elapsed().as_secs_f64() * 1000.0;

                            draw_cmds.push(GpuDrawCmd::DrawTexture {
                                bind_group: fb_bind_group,
                                vbuf: quad_vbuf,
                            });
                            gpu_draw_calls += 1;
                        }
                    }
                }
                RenderItem::Shading(_) => {
                    // Scoped CPU fallback for this single shading item
                    fallback_cpu_items += 1;
                    if !fallback_reasons.iter().any(|r: &String| r == "Shading") {
                        fallback_reasons.push("Shading".to_string());
                    }

                    let t_c0 = Instant::now();
                    let mut sub_page = page.clone();
                    sub_page.items = vec![item.clone()];
                    if let Some(cpu_res) = NativeRasterizer::render_with_background(
                        &sub_page,
                        scale,
                        tiny_skia::Color::TRANSPARENT,
                    ) {
                        cpu_render_dur += t_c0.elapsed().as_secs_f64() * 1000.0;

                        let t_u = Instant::now();
                        let fb_tex = self.device.create_texture(&wgpu::TextureDescriptor {
                            label: Some("gpu_shading_fallback_texture"),
                            size: wgpu::Extent3d {
                                width: target_width,
                                height: target_height,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            usage: wgpu::TextureUsages::TEXTURE_BINDING
                                | wgpu::TextureUsages::COPY_DST,
                            view_formats: &[],
                        });

                        self.queue.write_texture(
                            wgpu::TexelCopyTextureInfo {
                                texture: &fb_tex,
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                                aspect: wgpu::TextureAspect::All,
                            },
                            &cpu_res.rgba_pixels,
                            wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(target_width * 4),
                                rows_per_image: Some(target_height),
                            },
                            wgpu::Extent3d {
                                width: target_width,
                                height: target_height,
                                depth_or_array_layers: 1,
                            },
                        );

                        let fb_view =
                            fb_tex.create_view(&wgpu::TextureViewDescriptor::default());
                        let fb_bind_group =
                            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                                label: Some("gpu_shading_fallback_bind_group"),
                                layout: &self.texture_bind_group_layout,
                                entries: &[
                                    wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: wgpu::BindingResource::TextureView(&fb_view),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 1,
                                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                                    },
                                ],
                            });

                        let w = target_width as f32;
                        let h = target_height as f32;
                        let quad_vertices = [
                            GpuTextureVertex {
                                position: [0.0, 0.0],
                                tex_coords: [0.0, 0.0],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GpuTextureVertex {
                                position: [0.0, h],
                                tex_coords: [0.0, 1.0],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GpuTextureVertex {
                                position: [w, h],
                                tex_coords: [1.0, 1.0],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GpuTextureVertex {
                                position: [0.0, 0.0],
                                tex_coords: [0.0, 0.0],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GpuTextureVertex {
                                position: [w, h],
                                tex_coords: [1.0, 1.0],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GpuTextureVertex {
                                position: [w, 0.0],
                                tex_coords: [1.0, 0.0],
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                        ];
                        let quad_vbuf =
                            self.device.create_buffer(&wgpu::BufferDescriptor {
                                label: Some("gpu_shading_fallback_quad_vbuf"),
                                size: (quad_vertices.len()
                                    * std::mem::size_of::<GpuTextureVertex>())
                                    as u64,
                                usage: wgpu::BufferUsages::VERTEX
                                    | wgpu::BufferUsages::COPY_DST,
                                mapped_at_creation: false,
                            });
                        self.queue.write_buffer(
                            &quad_vbuf,
                            0,
                            bytemuck::cast_slice(&quad_vertices),
                        );
                        upload_dur += t_u.elapsed().as_secs_f64() * 1000.0;

                        draw_cmds.push(GpuDrawCmd::DrawTexture {
                            bind_group: fb_bind_group,
                            vbuf: quad_vbuf,
                        });
                        gpu_draw_calls += 1;
                    }
                }
            }
        }

        // 4. Command encoder and single render pass drawing all items in exact sequential IR order
        let t_render0 = Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gpu_page_command_encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gpu_main_raster_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            let mut current_pipeline: Option<u8> = None; // 0 = vector, 1 = texture

            let full = ScissorRect::full(target_width, target_height);
            pass.set_scissor_rect(0, 0, full.width, full.height);

            for cmd in &draw_cmds {
                match cmd {
                    GpuDrawCmd::SetScissor(rect) => {
                        let clamped_w = rect
                            .width
                            .max(1)
                            .min(target_width.saturating_sub(rect.x).max(1));
                        let clamped_h = rect
                            .height
                            .max(1)
                            .min(target_height.saturating_sub(rect.y).max(1));
                        pass.set_scissor_rect(
                            rect.x.min(target_width.saturating_sub(1)),
                            rect.y.min(target_height.saturating_sub(1)),
                            clamped_w,
                            clamped_h,
                        );
                    }
                    GpuDrawCmd::DrawVector {
                        vbuf,
                        ibuf,
                        index_count,
                    } => {
                        if current_pipeline != Some(0) {
                            pass.set_pipeline(&self.vector_pipeline);
                            pass.set_bind_group(0, &uniform_bind_group, &[]);
                            current_pipeline = Some(0);
                        }
                        pass.set_vertex_buffer(0, vbuf.slice(..));
                        pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..*index_count, 0, 0..1);
                    }
                    GpuDrawCmd::DrawTexture { bind_group, vbuf } => {
                        if current_pipeline != Some(1) {
                            pass.set_pipeline(&self.texture_pipeline);
                            pass.set_bind_group(0, &uniform_bind_group, &[]);
                            current_pipeline = Some(1);
                        }
                        pass.set_bind_group(1, bind_group, &[]);
                        pass.set_vertex_buffer(0, vbuf.slice(..));
                        pass.draw(0..6, 0..1);
                    }
                }
            }
        } // pass is dropped here, encoder is completely unborrowed

        // 5. Staging buffer copy for GPU readback
        let bytes_per_pixel = 4;
        let unpadded_bytes_per_row = target_width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT; // 256
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
        let buffer_size = (padded_bytes_per_row * target_height) as u64;

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_readback_buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(target_height),
                },
            },
            wgpu::Extent3d {
                width: target_width,
                height: target_height,
                depth_or_array_layers: 1,
            },
        );

        // Submit to GPU queue
        self.queue.submit([encoder.finish()]);
        let render_dur = t_render0.elapsed().as_secs_f64() * 1000.0;

        // Readback
        let t_read0 = Instant::now();
        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });

        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| format!("GPU device poll error: {e:?}"))?;

        rx.recv()
            .map_err(|e| format!("Channel receive error: {e:?}"))?
            .map_err(|e| format!("Buffer map error: {e:?}"))?;

        let mapped_data = buffer_slice
            .get_mapped_range()
            .map_err(|e| format!("Get mapped range error: {e:?}"))?;
        let mut rgba_pixels = Vec::with_capacity((target_width * target_height * 4) as usize);

        for row in 0..target_height {
            let row_start = (row * padded_bytes_per_row) as usize;
            let row_end = row_start + (target_width * 4) as usize;
            rgba_pixels.extend_from_slice(&mapped_data[row_start..row_end]);
        }
        drop(mapped_data);
        staging_buffer.unmap();

        let readback_dur = t_read0.elapsed().as_secs_f64() * 1000.0;
        let total_frame_dur = frame_start.elapsed().as_secs_f64() * 1000.0;

        Ok(GpuRenderResult {
            rgba_pixels,
            width: target_width,
            height: target_height,
            metrics: GpuPerformanceMetrics {
                upload_duration_ms: upload_dur,
                render_duration_ms: render_dur,
                readback_duration_ms: readback_dur,
                total_frame_duration_ms: total_frame_dur,
                cpu_render_duration_ms: cpu_render_dur,
                gpu_draw_calls,
                gpu_paths_rendered,
                gpu_glyphs_rendered,
                gpu_images_rendered,
                fallback_cpu_items,
                fallback_reasons,
            },
        })
    }

    fn convert_path_to_screen_lyon(
        verbs: &[PathVerb],
        transform: &Transform2D,
    ) -> Option<lyon_path::Path> {
        if verbs.is_empty() {
            return None;
        }
        let mut builder = lyon_path::Path::svg_builder();
        let mut has_moved = false;

        for verb in verbs {
            match verb {
                PathVerb::MoveTo(x, y) => {
                    let (px, py) = transform.transform_point(*x, *y);
                    builder.move_to(point(px, py));
                    has_moved = true;
                }
                PathVerb::LineTo(x, y) => {
                    let (px, py) = transform.transform_point(*x, *y);
                    if has_moved {
                        builder.line_to(point(px, py));
                    } else {
                        builder.move_to(point(px, py));
                        has_moved = true;
                    }
                }
                PathVerb::QuadTo(cx, cy, x, y) => {
                    let (pcx, pcy) = transform.transform_point(*cx, *cy);
                    let (px, py) = transform.transform_point(*x, *y);
                    if !has_moved {
                        builder.move_to(point(pcx, pcy));
                        has_moved = true;
                    }
                    builder.quadratic_bezier_to(point(pcx, pcy), point(px, py));
                }
                PathVerb::CubicTo(cx0, cy0, cx1, cy1, x, y) => {
                    let (pcx0, pcy0) = transform.transform_point(*cx0, *cy0);
                    let (pcx1, pcy1) = transform.transform_point(*cx1, *cy1);
                    let (px, py) = transform.transform_point(*x, *y);
                    if !has_moved {
                        builder.move_to(point(pcx0, pcy0));
                        has_moved = true;
                    }
                    builder.cubic_bezier_to(point(pcx0, pcy0), point(pcx1, pcy1), point(px, py));
                }
                PathVerb::Close => {
                    if has_moved {
                        builder.close();
                    }
                }
            }
        }
        Some(builder.build())
    }
}

struct LyonPen<'a, B: SvgPathBuilder> {
    builder: &'a mut B,
    transform: Transform2D,
    has_moved: bool,
}

impl<B: SvgPathBuilder> OutlinePen for LyonPen<'_, B> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (px, py) = self.transform.transform_point(x, y);
        self.builder.move_to(point(px, py));
        self.has_moved = true;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (px, py) = self.transform.transform_point(x, y);
        if self.has_moved {
            self.builder.line_to(point(px, py));
        } else {
            self.builder.move_to(point(px, py));
            self.has_moved = true;
        }
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let (pcx0, pcy0) = self.transform.transform_point(cx0, cy0);
        let (px, py) = self.transform.transform_point(x, y);
        if !self.has_moved {
            self.builder.move_to(point(pcx0, pcy0));
            self.has_moved = true;
        }
        self.builder.quadratic_bezier_to(point(pcx0, pcy0), point(px, py));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let (pcx0, pcy0) = self.transform.transform_point(cx0, cy0);
        let (pcx1, pcy1) = self.transform.transform_point(cx1, cy1);
        let (px, py) = self.transform.transform_point(x, y);
        if !self.has_moved {
            self.builder.move_to(point(pcx0, pcy0));
            self.has_moved = true;
        }
        self.builder.cubic_bezier_to(point(pcx0, pcy0), point(pcx1, pcy1), point(px, py));
    }

    fn close(&mut self) {
        if self.has_moved {
            self.builder.close();
            self.has_moved = false;
        }
    }
}
