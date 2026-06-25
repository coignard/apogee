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

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VsOut {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    let xy = corners[vid];
    var out: VsOut;
    out.pos = vec4<f32>(xy, 0.0, 1.0);
    out.uv = vec2<f32>(xy.x * 0.5 + 0.5, 1.0 - (xy.y * 0.5 + 0.5));
    return out;
}

@group(0) @binding(0) var screen_tex: texture_2d<f32>;
@group(0) @binding(1) var screen_smp: sampler;
@group(0) @binding(2) var burn_tex: texture_2d<f32>;

struct BurnParams {
    decay: vec4<f32>,
};
@group(0) @binding(3) var<uniform> bp: BurnParams;

fn grey(v: vec3<f32>) -> f32 {
    return dot(v, vec3<f32>(0.21, 0.72, 0.04));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let txt = textureSample(screen_tex, screen_smp, in.uv).rgb;
    let acc = textureSample(burn_tex, screen_smp, in.uv);

    let prev_mask = acc.a;
    var blur_decay = clamp(bp.decay.x, 0.0, 1.0);
    blur_decay = max(0.0, blur_decay - prev_mask);

    let color = max(acc.rgb - vec3<f32>(blur_decay), txt);
    let curr_mask = step(grey(color), grey(txt));

    return vec4<f32>(color, curr_mask);
}
