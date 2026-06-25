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

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;
@group(0) @binding(2) var bloomSource: texture_2d<f32>;

struct Params {
    p0: vec4<f32>,
    p1: vec4<f32>,
};
@group(0) @binding(3) var<uniform> u: Params;

fn min2(v: vec2<f32>) -> f32 { return min(v.x, v.y); }
fn max2(v: vec2<f32>) -> f32 { return max(v.x, v.y); }
fn rand2(v: vec2<f32>) -> f32 {
    return fract(sin(dot(v, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn distortCoordinates(coords: vec2<f32>, screenCurvature: f32, frameSize: f32) -> vec2<f32> {
    let paddedCoords = coords * (vec2<f32>(1.0) + frameSize * 2.0) - vec2<f32>(frameSize);
    let cc = paddedCoords - vec2<f32>(0.5);
    let dist = dot(cc, cc) * screenCurvature;
    return paddedCoords + cc * (1.0 + dist) * dist;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let screenCurvature = u.p0.x;
    let rgbShift = u.p0.y;
    let frameSize = u.p0.z;
    let screen_brightness = u.p0.w;
    let bloom = u.p1.x;
    let frameShininess = u.p1.y;
    let dither = u.p1.z;
    let frameEnabled = u.p1.w;

    let qt = in.uv;

    let curvatureCoords = distortCoordinates(qt, screenCurvature, frameSize);
    let lo = step(vec2<f32>(0.0), curvatureCoords);
    let hi = step(vec2<f32>(1.0), curvatureCoords);
    let shownDraw = max2(lo - hi);
    let isScreen = min2(lo - hi);
    let isReflection = shownDraw - isScreen;

    let aa = max(fwidth(curvatureCoords), vec2<f32>(1e-6));
    let covLo = smoothstep(vec2<f32>(0.0), aa, curvatureCoords);
    let covHi = vec2<f32>(1.0) - smoothstep(vec2<f32>(1.0) - aa, vec2<f32>(1.0), curvatureCoords);
    let cov = covLo * covHi;
    let screenAA = cov.x * cov.y;

    let mask = mix(screenAA, shownDraw, frameEnabled);
    let txt_coords = curvatureCoords * (-1.0 + 2.0 * lo - 2.0 * hi);

    var txt_color = textureSample(source, smp, txt_coords).rgb;

    let displacement = vec2<f32>(rgbShift, 0.0);
    let rightColor = textureSample(source, smp, txt_coords + displacement).rgb;
    let leftColor = textureSample(source, smp, txt_coords - displacement).rgb;
    txt_color.r = leftColor.r * 0.10 + rightColor.r * 0.30 + txt_color.r * 0.60;
    txt_color.g = leftColor.g * 0.20 + rightColor.g * 0.20 + txt_color.g * 0.60;
    txt_color.b = leftColor.b * 0.30 + rightColor.b * 0.10 + txt_color.b * 0.60;

    var finalColor = txt_color * mask;

    let bloomFullColor = textureSample(bloomSource, smp, txt_coords);
    let bloomColor = bloomFullColor.rgb;
    let bloomAlpha = bloomFullColor.a;

    let bloomOnScreen = bloomColor * screenAA;
    finalColor += clamp(bloomOnScreen * bloom * bloomAlpha, vec3<f32>(0.0), vec3<f32>(0.5));
    finalColor /= 1.0 + max(bloom, 0.0);

    let reflectionColor = mix(bloomColor * bloomAlpha * 2.0, finalColor, frameShininess * 0.5);
    finalColor = mix(finalColor, reflectionColor, isReflection * frameEnabled);

    finalColor *= screen_brightness;

    let noise = rand2(qt) - 0.5;
    finalColor = clamp(finalColor + vec3<f32>(noise * dither), vec3<f32>(0.0), vec3<f32>(1.0));

    return vec4<f32>(finalColor, 1.0);
}
