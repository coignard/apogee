// This file is part of Apogee.
//
// Copyright (c) 2026  René Coignard <contact@renecoignard.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::error::Error;
use std::time::Instant;

use pixels::PixelsContext;
use pixels::wgpu;
use serde::Deserialize;

type DynError = Box<dyn Error + Send + Sync>;

const NOISE_DIM: u32 = 512;
const BLOOM_DIV: u32 = 3;
const BLOOM_SPREAD: f32 = 1.5;
const DITHER: f32 = 0.025;
const VEC4_BYTES: u64 = 16;
const DYN_BYTES: u64 = 32 * 4;
const STATIC_BYTES: u64 = 8 * 4;
const FRAME_BYTES: u64 = 3 * 16;
const UP_BYTES: u64 = 16;
const INTER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preset {
    pub ambient_light: f32,
    pub background_color: String,
    pub bloom: f32,
    pub brightness: f32,
    pub burn_in: f32,
    pub chroma_color: f32,
    pub contrast: f32,
    pub flickering: f32,
    pub foreground_color: String,
    pub frame_color: String,
    pub frame_enabled: bool,
    pub frame_shininess: f32,
    pub frame_size: f32,
    pub glowing_line: f32,
    pub horizontal_sync: f32,
    pub jitter: f32,
    pub preserve_color: bool,
    pub rasterization: u32,
    pub rgb_shift: f32,
    pub saturation_color: f32,
    pub screen_curvature: f32,
    pub screen_radius: f32,
    pub static_noise: f32,
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            ambient_light: 0.2,
            background_color: "#000000".to_string(),
            bloom: 0.55,
            brightness: 0.5,
            burn_in: 0.25,
            chroma_color: 0.25,
            contrast: 0.8,
            flickering: 0.1,
            foreground_color: "#ffffff".to_string(),
            frame_color: "#cfcfcf".to_string(),
            frame_enabled: true,
            frame_shininess: 0.0,
            frame_size: 0.0,
            glowing_line: 0.0,
            horizontal_sync: 0.08,
            jitter: 0.2,
            preserve_color: false,
            rasterization: 1,
            rgb_shift: 0.0,
            saturation_color: 0.0,
            screen_curvature: 0.3,
            screen_radius: 0.1,
            static_noise: 0.1,
        }
    }
}

impl Preset {
    pub fn default_preset() -> Self {
        Self::from_json(include_str!("../../shaders/apogee.json"))
            .expect("compiled-in default preset is valid JSON")
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    fn foreground_rgb(&self) -> [f32; 3] {
        let fg = hex_rgb(&self.foreground_color);
        let bg = hex_rgb(&self.background_color);
        let saturated = mix3(fg, [1.0, 1.0, 1.0], self.saturation_color * 0.5);
        mix3(bg, saturated, 0.7 + self.contrast * 0.3)
    }

    fn background_rgb(&self) -> [f32; 3] {
        let fg = hex_rgb(&self.foreground_color);
        let bg = hex_rgb(&self.background_color);
        let saturated = mix3(fg, [1.0, 1.0, 1.0], self.saturation_color * 0.5);
        mix3(saturated, bg, 0.7 + self.contrast * 0.3)
    }

    fn frame_rgb(&self) -> [f32; 3] {
        let fc = hex_rgb(&self.frame_color);
        let fg = hex_rgb(&self.foreground_color);
        let bg = hex_rgb(&self.background_color);
        let static_frame = [
            (fc[0] + 0.1).min(1.0),
            (fc[1] + 0.1).min(1.0),
            (fc[2] + 0.1).min(1.0),
        ];
        let light = mix3(fg, bg, 0.2);
        let scaled_light = [light[0] * 0.2, light[1] * 0.2, light[2] * 0.2];
        mix3(
            scaled_light,
            static_frame,
            0.125 + 0.750 * self.ambient_light,
        )
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    (1.0 - t) * a + t * b
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn hex_rgb(s: &str) -> [f32; 3] {
    let t = s.trim().trim_start_matches('#');
    let ch = |i: usize| {
        u8::from_str_radix(t.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0) as f32 / 256.0
    };
    if t.len() >= 6 {
        [ch(0), ch(2), ch(4)]
    } else {
        [0.0, 0.0, 0.0]
    }
}

fn bytes(values: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 4);
    for v in values {
        out.extend_from_slice(&v.to_ne_bytes());
    }
    out
}

fn load_noise() -> Vec<u8> {
    let bytes = include_bytes!("shaders/allNoise512.png");
    match image::load_from_memory(bytes) {
        Ok(img) => img.to_rgba8().into_raw(),
        Err(_) => vec![128u8; (NOISE_DIM * NOISE_DIM * 4) as usize],
    }
}

fn sample_noise(data: &[u8], u: f32, v: f32) -> [f32; 4] {
    let dim = NOISE_DIM as f32;
    let fx = (u * dim - 0.5).rem_euclid(dim);
    let fy = (v * dim - 0.5).rem_euclid(dim);
    let x0 = fx.floor();
    let y0 = fy.floor();
    let tx = fx - x0;
    let ty = fy - y0;
    let xi0 = (x0 as i64).rem_euclid(NOISE_DIM as i64) as u32;
    let yi0 = (y0 as i64).rem_euclid(NOISE_DIM as i64) as u32;
    let xi1 = (xi0 + 1) % NOISE_DIM;
    let yi1 = (yi0 + 1) % NOISE_DIM;
    let at = |x: u32, y: u32, c: usize| data[((y * NOISE_DIM + x) * 4) as usize + c] as f32 / 255.0;
    let mut out = [0.0f32; 4];
    for (c, o) in out.iter_mut().enumerate() {
        let top = at(xi0, yi0, c) * (1.0 - tx) + at(xi1, yi0, c) * tx;
        let bot = at(xi0, yi1, c) * (1.0 - tx) + at(xi1, yi1, c) * tx;
        *o = top * (1.0 - ty) + bot * ty;
    }
    out
}

struct Gpu {
    upscale_pipeline: wgpu::RenderPipeline,
    blur_pipeline: wgpu::RenderPipeline,
    burnin_pipeline: wgpu::RenderPipeline,
    static_pipeline: wgpu::RenderPipeline,
    frame_pipeline: wgpu::RenderPipeline,
    dynamic_pipeline: wgpu::RenderPipeline,

    upscale_bgl: wgpu::BindGroupLayout,
    blur_bgl: wgpu::BindGroupLayout,
    burnin_bgl: wgpu::BindGroupLayout,
    static_bgl: wgpu::BindGroupLayout,
    frame_bgl: wgpu::BindGroupLayout,
    dynamic_bgl: wgpu::BindGroupLayout,

    lin_smp: wgpu::Sampler,
    noise_smp: wgpu::Sampler,

    up_buf: wgpu::Buffer,
    blur_h_buf: wgpu::Buffer,
    blur_v_buf: wgpu::Buffer,
    burn_buf: wgpu::Buffer,
    static_buf: wgpu::Buffer,
    frame_buf: wgpu::Buffer,
    dyn_buf: wgpu::Buffer,

    noise_texture: wgpu::Texture,
    noise_view: wgpu::TextureView,

    screen_view: wgpu::TextureView,
    bloom_a_view: wgpu::TextureView,
    bloom_b_view: wgpu::TextureView,
    burn_views: [wgpu::TextureView; 2],
    static_view: wgpu::TextureView,
    frame_view: wgpu::TextureView,

    upscale_bg: wgpu::BindGroup,
    blur_h_bg: wgpu::BindGroup,
    blur_v_bg: wgpu::BindGroup,
    burnin_bg: [wgpu::BindGroup; 2],
    static_bg: wgpu::BindGroup,
    frame_bg: wgpu::BindGroup,
    dynamic_bg: [wgpu::BindGroup; 2],

    surface_size: (u32, u32),
    content_size: (u32, u32),
    fb_size: (u32, u32),
    burn_cleared: bool,
    noise_uploaded: bool,
}

pub struct ShaderRenderer {
    preset: Preset,
    foreground_color: [f32; 3],
    bg_color: [f32; 3],
    frame_color: [f32; 3],
    format: Option<wgpu::TextureFormat>,
    surface_size: (u32, u32),
    fb_size: (u32, u32),
    gpu: Option<Gpu>,
    noise: Vec<u8>,
    start: Instant,
    last_time: f32,
    burn_write: usize,
}

impl ShaderRenderer {
    pub fn new(fb_width: u32, fb_height: u32, preset: Preset) -> Self {
        let foreground_color = preset.foreground_rgb();
        let bg_color = preset.background_rgb();
        let frame_color = preset.frame_rgb();
        Self {
            preset,
            foreground_color,
            bg_color,
            frame_color,
            format: None,
            surface_size: (0, 0),
            fb_size: (fb_width.max(1), fb_height.max(1)),
            gpu: None,
            noise: load_noise(),
            start: Instant::now(),
            last_time: 0.0,
            burn_write: 0,
        }
    }

    pub fn set_format(&mut self, format: wgpu::TextureFormat) {
        self.format = Some(format);
    }

    pub fn set_surface_size(&mut self, width: u32, height: u32) {
        self.surface_size = (width.max(1), height.max(1));
    }

    pub fn set_virtual_resolution(&mut self, fb_width: u32, fb_height: u32) {
        self.fb_size = (fb_width.max(1), fb_height.max(1));
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        render_target: &wgpu::TextureView,
        context: &PixelsContext,
    ) -> Result<(), DynError> {
        self.ensure(context)?;

        let (sw, sh) = self.surface_size;
        let (fbw, fbh) = self.fb_size;
        let (cw, ch) = content_size((sw, sh), (fbw, fbh));
        let (cwf, chf) = (cw as f32, ch as f32);
        let off_x = ((sw - cw) / 2) as f32;
        let off_y = ((sh - ch) / 2) as f32;

        let time = self.start.elapsed().as_secs_f32();
        let dt = (time - self.last_time).max(0.0);
        self.last_time = time;

        let write = self.burn_write;
        let read = 1 - write;

        let gpu = self.gpu.as_ref().expect("gpu built in ensure()");
        let p = &self.preset;

        let nws = 1024.0 / (0.5 * cwf + 0.5 * chf);
        let curvature = p.screen_curvature * 0.6 * nws;
        let frame_size = if p.frame_enabled {
            p.frame_size * nws
        } else {
            0.0
        };
        let bloom = p.bloom * 2.5;
        let glowing_line = p.glowing_line * 0.2;
        let hsync_strength = lerp(0.05, 0.35, p.horizontal_sync);
        let screen_brightness = lerp(0.5, 1.5, p.brightness);
        let rgb_shift = p.rgb_shift * (4.0 / cwf);
        let density = (cwf / fbw as f32).min(chf / fbh as f32);
        let raster_intensity = smoothstep(1.0, 2.0, density);
        let burn_in_time = 1.0 / lerp(0.16, 1.6, p.burn_in);
        let scale_noise = [cwf * 0.75 / NOISE_DIM as f32, chf * 0.75 / NOISE_DIM as f32];

        let nx = (time / 2.048).fract().rem_euclid(1.0);
        let ny = (time / 1_048.576).fract().rem_euclid(1.0);
        let n = sample_noise(&self.noise, nx, ny);
        let v_brightness = 1.0 + (n[1] - 0.5) * p.flickering;
        let randval = hsync_strength - n[0];
        let v_dist_scale = step(randval) * randval * hsync_strength * p.horizontal_sync;
        let v_dist_freq = lerp(4.0, 40.0, n[1]) * step(p.horizontal_sync);

        let frame_on = if p.frame_enabled { 1.0 } else { 0.0 };

        let static_params = [
            curvature,
            rgb_shift,
            frame_size,
            screen_brightness,
            bloom,
            p.frame_shininess,
            DITHER,
            frame_on,
        ];
        context
            .queue
            .write_buffer(&gpu.static_buf, 0, &bytes(&static_params));

        let dyn_params = [
            fbw as f32,
            fbh as f32,
            time,
            curvature,
            raster_intensity,
            burn_in_time,
            p.burn_in,
            p.static_noise,
            glowing_line,
            p.chroma_color,
            0.007 * p.jitter,
            0.002 * p.jitter,
            p.horizontal_sync,
            p.flickering,
            scale_noise[0],
            scale_noise[1],
            bloom,
            v_brightness,
            v_dist_scale,
            v_dist_freq,
            self.foreground_color[0],
            self.foreground_color[1],
            self.foreground_color[2],
            p.rasterization as f32,
            self.bg_color[0],
            self.bg_color[1],
            self.bg_color[2],
            frame_size,
            time,
            p.jitter,
            0.0,
            0.0,
        ];
        context
            .queue
            .write_buffer(&gpu.dyn_buf, 0, &bytes(&dyn_params));

        let decay = dt * burn_in_time;
        context
            .queue
            .write_buffer(&gpu.burn_buf, 0, &bytes(&[decay, 0.0, 0.0, 0.0]));

        let desaturate = if p.preserve_color { 0.0 } else { 1.0 };
        context
            .queue
            .write_buffer(&gpu.up_buf, 0, &bytes(&[desaturate, 0.0, 0.0, 0.0]));

        let frame_params = [
            curvature,
            frame_size,
            p.screen_radius,
            p.ambient_light,
            self.frame_color[0],
            self.frame_color[1],
            self.frame_color[2],
            p.frame_shininess,
            cwf,
            chf,
            frame_on,
            0.0,
        ];
        context
            .queue
            .write_buffer(&gpu.frame_buf, 0, &bytes(&frame_params));

        pass(
            encoder,
            "apogee_shader_upscale",
            &gpu.screen_view,
            &gpu.upscale_pipeline,
            &gpu.upscale_bg,
        );
        pass(
            encoder,
            "apogee_shader_bloom_h",
            &gpu.bloom_a_view,
            &gpu.blur_pipeline,
            &gpu.blur_h_bg,
        );
        pass(
            encoder,
            "apogee_shader_bloom_v",
            &gpu.bloom_b_view,
            &gpu.blur_pipeline,
            &gpu.blur_v_bg,
        );
        if p.burn_in > 0.0 {
            pass(
                encoder,
                "apogee_shader_burnin",
                &gpu.burn_views[write],
                &gpu.burnin_pipeline,
                &gpu.burnin_bg[read],
            );
        }
        pass(
            encoder,
            "apogee_shader_static",
            &gpu.static_view,
            &gpu.static_pipeline,
            &gpu.static_bg,
        );
        pass(
            encoder,
            "apogee_shader_frame",
            &gpu.frame_view,
            &gpu.frame_pipeline,
            &gpu.frame_bg,
        );
        pass_viewport(
            encoder,
            "apogee_shader_dynamic",
            render_target,
            &gpu.dynamic_pipeline,
            &gpu.dynamic_bg[write],
            (off_x, off_y, cwf, chf),
        );

        self.burn_write = read;
        Ok(())
    }

    fn ensure(&mut self, context: &PixelsContext) -> Result<(), DynError> {
        let format = self
            .format
            .ok_or("ShaderRenderer::set_format must be called before rendering")?;
        let device = &context.device;

        let fresh = self.gpu.is_none();
        if fresh {
            self.gpu = Some(build_pipelines(device, format));
        }

        let (sw, sh) = self.surface_size;
        let fb = self.fb_size;
        let content = content_size((sw, sh), fb);
        let (size_dirty, fb_dirty) = {
            let gpu = self.gpu.as_ref().unwrap();
            (
                fresh || gpu.surface_size != (sw, sh) || gpu.content_size != content,
                fresh || gpu.fb_size != fb,
            )
        };

        if size_dirty {
            self.rebuild_targets(context, content);
        }
        if size_dirty || fb_dirty {
            self.rebuild_bind_groups(context, fb);
        }

        let noise = &self.noise;
        let gpu = self.gpu.as_mut().unwrap();
        if !gpu.noise_uploaded {
            write_texture(
                &context.queue,
                &gpu.noise_texture,
                noise,
                NOISE_DIM,
                NOISE_DIM,
            );
            gpu.noise_uploaded = true;
        }
        Ok(())
    }

    fn rebuild_targets(&mut self, context: &PixelsContext, content: (u32, u32)) {
        let device = &context.device;
        let gpu = self.gpu.as_mut().unwrap();

        let (cw, ch) = content;
        let bw = (cw / BLOOM_DIV).max(1);
        let bh = (ch / BLOOM_DIV).max(1);

        gpu.screen_view = target(device, INTER_FORMAT, cw, ch, "apogee_shader_screen");
        gpu.bloom_a_view = target(device, INTER_FORMAT, bw, bh, "apogee_shader_bloom_a");
        gpu.bloom_b_view = target(device, INTER_FORMAT, bw, bh, "apogee_shader_bloom_b");
        gpu.burn_views = [
            target(device, INTER_FORMAT, cw, ch, "apogee_shader_burn0"),
            target(device, INTER_FORMAT, cw, ch, "apogee_shader_burn1"),
        ];
        gpu.static_view = target(device, INTER_FORMAT, cw, ch, "apogee_shader_static");
        gpu.frame_view = target(device, INTER_FORMAT, cw, ch, "apogee_shader_frame");
        gpu.surface_size = self.surface_size;
        gpu.content_size = content;
        gpu.burn_cleared = false;

        context.queue.write_buffer(
            &gpu.blur_h_buf,
            0,
            &bytes(&[BLOOM_SPREAD / bw as f32, 0.0, 0.0, 0.0]),
        );
        context.queue.write_buffer(
            &gpu.blur_v_buf,
            0,
            &bytes(&[0.0, BLOOM_SPREAD / bh as f32, 0.0, 0.0]),
        );
    }

    fn rebuild_bind_groups(&mut self, context: &PixelsContext, fb: (u32, u32)) {
        let device = &context.device;
        let gpu = self.gpu.as_mut().unwrap();
        let fb_view = context
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        gpu.upscale_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("apogee_shader_upscale_bg"),
            layout: &gpu.upscale_bgl,
            entries: &[tex(0, &fb_view), smp(1, &gpu.lin_smp), buf(2, &gpu.up_buf)],
        });
        gpu.blur_h_bg = blur_bg(
            device,
            &gpu.blur_bgl,
            &gpu.screen_view,
            &gpu.lin_smp,
            &gpu.blur_h_buf,
        );
        gpu.blur_v_bg = blur_bg(
            device,
            &gpu.blur_bgl,
            &gpu.bloom_a_view,
            &gpu.lin_smp,
            &gpu.blur_v_buf,
        );
        gpu.static_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("apogee_shader_static_bg"),
            layout: &gpu.static_bgl,
            entries: &[
                tex(0, &gpu.screen_view),
                smp(1, &gpu.lin_smp),
                tex(2, &gpu.bloom_b_view),
                buf(3, &gpu.static_buf),
            ],
        });
        gpu.frame_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("apogee_shader_frame_bg"),
            layout: &gpu.frame_bgl,
            entries: &[buf(0, &gpu.frame_buf)],
        });
        for i in 0..2 {
            gpu.burnin_bg[i] = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("apogee_shader_burnin_bg"),
                layout: &gpu.burnin_bgl,
                entries: &[
                    tex(0, &gpu.screen_view),
                    smp(1, &gpu.lin_smp),
                    tex(2, &gpu.burn_views[i]),
                    buf(3, &gpu.burn_buf),
                ],
            });
            gpu.dynamic_bg[i] = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("apogee_shader_dynamic_bg"),
                layout: &gpu.dynamic_bgl,
                entries: &[
                    tex(0, &gpu.static_view),
                    smp(1, &gpu.lin_smp),
                    tex(2, &gpu.burn_views[i]),
                    tex(3, &gpu.noise_view),
                    smp(4, &gpu.noise_smp),
                    tex(5, &gpu.frame_view),
                    buf(6, &gpu.dyn_buf),
                ],
            });
        }
        gpu.fb_size = fb;

        if !gpu.burn_cleared {
            let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("apogee_shader_clear_burn"),
            });
            for view in &gpu.burn_views {
                enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("apogee_shader_clear_burn_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
            }
            context.queue.submit(Some(enc.finish()));
            gpu.burn_cleared = true;
        }
    }
}

fn step(x: f32) -> f32 {
    if x >= 0.0 { 1.0 } else { 0.0 }
}

fn content_size(surface: (u32, u32), fb: (u32, u32)) -> (u32, u32) {
    let (sw, sh) = (surface.0.max(1), surface.1.max(1));
    let (fw, fh) = (fb.0.max(1), fb.1.max(1));
    let fb_aspect = fw as f32 / fh as f32;
    let surf_aspect = sw as f32 / sh as f32;
    if surf_aspect > fb_aspect {
        (((sh as f32 * fb_aspect).round() as u32).max(1), sh)
    } else {
        (sw, ((sw as f32 / fb_aspect).round() as u32).max(1))
    }
}

fn pass(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
) {
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    rpass.set_pipeline(pipeline);
    rpass.set_bind_group(0, bind_group, &[]);
    rpass.draw(0..3, 0..1);
}

#[allow(clippy::too_many_arguments)]
fn pass_viewport(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
    viewport: (f32, f32, f32, f32),
) {
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    rpass.set_viewport(viewport.0, viewport.1, viewport.2, viewport.3, 0.0, 1.0);
    rpass.set_pipeline(pipeline);
    rpass.set_bind_group(0, bind_group, &[]);
    rpass.draw(0..3, 0..1);
}

fn target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    w: u32,
    h: u32,
    label: &str,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn write_texture(queue: &wgpu::Queue, texture: &wgpu::Texture, data: &[u8], w: u32, h: u32) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w * 4),
            rows_per_image: Some(h),
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
}

fn tex(binding: u32, view: &wgpu::TextureView) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::TextureView(view),
    }
}
fn smp(binding: u32, sampler: &wgpu::Sampler) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::Sampler(sampler),
    }
}
fn buf(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

fn blur_bg(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("apogee_shader_blur_bg"),
        layout,
        entries: &[tex(0, view), smp(1, sampler), buf(2, uniform)],
    })
}

fn tex_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            multisampled: false,
            view_dimension: wgpu::TextureViewDimension::D2,
        },
        count: None,
    }
}
fn smp_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}
fn buf_entry(binding: u32, size: u64) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(size),
        },
        count: None,
    }
}

fn make_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    module: &wgpu::ShaderModule,
    layout: &wgpu::BindGroupLayout,
    label: &str,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent::REPLACE,
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn make_uniform(device: &wgpu::Device, label: &str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn data_texture(device: &wgpu::Device, w: u32, h: u32, label: &str) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn build_pipelines(device: &wgpu::Device, format: wgpu::TextureFormat) -> Gpu {
    let upscale_mod = device.create_shader_module(wgpu::include_wgsl!("shaders/upscale.wgsl"));
    let blur_mod = device.create_shader_module(wgpu::include_wgsl!("shaders/blur.wgsl"));
    let burnin_mod = device.create_shader_module(wgpu::include_wgsl!("shaders/burnin.wgsl"));
    let static_mod = device.create_shader_module(wgpu::include_wgsl!("shaders/static.wgsl"));
    let frame_mod = device.create_shader_module(wgpu::include_wgsl!("shaders/frame.wgsl"));
    let dynamic_mod = device.create_shader_module(wgpu::include_wgsl!("shaders/dynamic.wgsl"));

    let upscale_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("apogee_shader_upscale_bgl"),
        entries: &[tex_entry(0), smp_entry(1), buf_entry(2, UP_BYTES)],
    });
    let blur_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("apogee_shader_blur_bgl"),
        entries: &[tex_entry(0), smp_entry(1), buf_entry(2, VEC4_BYTES)],
    });
    let burnin_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("apogee_shader_burnin_bgl"),
        entries: &[
            tex_entry(0),
            smp_entry(1),
            tex_entry(2),
            buf_entry(3, VEC4_BYTES),
        ],
    });
    let static_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("apogee_shader_static_bgl"),
        entries: &[
            tex_entry(0),
            smp_entry(1),
            tex_entry(2),
            buf_entry(3, STATIC_BYTES),
        ],
    });
    let frame_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("apogee_shader_frame_bgl"),
        entries: &[buf_entry(0, FRAME_BYTES)],
    });
    let dynamic_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("apogee_shader_dynamic_bgl"),
        entries: &[
            tex_entry(0),
            smp_entry(1),
            tex_entry(2),
            tex_entry(3),
            smp_entry(4),
            tex_entry(5),
            buf_entry(6, DYN_BYTES),
        ],
    });

    let upscale_pipeline = make_pipeline(
        device,
        INTER_FORMAT,
        &upscale_mod,
        &upscale_bgl,
        "apogee_shader_upscale",
    );
    let blur_pipeline = make_pipeline(
        device,
        INTER_FORMAT,
        &blur_mod,
        &blur_bgl,
        "apogee_shader_blur",
    );
    let burnin_pipeline = make_pipeline(
        device,
        INTER_FORMAT,
        &burnin_mod,
        &burnin_bgl,
        "apogee_shader_burnin",
    );
    let static_pipeline = make_pipeline(
        device,
        INTER_FORMAT,
        &static_mod,
        &static_bgl,
        "apogee_shader_static",
    );
    let frame_pipeline = make_pipeline(
        device,
        INTER_FORMAT,
        &frame_mod,
        &frame_bgl,
        "apogee_shader_frame",
    );
    let dynamic_pipeline = make_pipeline(
        device,
        format,
        &dynamic_mod,
        &dynamic_bgl,
        "apogee_shader_dynamic",
    );

    let lin_smp = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("apogee_shader_linear"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: 1.0,
        compare: None,
        anisotropy_clamp: 1,
        border_color: None,
    });
    let noise_smp = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("apogee_shader_noise"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: 1.0,
        compare: None,
        anisotropy_clamp: 1,
        border_color: None,
    });

    let up_buf = make_uniform(device, "apogee_shader_up", UP_BYTES);
    let blur_h_buf = make_uniform(device, "apogee_shader_blur_h", VEC4_BYTES);
    let blur_v_buf = make_uniform(device, "apogee_shader_blur_v", VEC4_BYTES);
    let burn_buf = make_uniform(device, "apogee_shader_burn", VEC4_BYTES);
    let static_buf = make_uniform(device, "apogee_shader_static", STATIC_BYTES);
    let frame_buf = make_uniform(device, "apogee_shader_frame", FRAME_BYTES);
    let dyn_buf = make_uniform(device, "apogee_shader_dyn", DYN_BYTES);

    let noise_texture = data_texture(device, NOISE_DIM, NOISE_DIM, "apogee_shader_noise_tex");
    let noise_view = noise_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let ph = || target(device, format, 1, 1, "apogee_shader_placeholder");
    let upscale_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ph_upscale"),
        layout: &upscale_bgl,
        entries: &[tex(0, &ph()), smp(1, &lin_smp), buf(2, &up_buf)],
    });
    let blur_h_bg = blur_bg(device, &blur_bgl, &ph(), &lin_smp, &blur_h_buf);
    let blur_v_bg = blur_bg(device, &blur_bgl, &ph(), &lin_smp, &blur_v_buf);
    let static_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ph_static"),
        layout: &static_bgl,
        entries: &[
            tex(0, &ph()),
            smp(1, &lin_smp),
            tex(2, &ph()),
            buf(3, &static_buf),
        ],
    });
    let frame_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ph_frame"),
        layout: &frame_bgl,
        entries: &[buf(0, &frame_buf)],
    });
    let burnin_ph = || {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ph_burnin"),
            layout: &burnin_bgl,
            entries: &[
                tex(0, &ph()),
                smp(1, &lin_smp),
                tex(2, &ph()),
                buf(3, &burn_buf),
            ],
        })
    };
    let dynamic_ph = || {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ph_dynamic"),
            layout: &dynamic_bgl,
            entries: &[
                tex(0, &ph()),
                smp(1, &lin_smp),
                tex(2, &ph()),
                tex(3, &noise_view),
                smp(4, &noise_smp),
                tex(5, &ph()),
                buf(6, &dyn_buf),
            ],
        })
    };
    let burnin_bg = [burnin_ph(), burnin_ph()];
    let dynamic_bg = [dynamic_ph(), dynamic_ph()];

    let screen_view = ph();
    let bloom_a_view = ph();
    let bloom_b_view = ph();
    let burn_views = [ph(), ph()];
    let static_view = ph();
    let frame_view = ph();

    Gpu {
        upscale_pipeline,
        blur_pipeline,
        burnin_pipeline,
        static_pipeline,
        frame_pipeline,
        dynamic_pipeline,
        upscale_bgl,
        blur_bgl,
        burnin_bgl,
        static_bgl,
        frame_bgl,
        dynamic_bgl,
        lin_smp,
        noise_smp,
        up_buf,
        blur_h_buf,
        blur_v_buf,
        burn_buf,
        static_buf,
        frame_buf,
        dyn_buf,
        noise_texture,
        noise_view,
        screen_view,
        bloom_a_view,
        bloom_b_view,
        burn_views,
        static_view,
        frame_view,
        upscale_bg,
        blur_h_bg,
        blur_v_bg,
        burnin_bg,
        static_bg,
        frame_bg,
        dynamic_bg,
        surface_size: (0, 0),
        content_size: (0, 0),
        fb_size: (0, 0),
        burn_cleared: false,
        noise_uploaded: false,
    }
}
