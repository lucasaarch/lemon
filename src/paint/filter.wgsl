struct Params { sigma:f32, brightness:f32, width:u32, height:u32 }
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<OUTPUT_FORMAT, write>;
@group(0) @binding(2) var<uniform> params: Params;
const VERTICAL: bool = VERTICAL_PASS;
@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) gid:vec3<u32>) {
    if gid.x>=params.width || gid.y>=params.height { return; }
    let radius=i32(ceil(params.sigma*3.0));
    var color=vec4<f32>(0.0);
    var total=0.0;
    for (var i=-radius; i<=radius; i=i+1) {
        var weight=1.0;
        if params.sigma>0.0 { weight=exp(-f32(i*i)/(2.0*params.sigma*params.sigma)); }
        var point=vec2<i32>(gid.xy);
        if VERTICAL { point.y+=i; } else { point.x+=i; }
        if all(point>=vec2<i32>(0)) && all(point<vec2<i32>(i32(params.width),i32(params.height))) {
            var sample_color=textureLoad(source,point,0);
            // Vello outputs straight alpha; convolve premultiplied colors to avoid fringes.
            if !VERTICAL { sample_color=vec4<f32>(sample_color.rgb*sample_color.a,sample_color.a); }
            color+=sample_color*weight;
        }
        total+=weight;
    }
    color/=max(total,0.00001);
    if VERTICAL {
        if color.a>0.0 { color=vec4<f32>(clamp(color.rgb/color.a*params.brightness,vec3<f32>(0),vec3<f32>(1)),color.a); }
        else { color=vec4<f32>(0); }
    }
    textureStore(output_texture,vec2<i32>(gid.xy),color);
}
