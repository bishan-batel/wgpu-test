use std::{
    error::Error,
    path::PathBuf,
    process::{Command, Stdio},
};

fn compile_shader(shader: PathBuf) -> Result<(), Box<dyn Error>> {
    let cwd = std::env::current_dir().unwrap();

    let path = shader.as_path();

    let relative_path = path.strip_prefix(cwd).unwrap_or(path);

    let output_path = path.with_added_extension("wgsl");

    println!("cargo::rerun-if-changed={}", path.to_str().unwrap());
    println!("cargo::rerun-if-changed={}", output_path.to_str().unwrap());

    let output = Command::new("slangc")
        .arg(path)
        .arg("-target")
        .arg("wgsl")
        .arg("-o")
        .arg(output_path)
        .output()
        .expect("Failed to run slangc");

    if !output.status.success() {
        println!("cargo::error={} {:?}", "Failed to compile ", relative_path);

        let file_name = relative_path
            .file_name()
            .map(|x| String::from(x.to_string_lossy()))
            .unwrap_or("Slang".into());

        for error in str::from_utf8(&output.stderr).unwrap().split('\n') {
            println!("cargo::error=[{:?}] {}", file_name, error);
        }
    }
    return Ok(());
}

// build.rs
fn main() -> Result<(), Box<dyn Error>> {
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=X11");
        println!("cargo:rustc-link-lib=Xcursor");
        println!("cargo:rustc-link-lib=Xrandr");
        println!("cargo:rustc-link-lib=Xi");
        println!("cargo:rustc-link-lib=vulkan");
    } else {
        return Ok(());
    }

    println!("cargo::rerun-if-changed=build.rs");

    println!("cargo::warning=Recompiling Slang Shaders");

    for path in glob::glob("./src/**/*.slang").unwrap().map(|x| x.unwrap()) {
        compile_shader(path)?;
    }

    println!("cargo::warning=Compiled Slang Shaders");
    println!("cargo::warning={:?}", std::env::current_dir().unwrap());

    Ok(())
}
