#version 140

in vec4 v_colour;
in vec2 v_uv;

out vec4 colour;

uniform sampler2D image;

void main() {
    vec4 image_colour = texture(image, v_uv);
    colour = mix(image_colour, vec4(v_colour.rgb, 1.0), v_colour.a);
}
