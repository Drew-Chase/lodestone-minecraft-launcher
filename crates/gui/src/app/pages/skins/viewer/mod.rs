//! The 3D skin viewer: a player model built from the six boxes of a skin, drawn with wgpu through iced's shader widget.
//!
//! The CPU lays out the model's vertices for the pose at the moment shown, and the GPU samples the skin texture with
//! nearest filtering, so pixel art stays sharp. Dragging the model turns it, and the pose moves: a walk swings the limbs
//! through its stride and a run through a faster one, unless reduced motion is on. The model fills the viewer's height. A
//! cape the player wears is packed beside the skin in the same texture, so the model is still one draw. The skin previews
//! draw the same model on the CPU, in `raster`.

mod geometry;
mod math;
pub mod raster;

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::widget::Shader;
use iced::widget::shader::{self, Program};
use iced::{Element, Event, Fill, Rectangle, mouse, window};

use self::geometry::model_vertices;
use self::math::{Mat4, flatten, floats_to_bytes, mul, perspective, rotate_x, rotate_y, scale, translate};
use crate::app::theme::effects;

/// How far the camera stands from the middle of the model. It is far enough that the model's nearest corner, which is a little
/// closer to the camera than its middle, still fits the viewer's height with a margin.
const DISTANCE: f32 = 3.4;

/// The vertical field of view, in radians.
const FOV: f32 = 0.7;

/// The share of the viewer's half-width that a wide pose may reach, so its outer edges keep clear of the viewer's edges.
const WIDTH_MARGIN: f32 = 0.92;

/// The turn the viewer starts with, and the turn the skin previews are drawn with, in radians.
pub const YAW: f32 = 0.6;
pub const PITCH: f32 = -0.1;

/// How long a frame of the pose's motion lasts: the viewer asks for a frame this often while the pose moves.
const FRAME: Duration = Duration::from_millis(33);

/// The side of a skin texture, in texels.
const SKIN_SIZE: u32 = 64;

/// The width and height of the texture the viewer samples: the skin on the left, and the cape beside it on the right.
const ATLAS_WIDTH: u32 = 2 * SKIN_SIZE;
const ATLAS_HEIGHT: u32 = SKIN_SIZE;

/// Where the cape starts on the texture, across, and its size. A cape is 64×32 texels, the standard shape.
const CAPE_X: u32 = SKIN_SIZE;
const CAPE_WIDTH: u32 = SKIN_SIZE;
const CAPE_HEIGHT: u32 = SKIN_SIZE / 2;

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

    /// The moment of the pose's motion that a skin preview draws: the moment its limbs are furthest through the motion, so a
    /// still preview shows the pose at its fullest. A pose that holds still draws at the start.
    pub fn preview_time(self) -> f32 {
        match self {
            Pose::Walk => geometry::WALK_PERIOD / 4.0,
            Pose::Run => geometry::RUN_PERIOD / 4.0,
            Pose::Stand | Pose::Arms | Pose::Crouch => 0.0,
        }
    }

    /// How far the pose reaches sideways from the body, in model units, as the viewer's turn shows it. The camera steps back
    /// for a wide pose, so that it fits the view.
    pub fn reach(self) -> f32 {
        match self {
            Pose::Stand | Pose::Crouch => 0.56,
            Pose::Walk => 0.7,
            Pose::Run => 0.85,
            Pose::Arms => 1.05,
        }
    }
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

/// A decoded skin: the texels of the texture the viewer samples, with the skin on the left and, when one is worn, the
/// cape on the right. Its id changes with the picture, so the GPU texture is uploaded once per picture.
#[derive(Debug)]
pub struct Skin {
    pub id: u64,
    pub texels: Vec<u8>,
    /// Whether the texture carries a cape, so the model draws one.
    pub cape: bool,
}

impl Skin {
    /// Decodes a PNG skin and, when one is given, the PNG cape worn with it, and packs both into the texture the viewer
    /// samples. A 64×32 skin fills the top half of the skin area, and the bottom half stays clear.
    pub fn compose(png: &[u8], cape_png: Option<&[u8]>) -> Option<Skin> {
        let base = skin_texels(png)?;
        let cape = cape_png.and_then(decode_cape);
        let texels = atlas(&base, cape.as_ref());
        let mut hasher = DefaultHasher::new();
        texels.hash(&mut hasher);
        Some(Skin {
            id: hasher.finish(),
            texels,
            cape: cape.is_some(),
        })
    }
}

/// The 64 by 64 texels of a skin PNG, with a 64 by 32 skin's bottom half clear. `None` when the bytes are not a skin.
pub fn skin_texels(png: &[u8]) -> Option<Vec<u8>> {
    let image = image::load_from_memory(png).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    if width != SKIN_SIZE || !(height == SKIN_SIZE || height == SKIN_SIZE / 2) {
        return None;
    }
    let mut texels = vec![0u8; (SKIN_SIZE * SKIN_SIZE * 4) as usize];
    texels[..image.as_raw().len()].copy_from_slice(image.as_raw());
    Some(texels)
}

/// The texture the viewer samples, from a skin's texels and the cape worn with it: the skin on the left, and the cape, when
/// there is one, beside it on the right.
pub fn atlas(base: &[u8], cape: Option<&image::RgbaImage>) -> Vec<u8> {
    let row = (ATLAS_WIDTH * 4) as usize;
    let mut texels = vec![0u8; (ATLAS_WIDTH * ATLAS_HEIGHT * 4) as usize];
    for (y, source) in base.chunks_exact((SKIN_SIZE * 4) as usize).enumerate() {
        texels[y * row..y * row + source.len()].copy_from_slice(source);
    }
    if let Some(cape) = cape {
        let start_x = (CAPE_X * 4) as usize;
        for (y, source) in cape.as_raw().chunks_exact((CAPE_WIDTH * 4) as usize).enumerate() {
            texels[y * row + start_x..y * row + start_x + source.len()].copy_from_slice(source);
        }
    }
    texels
}

/// Decodes a cape PNG to the 64×32 texels the texture holds. A cape of another size is resampled, nearest, to that size.
pub fn decode_cape(png: &[u8]) -> Option<image::RgbaImage> {
    let image = image::load_from_memory(png).ok()?.to_rgba8();
    if image.dimensions() == (CAPE_WIDTH, CAPE_HEIGHT) {
        return Some(image);
    }
    Some(image::imageops::resize(
        &image,
        CAPE_WIDTH,
        CAPE_HEIGHT,
        image::imageops::FilterType::Nearest,
    ))
}

/// The model the viewer shows: the skin, the pose, the layers, whether the arms are the slim model, and how far into the
/// pose's motion it stands, in seconds.
#[derive(Debug, Clone)]
pub struct Model {
    pub skin: Option<Arc<Skin>>,
    pub pose: Pose,
    pub layers: Layers,
    pub slim: bool,
    pub time: f32,
}

/// The viewer's state between frames: the turn of the model, whether a drag is under way, where the pointer last was, and
/// how far the pose's motion has run.
#[derive(Debug, Clone, Copy)]
pub struct Turn {
    yaw: f32,
    pitch: f32,
    /// Whether the left button went down over the viewer, so pointer moves turn the model until it comes up.
    dragging: bool,
    /// The pointer's place in the window at its last move. A drag turns the model by how far the pointer moved since then.
    /// Both ends are window coordinates, as `CursorMoved` reports them.
    pointer: Option<(f32, f32)>,
    /// The moment the last frame of the pose's motion was drawn, and the seconds of motion run so far.
    clock: Option<Instant>,
    time: f32,
}

impl Default for Turn {
    fn default() -> Self {
        Self {
            yaw: YAW,
            pitch: PITCH,
            dragging: false,
            pointer: None,
            clock: None,
            time: 0.0,
        }
    }
}

/// The shader program: it turns the model with the pointer, moves it through its pose, and draws it.
pub struct ModelProgram {
    model: Model,
}

/// The viewer, as an element of the page. It fills the space it is given. Dragging with the left button turns the model.
pub fn viewer<'a, M: 'a>(model: Model) -> Element<'a, M> {
    Shader::new(ModelProgram { model }).width(Fill).height(Fill).into()
}

/// The camera's matrix for a view `aspect` wide for each unit high, turned by `yaw` and `pitch`. The model stands at
/// [`DISTANCE`], which fits it to the height, or further back when `pose` reaches wider than the view allows.
pub fn view_proj(aspect: f32, pitch: f32, yaw: f32, pose: Pose) -> Mat4 {
    let half_width = (FOV / 2.0).tan() * aspect * WIDTH_MARGIN;
    let distance = DISTANCE.max(pose.reach() / half_width);
    let camera = mul(
        perspective(FOV, aspect, 0.1, 100.0),
        mul(translate(0.0, 0.0, -distance), mul(rotate_x(pitch), rotate_y(yaw))),
    );
    mul(camera, mul(scale(1.0 / 16.0), translate(0.0, -16.0, 0.0)))
}

impl<M> Program<M> for ModelProgram {
    type State = Turn;
    type Primitive = Draw;

    fn update(&self, turn: &mut Turn, event: &Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<shader::Action<M>> {
        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                // While the pose moves, the next frame is asked for, and the motion runs on by the time that has passed. Reduced
                // motion holds the pose still, and the viewer asks for no more frames.
                if !effects::ambient() {
                    turn.clock = None;
                    return None;
                }
                if let Some(last) = turn.clock {
                    turn.time += now.saturating_duration_since(last).as_secs_f32();
                }
                turn.clock = Some(*now);
                Some(shader::Action::request_redraw_at(*now + FRAME))
            }
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
        // Reduced motion draws the pose as it is held, not as it moves.
        let time = if effects::ambient() { turn.time } else { self.model.pose.preview_time() };
        let model = Model { time, ..self.model.clone() };
        Draw {
            vertices: model_vertices(&model),
            view_proj: view_proj(aspect, turn.pitch, turn.yaw, model.pose),
            skin: model.skin.clone(),
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
                    width: ATLAS_WIDTH,
                    height: ATLAS_HEIGHT,
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
                    bytes_per_row: Some(ATLAS_WIDTH * 4),
                    rows_per_image: Some(ATLAS_HEIGHT),
                },
                iced::wgpu::Extent3d {
                    width: ATLAS_WIDTH,
                    height: ATLAS_HEIGHT,
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

/// The shader: each vertex goes through the camera, and the texel is shaded by the face's light.
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

#[cfg(test)]
#[path = "../../../../../tests/app/pages/skins/viewer/mod.rs"]
mod tests;
