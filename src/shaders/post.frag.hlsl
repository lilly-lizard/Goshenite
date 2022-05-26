Texture2D renderTexture : register(t0);
SamplerState renderSampler : register(s0);

float4 main([[vk::location(0)]] float2 inUV : TEXCOORD0) : SV_TARGET
{
  return renderTexture.Sample(renderSampler, float2(inUV.x, 1.0 - inUV.y));
}