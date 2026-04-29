struct Globals_std140_0 {
    @align(16) color_0: vec4<f32>,
};

@binding(0) @group(0) var<uniform> globals_0: Globals_std140_0;

struct VertexOutput_0 {
    @builtin(position) clip_position_0: vec4<f32>,
};

struct vertexInput_0 {
    @location(0) position_0: vec2<f32>,
};

struct VertexInput_0 {
    position_1: vec2<f32>,
};

@vertex
fn vs_main(_S1: vertexInput_0) -> VertexOutput_0 {
    var _S2: VertexInput_0;
    _S2.position_1 = _S1.position_0;
    var _S3: mat2x2<f32> = mat2x2<f32>(vec2<f32>(cos(globals_0.color_0.x), sin(globals_0.color_0.x)), vec2<f32>(- sin(globals_0.color_0.x), cos(globals_0.color_0.x)));
    var _S4: mat2x2<f32> = mat2x2<f32>(0.5f, 0.5f, 0.5f, 0.5f);
    var _S5: vec2<f32> = (((_S2.position_1) * (mat2x2<f32>(_S4[0] * _S3[0], _S4[1] * _S3[1]))));
    _S2.position_1 = _S5;
    var output_0: VertexOutput_0;
    output_0.clip_position_0 = vec4<f32>(_S5.x, _S5.y, 0.0f, 1.0f);
    return output_0;
}

struct FragmentOutput_0 {
    @location(0) color_1: vec4<f32>,
    @location(1) color2_0: vec4<f32>,
};

@fragment
fn fs_main(@builtin(position) clip_position_1: vec4<f32>) -> FragmentOutput_0 {
    var output_1: FragmentOutput_0;
    output_1.color_1[i32(3)] = 1.0f;
    var _S6: vec3<f32> = globals_0.color_0.xyz;
    output_1.color_1.x = _S6.x;
    output_1.color_1.y = _S6.y;
    output_1.color_1.z = _S6.z;
    return output_1;
}

