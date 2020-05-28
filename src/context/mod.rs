mod lines;
mod resources;

pub use self::lines::*;

pub use self::resources::{Image, Model, MeshArray, Resources, BILLBOARD_VERTICES};
pub const WHITE: [f32; 3] = [1.0; 3];

use {
    winit::{
        self,
        *,
    },
    cgmath::*,
    crate::camera::*,
    crate::*,
    std::f32::consts::PI,
    zerocopy::AsBytes,
    wgpu::vertex_attr_array,
    pedot::Gui,
};

// ** Line Rendering Woes **
// rendering in 2d: doesnt work with rest of scene, rendering lines that go behind the camera is hard
// gl_lines: has a max width of 1 on my laptop
// 2d lines in 3d: getting lines to join nicely is hard, too flat
// geometry shader: complicated
// assembling triangle/square line meshes by hand: complicated, but might be best shot

#[derive(Clone, Copy)]
pub enum Mode {
    Normal = 1,
    Shadeless = 2,
    White = 3,
    VertexColoured = 4
}

#[repr(C)]
#[derive(Copy, Clone, Serialize, Deserialize, zerocopy::AsBytes)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub diff_texture: [f32; 2],
    pub spec_texture: [f32; 2]
}

impl Vertex {
    pub fn with_colour(position: Vector3<f32>, colour: [f32; 3]) -> Self {
        Self {
            position: position.into(),
            normal: colour,
            diff_texture: [0.0; 2],
            spec_texture: [0.0; 2]
        }
    }
}

pub struct Context {
    window: winit::window::Window,
    device: wgpu::Device,
    swap_chain: wgpu::SwapChain,
    queue: wgpu::Queue,
    surface: wgpu::Surface,

    bind_group_layout: wgpu::BindGroupLayout,
    triangle_pipeline: wgpu::RenderPipeline,
    billboard_pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,

    identity_instance: wgpu::Buffer,
    billboard_vertices: wgpu::Buffer,

    depth_texture: wgpu::TextureView,

    pub resources: Resources,

    lines: LineRenderer,

    glyph_brush: wgpu_glyph::GlyphBrush<'static, ()>,

    pub gui: Gui
}

impl Context {
    pub async fn new(event_loop: &event_loop::EventLoop<()>) -> (Self, MeshArray) {
        let window = winit::window::Window::new(event_loop).unwrap();

        #[cfg(feature = "wasm")]
        {
            window.set_inner_size(winit::dpi::LogicalSize::new(1850.0, 1000.0));

            use winit::platform::web::WindowExtWebSys;
            let web_win = web_sys::window().unwrap();
            let document = web_win.document().unwrap();
            let body = document.body().unwrap();

            let canvas = window.canvas();
            canvas.set_oncontextmenu(Some(&js_sys::Function::new_no_args("return false;")));
            body.append_child(&web_sys::Element::from(canvas)).unwrap();
        }

        let instance = wgpu::Instance::new();
        let surface = unsafe {
            instance.create_surface(&window)
        };

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: None,
            },
            wgpu::BackendBit::PRIMARY,
        )
            .await
            .unwrap();
    
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default(),
        }, None).await.unwrap();

        let vs = include_bytes!("shaders/shader.vert.spv");
        let vs_module =
            device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&vs[..])).unwrap());
    
        let fs = include_bytes!("shaders/shader.frag.spv");
        let fs_module =
            device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&fs[..])).unwrap());
        
        let mut init_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        
        let (resources, meshes) = Resources::new(&mut init_encoder, &device).unwrap();
        
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: wgpu::CompareFunction::Undefined,
            label: None
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT | wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer {
                            dynamic: false
                        },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Float,
                        },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                    },
                ],
                label: None,
            });
    
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

        let triangle_pipeline = create_pipeline(&device, &pipeline_layout, &vs_module, &fs_module, wgpu::PrimitiveTopology::TriangleList, true);
        let billboard_pipeline = create_pipeline(&device, &pipeline_layout, &vs_module, &fs_module, wgpu::PrimitiveTopology::TriangleList, false);

        let identity_instance = device.create_buffer_with_data(InstanceVertex::identity().as_bytes(), wgpu::BufferUsage::VERTEX);
        let billboard_vertices = device.create_buffer_with_data(BILLBOARD_VERTICES.as_bytes(), wgpu::BufferUsage::VERTEX);

        let window_size = window.inner_size();

        let (swap_chain, depth_texture) = create_swap_chain_and_depth_texture(&device, &surface, window_size.width, window_size.height);

        queue.submit(Some(init_encoder.finish()));

        let lines = LineRenderer::new(&device, &sampler, &resources);

        let gui = Gui::new(window_size.width as f32, window_size.height as f32);

        let font: &[u8] = include_bytes!("pixel_operator/PixelOperator.ttf");

        let glyph_brush = wgpu_glyph::GlyphBrushBuilder::using_font_bytes(font)
            .unwrap()
            .texture_filter_method(wgpu::FilterMode::Nearest)
            .build(&device, wgpu::TextureFormat::Bgra8Unorm);


        (
            Self {
                swap_chain, triangle_pipeline, bind_group_layout, identity_instance, billboard_pipeline,
                queue, sampler, resources, device, window, surface, depth_texture, billboard_vertices, lines, glyph_brush, gui,
            },
            meshes
        )
    }

    pub fn copy_event(&mut self, event: &event::WindowEvent) {
        self.gui.update(event);
    }

    pub fn dpi(&self) -> f32 {
        self.window.scale_factor() as f32
    }

    pub fn screen_dimensions(&self) -> (f32, f32) {
        let dimensions = self.window.inner_size();
        (dimensions.width as f32, dimensions.height as f32)
    }

    fn aspect_ratio(&self) -> f32 {
        let (width, height) = self.screen_dimensions();
        height / width
    }

    fn perspective_matrix(&self) -> Matrix4<f32> {
        perspective_matrix(self.aspect_ratio())
    }

    fn uniforms(&self, view_matrix: Matrix4<f32>, system: &StarSystem, mode: Mode) -> wgpu::Buffer {
        let uniforms = Uniforms {
            view: view_matrix.into(),
            perspective: self.perspective_matrix().into(),
            light_direction: [system.light.x, system.light.y, system.light.z, 0.0],
            ambient_colour: [system.ambient_colour[0], system.ambient_colour[1], system.ambient_colour[2], 0.0],
            mode: mode as i32,
            dpi: self.dpi()
        };
        
        self.device.create_buffer_with_data(uniforms.as_bytes(), wgpu::BufferUsage::UNIFORM)
    }

    fn bind_group_from_uniforms(&self, view_matrix: Matrix4<f32>, system: &StarSystem, mode: Mode) -> wgpu::BindGroup {
        let uniforms = self.uniforms(view_matrix, system, mode);
        create_bind_group(&self.device, &self.bind_group_layout, &uniforms, &self.resources.image, &self.sampler)
    }

    pub fn render(
        &mut self,
        model_buffers: &mut ModelBuffers, lines: &mut LineBuffers, billboards: &mut BillboardBuffer, text: &mut TextBuffer,
        clear_colour: wgpu::Color, camera: &Camera, system: &mut StarSystem
    ) {        
        let normal_bind_group = self.bind_group_from_uniforms(camera.view_matrix(), system, Mode::Normal);
        let shadeless_bind_group = self.bind_group_from_uniforms(camera.view_matrix(), system, Mode::Shadeless);
        let nebula_bind_group = self.bind_group_from_uniforms(camera.view_matrix_only_direction(), system, Mode::VertexColoured);
        let star_bind_group = self.bind_group_from_uniforms(camera.view_matrix_only_direction(), system, Mode::White);

        let gpu_model_buffers = model_buffers.upload(&self.device);
        let nebula_buffer = self.device.create_buffer_with_data(&system.background.as_bytes(), wgpu::BufferUsage::VERTEX);
        let background_len = system.background.len() as u32;
        let stars_len = system.stars.len() as u32;
        let star_buffer = system.star_buffer(&self.device);

        let gpu_lines = lines.upload(&self.device);
        let gpu_billboards = billboards.upload(&self.device);

        let output = self.swap_chain.get_next_texture().unwrap();
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None});
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &output.view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: clear_colour,
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture,
                    depth_load_op: wgpu::LoadOp::Clear,
                    depth_store_op: wgpu::StoreOp::Store,
                    stencil_load_op: wgpu::LoadOp::Clear,
                    stencil_store_op: wgpu::StoreOp::Store,
                    clear_depth: 1.0,
                    clear_stencil: 0,
                }),
            });

            pass.set_pipeline(&self.triangle_pipeline);
            pass.set_bind_group(0, &normal_bind_group, &[]);

            for i in 0 .. 6 {
                if let Some(instances) = &gpu_model_buffers[i] {
                    let model = &self.resources.models[i];

                    let vertices_slice = model.vertices.slice(0 .. 0);
                    let instances_slice = instances.slice(0 .. 0);
                    pass.set_vertex_buffer(0, vertices_slice);
                    pass.set_vertex_buffer(1, instances_slice);
                    pass.draw(0 .. model.vertices_len as u32, 0 .. model_buffers.inner[i].len() as u32);
                }
            }

            // Background
            pass.set_bind_group(0, &nebula_bind_group, &[]);
            pass.set_vertex_buffer(0, nebula_buffer.slice(0 .. 0));
            pass.set_vertex_buffer(1, self.identity_instance.slice(0 .. 0));
            pass.draw(0 .. background_len, 0 .. 1); 

            // Stars
            pass.set_bind_group(0, &star_bind_group, &[]);
            pass.set_vertex_buffer(0, self.billboard_vertices.slice(0 .. 0));
            pass.set_vertex_buffer(1, star_buffer.slice(0 .. 0));
            pass.draw(0 .. BILLBOARD_VERTICES.len() as u32, 0 .. stars_len);

            if let Some((vertices, indices)) = &gpu_lines {
                pass.set_pipeline(&self.lines.pipeline);
                pass.set_bind_group(0, &self.lines.bind_group, &[]);
                pass.set_index_buffer(indices.slice(0 .. 0));
                pass.set_vertex_buffer(0, vertices.slice(0 .. 0));
                pass.draw_indexed(0 .. lines.num_indices(), 0, 0 .. 1);
            }

            if let Some(instances) = &gpu_billboards {
                pass.set_pipeline(&self.billboard_pipeline);
                pass.set_bind_group(0, &shadeless_bind_group, &[]);
                pass.set_vertex_buffer(0, self.billboard_vertices.slice(0 .. 0));
                pass.set_vertex_buffer(1, instances.slice(0 .. 0));
                pass.draw(0 .. BILLBOARD_VERTICES.len() as u32, 0 .. billboards.inner.len() as u32);
            }
        }

        for section in text.inner.drain(..) {
            let layout = wgpu_glyph::PixelPositioner(section.layout);
            self.glyph_brush.queue_custom_layout(&section, &layout);
        }

        let dimensions = self.window.inner_size();

        self.glyph_brush.draw_queued(
            &self.device,
            &mut encoder,
            &output.view,
            dimensions.width,
            dimensions.height,
        ).unwrap();

        self.queue.submit(Some(encoder.finish()));
        model_buffers.clear();
        lines.clear();
        billboards.clear();
        text.clear();
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let (swap_chain, depth_texture) = create_swap_chain_and_depth_texture(&self.device, &self.surface, width, height);
        self.swap_chain = swap_chain;
        self.depth_texture = depth_texture;
    }
}

fn create_swap_chain_and_depth_texture(device: &wgpu::Device, surface: &wgpu::Surface, width: u32, height: u32) -> (wgpu::SwapChain, wgpu::TextureView) {
    let swap_chain_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width,
        height,
        present_mode: wgpu::PresentMode::Fifo,
    };

    let swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);

    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width,
            height,
            depth: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        label: None,
    });

    let depth_texture_view = depth_texture.create_default_view();

    (swap_chain, depth_texture_view)
}

fn create_bind_group(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, uniforms_buffer: &wgpu::Buffer, texture: &wgpu::TextureView, sampler: &wgpu::Sampler) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout,
        bindings: &[
            wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(uniforms_buffer.slice(0 .. Uniforms::default().as_bytes().len() as u64)),
            },
            wgpu::Binding {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(texture)
            },
            wgpu::Binding {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler)
            }
        ],
        label: None
    })
}

fn create_pipeline(
    device: &wgpu::Device, layout: &wgpu::PipelineLayout, vs_module: &wgpu::ShaderModule, fs_module: &wgpu::ShaderModule,
    primitive_topology: wgpu::PrimitiveTopology, depth_write: bool,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout,
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Cw,
            cull_mode: wgpu::CullMode::Back,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        }),
        primitive_topology,
        color_states: &[wgpu::ColorStateDescriptor {
            format: wgpu::TextureFormat::Bgra8Unorm,
            color_blend: wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha_blend: wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: depth_write,
            depth_compare: wgpu::CompareFunction::Less,
            stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
            stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
            stencil_read_mask: 0,
            stencil_write_mask: 0,
        }),
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[
                wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &vertex_attr_array![0 => Float3, 1 => Float3, 2 => Float2, 3 => Float2],
                },
                wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<InstanceVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Instance,
                    attributes: &vertex_attr_array![4 => Float2, 5 => Float2, 6 => Float4, 7 => Float4, 8 => Float4, 9 => Float4],
                }
            ],
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    })
}

#[repr(C)]
#[derive(zerocopy::AsBytes, Default, Copy, Clone, Debug)]
pub struct Uniforms {
    perspective: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    light_direction: [f32; 4],
    ambient_colour: [f32; 4],
    mode: i32,
    dpi: f32,
}

#[derive(Default)]
pub struct TextBuffer {
    inner: Vec<wgpu_glyph::OwnedVariedSection<wgpu_glyph::DrawMode>>
}

impl TextBuffer {
    fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn push_text(&mut self, text: &str, x: f32, y: f32, dpi: f32) {
        let scale = 16.0 * dpi.round();
        
        let section = wgpu_glyph::OwnedVariedSection {
            screen_position: (x, y),
            text: vec![
                wgpu_glyph::OwnedSectionText {
                    text: text.to_string(),
                    scale: wgpu_glyph::Scale::uniform(scale),
                    color: [1.0; 4],
                    font_id: wgpu_glyph::FontId(0),
                    custom: wgpu_glyph::DrawMode::pixelated(1.0),
                }
            ],
            ..wgpu_glyph::OwnedVariedSection::default()
        };

        self.inner.push(section);
    }
}

#[derive(Default)]
pub struct BillboardBuffer {
    inner: Vec<InstanceVertex>
}

impl BillboardBuffer {
    fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn push_billboard(&mut self, matrix: Matrix4<f32>, image: Image) {
        let vertex = InstanceVertex {
            instance_pos: matrix.into(),
            uv_dimensions: image.dimensions(),
            uv_offset: image.offset(),
        };
        self.inner.push(vertex);
    }

    fn upload(&self, device: &wgpu::Device) -> Option<wgpu::Buffer> {
        if self.inner.is_empty() {
            None
        } else {
            Some(device.create_buffer_with_data(self.inner.as_bytes(), wgpu::BufferUsage::VERTEX))
        }
    }
}

#[derive(Default)]
pub struct ModelBuffers {
    inner: [Vec<InstanceVertex>; 6]
}

impl ModelBuffers {
    fn clear(&mut self) {
        for buffer in &mut self.inner {
            buffer.clear();
        }
    }

    pub fn push_model(&mut self, model: Model, instance: InstanceVertex) {
        self.inner[model as usize].push(instance);
    }

    fn upload(&self, device: &wgpu::Device) -> [Option<wgpu::Buffer>; 6] {
        [
            self.buffer(0, device),
            self.buffer(1, device),
            self.buffer(2, device),
            self.buffer(3, device),
            self.buffer(4, device),
            self.buffer(5, device),
        ]
    }

    fn buffer(&self, index: usize, device: &wgpu::Device) -> Option<wgpu::Buffer> {
        let bytes = self.inner[index].as_bytes();

        if bytes.is_empty() {
            None
        } else {
            let buffer = device.create_buffer_with_data(bytes, wgpu::BufferUsage::VERTEX);
            Some(buffer)
        }        
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, zerocopy::AsBytes)]
pub struct InstanceVertex {
    pub uv_offset: [f32; 2],
    pub uv_dimensions: [f32; 2],
    pub instance_pos: [[f32; 4]; 4],
}

impl InstanceVertex {
    pub fn new(matrix: Matrix4<f32>) -> Self {
        Self {
            instance_pos: matrix.into(),
            uv_offset: [0.0; 2],
            uv_dimensions: [1.0; 2]
        }
    }

    pub fn identity() -> Self {
        Self::new(Matrix4::identity())
    }
}
