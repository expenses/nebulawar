#version 450

layout(location = 0) in vec4 v_colour;
layout(location = 1) in vec2 v_uv;

layout(location = 0) out vec4 colour;

layout(set = 0, binding = 0) uniform texture2D tex;
layout(set = 0, binding = 1) uniform sampler samp;

void main() {
    vec4 image_colour = texture(sampler2D(tex, samp), v_uv);
    colour = mix(image_colour, vec4(v_colour.rgb, 1.0), v_colour.a);
}
