#version 140

uniform sampler2D sampler;
uniform vec4 colour;
uniform float time;

in vec2 out_uv;
in vec2 out_pos;

out vec4 texel;

void main() {
    float value = texture(sampler, out_uv).r;

    float random = noise1((gl_FragCoord + time) / 10);
    
    if (random > 0.5) {
        texel = vec4(1.0, 1.0, 1.0, value);
    } else {
        texel = colour * vec4(1.0, 1.0, 1.0, value);
    }
}