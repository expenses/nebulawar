use lyon::tessellation::geometry_builder::*;
use lyon::math::*;
use lyon::lyon_tessellation::*;
use lyon::lyon_tessellation::basic_shapes::*;
use crate::util::screen_pos_to_opengl_pos;

use super::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, zerocopy::AsBytes, Default)]
struct Vertex2d {
    // Position in screen space (x, y, depth)
    position: [f32; 3],
    colour: [f32; 4],
    uv: [f32; 2]
}

impl Vertex2d {
    fn new(pos: Vector2<f32>, window_size: Vector2<f32>, uv: [f32; 2], colour: [f32; 4], depth: f32) -> Self {
        let (x, y) = screen_pos_to_opengl_pos(pos.x, pos.y, window_size.x, window_size.y);
        Self {
            position: [x, y, depth],
            uv, colour
        }
    }
}

struct Constructor {
    colour: [f32; 4],
    window_size: Vector2<f32>,
    line_depths: Option<(f32, f32)>
}

impl Constructor {
    fn new(colour: [f32; 4], window_size: Vector2<f32>, line_depths: Option<(f32, f32)>) -> Self {
        Self {
            colour, window_size, line_depths
        }
    }
}

impl FillVertexConstructor<Vertex2d> for Constructor {
    fn new_vertex(&mut self, point: Point, _: FillAttributes) -> Vertex2d {
        Vertex2d::new(point.to_array().into(), self.window_size, [0.0; 2], self.colour, 0.0)
    }
}
impl StrokeVertexConstructor<Vertex2d> for Constructor {
    fn new_vertex(&mut self, point: Point, attr: StrokeAttributes) -> Vertex2d {
        let depth = if let Some((start_depth, end_depth)) = self.line_depths {
            if attr.advancement() == 0.0 {
                start_depth
            } else {
                end_depth
            }
        } else {
            0.0
        };
        Vertex2d::new(point.to_array().into(), self.window_size, [0.0; 2], self.colour, depth)
    }
}

pub struct LineRenderer {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
}

impl LineRenderer {
    pub fn new(device: &wgpu::Device, sampler: &wgpu::Sampler, resources: &Resources) -> Self {
        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Float,
                        },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                    },
                ],
                label: Some("Hectic 2d"),
            });
    
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

        let vs = include_bytes!("shaders/lines.vert.spv");
        let vs_module =
            device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&vs[..])).unwrap());
    
        let fs = include_bytes!("shaders/lines.frag.spv");
        let fs_module =
            device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&fs[..])).unwrap());
        

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: None,
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8Unorm,
                color_blend: wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha_blend: wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::DstAlpha,
                    operation: wgpu::BlendOperation::Max,
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
                        stride: std::mem::size_of::<Vertex2d>() as wgpu::BufferAddress,
                        step_mode: wgpu::InputStepMode::Vertex,
                        attributes: &vertex_attr_array![0 => Float3, 1 => Float4, 2 => Float2],
                    },
                ],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&resources.image)
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler)
                }
            ],
            label: None
        });

        Self {
            pipeline, bind_group,
        }
    }
}

#[derive(Default)]
pub struct LineBuffers {
    vertex_buffers: VertexBuffers<Vertex2d, u16>
}

impl LineBuffers {
    pub fn push_3d_lines<I: Iterator<Item=Vector3<f32>>>(&mut self, iterator: I, colour: [f32; 3], window_size: Vector2<f32>, camera: &Camera) {
        let mut last = None;

        for point in iterator {
            if let Some(last) = last {
                self.push_3d_line(last, point, colour, window_size, camera);
            }

            last = Some(point);
        }
    }

    pub fn push_circle(&mut self, position: Vector3<f32>, size: f32, colour: [f32; 3], window_size: Vector2<f32>, camera: &Camera) {
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

        self.push_3d_lines(iterator, colour, window_size, camera);
    }


    pub fn push_3d_line(&mut self, start: Vector3<f32>, end: Vector3<f32>, colour: [f32; 3], window_size: Vector2<f32>, camera: &Camera) {
        let mut start = camera.screen_position(start, window_size.into(), true).unwrap();
        let mut end = camera.screen_position(end, window_size.into(), true).unwrap();

        if end.z >= 1.0 {
            // todo: set to screen edge
            end.x = start.x - (end.x - start.x) * 10.0;
            end.y = start.y - (end.y - start.y) * 10.0;
            end.z = 1.0;
        }

        if start.z >= 1.0 {
            // todo: set to screen edge
            start.x = end.x - (start.x - end.x) * 10.0;
            start.y = end.y - (start.y - end.y) * 10.0;
            start.z = 1.0;
        }

        stroke_polyline(
            iter_owned([point(start.x, start.y), point(end.x, end.y)]),
            false,
            // todo: maybe replace this thickness with dpi
            &StrokeOptions::tolerance(0.5).with_line_width(2.0),
            &mut BuffersBuilder::new(&mut self.vertex_buffers, Constructor::new([colour[0], colour[1], colour[2], 1.0], window_size, Some((start.z, end.z))))
        ).unwrap();
    }

    pub fn push_rect(&mut self, (left, top): (f32, f32), (right, bottom): (f32, f32), window_size: Vector2<f32>) {
        let origin_x = left.min(right);
        let origin_y = top.min(bottom);
        let width = (right - left).abs();
        let height = (bottom - top).abs();

        stroke_rectangle(
            &rect(origin_x, origin_y, width, height),
            &StrokeOptions::tolerance(0.5).with_line_width(1.0),
            &mut BuffersBuilder::new(&mut self.vertex_buffers, Constructor::new([1.0; 4], window_size, None))
        ).unwrap();
    }
    
    pub fn push_image(&mut self, image: Image, x: f32, y: f32, width: f32, height: f32, overlay: [f32; 4], window_size: Vector2<f32>) {
        let len = self.vertex_buffers.vertices.len() as u16;

        self.vertex_buffers.vertices.extend_from_slice(&[
            Vertex2d::new(Vector2::new(x - width / 2.0, y - height / 2.0), window_size, image.translate([0.0, 1.0]), overlay, 0.0),
            Vertex2d::new(Vector2::new(x + width / 2.0, y - height / 2.0), window_size, image.translate([1.0, 1.0]), overlay, 0.0),
            Vertex2d::new(Vector2::new(x - width / 2.0, y + height / 2.0), window_size, image.translate([0.0, 0.0]), overlay, 0.0),
            Vertex2d::new(Vector2::new(x + width / 2.0, y + height / 2.0), window_size, image.translate([1.0, 0.0]), overlay, 0.0)
        ]);

        self.vertex_buffers.indices.extend_from_slice(&[
            len + 0, len + 1, len + 2,
            len + 1, len + 2, len + 3
        ]);
    }

    pub fn upload(&mut self, device: &wgpu::Device) -> Option<(wgpu::Buffer, wgpu::Buffer)> {
        if self.vertex_buffers.vertices.is_empty() {
            return None;
        }

        let vertices = device.create_buffer_with_data(&self.vertex_buffers.vertices.as_bytes(), wgpu::BufferUsage::VERTEX);
        let indices = device.create_buffer_with_data(&self.vertex_buffers.indices.as_bytes(), wgpu::BufferUsage::INDEX);

        Some((vertices, indices))
    }

    pub fn clear(&mut self) {
        self.vertex_buffers.vertices.clear();
        self.vertex_buffers.indices.clear();
    }

    pub fn num_indices(&self) -> u32 {
        self.vertex_buffers.indices.len() as u32
    }
}
