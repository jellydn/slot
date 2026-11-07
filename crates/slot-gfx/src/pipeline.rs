use crate::lcd3x::ScreenEffect;
use crate::power::{screen_brightness, screen_rect_above};
use crate::quad::Quad;
use crate::shaders::{GAME_FRAG, RECT_VERT};
use crate::surface::{GfxError, OUT_H, OUT_W};

pub const SCALE: u32 = 3;
pub const SRC_W: u32 = OUT_W / SCALE;
pub const SRC_H: u32 = OUT_H / SCALE;

pub const WHOLE_TEXTURE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

const FULL: [f32; 4] = [0.0, 0.0, OUT_W as f32, OUT_H as f32];

pub struct GamePass {
    prog: gl::types::GLuint,
    game: gl::types::GLuint,
    size: (u32, u32),
    aspect: Option<f32>,
    u_rect: gl::types::GLint,
    u_bright: gl::types::GLint,
    u_uv: gl::types::GLint,
    u_mode: gl::types::GLint,
    u_paper_size: gl::types::GLint,
    u_fbo: gl::types::GLint,
    fbo_scale: [f32; 2],
    u_origin: gl::types::GLint,
    origin: [f32; 2],
    u_pic: gl::types::GLint,
    pic: [f32; 4],
    paper: gl::types::GLuint,
    paper_size: f32,
    effect: ScreenEffect,
    power: f32,
    fills: bool,
    /// Pixels kept for the carousel footer. The picture scales in the space above them.
    bottom: f32,
    src: [f32; 4],
    layout: [f32; 4],
}

impl GamePass {
    pub fn new() -> Result<Self, GfxError> {
        let prog = crate::shaders::program(RECT_VERT, GAME_FRAG)?;
        let game = crate::gl::texture(SRC_W, SRC_H, gl::NEAREST, gl::CLAMP_TO_EDGE, gl::BGRA, None);
        let paper = crate::gl::texture(
            1,
            1,
            gl::NEAREST,
            gl::REPEAT,
            crate::gl::gray_format(),
            Some(&[255]),
        );
        let (u_rect, u_bright, u_uv, u_mode, u_paper_size, u_pic, u_fbo, u_origin);
        unsafe {
            gl::UseProgram(prog);
            gl::Uniform1i(crate::gl::uniform_location(prog, "u_game"), 0);
            gl::Uniform2f(
                crate::gl::uniform_location(prog, "u_src"),
                SRC_W as f32,
                SRC_H as f32,
            );
            gl::Uniform2f(
                crate::gl::uniform_location(prog, "u_target"),
                OUT_W as f32,
                OUT_H as f32,
            );
            u_rect = crate::gl::uniform_location(prog, "u_rect");
            u_bright = crate::gl::uniform_location(prog, "u_bright");
            u_uv = crate::gl::uniform_location(prog, "u_uv");
            gl::Uniform1i(crate::gl::uniform_location(prog, "u_paper"), 1);
            u_mode = crate::gl::uniform_location(prog, "u_mode");
            u_paper_size = crate::gl::uniform_location(prog, "u_paper_size");
            u_fbo = crate::gl::uniform_location(prog, "u_fbo");
            u_pic = crate::gl::uniform_location(prog, "u_pic");
            u_origin = crate::gl::uniform_location(prog, "u_origin");
        }
        Ok(GamePass {
            prog,
            game,
            size: (SRC_W, SRC_H),
            aspect: None,
            u_rect,
            u_bright,
            u_uv,
            u_mode,
            u_paper_size,
            u_fbo,
            fbo_scale: [1.0, 1.0],
            u_origin,
            origin: [0.0, 0.0],
            u_pic,
            pic: WHOLE_TEXTURE,
            paper,
            paper_size: 1.0,
            effect: ScreenEffect::Lcd3x,
            power: 1.0,
            fills: false,
            bottom: 0.0,
            src: WHOLE_TEXTURE,
            layout: FULL,
        })
    }

    pub fn set_power(&mut self, t: f32) {
        self.power = t.clamp(0.0, 1.0);
    }

    pub fn set_bottom(&mut self, px: f32) {
        self.bottom = px.max(0.0);
    }

    pub fn frame(&self) -> (f32, f32, f32, f32) {
        let foot = self.bottom;
        match self.fills {
            true => (0.0, 0.0, OUT_W as f32, OUT_H as f32 - foot),
            false => screen_rect_above(self.power, foot),
        }
    }

    pub fn native_video(&self) -> bool {
        self.aspect.is_some()
    }

    pub fn set_layout(&mut self, rect: [f32; 4]) {
        self.layout = rect;
    }

    pub fn set_fills_frame(&mut self, on: bool) {
        self.fills = on;
    }

    pub fn set_picture(&mut self, rect: [f32; 4]) {
        self.pic = rect;
    }

    pub fn set_fbo_scale(&mut self, x: f32, y: f32) {
        self.fbo_scale = [x, y];
    }

    pub fn set_origin(&mut self, x: f32, y: f32) {
        self.origin = [x, y];
    }

    pub fn set_effect(&mut self, effect: ScreenEffect) {
        self.effect = effect;
    }

    pub fn set_paper(&mut self, size: u32, gray: &[u8]) {
        let tex = crate::gl::texture(
            size,
            size,
            gl::NEAREST,
            gl::REPEAT,
            crate::gl::gray_format(),
            Some(gray),
        );
        unsafe { gl::DeleteTextures(1, &self.paper) };
        self.paper = tex;
        self.paper_size = size as f32;
    }

    pub fn set_source_rect(&mut self, rect: [f32; 4]) {
        self.src = rect;
    }

    pub fn upload(&mut self, xrgb8888: &[u8]) {
        self.upload_frame(xrgb8888, (SRC_W, SRC_H), None);
    }

    pub fn upload_native(&mut self, xrgb8888: &[u8], size: (u32, u32), aspect: f32) -> bool {
        if !aspect.is_finite() || aspect <= 0.0 {
            return false;
        }
        self.upload_frame(xrgb8888, size, Some(aspect))
    }

    fn upload_frame(&mut self, xrgb8888: &[u8], size: (u32, u32), aspect: Option<f32>) -> bool {
        let (w, h) = size;
        if !(1..=2048).contains(&w)
            || !(1..=2048).contains(&h)
            || xrgb8888.len() < (w * h * 4) as usize
        {
            return false;
        }
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.game);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            let pixels = xrgb8888.as_ptr() as *const std::ffi::c_void;
            if self.size != size {
                let mut max_size = 0;
                gl::GetIntegerv(gl::MAX_TEXTURE_SIZE, &mut max_size);
                if w as i32 > max_size || h as i32 > max_size {
                    return false;
                }
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    crate::gl::internal_format(gl::BGRA),
                    w as i32,
                    h as i32,
                    0,
                    gl::BGRA,
                    gl::UNSIGNED_BYTE,
                    pixels,
                );
            } else {
                gl::TexSubImage2D(
                    gl::TEXTURE_2D,
                    0,
                    0,
                    0,
                    w as i32,
                    h as i32,
                    gl::BGRA,
                    gl::UNSIGNED_BYTE,
                    pixels,
                );
            }
        }
        self.size = size;
        self.aspect = aspect;
        true
    }

    pub fn draw(&self, quad: &Quad) {
        let src = match (self.aspect, self.effect) {
            (Some(_), _) => WHOLE_TEXTURE,
            (_, ScreenEffect::Grid) if self.layout == FULL => {
                whole_scale_source(self.src, self.fbo_scale)
            }
            _ => self.src,
        };
        self.draw_source(self.game, quad, src, self.layout);
    }

    pub fn draw_still(&self, tex: gl::types::GLuint, quad: &Quad) {
        let src = if self.aspect.is_some() {
            WHOLE_TEXTURE
        } else {
            self.src
        };
        self.draw_source(tex, quad, src, self.layout);
    }

    fn draw_source(&self, tex: gl::types::GLuint, quad: &Quad, src: [f32; 4], layout: [f32; 4]) {
        let (mut x, mut y, mut w, mut h) = placed_in(self.frame(), layout);
        if let Some(aspect) = self.aspect {
            let fw = w.min(h * aspect);
            let fh = h.min(w / aspect);
            x += (w - fw) / 2.0;
            y += (h - fh) / 2.0;
            (w, h) = (fw, fh);
        }
        let bright = if self.fills {
            1.0
        } else {
            screen_brightness(self.power)
        };
        // The footer covers the bottom of a full-frame picture. Keep the part that sat above
        // it, so opening the scale does not squash the picture into the shorter rect.
        let mut src = src;
        if self.bottom > 0.0 && self.aspect.is_none() {
            src[3] *= (OUT_H as f32 - self.bottom) / OUT_H as f32;
        }
        let pic = if self.aspect.is_some() {
            WHOLE_TEXTURE
        } else {
            self.pic
        };
        unsafe {
            gl::UseProgram(self.prog);
            gl::Uniform2f(
                crate::gl::uniform_location(self.prog, "u_src"),
                self.size.0 as f32,
                self.size.1 as f32,
            );
            gl::Uniform4f(self.u_rect, x, y, w, h);
            gl::Uniform4f(self.u_uv, src[0], src[1], src[2], src[3]);
            gl::Uniform1f(self.u_bright, bright);
            gl::Uniform1f(self.u_mode, self.effect.mode());
            gl::Uniform1f(self.u_paper_size, self.paper_size);
            gl::Uniform2f(self.u_fbo, self.fbo_scale[0], self.fbo_scale[1]);
            gl::Uniform2f(self.u_origin, self.origin[0], self.origin[1]);
            gl::Uniform4f(self.u_pic, pic[0], pic[1], pic[2], pic[3]);
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, self.paper);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, tex);
        }
        quad.draw();
    }
}

fn placed_in(frame: (f32, f32, f32, f32), layout: [f32; 4]) -> (f32, f32, f32, f32) {
    let (x, y, w, h) = frame;
    let [lx, ly, lw, lh] = layout;
    let (sx, sy) = (lw / OUT_W as f32, lh / OUT_H as f32);
    (lx + x * sx, ly + y * sy, w * sx, h * sy)
}

fn whole_scale_source(src: [f32; 4], fbo: [f32; 2]) -> [f32; 4] {
    let (x, w) = whole_scale(src[0], src[2], SRC_W as f32, OUT_W as f32 * fbo[0]);
    let (y, h) = whole_scale(src[1], src[3], SRC_H as f32, OUT_H as f32 * fbo[1]);
    [x, y, w, h]
}

fn whole_scale(at: f32, len: f32, texels: f32, out: f32) -> (f32, f32) {
    let shown = len * texels;
    let scale = (out / shown).ceil();
    let trim = scale * shown - out;
    if trim >= 0.1 * shown {
        return (at, len);
    }
    let top = (trim / 2.0).floor();
    (at + top / scale / texels, out / scale / texels)
}

impl Drop for GamePass {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.game);
            gl::DeleteTextures(1, &self.paper);
            gl::DeleteProgram(self.prog);
        }
    }
}
