use std::time::Instant;
use crate::render::ir::{RenderItem, RenderPage};
use crate::render::rasterizer::NativeRasterizer;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GpuPerformanceMetrics {
    pub upload_duration_ms: f64,
    pub render_duration_ms: f64,
    pub readback_duration_ms: f64,
    pub total_frame_duration_ms: f64,
    pub gpu_draw_calls: usize,
    pub fallback_cpu_items: usize,
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
struct QuadVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

pub struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    adapter_name: String,
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

        let shader_src = r#"
            struct VertexInput {
                @location(0) position: vec2<f32>,
                @location(1) tex_coords: vec2<f32>,
            };

            struct VertexOutput {
                @builtin(position) clip_position: vec4<f32>,
                @location(0) tex_coords: vec2<f32>,
            };

            @vertex
            fn vs_main(in: VertexInput) -> VertexOutput {
                var out: VertexOutput;
                out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
                out.tex_coords = in.tex_coords;
                return out;
            }

            @group(0) @binding(0) var t_diffuse: texture_2d<f32>;
            @group(0) @binding(1) var s_diffuse: sampler;

            @fragment
            fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                return textureSample(t_diffuse, s_diffuse, in.tex_coords);
            }
        "#;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("quad_shader"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("quad_bind_group_layout"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quad_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
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
            ],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("quad_render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(vertex_buffer_layout)],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Ok(Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
            sampler,
            adapter_name,
        })
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    pub fn render_page(&self, page: &RenderPage, scale: f32) -> Result<GpuRenderResult, String> {
        let frame_start = Instant::now();
        let mut upload_dur = 0.0;
        let mut gpu_draw_calls = 0;
        let mut fallback_cpu_items = 0;

        let (target_width, target_height) = match page.rotate % 360 {
            90 | 270 => (
                ((page.height * scale).round() as u32).max(1),
                ((page.width * scale).round() as u32).max(1),
            ),
            _ => (
                ((page.width * scale).round() as u32).max(1),
                ((page.height * scale).round() as u32).max(1),
            ),
        };

        let texture_desc = wgpu::TextureDescriptor {
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
        };
        let target_texture = self.device.create_texture(&texture_desc);
        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gpu_frame_encoder"),
            });

        // 1. Initial clear pass to white
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear_pass"),
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
        }

        // 2. Identify and execute GPU accelerated vs CPU fallback layers
        let has_direct_gpu_images = page.items.iter().any(|it| matches!(it, RenderItem::Image(_)));

        if has_direct_gpu_images {
            let t_up0 = Instant::now();
            let cpu_rendered = NativeRasterizer::render(page, scale)
                .ok_or_else(|| "CPU fallback render failed".to_string())?;
            fallback_cpu_items += page.items.len();

            let upload_tex = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("cpu_layer_upload"),
                size: wgpu::Extent3d {
                    width: target_width,
                    height: target_height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &upload_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &cpu_rendered.rgba_pixels,
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

            let upload_view = upload_tex.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cpu_layer_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&upload_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

            // Fullscreen quad in NDC [-1, 1]
            let vertices = [
                QuadVertex { position: [-1.0, 1.0], tex_coords: [0.0, 0.0] },
                QuadVertex { position: [-1.0, -1.0], tex_coords: [0.0, 1.0] },
                QuadVertex { position: [1.0, -1.0], tex_coords: [1.0, 1.0] },
                QuadVertex { position: [-1.0, 1.0], tex_coords: [0.0, 0.0] },
                QuadVertex { position: [1.0, -1.0], tex_coords: [1.0, 1.0] },
                QuadVertex { position: [1.0, 1.0], tex_coords: [1.0, 0.0] },
            ];

            let vertex_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("quad_vertex_buffer"),
                size: (vertices.len() * std::mem::size_of::<QuadVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(&vertex_buf, 0, bytemuck::cast_slice(&vertices));
            upload_dur += t_up0.elapsed().as_secs_f64() * 1000.0;

            // Render pass
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("composite_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target_view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buf.slice(..));
                pass.draw(0..6, 0..1);
            }
            gpu_draw_calls += 1;
        } else {
            let t_up0 = Instant::now();
            let cpu_rendered = NativeRasterizer::render(page, scale)
                .ok_or_else(|| "CPU fallback render failed".to_string())?;
            fallback_cpu_items += page.items.len();

            let upload_tex = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("general_layer_upload"),
                size: wgpu::Extent3d {
                    width: target_width,
                    height: target_height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &upload_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &cpu_rendered.rgba_pixels,
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

            let upload_view = upload_tex.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("general_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&upload_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

            let vertices = [
                QuadVertex { position: [-1.0, 1.0], tex_coords: [0.0, 0.0] },
                QuadVertex { position: [-1.0, -1.0], tex_coords: [0.0, 1.0] },
                QuadVertex { position: [1.0, -1.0], tex_coords: [1.0, 1.0] },
                QuadVertex { position: [-1.0, 1.0], tex_coords: [0.0, 0.0] },
                QuadVertex { position: [1.0, -1.0], tex_coords: [1.0, 1.0] },
                QuadVertex { position: [1.0, 1.0], tex_coords: [1.0, 0.0] },
            ];

            let vertex_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("quad_vertex_buffer"),
                size: (vertices.len() * std::mem::size_of::<QuadVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(&vertex_buf, 0, bytemuck::cast_slice(&vertices));
            upload_dur += t_up0.elapsed().as_secs_f64() * 1000.0;

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("composite_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target_view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buf.slice(..));
                pass.draw(0..6, 0..1);
            }
            gpu_draw_calls += 1;
        }

        // 3. Staging buffer copy for GPU readback
        let bytes_per_pixel = 4;
        let unpadded_bytes_per_row = target_width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT; // 256
        let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;
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
        let t_render0 = Instant::now();
        self.queue.submit([encoder.finish()]);
        let render_dur = t_render0.elapsed().as_secs_f64() * 1000.0;

        // Readback
        let t_read0 = Instant::now();
        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });

        self.device.poll(wgpu::PollType::wait_indefinitely())
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
                gpu_draw_calls,
                fallback_cpu_items,
            },
        })
    }
}
