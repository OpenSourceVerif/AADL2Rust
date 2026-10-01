#![allow(unused_imports)]

mod aadl_ast2rust_code;
pub mod aadlight_parser;
mod ast;
pub mod model_statistics;
// pub mod printmessage;
pub mod transform;
pub mod transform_annex;
// pub mod build_project_tool;

use crate::model_statistics::*;
use aadl_intermediate::RustCodeGenerator;
use aadl_ast2rust_code::merge_utils::*;
use aadlight_parser::AADLParser;
use clap::Parser as ClapParser;
use compiler::printmessage::*;
use pest::error::ErrorVariant;
use pest::Parser;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::{aadl_ast2rust_code::converter::AadlConverter, ast::aadl_ast_cj::Package};
use compiler::build_project_tool::*;

#[derive(ClapParser)]
struct Args {
    #[arg(short, long)]
    input: Option<String>,
    /// Read this exact AADL file, independently of the legacy AADLSource layout.
    #[arg(long, conflicts_with = "input", requires = "output_dir")]
    input_file: Option<std::path::PathBuf>,
    #[arg(long, requires = "input_file")]
    output_dir: Option<std::path::PathBuf>,
    /// Directory containing ba.h and the CompCert objects.
    #[arg(long, requires = "input_file")]
    ba_artifacts: Option<std::path::PathBuf>,
    /// Shared/generated Rust ABI binding file; referenced, not duplicated.
    #[arg(long, requires = "input_file")]
    glue: Option<std::path::PathBuf>,
    /// Bind one existing thread implementation to the supplied ba_glue module.
    #[arg(long)]
    external_ba: Option<String>,
}

fn main() {
    // ================= CLI mode: --input <folder> =================
    let args = Args::parse();

    if let Some(input) = args.input_file.as_deref() {
        if let Err(error) = generate_explicit_project(input, args.output_dir.as_deref().unwrap(),
            args.external_ba.as_deref(), args.ba_artifacts.as_deref(), args.glue.as_deref()) {
            eprintln!("Explicit project generation failed: {error}");
            std::process::exit(2);
        }
        return;
    }
    if args.external_ba.is_some() && args.input.is_none() {
        eprintln!("--external-ba requires --input or --input-file");
        std::process::exit(2);
    }

    if let Some(input_name) = args.input.as_deref() {
        // Construct a TestCase directly from the command-line input folder
        let test_case = TestCase {
            id: 0, // ID is meaningless in CLI mode
            name: format!("CLI input ({})", input_name),
            path: format!("AADLSource/{}", input_name),
            output_name: input_name.to_string(),
        };

        println!("CLI mode");
        println!("Input path: {}", test_case.path);
        println!("Output name: {}", test_case.output_name);

        // Ensure the generate directory exists
        if fs::metadata("generate").is_err() {
            fs::create_dir("generate").expect("Failed to create generate directory");
        }

        let output_dir = format!("generate/project/{}/src", test_case.output_name);
        if std::path::Path::new(&output_dir).exists() {
            println!("Cleaning directory: {}", output_dir);
            fs::remove_dir_all(&output_dir).unwrap();
        }

        process_test_case(&test_case, args.external_ba.as_deref());
        return; // Do not enter the interactive mode below
    }

    // ================= Interactive menu mode =================
    // Define available test cases
    let test_cases = vec![
        TestCase {
            id: 1,
            name: "PingPong (Ocarina)".to_string(),
            path: "AADLSource/pingpong_ocarina.aadl".to_string(),
            output_name: "pingpong_ocarina".to_string(),
        },
        TestCase {
            id: 2,
            name: "PingPong (Simple)".to_string(),
            path: "pingpong.aadl".to_string(),
            output_name: "pingpong_simple".to_string(),
        },
        TestCase {
            id: 3,
            name: "RMA".to_string(),
            path: "AADLSource/rma.aadl".to_string(),
            output_name: "rma".to_string(),
        },
        TestCase {
            id: 4,
            name: "Toy".to_string(),
            path: "AADLSource/toy.aadl".to_string(),
            output_name: "toy".to_string(),
        },
        TestCase {
            id: 5,
            name: "Robot(v1)".to_string(),
            path: "AADLSource/robotv1.aadl".to_string(),
            output_name: "robotv1".to_string(),
        },
        TestCase {
            id: 6,
            name: "Robot(v2)".to_string(),
            path: "AADLSource/robotv2.aadl".to_string(),
            output_name: "robotv2".to_string(),
        },
        TestCase {
            id: 7,
            name: "RMS".to_string(),
            path: "AADLSource/rms.aadl".to_string(),
            output_name: "rms".to_string(),
        },
        TestCase {
            id: 8,
            name: "PingPong (Timed)".to_string(),
            path: "AADLSource/pingpong_timed_aperiodic.aadl".to_string(),
            output_name: "pingpong_timed_aperiodic".to_string(),
        },
        TestCase {
            id: 9,
            name: "base_types".to_string(),
            path: "AADLSource/base_types.aadl".to_string(),
            output_name: "base_types".to_string(),
        },
        TestCase {
            id: 10,
            name: "composite_types".to_string(),
            path: "AADLSource/composite_types.aadl".to_string(),
            output_name: "composite_types".to_string(),
        },
        TestCase {
            id: 11,
            name: "car_all".to_string(),
            path: "AADLSource/car_all.aadl".to_string(),
            output_name: "car_all".to_string(),
        },
        TestCase {
            id: 12,
            name: "pingpong_ocarina_inout".to_string(),
            path: "AADLSource/pingpong_ocarina_inout.aadl".to_string(),
            output_name: "pingpong_ocarina_inout".to_string(),
        },
        TestCase {
            id: 13,
            name: "pingpong_2trigger".to_string(),
            path: "AADLSource/pingpong_2trigger.aadl".to_string(),
            output_name: "pingpong_2trigger".to_string(),
        },
        TestCase {
            id: 14,
            name: "pingpong_example".to_string(),
            path: "AADLSource/pingpong_example.aadl".to_string(),
            output_name: "pingpong_example".to_string(),
        },
        TestCase {
            id: 15,
            name: "car_device".to_string(),
            path: "AADLSource/car_device.aadl".to_string(),
            output_name: "car_device".to_string(),
        },
        TestCase {
            id: 16,
            name: "composite_test".to_string(),
            path: "AADLSource/composite_test.aadl".to_string(),
            output_name: "composite_test".to_string(),
        },
        TestCase {
            id: 17,
            name: "drone".to_string(),
            path: "AADLSource/drone".to_string(),
            output_name: "drone".to_string(),
        },
        TestCase {
            id: 18,
            name: "cpp".to_string(),
            path: "AADLSource/cpp".to_string(),
            output_name: "cpp".to_string(),
        },
        TestCase {
            id: 19,
            name: "producer-consumer".to_string(),
            path: "AADLSource/producer-consumer".to_string(),
            output_name: "producer_consumer".to_string(),
        },
        TestCase {
            id: 20,
            name: "some-types".to_string(),
            path: "AADLSource/some-types".to_string(),
            output_name: "some-types".to_string(),
        },
        TestCase {
            id: 21,
            name: "some-types-stdint".to_string(),
            path: "AADLSource/some-types-stdint".to_string(),
            output_name: "some-types-stdint".to_string(),
        },
        TestCase {
            id: 22,
            name: "sunseeker".to_string(),
            path: "AADLSource/sunseeker".to_string(),
            output_name: "sunseeker".to_string(),
        },
        TestCase {
            id: 23,
            name: "flight-mgmt".to_string(),
            path: "AADLSource/flight-mgmt".to_string(),
            output_name: "flight-mgmt".to_string(),
        },
        TestCase {
            id: 24,
            name: "monitor".to_string(),
            path: "AADLSource/monitor".to_string(),
            output_name: "monitor".to_string(),
        },
    ];

    // Display available test cases
    println!("=== AADL2Rust Test Case Selection ===");
    println!("Please select an AADL file to test:");
    for test_case in &test_cases {
        println!("  {}: {}", test_case.id, test_case.name);
    }
    println!("  0: Exit");
    print!("Enter your choice (0-{}): ", test_cases.len());
    io::stdout().flush().unwrap();

    // Read user input
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");

    let choice: u32 = match input.trim().parse() {
        Ok(num) => num,
        Err(_) => {
            println!("Invalid input, please enter a number");
            return;
        }
    };

    if choice == 0 {
        println!("Program exited");
        return;
    }

    // Find the selected test case
    let selected_test = test_cases.iter().find(|tc| tc.id == choice);
    match selected_test {
        Some(test_case) => {
            println!("Selected: {}", test_case.name);
            println!("File path: {}", test_case.path);

            // Ensure the generate directory exists
            if fs::metadata("generate").is_err() {
                fs::create_dir("generate").expect("Failed to create generate directory");
            }

            // Process the selected test case
            process_test_case(test_case, None);
        }
        None => {
            println!(
                "Invalid selection, please enter a number between 0 and {}",
                test_cases.len()
            );
        }
    }
}

/// Explicit paths keep generated files inside the caller-selected project.
/// Existing application environment files live outside this directory.
fn generate_explicit_project(input: &Path, output: &Path, target: Option<&str>,
    artifacts: Option<&Path>, glue: Option<&Path>) -> Result<(), String> {
    let content = fs::read_to_string(input).map_err(|e| e.to_string())?;
    let pairs = AADLParser::parse(aadlight_parser::Rule::file, &content)
        .map_err(|e| format!("Parsing failed: {e}"))?;
    let packages = transform::AADLTransformer::transform_file(pairs.collect());
    let mut converter = AadlConverter::default();
    converter.set_available_packages(&packages);
    if let Some(target) = target {
        converter.set_external_ba_target(target, &packages)?;
        if artifacts.is_none() || glue.is_none() {
            return Err("external BA requires --ba-artifacts and --glue".into());
        }
    }
    // Refuse an unrelated existing directory rather than deleting user files.
    let marker = output.join(".aadl2rust-generated");
    if output.exists() && !marker.exists() && fs::read_dir(output).map_err(|e| e.to_string())?.next().is_some() {
        return Err("output directory is nonempty and is not an AADL2Rust generated project".into());
    }
    fs::create_dir_all(output.join("src")).map_err(|e| e.to_string())?;
    // The manifest records only files owned by this generator, so stale package
    // modules can be removed without recursively deleting the selected directory.
    if marker.exists() {
        for entry in fs::read_to_string(&marker).map_err(|e| e.to_string())?.lines() {
            if entry.contains('/') || entry.contains('\\') || !entry.ends_with(".rs") {
                return Err("invalid generated-file manifest".into());
            }
            let old = output.join("src").join(entry);
            if old.is_file() { fs::remove_file(old).map_err(|e| e.to_string())?; }
        }
    }
    let mut generated = Vec::new();
    for package in &packages {
        let module = merge_item_defs(converter.convert_package(package));
        let source = RustCodeGenerator::new().generate_module_code(&module);
        let filename = format!("{}.rs", package.name.to_string().replace("::", "_").to_lowercase());
        fs::write(output.join("src").join(&filename), source).map_err(|e| e.to_string())?;
        generated.push(filename);
    }
    let artifact_dir = artifacts.unwrap_or_else(|| input.parent().unwrap_or(Path::new(".")));
    let case = TestCase { id: 0, name: "explicit input".into(),
        path: artifact_dir.to_string_lossy().into_owned(), output_name: "ba_case".into() };
    assemble_rust_project_at(&case, &output.to_string_lossy(), target.is_some(), glue);
    generated.extend(["common_traits.rs", "posix.rs", "lib.rs", "main.rs", "ba_glue.rs"].map(String::from));
    fs::write(marker, generated.join("\n")).map_err(|e| e.to_string())?;
    Ok(())
}

fn process_test_case(test_case: &TestCase, external_ba: Option<&str>) {
    println!("Start processing: {}", test_case.name);

    let aadl_input = match read_aadl_inputs(&test_case.path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Failed to read file: {}", err);
            return;
        }
    };

    match AADLParser::parse(aadlight_parser::Rule::file, &aadl_input) {
        Ok(pairs) => {
            println!(
                "=== Parsing succeeded, total {} pairs ===",
                pairs.clone().count()
            );

            // Ensure generate/temp directory exists
            if fs::metadata("generate/temp").is_err() {
                fs::create_dir("generate/temp").expect("Failed to create generate/temp directory");
            }

            // Ensure generate/project directory exists
            if fs::metadata("generate/project").is_err() {
                fs::create_dir("generate/project")
                    .expect("Failed to create generate/project directory");
            }

            // Write parsing results to file
            let pairs_debug_path = format!(
                "generate/temp/{}_pairs_debug.txt",
                test_case.output_name.clone()
            );
            fs::write(&pairs_debug_path, format!("{:#?}", pairs)).unwrap();
            println!("Parsing result saved to: {}", pairs_debug_path);

            // Model size statistics
            ModelStatistics::from_pairs(pairs.clone(), test_case.output_name.clone())
                .unwrap_or_else(|e| {
                    eprintln!("Failed to write model statistics file: {}", e);
                });

            // Transform to AST
            let ast: Vec<ast::aadl_ast_cj::Package> =
                transform::AADLTransformer::transform_file(pairs.clone().collect());
            println!("=== Transformed into {} packages ===", ast.len());

            println!("\n==================================== Generating Rust Code ===================================");
            let mut converter = AadlConverter::default();
            // Register the complete case before converting individual packages so
            // external AADL libraries do not become unresolved Rust module imports.
            converter.set_available_packages(&ast);
            if let Some(target) = external_ba {
                if let Err(error) = converter.set_external_ba_target(target, &ast) {
                    eprintln!("External BA integration error: {error}");
                    std::process::exit(2);
                }
            }
            for package in ast.iter() {
                generate_rust_code_for_test_case(package, test_case, ast.len(), &mut converter);
            }

            // Generate Cargo.toml, build.rs, etc. for the project
            if external_ba.is_some() {
                assemble_external_ba_project(test_case);
            } else {
                assemble_rust_project(test_case);
            }
        }
        Err(e) => {
            eprintln!("Parsing failed: {}", e);
            eprintln!("Detailed parsing error: {:?}", e);
            eprintln!("Error location: {:?}", e.location);

            if let ErrorVariant::ParsingError {
                positives,
                negatives,
            } = e.variant
            {
                if !positives.is_empty() {
                    eprintln!("Expected rules: {:?}", positives);
                }
                if !negatives.is_empty() {
                    eprintln!("Unexpected rules: {:?}", negatives);
                }
            }

            eprintln!("Parsing failed, processing aborted");
            if external_ba.is_some() {
                // Integration runners must not mistake an unsupported parser feature for success.
                std::process::exit(2);
            }
        }
    }
}

// Read file or directory
fn read_aadl_inputs(path: &str) -> Result<String, std::io::Error> {
    let path = Path::new(path);

    if path.is_file() {
        // Original behavior: single file
        return fs::read_to_string(path);
    }

    if path.is_dir() {
        let mut merged = String::new();

        let mut entries: Vec<_> = fs::read_dir(path)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().map(|e| e == "aadl").unwrap_or(false))
            .collect();

        // Sort to ensure determinism
        entries.sort();

        for file in entries {
            let content = fs::read_to_string(&file)?;

            // —— Merge boundary handling ——
            merged.push_str("\n\n");
            merged.push_str("-- ================================\n");
            merged.push_str(&format!("-- merged from file: {}\n", file.display()));
            merged.push_str("-- ================================\n");
            merged.push('\n');

            merged.push_str(&content);
            merged.push('\n');
        }

        return Ok(merged);
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "path is neither file nor directory",
    ))
}

pub fn generate_rust_code_for_test_case(
    aadl_pkg: &Package,
    test_case: &TestCase,
    _number_of_packages: usize,
    converter: &mut AadlConverter,
) {
    // First-level translation: semantic conversion
    let rust_module = converter.convert_package(aadl_pkg);

    // Save intermediate AST to file
    let ast_debug_path = format!("generate/temp/{}_ast_debug.txt", test_case.output_name);
    fs::write(&ast_debug_path, format!("{:#?}", rust_module)).unwrap();

    let merge_rust_module = merge_item_defs(rust_module);

    let merged_ast_path = format!("generate/temp/{}_merged_ast.txt", test_case.output_name);
    fs::write(&merged_ast_path, format!("{:#?}", merge_rust_module)).unwrap();

    let mut code_generator = RustCodeGenerator::new();
    let rust_code = code_generator.generate_module_code(&merge_rust_module);

    // Determine output path based on package name
    let package_name = aadl_pkg.name.to_string().replace("::", "_").to_lowercase();
    let output_dir = format!("generate/project/{}/src", test_case.output_name);

    fs::create_dir_all(&output_dir).expect("Failed to create output directory");
    let output_path = format!("{}/{}.rs", output_dir, package_name);

    fs::write(&output_path, rust_code).expect("Failed to write Rust code");
    // println!(
    //     "Rust code generated (package: {}): {}",
    //     package_name, output_path
    // );
}
