#version 140

uniform sampler2D sampler;
uniform vec4 colour;

in vec2 out_uv;

out vec4 texel;

void main() {
    // Colour the texel by the colour and the multiplied alpha values
    texel = vec4(colour.rgb, texture(sampler, out_uv).r * colour.a);
}