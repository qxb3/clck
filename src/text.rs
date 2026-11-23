//! Minimal GLES3 text renderer: rasterises Iosevka glyphs with `fontdue` into a
//! single-channel atlas and draws them as textured quads.

use std::collections::HashMap;

use fontdue::{Font, FontSettings};
use glow::HasContext;

const ATLAS_SIZE: u32 = 1024;
const PAD: u32 = 2;

const VERT_SRC: &str = r#"#version 300 es
layout (location = 0) in vec2 a_pos;
layout (location = 1) in vec2 a_uv;
layout (location = 2) in vec4 a_color;
layout (location = 3) in vec4 a_clip;
uniform highp vec2 u_res;
out vec2 v_uv;
out vec4 v_color;
out vec4 v_clip;
void main() {
    vec2 ndc = (a_pos / u_res) * 2.0 - 1.0;
    gl_Position = vec4(ndc.x, -ndc.y, 0.0, 1.0);
    v_uv = a_uv;
    v_color = a_color;
    v_clip = a_clip;
}
"#;

const FRAG_SRC: &str = r#"#version 300 es
precision mediump float;
uniform sampler2D u_atlas;
uniform highp vec2 u_res;
in vec2 v_uv;
in vec4 v_color;
in vec4 v_clip;
out vec4 frag_color;
void main() {
    // Clip rectangle is in top-left-origin pixels, like the geometry.
    vec2 px = vec2(gl_FragCoord.x, u_res.y - gl_FragCoord.y);
    if (px.x < v_clip.x || px.y < v_clip.y || px.x > v_clip.z || px.y > v_clip.w) {
        discard;
    }
    float coverage = texture(u_atlas, v_uv).r;
    frag_color = vec4(v_color.rgb, v_color.a * coverage);
}
"#;

struct Glyph {
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    width: f32,
    height: f32,
    /// Horizontal bearing from the pen position to the left edge of the bitmap.
    left: f32,
    /// Distance from the baseline up to the top edge of the bitmap.
    above_baseline: f32,
    advance: f32,
}

/// Shelf packer for the coverage atlas.
struct Packer {
    x: u32,
    y: u32,
    row_height: u32,
}

impl Packer {
    fn alloc(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        if width + PAD > ATLAS_SIZE || height + PAD > ATLAS_SIZE {
            return None;
        }
        if self.x + width + PAD > ATLAS_SIZE {
            self.x = 0;
            self.y += self.row_height + PAD;
            self.row_height = 0;
        }
        if self.y + height + PAD > ATLAS_SIZE {
            return None;
        }
        let pos = (self.x, self.y);
        self.x += width + PAD;
        self.row_height = self.row_height.max(height);
        Some(pos)
    }
}

pub struct TextRenderer {
    font: Font,
    glyphs: HashMap<(char, u32), Glyph>,
    packer: Packer,
    texture: glow::Texture,
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    u_res: glow::UniformLocation,
    u_atlas: glow::UniformLocation,
    vertices: Vec<f32>,
}

impl TextRenderer {
    pub unsafe fn new(gl: &glow::Context, font_bytes: &[u8]) -> Self {
        let font = Font::from_bytes(font_bytes, FontSettings::default())
            .expect("failed to parse embedded Iosevka font");

        let program = link_program(gl);
        let vao = gl.create_vertex_array().expect("vao");
        let vbo = gl.create_buffer().expect("vbo");
        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        let stride = 12 * std::mem::size_of::<f32>() as i32;
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 8);
        gl.enable_vertex_attrib_array(2);
        gl.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, stride, 16);
        gl.enable_vertex_attrib_array(3);
        gl.vertex_attrib_pointer_f32(3, 4, glow::FLOAT, false, stride, 32);
        gl.bind_vertex_array(None);

        let texture = gl.create_texture().expect("atlas texture");
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        let zeros = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE) as usize];
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::R8 as i32,
            ATLAS_SIZE as i32,
            ATLAS_SIZE as i32,
            0,
            glow::RED,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(&zeros)),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );

        let u_res = gl.get_uniform_location(program, "u_res").expect("u_res");
        let u_atlas = gl
            .get_uniform_location(program, "u_atlas")
            .expect("u_atlas");

        Self {
            font,
            glyphs: HashMap::new(),
            packer: Packer {
                x: 0,
                y: 0,
                row_height: 0,
            },
            texture,
            program,
            vao,
            vbo,
            u_res,
            u_atlas,
            vertices: Vec::new(),
        }
    }

    /// Clears the framebuffer and starts a new batch of text.
    pub unsafe fn begin(&mut self, gl: &glow::Context, width: i32, height: i32, bg: [f32; 3]) {
        gl.viewport(0, 0, width, height);
        gl.disable(glow::SCISSOR_TEST);
        gl.clear_color(bg[0], bg[1], bg[2], 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);

        self.vertices.clear();
        gl.use_program(Some(self.program));
        gl.bind_vertex_array(Some(self.vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
        gl.uniform_1_i32(Some(&self.u_atlas), 0);
        gl.uniform_2_f32(Some(&self.u_res), width as f32, height as f32);
        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
    }

    /// Advance width of a single character at `px`.
    pub fn advance(&self, ch: char, px: f32) -> f32 {
        self.font.metrics(ch, px).advance_width
    }

    /// Width a string occupies at `px`, including inter-glyph tracking.
    pub fn measure(&self, text: &str, px: f32, tracking: f32) -> f32 {
        let mut width = 0.0;
        let mut count = 0;
        for ch in text.chars() {
            width += self.font.metrics(ch, px).advance_width + tracking;
            count += 1;
        }
        if count > 0 {
            width -= tracking;
        }
        width
    }

    /// Offset from the baseline to the visual centre of a glyph, e.g. `'0'`.
    pub fn centre_offset(&self, ch: char, px: f32) -> f32 {
        let m = self.font.metrics(ch, px);
        m.ymin as f32 + m.height as f32 * 0.5
    }

    /// Draws a single line with its baseline at `baseline_y`.
    ///
    /// `clip` is `[x0, y0, x1, y1]` in window pixels with a top-left origin.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn draw(
        &mut self,
        gl: &glow::Context,
        text: &str,
        x: f32,
        baseline_y: f32,
        px: f32,
        color: [f32; 4],
        tracking: f32,
        clip: [f32; 4],
    ) -> f32 {
        let mut pen = x;
        for ch in text.chars() {
            pen += self.draw_slot(gl, ch, pen, baseline_y, px, color, clip) + tracking;
        }
        pen - x - if text.is_empty() { 0.0 } else { tracking }
    }

    /// Draws one glyph at `x` and returns its advance width.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn draw_slot(
        &mut self,
        gl: &glow::Context,
        ch: char,
        x: f32,
        baseline_y: f32,
        px: f32,
        color: [f32; 4],
        clip: [f32; 4],
    ) -> f32 {
        let glyph = self.glyph(gl, ch, px);
        if glyph.width > 0.0 && glyph.height > 0.0 {
            let left = x + glyph.left;
            let top = baseline_y - glyph.above_baseline;
            self.push_quad(left, top, &glyph, color, clip);
        }
        glyph.advance
    }

    /// Uploads the batch and draws it.
    pub unsafe fn end(&mut self, gl: &glow::Context) {
        if !self.vertices.is_empty() {
            let bytes: &[u8] = std::slice::from_raw_parts(
                self.vertices.as_ptr() as *const u8,
                self.vertices.len() * std::mem::size_of::<f32>(),
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STREAM_DRAW);
            gl.draw_arrays(glow::TRIANGLES, 0, (self.vertices.len() / 12) as i32);
        }
        gl.bind_vertex_array(None);
    }

    unsafe fn push_quad(
        &mut self,
        left: f32,
        top: f32,
        glyph: &Glyph,
        color: [f32; 4],
        clip: [f32; 4],
    ) {
        let right = left + glyph.width;
        let bottom = top + glyph.height;
        let (r, g, b, a) = (color[0], color[1], color[2], color[3]);
        let (cx0, cy0, cx1, cy1) = (clip[0], clip[1], clip[2], clip[3]);
        let corners = [
            (left, top, glyph.u0, glyph.v0),
            (left, bottom, glyph.u0, glyph.v1),
            (right, top, glyph.u1, glyph.v0),
            (right, top, glyph.u1, glyph.v0),
            (left, bottom, glyph.u0, glyph.v1),
            (right, bottom, glyph.u1, glyph.v1),
        ];
        for (px, py, u, v) in corners {
            self.vertices
                .extend_from_slice(&[px, py, u, v, r, g, b, a, cx0, cy0, cx1, cy1]);
        }
    }

    unsafe fn glyph(&mut self, gl: &glow::Context, ch: char, px: f32) -> Glyph {
        let key = (ch, px.round().max(1.0) as u32);
        if let Some(glyph) = self.glyphs.get(&key) {
            return Glyph {
                u0: glyph.u0,
                v0: glyph.v0,
                u1: glyph.u1,
                v1: glyph.v1,
                width: glyph.width,
                height: glyph.height,
                left: glyph.left,
                above_baseline: glyph.above_baseline,
                advance: glyph.advance,
            };
        }

        let (metrics, bitmap) = self.font.rasterize(ch, key.1 as f32);
        let (w, h) = (metrics.width as u32, metrics.height as u32);
        let mut u0: f32 = 0.0;
        let mut v0: f32 = 0.0;
        let mut u1: f32 = 0.0;
        let mut v1: f32 = 0.0;

        let mut placed = false;
        if w > 0 && h > 0 {
            if let Some((ax, ay)) = self.packer.alloc(w, h) {
                placed = true;
                gl.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    ax as i32,
                    ay as i32,
                    w as i32,
                    h as i32,
                    glow::RED,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(&bitmap)),
                );
                let size = ATLAS_SIZE as f32;
                u0 = ax as f32 / size;
                v0 = ay as f32 / size;
                u1 = (ax + w) as f32 / size;
                v1 = (ay + h) as f32 / size;
            } else {
                log("glyph atlas full; glyph dropped");
            }
        }

        let glyph = Glyph {
            u0,
            v0,
            u1,
            v1,
            width: if placed { w as f32 } else { 0.0 },
            height: if placed { h as f32 } else { 0.0 },
            left: metrics.xmin as f32,
            above_baseline: (metrics.ymin + metrics.height as i32) as f32,
            advance: metrics.advance_width,
        };
        self.glyphs.insert(key, glyph);
        let stored = self.glyphs.get(&key).unwrap();
        Glyph {
            u0: stored.u0,
            v0: stored.v0,
            u1: stored.u1,
            v1: stored.v1,
            width: stored.width,
            height: stored.height,
            left: stored.left,
            above_baseline: stored.above_baseline,
            advance: stored.advance,
        }
    }
}

unsafe fn link_program(gl: &glow::Context) -> glow::Program {
    let vs = gl
        .create_shader(glow::VERTEX_SHADER)
        .expect("vertex shader");
    gl.shader_source(vs, VERT_SRC);
    gl.compile_shader(vs);
    if !gl.get_shader_compile_status(vs) {
        panic!("vertex shader: {}", gl.get_shader_info_log(vs));
    }

    let fs = gl
        .create_shader(glow::FRAGMENT_SHADER)
        .expect("fragment shader");
    gl.shader_source(fs, FRAG_SRC);
    gl.compile_shader(fs);
    if !gl.get_shader_compile_status(fs) {
        panic!("fragment shader: {}", gl.get_shader_info_log(fs));
    }

    let program = gl.create_program().expect("program");
    gl.attach_shader(program, vs);
    gl.attach_shader(program, fs);
    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        panic!("program link: {}", gl.get_program_info_log(program));
    }
    gl.detach_shader(program, vs);
    gl.detach_shader(program, fs);
    gl.delete_shader(vs);
    gl.delete_shader(fs);
    program
}

fn log(msg: &str) {
    crate::android_log(msg);
}
