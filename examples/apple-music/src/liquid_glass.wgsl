struct GlassUniform {
    screen: vec4<f32>,
    glass: vec4<f32>,
    effect: vec4<f32>,
}

@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var source_sampler: sampler;

@group(0) @binding(2)
var<uniform> settings: GlassUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[index], 0.0, 1.0);
    return output;
}

fn rounded_distance(point: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let half_size = size * 0.5;
    let corner = max(half_size - vec2<f32>(radius), vec2<f32>(0.0));
    let offset = abs(point - half_size) - corner;
    return length(max(offset, vec2<f32>(0.0)))
        + min(max(offset.x, offset.y), 0.0)
        - radius;
}

fn surface_normal(point: vec2<f32>, size: vec2<f32>, radius: f32) -> vec2<f32> {
    let x = rounded_distance(point + vec2<f32>(1.0, 0.0), size, radius)
        - rounded_distance(point - vec2<f32>(1.0, 0.0), size, radius);
    let y = rounded_distance(point + vec2<f32>(0.0, 1.0), size, radius)
        - rounded_distance(point - vec2<f32>(0.0, 1.0), size, radius);
    return normalize(vec2<f32>(x, y) + vec2<f32>(0.0001));
}

fn noise(point: vec2<f32>) -> f32 {
    return fract(sin(dot(point, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

@fragment
fn glass_fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let local = position.xy;
    let size = settings.glass.xy;
    let radius = settings.glass.z;
    let distance = rounded_distance(local, size, radius);
    let normal = surface_normal(local, size, radius);
    let edge_depth = min(radius * 0.65, 16.0);
    let edge = 1.0 - smoothstep(0.0, edge_depth, max(-distance, 0.0));
    let refracted = settings.screen.xy + local
        - normal * settings.effect.x * edge * edge;
    let source_uv = clamp(
        refracted / settings.screen.zw,
        vec2<f32>(0.001),
        vec2<f32>(0.999),
    );

    let gaussian = array<f32, 3>(0.38774, 0.24477, 0.06136);
    let blur_step = max(settings.glass.w * 0.5, 0.5);
    var color = vec3<f32>(0.0);
    for (var y: i32 = -2; y <= 2; y += 1) {
        for (var x: i32 = -2; x <= 2; x += 1) {
            let weight = gaussian[u32(abs(x))] * gaussian[u32(abs(y))];
            let offset = vec2<f32>(f32(x), f32(y))
                * blur_step / settings.screen.zw;
            color += textureSampleLevel(
                source_texture,
                source_sampler,
                source_uv + offset,
                0.0,
            ).rgb * weight;
        }
    }

    let chroma = normal * edge * 0.65 / settings.screen.zw;
    let split = vec3<f32>(
        textureSampleLevel(source_texture, source_sampler, source_uv + chroma, 0.0).r,
        color.g,
        textureSampleLevel(source_texture, source_sampler, source_uv - chroma, 0.0).b,
    );
    color = mix(color, split, edge * 0.22);

    let luminance = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = mix(vec3<f32>(luminance), color, 1.16);
    color = mix(color, vec3<f32>(0.985, 0.985, 1.0), settings.effect.y);

    let rim = (1.0 - smoothstep(0.0, 1.4, abs(distance)))
        * (0.35 + 0.65 * max(dot(normal, normalize(vec2<f32>(-0.55, -0.84))), 0.0));
    let sheen = edge * edge * max(dot(normal, normalize(vec2<f32>(-0.35, -0.94))), 0.0);
    color += vec3<f32>(rim * 0.34 + sheen * 0.055);
    color += vec3<f32>((noise(floor(settings.screen.xy + local)) - 0.5) * 0.012);

    let mask = 1.0 - smoothstep(-1.0, 1.0, distance);
    let alpha = mask * (0.74 + settings.effect.y * 0.16);
    return vec4<f32>(max(color, vec3<f32>(0.0)), alpha);
}

@fragment
fn composite_fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = clamp(
        (position.xy - settings.screen.xy) / settings.glass.xy,
        vec2<f32>(0.0),
        vec2<f32>(1.0),
    );
    let glass = textureSampleLevel(source_texture, source_sampler, uv, 0.0);
    return vec4<f32>(glass.rgb * glass.a, glass.a);
}
