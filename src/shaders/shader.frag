#version 140

in vec3 v_normal;
in vec2 v_texture;
in vec3 v_position;

out vec4 color;
uniform vec3 light_direction;
uniform sampler2D tex;
uniform bool shadeless;

float map(float min, float max, float value) {
    return value * (max - min) + min;
}

void main() {
    vec3 texture_color = texture(tex, v_texture).rgb;

    if (shadeless) {
        color = vec4(texture_color, 1.0);
    } else {
        vec3 light_dir = normalize(light_direction);  

        float brightness = dot(normalize(v_normal), normalize(light_dir));
        
        float norm_brightness = mix(0.1, 1.0, max(brightness, 0.0));

        color = vec4(norm_brightness * texture_color, 1.0);
    }
}