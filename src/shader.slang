
struct Globals {
  float4 color;
}

[vk::binding(0, 0)]
ConstantBuffer<Globals> globals;

struct VertexInput {
  float2 position : POSITION;
};

struct VertexOutput {
  float4 clip_position : SV_POSITION;
};

struct FragmentOutput {
  float4 color : SV_Target0;
};

[shader("vertex")]
VertexOutput vs_main(VertexInput input) {
  VertexOutput output;

  input.position =
      mul(0.5f * float2x2(float2(cos(globals.color.r), sin(globals.color.r)),
                          float2(-sin(globals.color.r), cos(globals.color.r))),
          input.position);
  output.clip_position = float4(input.position.x, input.position.y, 0.0, 1.0, );

  return output;
}

[shader("fragment")]
FragmentOutput fs_main(VertexOutput input) {
  FragmentOutput output;

  output.color = globals.color;
  output.color.a = 1.0f;

  return output;
}

