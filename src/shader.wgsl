
alias float4 = vec4<f32>;

struct Globals {
    color: vec4<f32>
}

@group(0) @binding(0)
var<uniform> globals: Globals;

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
) -> @builtin(position) vec4<f32> {
    return vec4<f32>(position.x, position.y, 0., 1.);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    let color = globals.color;

    return vec4<f32>(color.rgb, 1.);
}
