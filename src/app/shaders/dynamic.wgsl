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

@group(0) @binding(0) var screenBuffer: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;
@group(0) @binding(2) var burnInSource: texture_2d<f32>;
@group(0) @binding(3) var noiseSource: texture_2d<f32>;
@group(0) @binding(4) var noiseSmp: sampler;
@group(0) @binding(5) var frameSource: texture_2d<f32>;

struct Params {
    p0: vec4<f32>,
    p1: vec4<f32>,
    p2: vec4<f32>,
    p3: vec4<f32>,
    p4: vec4<f32>,
    p5: vec4<f32>,
    p6: vec4<f32>,
    p7: vec4<f32>,
};
@group(0) @binding(6) var<uniform> u: Params;

fn rgb2grey(v: vec3<f32>) -> f32 { return dot(v, vec3<f32>(0.21, 0.72, 0.04)); }

fn srgb_to_lin(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((max(c, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

fn distortCoordinates(coords: vec2<f32>, screenCurvature: f32, frameSize: f32) -> vec2<f32> {
    let paddedCoords = coords * (vec2<f32>(1.0) + frameSize * 2.0) - vec2<f32>(frameSize);
    let cc = paddedCoords - vec2<f32>(0.5);
    let dist = dot(cc, cc) * screenCurvature;
    return paddedCoords + cc * (1.0 + dist) * dist;
}

fn applyRasterization(screenCoords: vec2<f32>, texel: vec3<f32>, virtualRes: vec2<f32>, intensity: f32, mode: f32) -> vec3<f32> {
    if (intensity <= 0.0 || mode < 0.5 || mode > 3.5) {
        return texel;
    }
    let INTENSITY = 0.30;
    let BRIGHTBOOST = 0.30;

    if (mode < 1.5) {
        let pixelHigh = ((1.0 + BRIGHTBOOST) - (0.2 * texel)) * texel;
        let pixelLow = ((1.0 - INTENSITY) + (0.1 * texel)) * texel;
        let coords = fract(screenCoords * virtualRes) * 2.0 - vec2<f32>(1.0);
        let mask = 1.0 - abs(coords.y);
        return mix(texel, mix(pixelLow, pixelHigh, mask), intensity);
    } else if (mode < 2.5) {
        let pixelHigh = ((1.0 + BRIGHTBOOST) - (0.2 * texel)) * texel;
        let pixelLow = ((1.0 - INTENSITY) + (0.1 * texel)) * texel;
        var coords = fract(screenCoords * virtualRes) * 2.0 - vec2<f32>(1.0);
        coords = coords * coords;
        let mask = 1.0 - coords.x - coords.y;
        return mix(texel, mix(pixelLow, pixelHigh, mask), intensity);
    } else {
        let SUBPIXELS = 3.0;
        let offsets = vec3<f32>(3.141592654) * vec3<f32>(0.5, 0.5 - 2.0 / 3.0, 0.5 - 4.0 / 3.0);
        let omega = vec2<f32>(3.141592654) * vec2<f32>(2.0) * virtualRes;
        let angle = screenCoords * omega;
        let xfactors = (SUBPIXELS + sin(vec3<f32>(angle.x) + offsets)) / (SUBPIXELS + 1.0);
        let result = texel * xfactors;
        let pixelHigh = ((1.0 + BRIGHTBOOST) - (0.2 * result)) * result;
        let pixelLow = ((1.0 - INTENSITY) + (0.1 * result)) * result;
        let coords = fract(screenCoords * virtualRes) * 2.0 - vec2<f32>(1.0);
        let mask = 1.0 - abs(coords.y);
        return mix(texel, mix(pixelLow, pixelHigh, mask), intensity);
    }
}

fn randomPass(coords: vec2<f32>, vresY: f32, time: f32) -> f32 {
    return fract(smoothstep(-120.0, 0.0, coords.y - (vresY + 120.0) * fract(time * 0.15)));
}

fn convertWithChroma(inColor: vec3<f32>, fgColor: vec3<f32>, backgroundColor: vec3<f32>, chromaColor: f32) -> vec3<f32> {
    let grey = rgb2grey(inColor);
    let foregroundColor = mix(fgColor, inColor * fgColor / max(grey, 0.0001), chromaColor);
    return mix(backgroundColor, foregroundColor, grey);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let virtualResolution = u.p0.xy;
    let time = u.p0.z;
    let screenCurvature = u.p0.w;
    let rasterizationIntensity = u.p1.x;
    let burnInTime = u.p1.y;
    let burnIn = u.p1.z;
    let staticNoise = u.p1.w;
    let glowingLine = u.p2.x;
    let chromaColor = u.p2.y;
    let jitterDisplacement = u.p2.zw;
    let horizontalSync = u.p3.x;
    let flickering = u.p3.y;
    let scaleNoiseSize = u.p3.zw;
    let bloom = u.p4.x;
    let vBrightness = u.p4.y;
    let vDistortionScale = u.p4.z;
    let vDistortionFreq = u.p4.w;
    let fgColor = u.p5.rgb;
    let rasterMode = u.p5.w;
    let backgroundColor = u.p6.rgb;
    let frameSize = u.p6.w;
    let burnInLastUpdate = u.p7.x;
    let jitter = u.p7.y;

    let qt = in.uv;
    let ccv = vec2<f32>(0.5) - qt;
    let distance = length(ccv);

    let staticCoords = distortCoordinates(qt, screenCurvature, frameSize);
    let aa = max(fwidth(staticCoords), vec2<f32>(1e-6));
    let covLo = smoothstep(vec2<f32>(0.0), aa, staticCoords);
    let covHi = vec2<f32>(1.0) - smoothstep(vec2<f32>(1.0) - aa, vec2<f32>(1.0), staticCoords);
    let cov = covLo * covHi;
    let isScreen = cov.x * cov.y;
    var coords = qt;

    let dst = sin((coords.y + time) * vDistortionFreq);
    coords.x += dst * vDistortionScale;

    let noiseTexel = textureSample(noiseSource, noiseSmp,
        scaleNoiseSize * coords + vec2<f32>(fract(time / 0.051), fract(time / 0.237)));

    let txt_coords = coords + (noiseTexel.ba - vec2<f32>(0.5)) * jitterDisplacement * jitter;

    var color = 0.0001;
    color += noiseTexel.a * staticNoise * (1.0 - distance * 1.3);
    color += randomPass(coords * virtualResolution, virtualResolution.y, time) * glowingLine;

    let frameColor = textureSample(frameSource, smp, qt);
    color *= (1.0 - frameColor.a) * isScreen;

    var txt_color = textureSample(screenBuffer, smp, txt_coords).rgb;
    txt_color *= 1.0 + max(bloom, 0.0);

    if (burnIn > 0.0) {
        let txt_blur = textureSample(burnInSource, smp, staticCoords);
        let blurDecay = clamp((time - burnInLastUpdate) * burnInTime, 0.0, 1.0);
        let burnInColor = 0.65 * (txt_blur.rgb - vec3<f32>(blurDecay)) * (1.0 - txt_blur.a) * isScreen;
        txt_color = max(txt_color, burnInColor);
    }

    txt_color += vec3<f32>(color);
    txt_color = min(txt_color, vec3<f32>(1.0));
    txt_color = applyRasterization(staticCoords, txt_color, virtualResolution, rasterizationIntensity, rasterMode);

    var finalColor = convertWithChroma(txt_color, fgColor, backgroundColor, chromaColor);
    finalColor *= mix(1.0, vBrightness, step(0.0, flickering));

    finalColor = mix(finalColor, frameColor.rgb, frameColor.a);

    return vec4<f32>(srgb_to_lin(finalColor), 1.0);
}
