use std::fs::File;
use std::io::{Read, Write};

// thanks to https://github.com/JakubKoralewski/mandelbrot-wgpu-rs/blob/master/src/build.rs
fn gen_shader_spirv() {
    println!("Generating spirv shaders...");

    // shader source directory
    let mut shader_src_dir = std::env::current_dir().unwrap();
    shader_src_dir.push("src");
    shader_src_dir.push("shaders");
    if !shader_src_dir.is_dir() {
        panic!("gen_shader_spirv: shader source path is not a directory");
    }

    // spirv directory
    let mut spirv_dir = std::env::current_dir().unwrap();
    spirv_dir.push("assets");
    spirv_dir.push("shader_binaries");
    if !shader_src_dir.is_dir() {
        panic!("gen_shader_spirv: spirv destination path is not a directory");
    }

    // iterate over files in shaders directory
    for dir_entry in std::fs::read_dir(shader_src_dir).unwrap() {
        let dir_entry = dir_entry.unwrap();

        // file name
        let file_name = match dir_entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };

        let shader_path = dir_entry.path();

        // determine shader type
        let file_ext = match shader_path.extension() {
            Some(e) => e,
            None => continue,
        };
        let shader_type = match file_ext.to_str().unwrap() {
            "vert" => shaderc::ShaderKind::Vertex,
            "frag" => shaderc::ShaderKind::Fragment,
            "comp" => shaderc::ShaderKind::Compute,
            _ => continue,
        };

        // read shader source
        let mut shader_text = String::new();
        let mut shader_file = File::open(&shader_path).unwrap();
        shader_file.read_to_string(&mut shader_text).unwrap();

        // compile spirv
        println!("Compiling {:?}...", file_name);
        let compiler = shaderc::Compiler::new().unwrap();
        let spirv_bin = compiler
            .compile_into_spirv(&shader_text, shader_type, &file_name, "main", None)
            .unwrap();
        let spirv_bin = spirv_bin.as_binary_u8();

        // write spirv to file
        let mut file_out_path = spirv_dir.clone();
        file_out_path.push(file_name + ".spv");
        let mut file_out = File::create(file_out_path).unwrap();
        file_out.write_all(&spirv_bin).unwrap();
    }
}

fn main() {
    gen_shader_spirv();

    // rerun when shaders change
    println!("cargo:rerun-if-changed=src/shaders/*");
}
