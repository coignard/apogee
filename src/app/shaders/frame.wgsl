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
    var c = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    let xy = c[vid];
    var out: VsOut;
    out.pos = vec4<f32>(xy, 0.0, 1.0);
    out.uv = vec2<f32>(xy.x * 0.5 + 0.5, 1.0 - (xy.y * 0.5 + 0.5));
    return out;
}

struct Params {
    p0: vec4<f32>,
    p1: vec4<f32>,
    p2: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: Params;

fn min2(v: vec2<f32>) -> f32 { return min(v.x, v.y); }
fn prod2(v: vec2<f32>) -> f32 { return v.x * v.y; }
fn rand2(v: vec2<f32>) -> f32 {
    return fract(sin(dot(v, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn distortCoordinates(coords: vec2<f32>, screenCurvature: f32, frameSize: f32) -> vec2<f32> {
    let paddedCoords = coords * (vec2<f32>(1.0) + frameSize * 2.0) - vec2<f32>(frameSize);
    let cc = paddedCoords - vec2<f32>(0.5);
    let dist = dot(cc, cc) * screenCurvature;
    return paddedCoords + cc * (1.0 + dist) * dist;
}

fn roundedRectSdfPixels(p: vec2<f32>, topLeft: vec2<f32>, bottomRight: vec2<f32>, radiusPixels: f32, viewportSize: vec2<f32>) -> f32 {
    let sizePixels = (bottomRight - topLeft) * viewportSize;
    let centerPixels = (topLeft + bottomRight) * 0.5 * viewportSize;
    let localPixels = p * viewportSize - centerPixels;
    let halfSize = sizePixels * 0.5 - vec2<f32>(radiusPixels);
    let d = abs(localPixels) - halfSize;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0) - radiusPixels;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let screenCurvature = u.p0.x;
    let frameSize = u.p0.y;
    let screenRadius = u.p0.z;
    let ambientLight = u.p0.w;
    let frameColor = u.p1.rgb;
    let frameShininess = u.p1.w;
    let viewportSize = u.p2.xy;

    if (u.p2.z < 0.5) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let staticCoords = in.uv;
    let coords = distortCoordinates(staticCoords, screenCurvature, frameSize);

    let screenRadiusPixels = screenRadius;
    let edgeSoftPixels = 1.0;
    let seamWidth = max(screenRadiusPixels, 0.5) / min2(viewportSize);

    let e = min(
        smoothstep(-seamWidth, seamWidth, coords.x - coords.y),
        smoothstep(-seamWidth, seamWidth, coords.x - (1.0 - coords.y)));
    let s = min(
        smoothstep(-seamWidth, seamWidth, coords.y - coords.x),
        smoothstep(-seamWidth, seamWidth, coords.x - (1.0 - coords.y)));
    let w = min(
        smoothstep(-seamWidth, seamWidth, coords.y - coords.x),
        smoothstep(-seamWidth, seamWidth, (1.0 - coords.x) - coords.y));
    let n = min(
        smoothstep(-seamWidth, seamWidth, coords.x - coords.y),
        smoothstep(-seamWidth, seamWidth, (1.0 - coords.x) - coords.y));

    let distPixels = roundedRectSdfPixels(coords, vec2<f32>(0.0), vec2<f32>(1.0), screenRadiusPixels, viewportSize);
    var frameShadow = e * 0.66 + w * 0.66 + n * 0.33 + s;
    frameShadow *= smoothstep(0.0, edgeSoftPixels * 5.0, distPixels);

    let frameAlpha = 1.0 - frameShininess * 0.4;
    let inScreen = smoothstep(0.0, edgeSoftPixels, -distPixels);
    let alpha = mix(frameAlpha, mix(0.0, 0.3, ambientLight), inScreen);
    let glass = clamp(ambientLight * pow(max(prod2(coords * (vec2<f32>(1.0) - coords.yx)) * 25.0, 0.0), 0.5) * inScreen, 0.0, 1.0);
    var frameTint = frameColor * frameShadow;
    let noise = rand2(staticCoords * viewportSize) - 0.5;
    frameTint = clamp(frameTint + vec3<f32>(noise * 0.04), vec3<f32>(0.0), vec3<f32>(1.0));
    let color = mix(frameTint, vec3<f32>(glass), inScreen);

    return vec4<f32>(color, alpha);
}
