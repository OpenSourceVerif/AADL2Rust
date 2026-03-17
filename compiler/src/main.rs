#![allow(unused_imports)]

mod aadl_ast2rust_code;
pub mod aadlight_parser;
mod ast;
pub mod model_statistics;
// pub mod printmessage;
pub mod transform;
pub mod transform_annex;
// pub mod build_project_tool;

use compiler::printmessage::*;
use aadl_ast2rust_code::intermediate_print::*;
use aadl_ast2rust_code::merge_utils::*;
// use aadlight_parser::AADLParser;
use aadlight_parser::aadl::{AADLParser, Rule};
use clap::Parser as ClapParser;
use crate::model_statistics::*;
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
}

fn main() {
    // ================= CLI 模式：--input <folder> =================
    let args = Args::parse();

    if let Some(input_name) = args.input.as_deref() {
        // 用命令行输入的文件夹名，直接构造一个 TestCase
        let test_case = TestCase {
            id: 0, // CLI 模式下 id 无意义
            name: format!("CLI input ({})", input_name),
            path: format!("AADLSource/{}", input_name),
            output_name: input_name.to_string(),
        };

        println!("CLI mode");
        println!("输入路径: {}", test_case.path);
        println!("输出名称: {}", test_case.output_name);

        // 确保 generate 目录存在
        if fs::metadata("generate").is_err() {
            fs::create_dir("generate").expect("无法创建 generate 目录");
        }

        let output_dir = format!("generate/project/{}/src", test_case.output_name);
        if std::path::Path::new(&output_dir).exists() {
            println!("清理目录:{}",output_dir);
            fs::remove_dir_all(&output_dir).unwrap();
        }
        
        process_test_case(&test_case);
        return; // 不进入下面的交互逻辑
    }

    // ================= 交互式菜单模式 =================
    // 定义可用的测试用例
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
            path: "AADLSource/drone.aadl".to_string(),
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

    // 显示可用的测试用例
    println!("=== AADL2Rust 测试用例选择 ===");
    println!("请选择要测试的AADL文件:");
    for test_case in &test_cases {
        println!("  {}: {}", test_case.id, test_case.name);
    }
    println!("  0: 退出程序");
    print!("请输入选择 (0-{}): ", test_cases.len());
    io::stdout().flush().unwrap();

    // 读取用户输入
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("无法读取输入");

    let choice: u32 = match input.trim().parse() {
        Ok(num) => num,
        Err(_) => {
            println!("无效输入，请输入数字");
            return;
        }
    };

    if choice == 0 {
        println!("程序退出");
        return;
    }

    // 查找选择的测试用例
    let selected_test = test_cases.iter().find(|tc| tc.id == choice);
    match selected_test {
        Some(test_case) => {
            println!("选择: {}", test_case.name);
            println!("文件路径: {}", test_case.path);

            // 确保generate目录存在
            if fs::metadata("generate").is_err() {
                fs::create_dir("generate").expect("无法创建generate目录");
            }

            // 处理选中的测试用例
            process_test_case(test_case);
        }
        None => {
            println!("无效的选择，请输入 0-{} 之间的数字", test_cases.len());
        }
    }
}

fn process_test_case(test_case: &TestCase) {
    println!("开始处理: {}", test_case.name);

    let aadl_input = match read_aadl_inputs(&test_case.path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("读取文件失败: {}", err);
            return;
        }
    };

    match AADLParser::parse(Rule::file, &aadl_input) {
        Ok(pairs) => {
            println!("=== 解析成功，共 {} 个pair ===", pairs.clone().count());

            // 确保generate/temp目录存在
            if fs::metadata("generate/temp").is_err() {
                fs::create_dir("generate/temp").expect("无法创建generate/temp目录");
            }

            // 确保generate/project目录存在
            if fs::metadata("generate/project").is_err() {
                fs::create_dir("generate/project").expect("无法创建generate/project目录");
            }

            // 将解析结果写入文件
            let pairs_debug_path = format!(
                "generate/temp/{}_pairs_debug.txt",
                test_case.output_name.clone()
            );
            fs::write(&pairs_debug_path, format!("{:#?}", pairs)).unwrap();
            println!("解析结果已保存到: {}", pairs_debug_path);

            // 模型规模统计
            ModelStatistics::from_pairs(pairs.clone(), test_case.output_name.clone())
                .unwrap_or_else(|e| {
                    eprintln!("写入模型统计文件失败: {}", e);
                });

            // 转换到AST
            let ast: Vec<ast::aadl_ast_cj::Package> =
                transform::AADLTransformer::transform_file(pairs.clone().collect());
            println!("=== 转换得到 {} 个package ===", ast.len());

            // 打印AST
            // println!("\n================================== AST ==================================");
            // print_ast(&ast);

            println!("\n==================================== 生成Rust代码 ===================================");
            let mut converter = AadlConverter::default();
            for package in ast.iter() {
                generate_rust_code_for_test_case(package, test_case, ast.len(), &mut converter);
            }

            // 生成项目所需的 Cargo.toml, build.rs 等文件
            assemble_rust_project(test_case);

            // println!("✅ 代码生成完成！输出文件保存在 generate/ 目录下");
        }
        Err(e) => {
            eprintln!("解析失败: {}", e);
            // 打印详细的错误信息
            eprintln!("解析错误: {:?}", e);

            // 显示错误位置和上下文
            eprintln!("错误位置: {:?}", e.location);

            // 显示期望的规则
            if let ErrorVariant::ParsingError {
                positives,
                negatives,
            } = e.variant
            {
                if !positives.is_empty() {
                    eprintln!("期望匹配的规则: {:?}", positives);
                }
                if !negatives.is_empty() {
                    eprintln!("不应该匹配的规则: {:?}", negatives);
                }
            }

            eprintln!("解析失败，无法继续处理");
        }
    }
}

// 读取文件/文件夹
fn read_aadl_inputs(path: &str) -> Result<String, std::io::Error> {
    let path = Path::new(path);

    if path.is_file() {
        // 原有行为：单文件
        return fs::read_to_string(path);
    }

    if path.is_dir() {
        let mut merged = String::new();

        let mut entries: Vec<_> = fs::read_dir(path)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().map(|e| e == "aadl").unwrap_or(false))
            .collect();

        // 排序，保证确定性
        entries.sort();

        for file in entries {
            let content = fs::read_to_string(&file)?;

            // —— 合并边界处理 ——
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
    // 第一级转换：语义转换
    //let mut converter = AadlConverter::default();

    let rust_module = converter.convert_package(aadl_pkg);
    //println!("\n==================================== rust_module ===================================");

    // 保存中间AST到文件
    let ast_debug_path = format!("generate/temp/{}_ast_debug.txt", test_case.output_name);
    fs::write(&ast_debug_path, format!("{:#?}", rust_module)).unwrap();
    //println!("中间AST已保存到: {}", ast_debug_path);

    let merge_rust_module = merge_item_defs(rust_module);

    let merged_ast_path = format!("generate/temp/{}_merged_ast.txt", test_case.output_name);
    fs::write(&merged_ast_path, format!("{:#?}", merge_rust_module)).unwrap();
    //println!("合并后AST已保存到: {}", merged_ast_path);

    let mut code_generator = RustCodeGenerator::new();
    let rust_code = code_generator.generate_module_code(&merge_rust_module);

    // 根据包数量决定输出路径
    let package_name = aadl_pkg.name.to_string().replace("::", "_").to_lowercase();
    // let output_path = if number_of_packages == 1 {
    //     // 如果只有一个包，直接生成文件，文件名是test_case
    //     format!("generate/code/{}.rs", test_case.output_name)
    // } else {
    // 使用文件夹结构
    let output_dir = format!("generate/project/{}/src", test_case.output_name);

    fs::create_dir_all(&output_dir).expect("Failed to create output directory");
    let output_path = format!("{}/{}.rs", output_dir, package_name);
    // };

    fs::write(&output_path, rust_code).expect("Failed to write Rust code");
    println!("Rust代码已生成 (包: {}): {}", package_name, output_path);

}

