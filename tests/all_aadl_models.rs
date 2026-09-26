// use compiler::test_mod;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn all_aadl_models_should_generate_rust_code() {
    //test_mod::run_all_test_cases();
    run_all_case_folders();
    assert_data_case_uses_compilable_string_and_union_types();
    assert_rma_skips_unavailable_processor_module();
}

fn assert_data_case_uses_compilable_string_and_union_types() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_src = manifest_dir.join("generate").join("project").join("data").join("src");

    let base_types = std::fs::read_to_string(data_src.join("base_types.rs"))
        .expect("the data case must generate base_types.rs");
    assert!(
        base_types.contains("pub type String = std::string::String;"),
        "the generated AADL String alias must use an absolute Rust path"
    );

    let composite_types = std::fs::read_to_string(data_src.join("base_types_example_types.rs"))
        .expect("the data case must generate base_types_example_types.rs");
    for union_name in ["A_Union1", "A_Union2"] {
        let expected = format!("#[derive(Copy, Clone)]\npub union {union_name}");
        assert!(
            composite_types.contains(&expected),
            "generated union {union_name} must derive Copy and Clone without Debug"
        );
    }
}

fn assert_rma_skips_unavailable_processor_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let generated_rma = manifest_dir
        .join("generate")
        .join("project")
        .join("rma")
        .join("src")
        .join("rmaaadl.rs");
    let source =
        std::fs::read_to_string(generated_rma).expect("the rma case must generate rmaaadl.rs");

    assert!(
        !source.contains("use crate::processors::*;"),
        "an unavailable external AADL package must not become a Rust module import"
    );
}

pub fn all_case_folders() -> Vec<&'static str> {
    vec![
        "aocs/",
        "ardupilot/",
        "arinc653_annex/",
        "arrays/",
        "bit_codec/",
        "building_control_gen_mixed/",
        "car/",
        "cpp/",
        "data/",
        "fcs/",
        "file-store/",
        "flight-mgmt/",
        "isr/",
        "latency/",
        "line_follower/",
        "lua/",
        "minepump/",
        "minepump_ba/",
        "mixin/",
        "monitor/",
        "mosart/",
        "nested_feature_groups/",
        "packet-store/",
        "pathfinder_system/",
        "periodicDispatch/",
        "ping_spark/",
        "pingpong/",
        "pingpong_timed/",
        "priority_test/",
        "producer-consumer/",
        "producer_consumer_ba/",
        "producer_filter_consumer_mixed/",
        "producer_filter_consumer_periodic/",
        "producer_filter_consumer_sporadic/",
        "radar/",
        "ravenscar/",
        "rma/",
        "rms/",
        "robotv1/",
        "robotv2/",
        "round_robin/",
        "rpc/",
        "satellite/",
        "stm32discovery_ada/",
        "sunseeker/",
        "test_data_port_periodic_domains/",
        "test_event_data_port_periodic_domains/",
        "test_event_port_periodic_domains/",
        "testdpmon-periodic/",
        "testdpmon/",
        "testepmon/",
        "testevent/",
        "testshare/",
        "testsubprogram/",
        "time_triggered/",
        "toy/",
        "wms/",
    ]
}

pub fn run_all_case_folders() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo_bin = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let mut failures: Vec<String> = Vec::new();
    let mut passed: usize = 0;
    let total = all_case_folders().len();

    println!("\n=== AADL2Rust: codegen smoke test ({} cases) ===", total);

    for (idx, raw) in all_case_folders().iter().enumerate() {
        let folder = normalize_folder(raw);

        let p1 = manifest_dir.join("generate").join("project").join(&folder);
        let p2 = manifest_dir.join(&folder);

        if !p1.exists() && !p2.exists() {
            eprintln!(
                "warning: case folder '{}' not found at '{}' or '{}'",
                folder,
                p1.display(),
                p2.display()
            );
        }

        print!("[{}/{}] {:<40} ... ", idx + 1, total, folder);

        let ok = run_single_case(&cargo_bin, &manifest_dir, &folder);

        if ok {
            passed += 1;
            println!("OK");
        } else {
            println!("FAILED");
            failures.push(folder);
        }
    }

    println!("\n=== Summary ===");
    println!("  Passed: {}", passed);
    println!("  Failed: {}", failures.len());

    if !failures.is_empty() {
        println!("  Failed cases: {}", failures.join(", "));
        panic!("Some cases failed.");
    }
}

fn run_single_case(cargo_bin: &str, workdir: &Path, folder: &str) -> bool {
    let mut cmd = Command::new(cargo_bin);
    cmd.current_dir(workdir)
        .arg("run")
        .arg("--quiet")
        .arg("--")
        .arg("--input")
        .arg(folder);

    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: failed to spawn cargo for case '{}': {}", folder, e);
            return false;
        }
    };

    if output.status.success() {
        true
    } else {
        eprintln!("case '{}' FAILED (status: {})", folder, output.status);

        let out = String::from_utf8_lossy(&output.stdout);
        let err = String::from_utf8_lossy(&output.stderr);

        if !out.trim().is_empty() {
            eprintln!("--- stdout ---\n{}", out);
        }
        if !err.trim().is_empty() {
            eprintln!("--- stderr ---\n{}", err);
        }

        false
    }
}

fn normalize_folder(raw: &str) -> String {
    raw.trim()
        .trim_end_matches('/')
        .trim_end_matches('\\')
        .to_string()
}
