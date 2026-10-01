use regex::Regex;
use std::fs;
use std::path::Path;

pub struct TestCase {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub output_name: String,
}

pub fn assemble_rust_project(test_case: &TestCase) {
    assemble_project(test_case, false);
}

/// Link an externally compiled BA while keeping the ordinary generated thread,
/// port types and project modules. Regeneration copies Glue from the input tree.
pub fn assemble_external_ba_project(test_case: &TestCase) {
    assemble_project(test_case, true);
}

fn assemble_project(test_case: &TestCase, external_ba: bool) {
    let project_root = format!("generate/project/{}", test_case.output_name);

    // ---------------- Cargo.toml ----------------
    generate_cargo_toml(&project_root, &test_case.output_name);

    // ---------------- C / H  ----------------
    copy_c_sources(&test_case.path, &project_root);

    let mut c_files = Vec::new();
    let mut h_files = Vec::new();

    let input_dir = Path::new(&test_case.path);

    for entry in fs::read_dir(input_dir)
        .expect("Failed to read input directory")
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext == "c" {
                c_files.push(path.file_name().unwrap().to_string_lossy().to_string());
            } else if ext == "h" {
                h_files.push(path.file_name().unwrap().to_string_lossy().to_string());
            }
        }
    }

    // ---------------- build.rs ----------------
    if external_ba {
        generate_external_ba_build(test_case, &project_root);
    } else if c_files.is_empty() || h_files.is_empty() {
        generate_empty_build_rs(&project_root);
    } else {
        generate_build_rs_from_c_files(&project_root, &c_files, &h_files);
    }

    // ---------------- Rust support files ----------------
    generate_common_traits_rs(&project_root);
    generate_posix_rs(&project_root);
    generate_lib_rs(&project_root);

    // ---------------- main.rs ----------------
    let (module_name, system_type) =
        find_system_impl(&project_root).expect("can find impl System for XXX_impl");

    generate_main_rs(
        &project_root,
        &system_type,
        &module_name,
        &test_case.output_name,
    );

    println!("📦 Project generation completed: {}", project_root);
}

/// The BA compiler and Rust must target the same ABI. The production object is
/// always separate from the optional object that adds read-only test probes.
fn generate_external_ba_build(test_case: &TestCase, project_root: &str) {
    let input = Path::new(&test_case.path);
    for (source, destination) in [
        ("ba_glue.rs", "src/ba_glue.rs"),
        ("ba_behavior.o", "c_src/ba_behavior.o"),
        ("ba_probes.o", "c_src/ba_probes.o"),
    ] {
        if input.join(source).is_file() {
            fs::copy(input.join(source), Path::new(project_root).join(destination))
                .expect("Failed to copy external BA artifact");
        }
    }
    let manifest_path = Path::new(project_root).join("Cargo.toml");
    let mut manifest = fs::read_to_string(&manifest_path).unwrap();
    manifest.push_str("\n[features]\n# Only tests may link private-state observation functions.\nba-probes = []\n");
    fs::write(manifest_path, manifest).unwrap();
    let build = r#"// CompCert supplies the BA object; do not compile its C with a different compiler.
use std::{env, path::Path};

fn main() {
    assert_eq!(env::var("CARGO_CFG_TARGET_ARCH").unwrap(), "x86_64", "BA object requires x86-64");
    assert_eq!(env::var("CARGO_CFG_TARGET_OS").unwrap(), "linux", "BA object requires Linux");
    let object = if env::var_os("CARGO_FEATURE_BA_PROBES").is_some() {
        "c_src/ba_probes.o"
    } else {
        "c_src/ba_behavior.o"
    };
    println!("cargo:rerun-if-changed={object}");
    println!("cargo:rerun-if-changed=c_include/ba.h");
    assert!(Path::new(object).is_file(), "Generate the CompCert BA object before building the host");
    cc::Build::new().object(object).compile("ba_behavior");
    // A single header owns lifecycle imports; no per-header output overwrites.
    bindgen::Builder::default()
        .header("c_include/ba.h")
        .clang_arg("-Ic_include")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate().expect("BA lifecycle bindings failed")
        .write_to_file(Path::new(&env::var("OUT_DIR").unwrap()).join("aadl_c_bindings.rs"))
        .expect("Failed to write BA bindings");
}
"#;
    fs::write(Path::new(project_root).join("build.rs"), build).unwrap();
}

/// generate Cargo.toml
fn generate_cargo_toml(project_root: &str, project_name: &str) {
    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"
build = "build.rs"

[build-dependencies]
cc = {{ version = "1.0", features = ["parallel"] }}
bindgen = "0.69"

[dependencies]
libc = "0.2"
lazy_static = "1.4"
crossbeam-channel = "0.5"
rand = "0.7"
tokio = {{ version = "1.40", features = ["sync"] }}
"#,
        project_name.replace('-', "_")
    );

    fs::write(format!("{}/Cargo.toml", project_root), cargo_toml)
        .expect("Failed to write Cargo.toml");
}

fn copy_c_sources(input_dir: &str, project_root: &str) {
    let c_src_dir = format!("{}/c_src", project_root);
    let c_include_dir = format!("{}/c_include", project_root);

    fs::create_dir_all(&c_src_dir).unwrap();
    fs::create_dir_all(&c_include_dir).unwrap();

    let entries = fs::read_dir(input_dir).unwrap();

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext == "c" {
                let dest = format!(
                    "{}/{}",
                    c_src_dir,
                    path.file_name().unwrap().to_string_lossy()
                );
                fs::copy(&path, &dest).unwrap();
            } else if ext == "h" {
                let dest = format!(
                    "{}/{}",
                    c_include_dir,
                    path.file_name().unwrap().to_string_lossy()
                );
                fs::copy(&path, &dest).unwrap();
            }
        }
    }
}

/// build.rs
fn generate_build_rs_from_c_files(project_root: &str, c_files: &[String], h_files: &[String]) {
    let mut build_rs = String::new();

    build_rs.push_str("fn main() {\n");

    // rerun-if-changed
    for c in c_files {
        build_rs.push_str(&format!(
            "    println!(\"cargo:rerun-if-changed=c_src/{}\");\n",
            c
        ));
    }
    for h in h_files {
        build_rs.push_str(&format!(
            "    println!(\"cargo:rerun-if-changed=c_include/{}\");\n",
            h
        ));
    }

    build_rs.push_str("\n    cc::Build::new()\n");

    for c in c_files {
        build_rs.push_str(&format!("        .file(\"c_src/{}\")\n", c));
    }

    build_rs.push_str(
        r#"
        .include("c_include")
        .flag_if_supported("-std=c11")
        .compile("c_lib");
    "#,
    );

    for h in h_files {
        build_rs.push_str("    bindgen::Builder::default()\n");
        build_rs.push_str(&format!("        .header(\"c_include/{}\")\n", h));
        build_rs.push_str(
            r#"
            .clang_arg("-Ic_include")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .generate()
            .expect("Binding generation failed")
            .write_to_file(
                std::path::Path::new(&std::env::var("OUT_DIR").unwrap())
                    .join("aadl_c_bindings.rs")
            )
            .expect("Failed to write binding file");
        "#,
        );
    }

    build_rs.push_str("}\n");

    let build_rs_path = format!("{}/build.rs", project_root);
    fs::write(&build_rs_path, build_rs).expect("Failed to write build.rs");

    println!("Build.rs generated: {}", build_rs_path);
}

fn generate_empty_build_rs(project_root: &str) {
    let path = format!("{}/build.rs", project_root);

    let content = r#"
use std::{env, fs, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let bindings_path = out_dir.join("aadl_c_bindings.rs");

    fs::write(
        &bindings_path,
        "// empty native bindings\n",
    )
    .expect("failed to write empty aadl_c_bindings.rs");

    println!("cargo:rerun-if-changed=build.rs");
}
"#;

    fs::write(&path, content).expect("Failed to write empty build.rs");

    println!("build.rs(empty binding version) generated: {}", path);
}

/// src/common_traits.rs
fn generate_common_traits_rs(project_root: &str) {
    let src_dir = format!("{}/src", project_root);
    let path = format!("{}/common_traits.rs", src_dir);

    let content = r#"// ---------------- System ----------------
pub trait System {
    fn new() -> Self
        where Self: Sized;
    fn run(self);
}

// ---------------- Process ----------------
pub trait Process {
    fn new(cpu_id: isize) -> Self
        where Self: Sized;
    fn run(self);
}

// ---------------- Thread ----------------
pub trait Thread {
    fn new(cpu_id: isize) -> Self
        where Self: Sized;
    fn run(self);
}

// ---------------- Device ----------------
pub trait Device {
    fn new() -> Self
        where Self: Sized;
    fn run(self);
}
"#;

    fs::write(&path, content).expect("Failed to write common_traits.rs");

    println!("common_traits.rs generated: {}", path);
}

/// src/posix.rs
fn generate_posix_rs(project_root: &str) {
    let src_dir = format!("{}/src", project_root);
    let path = format!("{}/posix.rs", src_dir);

    let content = r#"use libc::{
    cpu_set_t, CPU_SET, CPU_ZERO, sched_setaffinity,
};

// ---------------- cpu ----------------

pub fn set_thread_affinity(cpu: isize) {
    unsafe {
        let mut cpuset: cpu_set_t = std::mem::zeroed();
        CPU_ZERO(&mut cpuset);
        CPU_SET(cpu as usize, &mut cpuset);
        sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &cpuset);
    }
}
"#;

    fs::write(&path, content).expect("Failed to write posix.rs");

    println!("posix.rs generated: {}", path);
}

/// src/lib.rs
fn generate_lib_rs(project_root: &str) {
    let src_dir = format!("{}/src", project_root);
    let lib_rs_path = format!("{}/lib.rs", src_dir);

    let mut modules = Vec::new();

    let entries = fs::read_dir(&src_dir).expect("Failed to read src directory");

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let file_name = path.file_name().unwrap().to_string_lossy();

            if file_name == "lib.rs" || file_name == "main.rs" {
                continue;
            }

            let module_name = file_name.trim_end_matches(".rs").to_string();
            modules.push(module_name);
        }
    }

    modules.sort();

    let mut content = String::new();

    // crate-level attributes
    content.push_str("#![allow(non_snake_case)]\n");
    content.push_str("#![allow(non_camel_case_types)]\n\n");

    // content.push_str("pub mod common_traits;\n\n");

    for m in modules {
        content.push_str(&format!("pub mod {};\n", m));
    }

    fs::write(&lib_rs_path, content).expect("Failed to write lib.rs");

    println!("lib.rs generated: {}", lib_rs_path);
}

/// src/main.rs
fn generate_main_rs(project_root: &str, system_type: &str, module_name: &str, project_name: &str) {
    let main_rs_path = format!("{}/src/main.rs", project_root);

    let content = format!(
        r#"use {project_name}::common_traits::System;
use {project_name}::{module_name}::{system_type};

pub fn boot<S: System>() {{
    let system = S::new();
    system.run();

    loop {{
        std::thread::sleep(std::time::Duration::from_secs(60));
    }}
}}

fn main() {{
    boot::<{system_type}>();
}}
"#,
        system_type = system_type,
        module_name = module_name,
        project_name = project_name.replace('-', "_"),
    );

    fs::write(&main_rs_path, content).expect("Failed to write main.rs");

    println!("main.rs generated: {}", main_rs_path);
}

///  (module_name, system_type)
fn find_system_impl(project_root: &str) -> Option<(String, String)> {
    let src_dir = Path::new(project_root).join("src");

    let re = Regex::new(r"impl\s+System\s+for\s+([A-Za-z_][A-Za-z0-9_]*)").expect("invalid regex");

    let entries = fs::read_dir(&src_dir).ok()?;

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }

        let file_name = path.file_name()?.to_string_lossy();

        if file_name == "lib.rs" || file_name == "main.rs" || file_name == "common_traits.rs" {
            continue;
        }

        let content = fs::read_to_string(&path).ok()?;

        if let Some(caps) = re.captures(&content) {
            let system_type = caps.get(1)?.as_str().to_string();

            let module_name = file_name.trim_end_matches(".rs").to_string();

            return Some((module_name, system_type));
        }
    }

    None
}
