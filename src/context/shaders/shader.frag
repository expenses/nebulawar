#version 140

in vec3 v_normal;
in vec2 v_texture;

out vec4 color;
uniform vec3 light_direction;
uniform sampler2D tex;
uniform int mode;

float map(float min, float max, float value) {
    return value * (max - min) + min;
}

const int NORMAL = 1;
const int SHADELESS = 2;
const int STARS = 3;

const vec3 WHITE = vec3(1.0);
const float MIN_LIGHT = 0.1;

void main() {
    vec4 texture_color = texture(tex, v_texture);

    if (mode == SHADELESS) {
        color = texture_color;
    } else if (mode == STARS) {
        color = vec4(WHITE, v_texture.x);
    } else {
        vec3 light_dir = normalize(light_direction);  

        float brightness = dot(normalize(v_normal), normalize(light_dir));
        
        float norm_brightness = mix(MIN_LIGHT, 1.0, max(brightness, 0.0));

        color = vec4(norm_brightness * texture_color.rgb, texture_color.a);
    }
}