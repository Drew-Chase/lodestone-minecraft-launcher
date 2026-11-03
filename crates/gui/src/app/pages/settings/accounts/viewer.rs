//! The 3D skin viewer: a player model built from the six boxes of a skin, drawn with wgpu through iced's shader widget.
//!
//! The CPU lays out the model's vertices for the pose and the layers chosen, and the GPU samples the skin texture with
//! nearest filtering, so pixel art stays sharp. Dragging the model turns it. The overlay layers are the second skin
//! layer, drawn a little outside each limb.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use iced::widget::Shader;
use iced::widget::shader::{self, Program};
use iced::{Element, Event, Fill, Length, Rectangle, mouse};

/// The height of the viewer, in logical pixels.
pub const HEIGHT: f32 = 340.0;

/// The side of a skin texture, in texels.
const SKIN_SIZE: u32 = 64;

/// The depth format the viewer draws its model with.
const DEPTH_FORMAT: iced::wgpu::TextureFormat = iced::wgpu::TextureFormat::Depth32Float;

/// The floats in one vertex: position, texture coordinates and shade.
const FLOATS_PER_VERTEX: usize = 6;

/// The bytes in one vertex.
const VERTEX_BYTES: u64 = (FLOATS_PER_VERTEX * 4) as u64;

/// The poses the viewer offers, in the order the rail shows them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Pose {
    #[default]
    Stand,
    Arms,
    Walk,
    Run,
    Crouch,
}

impl Pose {
    /// Every pose, in rail order.
    pub const ALL: [Pose; 5] = [Pose::Stand, Pose::Arms, Pose::Walk, Pose::Run, Pose::Crouch];
}

/// The overlay layers the viewer draws over the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layers {
    pub hat: bool,
    pub jacket: bool,
    pub sleeves: bool,
    pub pants: bool,
}

impl Default for Layers {
    fn default() -> Self {
        Self {
            hat: true,
            jacket: true,
            sleeves: true,
            pants: true,
        }
    }
}

/// A decoded skin: its texels as RGBA, and an id that changes with the picture, so the GPU texture is uploaded once.
#[derive(Debug)]
pub struct Skin {
    pub id: u64,
    pub texels: Vec<u8>,
}

impl Skin {
    /// Decodes a PNG skin. A 64×32 skin fills the top half of the texture, and the bottom half stays clear.
    pub fn decode(png: &[u8]) -> Option<Skin> {
        let image = image::load_from_memory(png).ok()?.to_rgba8();
        let (width, height) = image.dimensions();
        if width != SKIN_SIZE || !(height == SKIN_SIZE || height == SKIN_SIZE / 2) {
            return None;
        }
        let mut texels = vec![0u8; (SKIN_SIZE * SKIN_SIZE * 4) as usize];
        texels[..image.as_raw().len()].copy_from_slice(image.as_raw());
        let mut hasher = DefaultHasher::new();
        texels.hash(&mut hasher);
        Some(Skin { id: hasher.finish(), texels })
    }
}

/// The model the viewer shows: the skin, the pose, the layers, and whether the arms are the slim model.
#[derive(Debug, Clone)]
pub struct Model {
    pub skin: Option<Arc<Skin>>,
    pub pose: Pose,
    pub layers: Layers,
    pub slim: bool,
}

/// The viewer's state between frames: the turn of the model, whether a drag is under way, and where the pointer last was.
#[derive(Debug, Clone, Copy)]
pub struct Turn {
    yaw: f32,
    pitch: f32,
    /// Whether the left button went down over the viewer, so pointer moves turn the model until it comes up.
    dragging: bool,
    /// The pointer's place in the window at its last move. A drag turns the model by how far the pointer moved since then.
    /// Both ends are window coordinates, as `CursorMoved` reports them.
    pointer: Option<(f32, f32)>,
}

impl Default for Turn {
    fn default() -> Self {
        Self {
            yaw: 0.6,
            pitch: -0.1,
            dragging: false,
            pointer: None,
        }
    }
}

/// The shader program: it turns the model with the pointer and draws it.
pub struct ModelProgram {
    model: Model,
}

/// The viewer, as an element of the page. Dragging with the left button turns the model.
pub fn viewer<'a, M: 'a>(model: Model) -> Element<'a, M> {
    Shader::new(ModelProgram { model }).width(Fill).height(Length::Fixed(HEIGHT)).into()
}

impl<M> Program<M> for ModelProgram {
    type State = Turn;
    type Primitive = Draw;

    fn update(&self, turn: &mut Turn, event: &Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<shader::Action<M>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if cursor.is_over(bounds) => {
                // The press keeps the current turn. Only the moves after it turn the model.
                turn.dragging = true;
                Some(shader::Action::capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let previous = turn.pointer.replace((position.x, position.y));
                if !turn.dragging {
                    return None;
                }
                let (last_x, last_y) = previous?;
                turn.yaw += (position.x - last_x) * 0.01;
                turn.pitch = (turn.pitch + (position.y - last_y) * 0.01).clamp(-1.2, 1.2);
                Some(shader::Action::request_redraw())
            }
            Event::Mouse(mouse::Event::CursorLeft) => {
                // The next move starts from where the pointer re-enters, not from where it left.
                turn.pointer = None;
                None
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                turn.dragging = false;
                None
            }
            _ => None,
        }
    }

    fn draw(&self, turn: &Turn, _cursor: mouse::Cursor, bounds: Rectangle) -> Draw {
        let aspect = if bounds.height > 0.0 { bounds.width / bounds.height } else { 1.0 };
        let camera = mul(
            perspective(0.7, aspect, 0.1, 100.0),
            mul(translate(0.0, 0.0, -4.2), mul(rotate_x(turn.pitch), rotate_y(turn.yaw))),
        );
        let view_proj = mul(camera, mul(scale(1.0 / 16.0), translate(0.0, -16.0, 0.0)));
        Draw {
            vertices: model_vertices(&self.model),
            view_proj,
            skin: self.model.skin.clone(),
        }
    }
}

/// One frame of the viewer: its vertices, its camera, and the skin to sample.
#[derive(Debug)]
pub struct Draw {
    vertices: Vec<f32>,
    view_proj: Mat4,
    skin: Option<Arc<Skin>>,
}

impl shader::Primitive for Draw {
    type Pipeline = Pipeline;

    fn prepare(
        &self,
        pipeline: &mut Pipeline,
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        let scale_factor = viewport.scale_factor();
        pipeline.viewport = [
            bounds.x * scale_factor,
            bounds.y * scale_factor,
            bounds.width * scale_factor,
            bounds.height * scale_factor,
        ];
        queue.write_buffer(&pipeline.uniforms, 0, &floats_to_bytes(&flatten(self.view_proj)));

        let vertex_count = self.vertices.len() / FLOATS_PER_VERTEX;
        if vertex_count as u64 > pipeline.vertex_capacity {
            pipeline.vertex_capacity = vertex_count as u64;
            pipeline.vertices = device.create_buffer(&iced::wgpu::BufferDescriptor {
                label: Some("skin viewer vertices"),
                size: pipeline.vertex_capacity * VERTEX_BYTES,
                usage: iced::wgpu::BufferUsages::VERTEX | iced::wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&pipeline.vertices, 0, &floats_to_bytes(&self.vertices));
        pipeline.vertex_count = vertex_count as u32;

        let uploaded = pipeline.skin.as_ref().map(|(id, _)| *id);
        if let Some(skin) = self.skin.as_ref().filter(|skin| Some(skin.id) != uploaded) {
            let texture = device.create_texture(&iced::wgpu::TextureDescriptor {
                label: Some("skin viewer texture"),
                size: iced::wgpu::Extent3d {
                    width: SKIN_SIZE,
                    height: SKIN_SIZE,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: iced::wgpu::TextureDimension::D2,
                format: iced::wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: iced::wgpu::TextureUsages::TEXTURE_BINDING | iced::wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                iced::wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: iced::wgpu::Origin3d::ZERO,
                    aspect: iced::wgpu::TextureAspect::All,
                },
                &skin.texels,
                iced::wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(SKIN_SIZE * 4),
                    rows_per_image: Some(SKIN_SIZE),
                },
                iced::wgpu::Extent3d {
                    width: SKIN_SIZE,
                    height: SKIN_SIZE,
                    depth_or_array_layers: 1,
                },
            );
            let view = texture.create_view(&iced::wgpu::TextureViewDescriptor::default());
            pipeline.skin = Some((skin.id, view));
            pipeline.bind_group = None;
        }
        if pipeline.bind_group.is_none()
            && let Some((_, view)) = &pipeline.skin
        {
            pipeline.bind_group = Some(device.create_bind_group(&iced::wgpu::BindGroupDescriptor {
                label: Some("skin viewer bindings"),
                layout: &pipeline.bind_layout,
                entries: &[
                    iced::wgpu::BindGroupEntry {
                        binding: 0,
                        resource: pipeline.uniforms.as_entire_binding(),
                    },
                    iced::wgpu::BindGroupEntry {
                        binding: 1,
                        resource: iced::wgpu::BindingResource::TextureView(view),
                    },
                    iced::wgpu::BindGroupEntry {
                        binding: 2,
                        resource: iced::wgpu::BindingResource::Sampler(&pipeline.sampler),
                    },
                ],
            }));
        }

        let size = viewport.physical_size();
        if pipeline
            .depth
            .as_ref()
            .is_none_or(|(_, width, height)| (*width, *height) != (size.width, size.height))
        {
            let texture = device.create_texture(&iced::wgpu::TextureDescriptor {
                label: Some("skin viewer depth"),
                size: iced::wgpu::Extent3d {
                    width: size.width.max(1),
                    height: size.height.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: iced::wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: iced::wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&iced::wgpu::TextureViewDescriptor::default());
            pipeline.depth = Some((view, size.width, size.height));
        }
    }

    fn render(&self, pipeline: &Pipeline, encoder: &mut iced::wgpu::CommandEncoder, target: &iced::wgpu::TextureView, clip_bounds: &Rectangle<u32>) {
        let (Some(bind_group), Some((depth_view, _, _))) = (&pipeline.bind_group, &pipeline.depth) else {
            return;
        };
        let mut pass = encoder.begin_render_pass(&iced::wgpu::RenderPassDescriptor {
            label: Some("skin viewer"),
            color_attachments: &[Some(iced::wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: iced::wgpu::Operations {
                    load: iced::wgpu::LoadOp::Load,
                    store: iced::wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(iced::wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(iced::wgpu::Operations {
                    load: iced::wgpu::LoadOp::Clear(1.0),
                    store: iced::wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_scissor_rect(clip_bounds.x, clip_bounds.y, clip_bounds.width, clip_bounds.height);
        let [x, y, width, height] = pipeline.viewport;
        pass.set_viewport(x, y, width, height, 0.0, 1.0);
        pass.set_pipeline(&pipeline.render);
        pass.set_bind_group(0, bind_group, &[]);
        pass.set_vertex_buffer(0, pipeline.vertices.slice(..));
        pass.draw(0..pipeline.vertex_count, 0..1);
    }
}

/// The GPU state of the viewer: its render pipeline, the camera, the skin texture, and the depth buffer.
pub struct Pipeline {
    render: iced::wgpu::RenderPipeline,
    bind_layout: iced::wgpu::BindGroupLayout,
    sampler: iced::wgpu::Sampler,
    uniforms: iced::wgpu::Buffer,
    vertices: iced::wgpu::Buffer,
    vertex_capacity: u64,
    vertex_count: u32,
    skin: Option<(u64, iced::wgpu::TextureView)>,
    bind_group: Option<iced::wgpu::BindGroup>,
    depth: Option<(iced::wgpu::TextureView, u32, u32)>,
    viewport: [f32; 4],
}

impl shader::Pipeline for Pipeline {
    fn new(device: &iced::wgpu::Device, _queue: &iced::wgpu::Queue, format: iced::wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(iced::wgpu::ShaderModuleDescriptor {
            label: Some("skin viewer shader"),
            source: iced::wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let bind_layout = device.create_bind_group_layout(&iced::wgpu::BindGroupLayoutDescriptor {
            label: Some("skin viewer bindings"),
            entries: &[
                iced::wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: iced::wgpu::ShaderStages::VERTEX,
                    ty: iced::wgpu::BindingType::Buffer {
                        ty: iced::wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                iced::wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: iced::wgpu::ShaderStages::FRAGMENT,
                    ty: iced::wgpu::BindingType::Texture {
                        sample_type: iced::wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: iced::wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                iced::wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: iced::wgpu::ShaderStages::FRAGMENT,
                    ty: iced::wgpu::BindingType::Sampler(iced::wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&iced::wgpu::PipelineLayoutDescriptor {
            label: Some("skin viewer layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });
        let render = device.create_render_pipeline(&iced::wgpu::RenderPipelineDescriptor {
            label: Some("skin viewer"),
            layout: Some(&layout),
            vertex: iced::wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[iced::wgpu::VertexBufferLayout {
                    array_stride: VERTEX_BYTES,
                    step_mode: iced::wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        iced::wgpu::VertexAttribute {
                            format: iced::wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        iced::wgpu::VertexAttribute {
                            format: iced::wgpu::VertexFormat::Float32x2,
                            offset: 12,
                            shader_location: 1,
                        },
                        iced::wgpu::VertexAttribute {
                            format: iced::wgpu::VertexFormat::Float32,
                            offset: 20,
                            shader_location: 2,
                        },
                    ],
                }],
            },
            primitive: iced::wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(iced::wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: iced::wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(iced::wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(iced::wgpu::ColorTargetState {
                    format,
                    blend: Some(iced::wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: iced::wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        let sampler = device.create_sampler(&iced::wgpu::SamplerDescriptor {
            label: Some("skin viewer sampler"),
            mag_filter: iced::wgpu::FilterMode::Nearest,
            min_filter: iced::wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let uniforms = device.create_buffer(&iced::wgpu::BufferDescriptor {
            label: Some("skin viewer camera"),
            size: 64,
            usage: iced::wgpu::BufferUsages::UNIFORM | iced::wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let vertices = device.create_buffer(&iced::wgpu::BufferDescriptor {
            label: Some("skin viewer vertices"),
            size: VERTEX_BYTES,
            usage: iced::wgpu::BufferUsages::VERTEX | iced::wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            render,
            bind_layout,
            sampler,
            uniforms,
            vertices,
            vertex_capacity: 1,
            vertex_count: 0,
            skin: None,
            bind_group: None,
            depth: None,
            viewport: [0.0; 4],
        }
    }
}

/// The shader: each vertex goes through the camera, and the skin texel is shaded by the face's light.
const SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var skin: texture_2d<f32>;
@group(0) @binding(2) var skin_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) shade: f32,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) shade: f32,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.shade = input.shade;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(skin, skin_sampler, input.uv);
    if (texel.a < 0.5) {
        discard;
    }
    return vec4<f32>(texel.rgb * input.shade, 1.0);
}
"#;

/// A column-major 4×4 matrix, as the shader reads it.
type Mat4 = [[f32; 4]; 4];

/// The identity matrix.
fn identity() -> Mat4 {
    [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]
}

/// `a` times `b`: applying `b` first, then `a`.
fn mul(a: Mat4, b: Mat4) -> Mat4 {
    let mut result = [[0.0; 4]; 4];
    for (column, out) in result.iter_mut().enumerate() {
        for (row, value) in out.iter_mut().enumerate() {
            *value = (0..4).map(|k| a[k][row] * b[column][k]).sum();
        }
    }
    result
}

/// A translation by `(x, y, z)`.
fn translate(x: f32, y: f32, z: f32) -> Mat4 {
    let mut matrix = identity();
    matrix[3] = [x, y, z, 1.0];
    matrix
}

/// A uniform scale.
fn scale(factor: f32) -> Mat4 {
    let mut matrix = identity();
    matrix[0][0] = factor;
    matrix[1][1] = factor;
    matrix[2][2] = factor;
    matrix
}

/// A rotation about the x axis.
fn rotate_x(angle: f32) -> Mat4 {
    let (sine, cosine) = angle.sin_cos();
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, cosine, sine, 0.0],
        [0.0, -sine, cosine, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// A rotation about the y axis.
fn rotate_y(angle: f32) -> Mat4 {
    let (sine, cosine) = angle.sin_cos();
    [
        [cosine, 0.0, -sine, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [sine, 0.0, cosine, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// A rotation about the z axis.
fn rotate_z(angle: f32) -> Mat4 {
    let (sine, cosine) = angle.sin_cos();
    [
        [cosine, sine, 0.0, 0.0],
        [-sine, cosine, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// A perspective projection onto the 0 to 1 depth range that the GPU uses.
fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let focal = 1.0 / (fov_y / 2.0).tan();
    [
        [focal / aspect, 0.0, 0.0, 0.0],
        [0.0, focal, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, far * near / (near - far), 0.0],
    ]
}

/// The matrix flattened in column order, for the uniform buffer.
fn flatten(matrix: Mat4) -> Vec<f32> {
    matrix.iter().flatten().copied().collect()
}

/// Floats as the bytes the GPU reads.
fn floats_to_bytes(floats: &[f32]) -> Vec<u8> {
    floats.iter().flat_map(|float| float.to_ne_bytes()).collect()
}

/// A rotation about `pivot`: the transform of a limb that swings from its joint.
fn about(pivot: [f32; 3], rotation: Mat4) -> Mat4 {
    mul(
        translate(pivot[0], pivot[1], pivot[2]),
        mul(rotation, translate(-pivot[0], -pivot[1], -pivot[2])),
    )
}

/// One box of the model: its corners in model pixels, its skin position, and the transform its pose gives it.
struct Part {
    origin: [f32; 3],
    size: [f32; 3],
    uv: (u32, u32),
    inflate: f32,
    transform: Mat4,
}

/// The boxes that make up the model for a model's pose, layers and arm width.
fn parts(model: &Model) -> Vec<Part> {
    let (arm_width, arm_x) = if model.slim { (3.0, 1.0) } else { (4.0, 0.0) };
    let (right_arm, left_arm, right_leg, left_leg) = arm_and_leg_angles(model.pose);
    let crouch = if model.pose == Pose::Crouch { 3.0 } else { 0.0 };
    let body_transform = translate(0.0, -crouch, 0.0);
    let mut out = vec![
        Part {
            origin: [-4.0, 24.0, -4.0],
            size: [8.0, 8.0, 8.0],
            uv: (0, 0),
            inflate: 0.0,
            transform: body_transform,
        },
        Part {
            origin: [-4.0, 12.0, -2.0],
            size: [8.0, 12.0, 4.0],
            uv: (16, 16),
            inflate: 0.0,
            transform: body_transform,
        },
        Part {
            origin: [-8.0 + arm_x, 12.0, -2.0],
            size: [arm_width, 12.0, 4.0],
            uv: (40, 16),
            inflate: 0.0,
            transform: mul(
                body_transform,
                about([-6.0, 22.0, 0.0], mul(rotate_z(right_arm.0), rotate_x(right_arm.1))),
            ),
        },
        Part {
            origin: [4.0, 12.0, -2.0],
            size: [arm_width, 12.0, 4.0],
            uv: (32, 48),
            inflate: 0.0,
            transform: mul(body_transform, about([6.0, 22.0, 0.0], mul(rotate_z(left_arm.0), rotate_x(left_arm.1)))),
        },
        Part {
            origin: [-4.0, 0.0, -2.0],
            size: [4.0, 12.0, 4.0],
            uv: (0, 16),
            inflate: 0.0,
            transform: about([-2.0, 12.0, 0.0], rotate_x(right_leg)),
        },
        Part {
            origin: [0.0, 0.0, -2.0],
            size: [4.0, 12.0, 4.0],
            uv: (16, 48),
            inflate: 0.0,
            transform: about([2.0, 12.0, 0.0], rotate_x(left_leg)),
        },
    ];
    if model.layers.hat {
        out.push(Part {
            origin: [-4.0, 24.0, -4.0],
            size: [8.0, 8.0, 8.0],
            uv: (32, 0),
            inflate: 0.5,
            transform: body_transform,
        });
    }
    if model.layers.jacket {
        out.push(Part {
            origin: [-4.0, 12.0, -2.0],
            size: [8.0, 12.0, 4.0],
            uv: (16, 32),
            inflate: 0.25,
            transform: body_transform,
        });
    }
    if model.layers.sleeves {
        out.push(Part {
            origin: [-8.0 + arm_x, 12.0, -2.0],
            size: [arm_width, 12.0, 4.0],
            uv: (40, 32),
            inflate: 0.25,
            transform: out[2].transform,
        });
        out.push(Part {
            origin: [4.0, 12.0, -2.0],
            size: [arm_width, 12.0, 4.0],
            uv: (48, 48),
            inflate: 0.25,
            transform: out[3].transform,
        });
    }
    if model.layers.pants {
        out.push(Part {
            origin: [-4.0, 0.0, -2.0],
            size: [4.0, 12.0, 4.0],
            uv: (0, 32),
            inflate: 0.25,
            transform: out[4].transform,
        });
        out.push(Part {
            origin: [0.0, 0.0, -2.0],
            size: [4.0, 12.0, 4.0],
            uv: (0, 48),
            inflate: 0.25,
            transform: out[5].transform,
        });
    }
    out
}

/// The swing of the arms and legs for a pose, as `(right arm, left arm, right leg, left leg)`. An arm is a pair of
/// angles: the z turn that lifts it sideways, and the x swing that moves it forward or back.
fn arm_and_leg_angles(pose: Pose) -> ((f32, f32), (f32, f32), f32, f32) {
    use std::f32::consts::FRAC_PI_2;
    match pose {
        Pose::Stand => ((0.0, 0.0), (0.0, 0.0), 0.0, 0.0),
        Pose::Arms => ((-FRAC_PI_2, 0.0), (FRAC_PI_2, 0.0), 0.0, 0.0),
        Pose::Walk => ((0.0, 0.5), (0.0, -0.5), -0.5, 0.5),
        Pose::Run => ((0.0, 0.9), (0.0, -0.9), -0.9, 0.9),
        Pose::Crouch => ((0.0, -0.6), (0.0, -0.6), -0.6, -0.6),
    }
}

/// The vertices of the model: six faces per box, each as two triangles, with the texel coordinates and the face's shade.
fn model_vertices(model: &Model) -> Vec<f32> {
    let mut vertices = Vec::new();
    for part in parts(model) {
        push_box(&mut vertices, &part);
    }
    vertices
}

/// One face of a box: its four corners in order, the texel rectangle `(u, v, width, height)` it takes from the skin, and its
/// shade.
type Face = ([[f32; 3]; 4], (f32, f32, f32, f32), f32);

/// The faces of one box. Each face is the rectangle of the skin that paints it, and each corner of the rectangle lands on
/// a corner of the box.
fn push_box(vertices: &mut Vec<f32>, part: &Part) {
    let (u, v) = (part.uv.0 as f32, part.uv.1 as f32);
    let (w, h, d) = (part.size[0], part.size[1], part.size[2]);
    let (wu, hv, du) = (w, h, d);
    let (min, max) = (
        [
            part.origin[0] - part.inflate,
            part.origin[1] - part.inflate,
            part.origin[2] - part.inflate,
        ],
        [
            part.origin[0] + w + part.inflate,
            part.origin[1] + h + part.inflate,
            part.origin[2] + d + part.inflate,
        ],
    );
    let (x0, y0, z0) = (min[0], min[1], min[2]);
    let (x1, y1, z1) = (max[0], max[1], max[2]);
    // Each face: four corners in order, the texel rectangle (u, v, width, height), and its shade.
    let faces: [Face; 6] = [
        // front (+z)
        ([[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]], (u + du, v + du, wu, hv), 0.95),
        // back (-z)
        (
            [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]],
            (u + 2.0 * du + wu, v + du, wu, hv),
            0.8,
        ),
        // right side (+x)
        (
            [[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]],
            (u + du + wu, v + du, du, hv),
            0.85,
        ),
        // left side (-x)
        ([[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]], (u, v + du, du, hv), 0.85),
        // top (+y)
        ([[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]], (u + du, v, wu, du), 1.0),
        // bottom (-y)
        ([[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]], (u + du + wu, v, wu, du), 0.6),
    ];
    for (corners, (texel_u, texel_v, texel_w, texel_h), shade) in faces {
        let uv = [
            [texel_u, texel_v + texel_h],
            [texel_u + texel_w, texel_v + texel_h],
            [texel_u + texel_w, texel_v],
            [texel_u, texel_v],
        ];
        let vertex = |index: usize| {
            let p = transform_point(part.transform, corners[index]);
            [p[0], p[1], p[2], uv[index][0] / SKIN_SIZE as f32, uv[index][1] / SKIN_SIZE as f32, shade]
        };
        for index in [0, 1, 2, 0, 2, 3] {
            vertices.extend_from_slice(&vertex(index));
        }
    }
}

/// `point` moved by `matrix`.
fn transform_point(matrix: Mat4, point: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0; 3];
    for (row, value) in out.iter_mut().enumerate() {
        *value = matrix[0][row] * point[0] + matrix[1][row] * point[1] + matrix[2][row] * point[2] + matrix[3][row];
    }
    out
}
