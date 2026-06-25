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

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_smp: sampler;

struct BlurParams {
    dir: vec4<f32>,
};
@group(0) @binding(2) var<uniform> bp: BlurParams;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let w0 = 0.227027;
    let w1 = 0.194595;
    let w2 = 0.121622;
    let w3 = 0.054054;
    let w4 = 0.016216;

    let d = bp.dir.xy;
    var c = textureSample(src_tex, src_smp, in.uv).rgb * w0;
    c += textureSample(src_tex, src_smp, in.uv + d * 1.0).rgb * w1;
    c += textureSample(src_tex, src_smp, in.uv - d * 1.0).rgb * w1;
    c += textureSample(src_tex, src_smp, in.uv + d * 2.0).rgb * w2;
    c += textureSample(src_tex, src_smp, in.uv - d * 2.0).rgb * w2;
    c += textureSample(src_tex, src_smp, in.uv + d * 3.0).rgb * w3;
    c += textureSample(src_tex, src_smp, in.uv - d * 3.0).rgb * w3;
    c += textureSample(src_tex, src_smp, in.uv + d * 4.0).rgb * w4;
    c += textureSample(src_tex, src_smp, in.uv - d * 4.0).rgb * w4;

    return vec4<f32>(c, 1.0);
}
