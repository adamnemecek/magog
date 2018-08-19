//! Glium-based backend for the Vitral GUI library.

#![deny(missing_docs)]

use crate::{
    AtlasCache, CanvasZoom, Color, Core, ImageBuffer, Keycode, MouseButton, TextureIndex, Vertex,
};
use euclid::{Point2D, Size2D};
use glium::glutin::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use glium::glutin::{self, Event, WindowEvent};
use glium::index::PrimitiveType;
use glium::{self, Surface};
use std::error::Error;
use std::fmt::Debug;
use std::hash::Hash;

/// Default texture type used by the backend.
type GliumTexture = glium::texture::SrgbTexture2d;

/// Glium-rendering backend for Vitral.
pub struct Backend<V> {
    display: glium::Display,
    events: glutin::EventsLoop,
    program: glium::Program,
    textures: Vec<GliumTexture>,

    keypress: Vec<KeyEvent>,

    canvas: Canvas,
    zoom: CanvasZoom,
    window_size: Size2D<u32>,

    phantom: ::std::marker::PhantomData<V>,
}

impl<V: glium::Vertex + Vertex> Backend<V> {
    /// Create a new Glium backend for Vitral.
    ///
    /// The backend requires an user-supplied vertex type as a type parameter and a shader program
    /// to render data of that type as argument to the constructor.
    pub fn new(
        display: glium::Display,
        events: glutin::EventsLoop,
        program: glium::Program,
        width: u32,
        height: u32,
    ) -> Backend<V> {
        let (w, h) = get_size(&display);
        let canvas = Canvas::new(&display, width, height);

        Backend {
            display,
            events,
            program,
            textures: Vec::new(),

            keypress: Vec::new(),

            canvas,
            zoom: CanvasZoom::PixelPerfect,
            window_size: Size2D::new(w, h),

            phantom: ::std::marker::PhantomData,
        }
    }

    /// Open a Glium window and start a backend for it.
    ///
    /// The custom shader must support a uniform named `tex` for texture data.
    pub fn start<'a, S, P>(
        width: u32,
        height: u32,
        title: S,
        shader: P,
    ) -> Result<Backend<V>, Box<Error>>
    where
        S: Into<String>,
        P: Into<glium::program::ProgramCreationInput<'a>>,
    {
        let events = glutin::EventsLoop::new();
        let window = glutin::WindowBuilder::new().with_title(title);
        let context = glutin::ContextBuilder::new()
            .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 2)));
        let display = glium::Display::new(window, context, &events)?;
        let program = glium::Program::new(&display, shader.into())?;

        {
            // Start the window as a good fit on the primary monitor.

            // Don't make it a completely fullscreen window, that might put the window title bar
            // outside the screen.
            const BUFFER: f64 = 8.0;
            let (width, height) = (width as f64, height as f64);

            let monitor_size = display
                .gl_window()
                .window()
                .get_primary_monitor()
                .get_dimensions();
            // Get the most conservative DPI if there's a weird multi-monitor setup.
            let dpi_factor = display
                .gl_window()
                .get_available_monitors()
                .map(|m| m.get_hidpi_factor())
                .max_by(|x, y| x.partial_cmp(y).unwrap())
                .expect("No monitors found!");
            debug!("Scaling starting size to monitor");
            debug!("Monitor size {:?}", monitor_size);
            debug!("DPI Factor {}", dpi_factor);

            let mut window_size = PhysicalSize::new(width, height);
            while window_size.width + width <= monitor_size.width - BUFFER
                && window_size.height + height <= monitor_size.height - BUFFER
            {
                window_size.width += width;
                window_size.height += height;
            }
            debug!("Adjusted window size: {:?}", window_size);
            let window_pos = PhysicalPosition::new(
                (monitor_size.width - window_size.width) / 2.0,
                (monitor_size.height - window_size.height) / 2.0,
            );

            display
                .gl_window()
                .set_inner_size(window_size.to_logical(dpi_factor));
            display
                .gl_window()
                .set_position(window_pos.to_logical(dpi_factor));
        }

        Ok(Backend::new(display, events, program, width, height))
    }

    /// Return the pixel resolution of the backend.
    ///
    /// Note that this is the logical size which will stay the same even when the
    /// desktop window is resized.
    pub fn canvas_size(&self) -> Size2D<u32> { self.canvas.size }

    /// Return the current number of textures.
    pub fn texture_count(&self) -> usize { self.textures.len() }

    /// Make a new empty internal texture.
    ///
    /// The new `TextureIndex` must equal the value `self.texture_count()` would have returned
    /// just before calling this.
    pub fn make_empty_texture(&mut self, width: u32, height: u32) -> TextureIndex {
        let tex = glium::texture::SrgbTexture2d::empty(&self.display, width, height).unwrap();
        self.textures.push(tex);
        self.textures.len() - 1
    }

    /// Rewrite an internal texture.
    pub fn write_to_texture(&mut self, img: &ImageBuffer, texture: TextureIndex) {
        assert!(
            texture < self.textures.len(),
            "Trying to write nonexistent texture"
        );
        let rect = glium::Rect {
            left: 0,
            bottom: 0,
            width: img.size.width,
            height: img.size.height,
        };
        let mut raw = glium::texture::RawImage2d::from_raw_rgba(
            img.pixels.clone(),
            (img.size.width, img.size.height),
        );
        raw.format = glium::texture::ClientFormat::U8U8U8U8;

        self.textures[texture].write(rect, raw);
    }

    /// Make a new internal texture using image data.
    pub fn make_texture(&mut self, img: ImageBuffer) -> TextureIndex {
        let mut raw = glium::texture::RawImage2d::from_raw_rgba(
            img.pixels,
            (img.size.width, img.size.height),
        );
        raw.format = glium::texture::ClientFormat::U8U8U8U8;

        let tex = glium::texture::SrgbTexture2d::new(&self.display, raw).unwrap();
        self.textures.push(tex);
        self.textures.len() - 1
    }

    /// Update or construct textures based on changes in atlas cache.
    pub fn sync_with_atlas_cache<T: Eq + Hash + Clone + Debug>(
        &mut self,
        atlas_cache: &mut AtlasCache<T>,
    ) {
        for a in atlas_cache.atlases_mut() {
            let idx = a.texture();
            // If there are sheets in the atlas that don't have corresponding textures yet,
            // construct those now.
            while idx >= self.texture_count() {
                self.make_empty_texture(a.size().width, a.size().height);
            }

            // Write the updated texture atlas to internal texture.
            a.update_texture(|buf, idx| self.write_to_texture(buf, idx));
        }
    }

    fn process_events(&mut self, core: &mut Core<V>) -> bool {
        self.keypress.clear();

        // polling and handling the events received by the window
        let mut event_list = Vec::new();
        self.events.poll_events(|event| event_list.push(event));

        for e in event_list {
            match e {
                Event::WindowEvent {
                    ref event,
                    window_id,
                }
                    if window_id == self.display.gl_window().id() =>
                {
                    match event {
                        &WindowEvent::CloseRequested => return false,
                        &WindowEvent::CursorMoved { position, .. } => {
                            let position =
                                position.to_physical(self.display.gl_window().get_hidpi_factor());
                            let pos = self.zoom.screen_to_canvas(
                                self.window_size,
                                self.canvas.size(),
                                Point2D::new(position.x as f32, position.y as f32),
                            );
                            core.input_mouse_move(pos.x as i32, pos.y as i32);
                        }
                        &WindowEvent::MouseInput { state, button, .. } => core.input_mouse_button(
                            match button {
                                glutin::MouseButton::Left => MouseButton::Left,
                                glutin::MouseButton::Right => MouseButton::Right,
                                _ => MouseButton::Middle,
                            },
                            state == glutin::ElementState::Pressed,
                        ),
                        &WindowEvent::ReceivedCharacter(c) => core.input_char(c),
                        &WindowEvent::KeyboardInput {
                            input:
                                glutin::KeyboardInput {
                                    state,
                                    scancode,
                                    virtual_keycode,
                                    ..
                                },
                            ..
                        } => {
                            self.keypress.push(KeyEvent {
                                state,
                                scancode: scancode as u8,
                                virtual_keycode,
                            });

                            let is_down = state == glutin::ElementState::Pressed;

                            use glium::glutin::VirtualKeyCode::*;
                            if let Some(vk) = match virtual_keycode {
                                Some(Tab) => Some(Keycode::Tab),
                                Some(LShift) | Some(RShift) => Some(Keycode::Shift),
                                Some(LControl) | Some(RControl) => Some(Keycode::Ctrl),
                                Some(NumpadEnter) | Some(Return) => Some(Keycode::Enter),
                                Some(Back) => Some(Keycode::Backspace),
                                Some(Delete) => Some(Keycode::Del),
                                Some(Numpad8) | Some(Up) => Some(Keycode::Up),
                                Some(Numpad2) | Some(Down) => Some(Keycode::Down),
                                Some(Numpad4) | Some(Left) => Some(Keycode::Left),
                                Some(Numpad6) | Some(Right) => Some(Keycode::Right),
                                _ => None,
                            } {
                                core.input_key_state(vk, is_down);
                            }
                        }
                        _ => (),
                    }
                }
                // Events in other windows, ignore
                Event::WindowEvent { .. } => {}
                Event::Awakened => {
                    // TODO: Suspend/awaken behavior
                }
                Event::DeviceEvent { .. } => {}
                Event::Suspended(_) => {}
            }
        }

        true
    }

    /// Return the next keypress event if there is one.
    pub fn poll_key(&mut self) -> Option<KeyEvent> { self.keypress.pop() }

    fn render(&mut self, core: &mut Core<V>) {
        let mut target = self.canvas.get_framebuffer_target(&self.display);
        target.clear_color(0.0, 0.0, 0.0, 0.0);
        let (w, h) = target.get_dimensions();

        for batch in core.end_frame() {
            // building the uniforms
            let uniforms = uniform! {
                matrix: [
                    [2.0 / w as f32, 0.0, 0.0, -1.0],
                    [0.0, -2.0 / h as f32, 0.0, 1.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0f32]
                ],
                tex: glium::uniforms::Sampler::new(&self.textures[batch.texture])
                    .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest),
            };

            let vertex_buffer =
                { glium::VertexBuffer::new(&self.display, &batch.vertices).unwrap() };

            // building the index buffer
            let index_buffer = glium::IndexBuffer::new(
                &self.display,
                PrimitiveType::TrianglesList,
                &batch.triangle_indices,
            ).unwrap();

            let params = glium::draw_parameters::DrawParameters {
                scissor: batch.clip.map(|clip| glium::Rect {
                    left: clip.origin.x as u32,
                    bottom: h - (clip.origin.y + clip.size.height) as u32,
                    width: clip.size.width as u32,
                    height: clip.size.height as u32,
                }),
                blend: glium::Blend::alpha_blending(),
                ..Default::default()
            };

            target
                .draw(
                    &vertex_buffer,
                    &index_buffer,
                    &self.program,
                    &uniforms,
                    &params,
                ).unwrap();
        }
    }

    fn update_window_size(&mut self) {
        let (w, h) = get_size(&self.display);
        self.window_size = Size2D::new(w, h);
    }

    /// Display the backend and read input events.
    pub fn update(&mut self, core: &mut Core<V>) -> bool {
        self.update_window_size();
        self.render(core);
        self.canvas.draw(&self.display, self.zoom);
        self.process_events(core)
    }

    /// Return an image for the current contents of the screen.
    pub fn screenshot(&self) -> ImageBuffer { self.canvas.screenshot() }
}

/// Type for key events not handled by Vitral.
#[derive(Debug)]
pub struct KeyEvent {
    /// Was the key pressed or released
    pub state: glutin::ElementState,
    /// Layout-dependent keycode
    pub virtual_keycode: Option<glutin::VirtualKeyCode>,
    /// Keyboard layout independent hardware scancode for the key
    pub scancode: u8,
}

/// Shader program for the `DefaultVertex` type
pub const DEFAULT_SHADER: glium::program::SourceCode = glium::program::SourceCode {
    vertex_shader: "
        #version 150 core

        uniform mat4 matrix;

        in vec2 pos;
        in vec4 color;
        in vec2 tex_coord;

        out vec4 v_color;
        out vec2 v_tex_coord;

        void main() {
            gl_Position = vec4(pos, 0.0, 1.0) * matrix;
            v_color = color;
            v_tex_coord = tex_coord;
        }",
    fragment_shader: "
        #version 150 core
        uniform sampler2D tex;
        in vec4 v_color;
        in vec2 v_tex_coord;
        out vec4 f_color;

        void main() {
            vec4 tex_color = texture(tex, v_tex_coord);

            // Discard fully transparent pixels to keep them from
            // writing into the depth buffer.
            if (tex_color.a == 0.0) discard;

            f_color = v_color * tex_color;
        }",
    tessellation_control_shader: None,
    tessellation_evaluation_shader: None,
    geometry_shader: None,
};

/// A regular vertex that implements exactly the fields used by Vitral.
#[derive(Copy, Clone)]
pub struct DefaultVertex {
    /// 2D position
    pub pos: [f32; 2],
    /// Texture coordinates
    pub tex_coord: [f32; 2],
    /// RGBA color
    pub color: Color,
}
implement_vertex!(DefaultVertex, pos, tex_coord, color);

impl Vertex for DefaultVertex {
    fn new(pos: Point2D<f32>, tex_coord: Point2D<f32>, color: Color) -> Self {
        DefaultVertex {
            pos: [pos.x, pos.y],
            tex_coord: [tex_coord.x, tex_coord.y],
            color,
        }
    }
}

/// A deferred rendering buffer for pixel-perfect display.
struct Canvas {
    size: Size2D<u32>,
    buffer: glium::texture::SrgbTexture2d,
    depth_buffer: glium::framebuffer::DepthRenderBuffer,
    shader: glium::Program,
}

impl Canvas {
    pub fn new(display: &glium::Display, width: u32, height: u32) -> Canvas {
        let shader = program!(
            display,
            150 => {
            vertex: "
                #version 150 core

                in vec2 pos;
                in vec2 tex_coord;

                out vec2 v_tex_coord;

                void main() {
                    v_tex_coord = tex_coord;
                    gl_Position = vec4(pos, 0.0, 1.0);
                }",
            fragment: "
                #version 150 core

                uniform sampler2D tex;
                in vec2 v_tex_coord;

                out vec4 f_color;

                void main() {
                    vec4 tex_color = texture(tex, v_tex_coord);
                    tex_color.a = 1.0;
                    f_color = tex_color;
                }"}).unwrap();

        let buffer = glium::texture::SrgbTexture2d::empty(display, width, height).unwrap();

        let depth_buffer = glium::framebuffer::DepthRenderBuffer::new(
            display,
            glium::texture::DepthFormat::F32,
            width,
            height,
        ).unwrap();

        Canvas {
            size: Size2D::new(width, height),
            buffer,
            depth_buffer,
            shader,
        }
    }

    /// Get the render target to the pixel-perfect framebuffer.
    pub fn get_framebuffer_target(
        &mut self,
        display: &glium::Display,
    ) -> glium::framebuffer::SimpleFrameBuffer {
        glium::framebuffer::SimpleFrameBuffer::with_depth_buffer(
            display,
            &self.buffer,
            &self.depth_buffer,
        ).unwrap()
    }

    pub fn draw(&mut self, display: &glium::Display, zoom: CanvasZoom) {
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);

        let (w, h) = get_size(display);

        // Build the geometry for the on-screen rectangle.
        let s_rect = zoom.fit_canvas(Size2D::new(w, h), self.size);

        let (sx, sy) = (s_rect.origin.x, s_rect.origin.y);
        let (sw, sh) = (s_rect.size.width, s_rect.size.height);

        // XXX: This could use glium::Surface::blit_whole_color_to instead of
        // the handmade blitting, but that was buggy on Windows around
        // 2015-03.

        let vertices = {
            #[derive(Copy, Clone)]
            struct BlitVertex {
                pos: [f32; 2],
                tex_coord: [f32; 2],
            }
            implement_vertex!(BlitVertex, pos, tex_coord);

            glium::VertexBuffer::new(
                display,
                &[
                    BlitVertex {
                        pos: [sx, sy],
                        tex_coord: [0.0, 0.0],
                    },
                    BlitVertex {
                        pos: [sx + sw, sy],
                        tex_coord: [1.0, 0.0],
                    },
                    BlitVertex {
                        pos: [sx + sw, sy + sh],
                        tex_coord: [1.0, 1.0],
                    },
                    BlitVertex {
                        pos: [sx, sy + sh],
                        tex_coord: [0.0, 1.0],
                    },
                ],
            ).unwrap()
        };

        let indices = glium::IndexBuffer::new(
            display,
            glium::index::PrimitiveType::TrianglesList,
            &[0u16, 1, 2, 0, 2, 3],
        ).unwrap();

        // Set up the rest of the draw parameters.
        let mut params: glium::DrawParameters = Default::default();
        // Set an explicit viewport to apply the custom resolution that fixes
        // pixel perfect rounding errors.
        params.viewport = Some(glium::Rect {
            left: 0,
            bottom: 0,
            width: w,
            height: h,
        });

        // TODO: Option to use smooth filter & non-pixel-perfect scaling
        let mag_filter = glium::uniforms::MagnifySamplerFilter::Nearest;

        let uniforms = glium::uniforms::UniformsStorage::new(
            "tex",
            glium::uniforms::Sampler(
                &self.buffer,
                glium::uniforms::SamplerBehavior {
                    magnify_filter: mag_filter,
                    minify_filter: glium::uniforms::MinifySamplerFilter::Linear,
                    ..Default::default()
                },
            ),
        );

        // Draw the graphics buffer to the window.
        target
            .draw(&vertices, &indices, &self.shader, &uniforms, &params)
            .unwrap();
        target.finish().unwrap();
    }

    pub fn size(&self) -> Size2D<u32> { self.size }

    pub fn screenshot(&self) -> ImageBuffer {
        let image: glium::texture::RawImage2d<u8> = self.buffer.read();

        ImageBuffer::from_fn(image.width, image.height, |x, y| {
            let i = (x * 4 + (image.height - y - 1) * image.width * 4) as usize;
            image.data[i] as u32
                + ((image.data[i + 1] as u32) << 8)
                + ((image.data[i + 2] as u32) << 16)
                + ((image.data[i + 3] as u32) << 24)
        })
    }
}

fn get_size(display: &glium::Display) -> (u32, u32) {
    let size = display
        .gl_window()
        .get_inner_size()
        .unwrap_or(LogicalSize::new(800.0, 600.0))
        .to_physical(display.gl_window().get_hidpi_factor());

    (size.width as u32, size.height as u32)
}
