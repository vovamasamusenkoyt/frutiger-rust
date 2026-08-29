// Frutiger Aero / Liquid Glass WGSL Shader

struct GlassUniforms {
    screen_size: vec2<f32>,
    time: f32,
    blur_strength: f32,

    refraction_strength: f32,
    chromatic_aberration: f32,
    corner_radius: f32,
    border_width: f32,

    border_color: vec4<f32>,
    tint_color: vec4<f32>,
    specular_strength: f32,
    frost_noise: f32,
};

@group(0) @binding(0) var<uniform> uniforms: GlassUniforms;
@group(0) @binding(1) var screen_texture: texture_2d<f32>;
@group(0) @binding(2) var texture_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) local_pos: vec2<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
    out.uv = model.uv;
    out.local_pos = model.position;
    return out;
}

// Pseudo-random noise for subtle frosted glass texture
fn hash(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    let p3_dot = dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3_dot);
}

// Signed distance field for rounded rectangle
fn sd_rounded_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - r;
}

// Dual-Kawase-like multi-sample blur kernel
fn sample_blurred(uv: vec2<f32>, blur: f32) -> vec4<f32> {
    let tex_size = uniforms.screen_size;
    let offset = (blur * 2.0) / tex_size;

    var color = textureSample(screen_texture, texture_sampler, uv) * 0.22;
    color += textureSample(screen_texture, texture_sampler, uv + vec2<f32>(-offset.x, -offset.y)) * 0.12;
    color += textureSample(screen_texture, texture_sampler, uv + vec2<f32>( offset.x, -offset.y)) * 0.12;
    color += textureSample(screen_texture, texture_sampler, uv + vec2<f32>(-offset.x,  offset.y)) * 0.12;
    color += textureSample(screen_texture, texture_sampler, uv + vec2<f32>( offset.x,  offset.y)) * 0.12;

    color += textureSample(screen_texture, texture_sampler, uv + vec2<f32>(0.0, -offset.y * 1.5)) * 0.075;
    color += textureSample(screen_texture, texture_sampler, uv + vec2<f32>(0.0,  offset.y * 1.5)) * 0.075;
    color += textureSample(screen_texture, texture_sampler, uv + vec2<f32>(-offset.x * 1.5, 0.0)) * 0.075;
    color += textureSample(screen_texture, texture_sampler, uv + vec2<f32>( offset.x * 1.5, 0.0)) * 0.075;

    return color;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let local = in.local_pos; // [-1.0, 1.0]

    // Liquid surface normal distortion (curvature towards edges)
    let edge_dist = length(local);
    let normal_xy = local * pow(edge_dist, 2.0) * uniforms.refraction_strength;

    // Chromatic aberration (RGB split) with refraction offset
    let refract_r = uv + normal_xy * (1.0 + uniforms.chromatic_aberration);
    let refract_g = uv + normal_xy;
    let refract_b = uv + normal_xy * (1.0 - uniforms.chromatic_aberration);

    let blur = uniforms.blur_strength;
    let sample_r = sample_blurred(refract_r, blur).r;
    let sample_g = sample_blurred(refract_g, blur).g;
    let sample_b = sample_blurred(refract_b, blur).b;

    var glass_color = vec3<f32>(sample_r, sample_g, sample_b);

    // Subtle frost noise
    let noise = (hash(in.clip_position.xy + uniforms.time) - 0.5) * uniforms.frost_noise;
    glass_color += vec3<f32>(noise);

    // Frutiger Aero Specular Highlight (Simulate overhead light source)
    let light_dir = normalize(vec2<f32>(-0.4, -0.9));
    let specular_edge = max(dot(-local, light_dir), 0.0);
    let gloss = pow(specular_edge, 3.5) * uniforms.specular_strength;

    // Linear glossy sheen along top edge
    let top_gloss = smoothstep(-0.2, 0.8, -local.y) * 0.15 * uniforms.specular_strength;

    // Mix tint and gloss
    var final_color = mix(glass_color, uniforms.tint_color.rgb, uniforms.tint_color.a);
    final_color += vec3<f32>(gloss + top_gloss);

    // Subtle border glow
    let border_factor = smoothstep(0.85, 0.98, edge_dist);
    final_color = mix(final_color, uniforms.border_color.rgb, border_factor * uniforms.border_color.a);

    return vec4<f32>(final_color, 1.0);
}
