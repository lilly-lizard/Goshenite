struct OutputVS
{
	float4 pos : SV_POSITION;
[[vk::location(0)]] float2 uv : TEXCOORD0;
};

OutputVS main(uint vertexIndex : SV_VertexID)
{
	OutputVS output = (OutputVS)0;
	output.uv = float2((vertexIndex << 1) & 2, vertexIndex & 2);
	output.pos = float4(output.UV * 2.0f + -1.0f, 0.0f, 1.0f);
	return output;
}