//mod lines;
mod resources;

//use self::lines::*;

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
    wgpu::vertex_attr_array
};

// ** Line Rendering Woes **
// rendering in 2d: doesnt work with rest of scene, rendering lines that go behind the camera is hard
// gl_lines: has a max width of 1 on my laptop
// 2d lines in 3d: getting lines to join nicely is hard, too flat
// geometry shader: complicated
// assembling triangle/square line meshes by hand: complicated, but might be best shot

const VERT: &str = include_str!("shaders/shader.vert");
const FRAG: &str = include_str!("shaders/shader.frag");

const DEFAULT_WIDTH: f32 = 1280.0;
const DEFAULT_HEIGHT: f32 = 800.0;

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
    pub texture: [f32; 2],
}

impl Vertex {
    pub fn with_brightness(position: Vector3<f32>, brightness: f32) -> Self {
        Self {
            position: position.into(),
            normal: [0.0; 3],
            texture: [brightness; 2]
        }
    }

    pub fn with_colour(position: Vector3<f32>, colour: [f32; 3]) -> Self {
        Self {
            position: position.into(),
            normal: colour,
            texture: [0.0; 2]
        }
    }
}

pub struct Context {
    window: winit::window::Window,
    pub device: wgpu::Device,
    pub swap_chain: wgpu::SwapChain,
    pub queue: wgpu::Queue,
    surface: wgpu::Surface,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub pipeline: wgpu::RenderPipeline,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub sampler: wgpu::Sampler,

    depth_texture: wgpu::TextureView,

    pub resources: Resources,
    /*
    display: Display,
    program: Program,
    target: Frame,
    resources: Resources,
    //lines: LineRenderer,
    text_program: Program,
    
    text_buffer: Vec<runic::Vertex>,
    lines_3d_buffer: Vec<Vertex>,
    
    smoke_buffer: Vec<InstanceVertex>,

    pub gui: Gui*/
}

impl Context {
    pub async fn new(event_loop: &event_loop::EventLoop<()>) -> (Self, MeshArray) {
        let window = winit::window::Window::new(event_loop).unwrap();

        #[cfg(feature = "wasm")]
        {
            use winit::platform::web::WindowExtWebSys;
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| doc.body())
                .and_then(|body| {
                    body.append_child(&web_sys::Element::from(window.canvas()))
                        .ok()
                })
                .expect("couldn't append canvas to document body");
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
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
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
                        visibility: wgpu::ShaderStage::all(),
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
                label: Some("Hectic BindGroupLayout"),
            });
    
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

        let pipeline = create_pipeline(&device, &pipeline_layout, &vs_module, &fs_module, wgpu::PrimitiveTopology::TriangleList);

        let window_size = window.inner_size();

        let (swap_chain, depth_texture) = create_swap_chain_and_depth_texture(&device, &surface, window_size.width, window_size.height);

        queue.submit(Some(init_encoder.finish()));

        (
            Self {
                swap_chain, pipeline, bind_group_layout, pipeline_layout, 
                queue, sampler, resources, device, window, surface, depth_texture,
            },
            meshes
        )
    }

    pub fn copy_event(&mut self, event: &event::WindowEvent) {
        //self.gui.update(event);
    }

    /*pub fn clear(&mut self) {
        self.target.clear_color_srgb_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);
    }

    fn flush_3d_lines(&mut self, camera: &Camera, system: &StarSystem) {
        let uniforms = uniform!{
            view: matrix_to_array(camera.view_matrix()),
            perspective: matrix_to_array(self.perspective_matrix()),
            light_direction: vector_to_array(system.light),
            mode: Mode::VertexColoured as i32
        };

        let vertices = VertexBuffer::new(&self.display, &self.lines_3d_buffer).unwrap();
        let indices = NoIndices(PrimitiveType::LinesList);

        let params = Self::draw_params();
        let instances = [InstanceVertex::new(Matrix4::identity())];
        let instance_buffer = VertexBuffer::new(&self.display, &instances).unwrap();

        self.target.draw((&vertices, instance_buffer.per_instance().unwrap()), &indices, &self.program, &uniforms, &params).unwrap();

        self.lines_3d_buffer.clear();
    }

    fn flush_text(&mut self) {
        //self.resources.font.render_vertices(&self.text_buffer, [1.0; 4], &mut self.target, &self.display, &self.text_program, true).unwrap();
        self.text_buffer.clear();
    }

    pub fn flush_ui(&mut self, camera: &Camera, system: &StarSystem) {
        self.lines.flush(&mut self.target, &self.display);

        self.flush_3d_lines(camera, system);
        self.flush_text();

        self.gui.clear();
    }

    pub fn finish(&mut self) {
        self.target.set_finish().unwrap();
        self.target = self.display.draw();
    }

    pub fn render_stars(&mut self, system: &StarSystem, camera: &Camera) {
        let uniforms = self.background_uniforms(camera, system, Mode::White);

        let vertices = VertexBuffer::new(&self.display, &system.stars).unwrap();
        let indices = NoIndices(PrimitiveType::Points);

        let params = DrawParameters {
            polygon_mode: PolygonMode::Point,
            point_size: Some(2.0 * self.dpi()),
            .. Self::draw_params()
        };

        let instances = [InstanceVertex::new(Matrix4::identity())];
        let instance_buffer = VertexBuffer::new(&self.display, &instances).unwrap();

        self.target.draw((&vertices, instance_buffer.per_instance().unwrap()), &indices, &self.program, &uniforms, &params).unwrap();
    }

    pub fn render_skybox(&mut self, system: &StarSystem, camera: &Camera, debug: bool) {
        let uniforms = self.background_uniforms(camera, system, Mode::VertexColoured);

        let vertices = VertexBuffer::new(&self.display, &system.background).unwrap();
        let indices = NoIndices(PrimitiveType::TrianglesList);

        let mut params = Self::draw_params();

        if debug {
            params.polygon_mode = PolygonMode::Line;
        }

        let instances = [InstanceVertex::new(Matrix4::identity())];
        let instance_buffer = VertexBuffer::new(&self.display, &instances).unwrap();

        self.target.draw((&vertices, instance_buffer.per_instance().unwrap()), &indices, &self.program, &uniforms, &params).unwrap();
    }

    pub fn flush_billboards(&mut self, system: &StarSystem, camera: &Camera) {
        let uniforms = self.uniforms(camera, system, &self.resources.image, Mode::Shadeless);
        let mut params = Self::draw_params();
        params.depth = Depth::default();

        let buffer = VertexBuffer::new(&self.display, &self.smoke_buffer).unwrap();
        
        self.target.draw((&VertexBuffer::new(&self.display, &BILLBOARD_VERTICES).unwrap(), buffer.per_instance().unwrap()), &NoIndices(PrimitiveType::TrianglesList), &self.program, &uniforms, &params).unwrap();

        self.smoke_buffer.clear();
    }

    pub fn background_uniforms<'a>(&self, camera: &Camera, system: &StarSystem, mode: Mode) -> impl Uniforms + 'a {
        uniform! {
            view: matrix_to_array(camera.view_matrix_only_direction()),
            perspective: matrix_to_array(self.perspective_matrix()),
            light_direction: vector_to_array(system.light),
            mode: mode as i32
        }
    }

    pub fn uniforms<'a>(&self, camera: &Camera, system: &StarSystem, texture: &'a SrgbTexture2d, mode: Mode) -> impl Uniforms + 'a {
        uniform!{
            view: matrix_to_array(camera.view_matrix()),
            perspective: matrix_to_array(self.perspective_matrix()),
            light_direction: vector_to_array(system.light),
            tex: Sampler::new(texture).minify_filter(MinifySamplerFilter::Nearest).magnify_filter(MagnifySamplerFilter::Nearest),
            ambient_colour: system.ambient_colour,
            mode: mode as i32
        }
    }*/

    /*pub fn render_model(&mut self, model: Model, info: &[InstanceVertex], camera: &Camera, system: &StarSystem) {
        let model = &self.resources.models[model as usize];

        let uniforms = self.uniforms(camera, system, &model.texture, Mode::Normal);
        let params = Self::draw_params();

        let buffer = VertexBuffer::new(&self.display, &info).unwrap();
        
        self.target.draw((&model.vertices, buffer.per_instance().unwrap()), &NoIndices(PrimitiveType::TrianglesList), &self.program, &uniforms, &params).unwrap();
    }*/

    /*pub fn render_text(&mut self, text: &str, x: f32, y: f32) {
        //let iterator = self.resources.font.get_pixelated_vertices(text, [x, y], 16.0, 1.0, &self.display).unwrap();
        //self.text_buffer.extend(iterator);
    }

    pub fn render_rect(&mut self, top_left: (f32, f32), bottom_right: (f32, f32)) {
        self.lines.render_rect(top_left, bottom_right);
    }*/

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

    /*fn dpi(&self) -> f32 {
        (**self.display.gl_window()).window().scale_factor() as f32
    }

    fn draw_params() -> DrawParameters<'static> {
        DrawParameters {
            depth: Depth {
                test: DepthTest::IfLess,
                write: true,
                .. Default::default()
            },
            backface_culling: BackfaceCullingMode::CullCounterClockwise,
            blend: Blend::alpha_blending(),
            .. Default::default()
        }
    }

    pub fn render_image(&mut self, image: Image, x: f32, y: f32, width: f32, height: f32, overlay: [f32; 4]) {
        self.lines.render_image(image, x, y, width, height, overlay, &mut self.target, &self.display, &self.resources)
    }*/

    fn uniforms(&self, camera: &Camera, system: &StarSystem, mode: Mode) -> wgpu::Buffer {
        let uniforms = Uniforms {
            view: camera.view_matrix().into(),
            perspective: self.perspective_matrix().into(),
            light_direction: system.light.into(),
            ambient_colour: system.ambient_colour,
            mode: mode as i32
        };
        
        //println!("{:?}", uniforms);

        self.device.create_buffer_with_data(uniforms.as_bytes(), wgpu::BufferUsage::UNIFORM)
    }

    pub fn render(&mut self, buffers: &mut Buffers, clear_colour: wgpu::Color, camera: &Camera, system: &StarSystem) {
        let model_uniforms = self.uniforms(camera, system, Mode::Normal);
        let nebula_uniforms = self.uniforms(camera, system, Mode::VertexColoured);

        let identity_instance = self.device.create_buffer_with_data(InstanceVertex::identity().as_bytes(), wgpu::BufferUsage::VERTEX);

        let mut gpu_buffers = buffers.to_gpu(&self.device, &self.bind_group_layout, &model_uniforms, &self.resources, &self.sampler);

        let nebula_buffer = self.device.create_buffer_with_data(&system.background.as_bytes(), wgpu::BufferUsage::VERTEX);
        let nebula_bind_group = create_bind_group(&self.device, &self.bind_group_layout, &nebula_uniforms, &self.resources.models[0].texture, &self.sampler);

        let output = self.swap_chain.get_next_texture().unwrap();
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Hectic CommandEncoder") });
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

            for i in 0 .. 6 {
                if let Some((instances, bind_group)) = &gpu_buffers.models[i] {
                    let model = &self.resources.models[i];

                    pass.set_pipeline(&self.pipeline);
                    pass.set_bind_group(0, &bind_group, &[]);
                    
                    let vertices_slice = model.vertices.slice(0 .. 0);
                    let instances_slice = instances.slice(0 .. 0);
                    pass.set_vertex_buffer(0, vertices_slice);
                    pass.set_vertex_buffer(1, instances_slice);
                    pass.draw(0 .. model.vertices_len as u32, 0 .. buffers.model_buffers[i].len() as u32);
                }
            }

            /*pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &nebula_bind_group, &[]);
            pass.set_vertex_buffer(0, nebula_buffer.slice(0 .. 0));
            pass.set_vertex_buffer(1, identity_instance.slice(0 .. 0));
            pass.draw(0 .. system.background.len() as u32, 0 .. 1);*/
            
        }

        self.queue.submit(Some(encoder.finish()));
        buffers.clear();
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
                resource: wgpu::BindingResource::Buffer(uniforms_buffer.slice(0 .. 0)),
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
    primitive_topology: wgpu::PrimitiveTopology,
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
            depth_write_enabled: true,
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
                    attributes: &[
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float3,
                            offset: 12,
                            shader_location: 1,
                        },
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float2,
                            offset: 24,
                            shader_location: 2,
                        }
                    ],
                },
                wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<InstanceVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Instance,
                    attributes: &vertex_attr_array![3 => Float2, 4 => Float2, 5 => Float4, 6 => Float4, 7 => Float4, 8 => Float4],
                    /*attributes: &[
                        wgpu::VertexAttributeDescriptor {
                            shader_location: 3,
                            offset: (3 + 3 + 2) * 4,
                            format: wgpu::VertexFormat::Float2
                        },
                        wgpu::VertexAttributeDescriptor {
                            shader_location: 4,
                            offset: (3 + 3 + 2 + 2) * 4,
                            format: wgpu::VertexFormat::Float2
                        },
                        
                        wgpu::VertexAttributeDescriptor {
                            shader_location: 5,
                            offset: (3 + 3 + 2 + 2 + 2) * 4,
                            format: wgpu::VertexFormat::Float4
                        },
                        wgpu::VertexAttributeDescriptor {
                            shader_location: 6,
                            offset: (3 + 3 + 2 + 2 + 2 + 4) * 4,
                            format: wgpu::VertexFormat::Float4
                        },
                        wgpu::VertexAttributeDescriptor {
                            shader_location: 7,
                            offset: (3 + 3 + 2 + 2 + 2 + 4 + 4) * 4,
                            format: wgpu::VertexFormat::Float4
                        },
                        wgpu::VertexAttributeDescriptor {
                            shader_location: 8,
                            offset: (3 + 3 + 2 + 2 + 2 + 4 + 4 + 4) * 4,
                            format: wgpu::VertexFormat::Float4
                        },
                        
                    ],*/
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
    light_direction: [f32; 3],
    ambient_colour: [f32; 3],
    mode: i32
}

#[derive(Default)]
pub struct Buffers {
    //text_buffer: Vec<runic::Vertex>,
    lines_3d_buffer: Vec<Vertex>,
    billboard_buffer: Vec<InstanceVertex>,
    pub model_buffers: [Vec<InstanceVertex>; 6]
}

pub struct GpuBuffers {
    models: [Option<(wgpu::Buffer, wgpu::BindGroup)>; 6]
}

impl Buffers {
    fn clear(&mut self) {
        //self.text_buffer.clear();
        self.lines_3d_buffer.clear();
        self.billboard_buffer.clear();
        for buffer in &mut self.model_buffers {
            buffer.clear();
        }
    }

    pub fn push_model(&mut self, model: Model, instance: InstanceVertex) {
        self.model_buffers[model as usize].push(instance);
    }

    pub fn push_3d_lines<I: Iterator<Item=Vector3<f32>>>(&mut self, iterator: I, colour: [f32; 3]) {
        let mut last = None;

        for vector in iterator {
            let vertex = Vertex::with_colour(vector, colour);

            if let Some(last) = last {
                self.lines_3d_buffer.push(last);
                self.lines_3d_buffer.push(vertex);
            }

            last = Some(vertex);
        }
    }

    pub fn push_3d_line(&mut self, start: Vector3<f32>, end: Vector3<f32>, colour: [f32; 3]) {
        self.push_3d_lines(iter_owned([start, end]), colour);
    }

    pub fn push_circle(&mut self, position: Vector3<f32>, size: f32, colour: [f32; 3], camera: &Camera) {
        let points = 20;

        let rotation = look_at(-camera.direction());

        let iterator = (0 .. points)
            .chain(iter_owned([0]))
            .map(|point| {
                let percentage = point as f32 / PI;

                position + rotation * Vector3::new(
                    percentage.sin() * size,
                    percentage.cos() * size,
                    0.0
                )
            });

        self.push_3d_lines(iterator, colour);
    }

    pub fn push_billboard(&mut self, matrix: Matrix4<f32>, image: Image, camera: &Camera, system: &StarSystem) {
        let vertex = InstanceVertex {
            instance_pos: matrix.into(),
            uv_dimensions: image.dimensions().into(),
            uv_offset: image.offset().into()
        };
        self.billboard_buffer.push(vertex);
    }

    fn to_gpu(&self, device: &wgpu::Device, bind_group_layout: &wgpu::BindGroupLayout, uniforms_buffer: &wgpu::Buffer, res: &Resources, sampler: &wgpu::Sampler) -> GpuBuffers {
        GpuBuffers {
            models: [
                self.buffer_and_bind_group(0, device, bind_group_layout, uniforms_buffer, res, sampler),
                self.buffer_and_bind_group(1, device, bind_group_layout, uniforms_buffer, res, sampler),
                self.buffer_and_bind_group(2, device, bind_group_layout, uniforms_buffer, res, sampler),
                self.buffer_and_bind_group(3, device, bind_group_layout, uniforms_buffer, res, sampler),
                self.buffer_and_bind_group(4, device, bind_group_layout, uniforms_buffer, res, sampler),
                self.buffer_and_bind_group(5, device, bind_group_layout, uniforms_buffer, res, sampler),
            ]
        }
    }

    fn buffer_and_bind_group(&self, index: usize, device: &wgpu::Device, bind_group_layout: &wgpu::BindGroupLayout, uniforms_buffer: &wgpu::Buffer, res: &Resources, sampler: &wgpu::Sampler) -> Option<(wgpu::Buffer, wgpu::BindGroup)> {
        if self.model_buffers[index].is_empty() {
            return None;
        }

        let bind_group = create_bind_group(device, bind_group_layout, uniforms_buffer, &res.models[index].texture, sampler);
        let buffer = device.create_buffer_with_data(self.model_buffers[index].as_bytes(), wgpu::BufferUsage::VERTEX);
        Some((buffer, bind_group))
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
