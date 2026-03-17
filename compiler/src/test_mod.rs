use crate::aadl_ast2rust_code::converter::AadlConverter;
use crate::aadl_ast2rust_code::intermediate_print::RustCodeGenerator;
use crate::aadl_ast2rust_code::merge_utils::merge_item_defs;
use crate::aadlight_parser::aadl::{AADLParser, Rule};
use crate::ast::aadl_ast_cj::Package;
use crate::transform::AADLTransformer;

use diffy::{create_patch, PatchFormatter};
use pest::Parser;
use std::fs;
use std::path::Path;

const BASELINE_ROOT: &str = "generate/code";
const TEST_OUTPUT_ROOT: &str = "generate_test/code";

// 测试用例描述
pub struct TestCase {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub output_name: String,
}

// 所有要回归测试的 AADL 模型
pub fn all_test_cases() -> Vec<TestCase> {
    vec![
        TestCase {
            id: 1,
            name: "PingPong (Ocarina)".into(),
            path: "AADLSource/pingpong_ocarina.aadl".into(),
            output_name: "pingpong_ocarina".into(),
        },
        TestCase {
            id: 3,
            name: "RMA".into(),
            path: "AADLSource/rma.aadl".into(),
            output_name: "rma".into(),
        },
        TestCase {
            id: 4,
            name: "Toy".into(),
            path: "AADLSource/toy.aadl".into(),
            output_name: "toy".into(),
        },
        TestCase {
            id: 5,
            name: "Robot(v1)".into(),
            path: "AADLSource/robotv1.aadl".into(),
            output_name: "robotv1".into(),
        },
        TestCase {
            id: 6,
            name: "Robot(v2)".into(),
            path: "AADLSource/robotv2.aadl".into(),
            output_name: "robotv2".into(),
        },
        TestCase {
            id: 7,
            name: "RMS".into(),
            path: "AADLSource/rms.aadl".into(),
            output_name: "rms".into(),
        },
        TestCase {
            id: 9,
            name: "base_types".into(),
            path: "AADLSource/base_types.aadl".into(),
            output_name: "base_types".into(),
        },
        TestCase {
            id: 10,
            name: "composite_types".into(),
            path: "AADLSource/composite_types.aadl".into(),
            output_name: "composite_types".into(),
        },
        TestCase {
            id: 11,
            name: "car".into(),
            path: "AADLSource/car.aadl".into(),
            output_name: "car".into(),
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
            name: "pingpong_example".into(),
            path: "AADLSource/pingpong_example.aadl".into(),
            output_name: "pingpong_example".into(),
        },
        TestCase {
            id: 15,
            name: "pingpong_timed".into(),
            path: "AADLSource/pingpong_timed.aadl".into(),
            output_name: "pingpong_timed".into(),
        },
        TestCase {
            id: 16,
            name: "pingpong_aperiodic".into(),
            path: "AADLSource/pingpong_aperiodic.aadl".into(),
            output_name: "pingpong_aperiodic".into(),
        },
        TestCase {
            id: 17,
            name: "pingpong_no_scheduling".into(),
            path: "AADLSource/pingpong_no_scheduling.aadl".into(),
            output_name: "pingpong_no_scheduling".into(),
        },
        TestCase {
            id: 18,
            name: "composite_test".into(),
            path: "AADLSource/composite_test.aadl".into(),
            output_name: "composite_test".into(),
        },
        TestCase {
            id: 19,
            name: "toy_test".into(),
            path: "AADLSource/toy_test.aadl".into(),
            output_name: "toy_test".into(),
        }
        
    ]
}

// 顶层入口：在 tests/all_aadl_models.rs 里调用它
pub fn run_all_test_cases() {
    ensure_dirs();

    let base = env!("CARGO_MANIFEST_DIR");
    let mut converter = AadlConverter::default();

    for tc in all_test_cases() {
        let full_path = format!("{}\\{}", base, tc.path);
        let result = run_single_test_case(&tc, &full_path, &mut converter);

        match result {
            Ok(_) => println!("模型 {} 测试成功", tc.name),
            Err(e) => println!("错误: 模型 {} 测试失败: {}", tc.name, e),
        }
    }
}

// 确保输出目录存在
fn ensure_dirs() {
    let base = env!("CARGO_MANIFEST_DIR");
    fs::create_dir_all(format!("{}\\{}", base, BASELINE_ROOT)).ok();
    fs::create_dir_all(format!("{}\\{}", base, TEST_OUTPUT_ROOT)).ok();
    fs::create_dir_all(format!("{}\\generate_test", base)).ok(); // 存 HTML diff
}

// 单个测试用例：AADL → AST → Rust 生成
fn run_single_test_case(
    test_case: &TestCase,
    full_path: &str,
    converter: &mut AadlConverter,
) -> Result<(), String> {
    let aadl_input = fs::read_to_string(full_path)
        .map_err(|e| format!("读取文件失败 [{}]: {}", full_path, e))?;

    let pairs = AADLParser::parse(Rule::file, &aadl_input)
        .map_err(|e| format!("解析失败 [{}]: {}", full_path, e))?;

    let ast: Vec<Package> = AADLTransformer::transform_file(pairs.clone().collect());

    for pkg in ast.iter() {
        generate_rust_code_for_test_case(pkg, test_case, ast.len(), converter)?;
    }

    Ok(())
}

// AADL 包 → Rust 模块代码 + 基准对比 + HTML diff 报告
fn generate_rust_code_for_test_case(
    aadl_pkg: &Package,
    test_case: &TestCase,
    number_of_packages: usize,
    converter: &mut AadlConverter,
) -> Result<(), String> {
    let base = env!("CARGO_MANIFEST_DIR");

    // 1. AADL → Rust module
    let rust_module = converter.convert_package(aadl_pkg);
    let merge_rust_module = merge_item_defs(rust_module);

    let mut code_generator = RustCodeGenerator::new();
    let rust_code = code_generator.generate_module_code(&merge_rust_module);

    let package_name = aadl_pkg.name.to_string().replace("::", "_");

    // 2. 测试输出路径：generate_test/code/...
    let test_output_path = if number_of_packages == 1 {
        format!("{}\\{}\\{}.rs", base, TEST_OUTPUT_ROOT, test_case.output_name)
    } else {
        let dir = format!("{}\\{}\\{}", base, TEST_OUTPUT_ROOT, test_case.output_name);
        fs::create_dir_all(&dir).ok();
        format!("{}\\{}.rs", dir, package_name)
    };

    fs::write(&test_output_path, &rust_code)
        .map_err(|e| format!("写入测试生成文件失败 [{}]: {}", test_output_path, e))?;

    // 3. baseline 路径：generate/code/...
    let baseline_path = if number_of_packages == 1 {
        format!("{}\\{}\\{}.rs", base, BASELINE_ROOT, test_case.output_name)
    } else {
        format!(
            "{}\\{}\\{}\\{}.rs",
            base, BASELINE_ROOT, test_case.output_name, package_name
        )
    };

    // 4. 如有基准文件，做 diff，并输出 HTML 报告
    if Path::new(&baseline_path).exists() {
        let baseline = fs::read_to_string(&baseline_path)
            .map_err(|e| format!("读取基准文件失败 [{}]: {}", baseline_path, e))?;

        if baseline != rust_code {
            // 4.1 文本 diff
            let patch = create_patch(&baseline, &rust_code);
            let text_diff = PatchFormatter::new().fmt_patch(&patch).to_string();

            // 4.2 把文本 diff 转成简单 HTML（不使用 ANSI 颜色）
            let html_body = diff_to_html(&text_diff);
            let html_page = format!(
                r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<title>Diff for {name}</title>
<style>
body {{
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  background: #1e1e1e;
  color: #d4d4d4;
}}
pre {{
  white-space: pre-wrap;
}}
.line-add    {{ background-color: #003800; color: #c8ffc8; }}
.line-remove {{ background-color: #3f0001; color: #ffcccc; }}
.line-header {{ color: #4fc1ff; font-weight: bold; }}
</style>
</head>
<body>
<h2>Diff for {name}</h2>
<pre>
{body}
</pre>
</body>
</html>
"#,
                name = test_case.name,
                body = html_body
            );

            // 4.3 HTML 报告路径
            let html_path = format!(
                "{}\\generate_test\\html\\diff_{}.html",
                base, test_case.output_name
            );

            fs::write(&html_path, html_page)
                .map_err(|e| format!("写入 HTML diff 失败 [{}]: {}", html_path, e))?;

            println!(
                "警告: 模型 {} 生成代码与基准不同 → 报告: {}",
                test_case.name, html_path
            );

            // Windows 上自动打开浏览器（可选）
            // #[cfg(target_os = "windows")]
            // {
            //     let _ = open::that(&html_path);
            // }
        } else {
            println!("模型 {} 输出一致", test_case.name);
        }
    } else {
        println!("注意: 未找到基准文件，跳过比较: {}", baseline_path);
    }

    Ok(())
}

// ===== 辅助函数：把纯文本 diff 转为 HTML，粗糙高亮 + / - / @@ 等 =====

fn diff_to_html(text_diff: &str) -> String {
    let mut out = String::new();

    for line in text_diff.lines() {
        let class = if line.starts_with('+') {
            "line-add"
        } else if line.starts_with('-') {
            "line-remove"
        } else if line.starts_with("@@") || line.starts_with("---") || line.starts_with("+++") {
            "line-header"
        } else {
            ""
        };

        let escaped = html_escape(line);

        if class.is_empty() {
            out.push_str(&escaped);
            out.push('\n');
        } else {
            out.push_str(&format!(r#"<span class="{class}">{escaped}</span>"#));
            out.push('\n');
        }
    }

    out
}

// 极简单的 HTML 转义
fn html_escape(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}
