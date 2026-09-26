//! Payload layout regressions: parse real AADL properties, generate the native
//! aliases and thread fields, then type-check and run channels without casts.
use aadl_intermediate::{Item, RustCodeGenerator, Type};
use compiler::aadl_ast2rust_code::converter::AadlConverter;
use compiler::aadlight_parser::{AADLParser, Rule};
use compiler::transform::AADLTransformer;
use pest::Parser;

fn generated_payload_types(declarations: &str) -> String {
    let source = format!("package Payload_Probe public with Data_Model; {declarations} end Payload_Probe;");
    let parsed = AADLParser::parse(Rule::file, &source).expect("payload declarations must parse");
    let packages = AADLTransformer::transform_file(parsed.collect());
    let mut converter = AadlConverter::default();
    converter.set_available_packages(&packages);
    let mut module = converter.convert_package(&packages[0]);
    // Keep the actual generated payload declarations and native port fields.
    // Runtime prelude imports and scheduling code are unrelated to this ABI test.
    module.items.retain(|item| matches!(item, Item::TypeAlias(_) | Item::Struct(_)));
    let generated = RustCodeGenerator::new().generate_module_code(&module);
    syn::parse_file(&generated).expect("generated aliases and fields must be valid Rust");
    generated
}

fn compile_and_run_payloads(generated: &str, fields: &[String], checks: &str) {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let directory = std::env::temp_dir().join(format!("aadl2rust-payload-types-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let source = directory.join("payloads.rs");
    let binary = directory.join(format!("payloads{}", std::env::consts::EXE_SUFFIX));
    let support = r#"
use std::sync::mpsc::{Receiver, Sender};
fn exchange<T: Clone + std::fmt::Debug + PartialEq>(slot: &mut Option<Receiver<T>>, value: T) {
    let (sender, receiver) = std::sync::mpsc::channel::<T>();
    sender.send(value.clone()).unwrap();
    *slot = Some(receiver);
    assert_eq!(slot.as_ref().unwrap().recv().unwrap(), value);
}
"#;
    let initial_fields = fields.iter().map(|field| format!("{field}: None")).collect::<Vec<_>>().join(",");
    std::fs::write(&source, format!(
        "{support}\n{generated}\nfn main() {{ let mut worker = WorkerThread {{ {initial_fields}, cpu_id: 0 }}; {checks} }}"
    )).unwrap();
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let compile = std::process::Command::new(rustc)
        .args(["--edition=2021", "-Awarnings"])
        .arg(&source).arg("-o").arg(&binary).output().unwrap();
    assert!(compile.status.success(), "generated payloads failed to compile at {}:\n{}", source.display(), String::from_utf8_lossy(&compile.stderr));
    let run = std::process::Command::new(&binary).output().unwrap();
    assert!(run.status.success(), "payload channel check failed: {}", String::from_utf8_lossy(&run.stderr));
}

#[test]
fn all_integer_widths_and_boolean_reach_scalar_and_array_ports() {
    let mut declarations = String::new();
    let mut features = String::new();
    let mut fields = Vec::new();
    let mut checks = String::new();
    for (prefix, representation) in [("i", "Signed"), ("u", "Unsigned")] {
        for (bits, bytes) in [(8, 1), (16, 2), (32, 4), (64, 8)] {
            let scalar = format!("Payload_{prefix}{bits}");
            let vector = format!("Vector_{prefix}{bits}");
            declarations.push_str(&format!(r#"
data {scalar}
properties
  Data_Model::Data_Representation => Integer;
  Source_Data_Size => {bytes} Bytes;
  Data_Model::Number_Representation => {representation};
end {scalar};
data {vector}
properties
  Data_Model::Data_Representation => Array;
  Data_Model::Dimension => (3);
  Data_Model::Base_Type => (classifier ({scalar}));
end {vector};
"#));
            let scalar_field = format!("scalar_{prefix}{bits}");
            let vector_field = format!("vector_{prefix}{bits}");
            features.push_str(&format!("{scalar_field}: in event data port {scalar}; {vector_field}: in event data port {vector};"));
            checks.push_str(&format!(
                "exchange::<{prefix}{bits}>(&mut worker.{scalar_field}, {prefix}{bits}::MAX); \
                 exchange::<[{prefix}{bits}; 3]>(&mut worker.{vector_field}, [{prefix}{bits}::MIN, 0, {prefix}{bits}::MAX]);"
            ));
            fields.extend([scalar_field, vector_field]);
        }
    }
    declarations.push_str(r#"
data Logical properties Data_Model::Data_Representation => Boolean; end Logical;
data LogicalVector properties
  Data_Model::Data_Representation => Array;
  Data_Model::Dimension => (3);
  Data_Model::Base_Type => (classifier (Logical));
end LogicalVector;
"#);
    features.push_str("scalar_bool: in event data port Logical; vector_bool: in event data port LogicalVector;");
    fields.extend(["scalar_bool".to_string(), "vector_bool".to_string()]);
    checks.push_str("exchange::<bool>(&mut worker.scalar_bool, true); exchange::<[bool; 3]>(&mut worker.vector_bool, [true, false, true]);");
    declarations.push_str(&format!("thread Worker features {features} end Worker;"));
    let generated = generated_payload_types(&declarations);
    compile_and_run_payloads(&generated, &fields, &checks);
}

#[test]
fn forward_nested_arrays_keep_classifier_references_and_dimensions() {
    let generated = generated_payload_types(r#"
data Matrix properties
  Data_Model::Data_Representation => Array;
  Data_Model::Dimension => (2);
  Data_Model::Base_Type => (classifier (Vector));
end Matrix;
data Vector properties
  Data_Model::Data_Representation => Array;
  Data_Model::Dimension => (3);
  Data_Model::Base_Type => (classifier (Payload_Probe::Leaf));
end Vector;
data Leaf properties
  Data_Model::Data_Representation => Integer;
  Source_Data_Size => 8 Bytes;
  Data_Model::Number_Representation => Unsigned;
end Leaf;
data DirectMatrix properties
  Data_Model::Data_Representation => Array;
  Data_Model::Dimension => (2, 3);
  Data_Model::Base_Type => classifier (Leaf);
end DirectMatrix;
thread Worker features
  nested: in event data port Matrix;
  direct: in event data port DirectMatrix;
end Worker;
"#);
    compile_and_run_payloads(&generated, &["nested".into(), "direct".into()],
        "exchange::<[[u64; 3]; 2]>(&mut worker.nested, [[0, 1, 2], [3, 4, u64::MAX]]); \
         exchange::<[[u64; 3]; 2]>(&mut worker.direct, [[0, 1, 2], [3, 4, u64::MAX]]);");
}

#[test]
fn explicit_storage_properties_override_cached_names_and_keep_units() {
    let generated = generated_payload_types(r#"
data Integer properties
  Data_Model::Data_Representation => Integer;
  Data_Size => 1 Bytes;
  Source_Data_Size => 64 Bits;
end Integer;
data SmallUnsigned properties
  Data_Model::Number_Representation => Unsigned;
  Data_Size => 2 Bytes;
  Data_Model::Data_Representation => Integer;
end SmallUnsigned;
data DefaultInteger properties Data_Model::Data_Representation => Integer; end DefaultInteger;
thread Worker features
  explicit_size: in event data port Integer;
  general_size: in event data port SmallUnsigned;
  default_size: in event data port DefaultInteger;
end Worker;
"#);
    compile_and_run_payloads(&generated,
        &["explicit_size".into(), "general_size".into(), "default_size".into()],
        "exchange::<i64>(&mut worker.explicit_size, i64::MIN); \
         exchange::<u16>(&mut worker.general_size, u16::MAX); \
         exchange::<i32>(&mut worker.default_size, i32::MIN);");
}

#[test]
#[should_panic(expected = "unsupported integer storage width for Packed: 24 bits")]
fn unsupported_integer_storage_width_does_not_fall_back_to_i32() {
    generated_payload_types(r#"
data Packed properties
  Data_Model::Data_Representation => Integer;
  Source_Data_Size => 24 Bits;
end Packed;
"#);
}

#[test]
fn base_types_library_keeps_existing_inherited_primitive_mappings() {
    let parsed = AADLParser::parse(Rule::file, include_str!("../AADLSource/data/base_types.aadl")).unwrap();
    let packages = AADLTransformer::transform_file(parsed.collect());
    let mut converter = AadlConverter::default();
    converter.set_available_packages(&packages);
    converter.convert_package(&packages[0]);
    for (classifier, expected) in [
        ("boolean", "bool"), ("integer", "i32"),
        ("integer_8", "i8"), ("integer_16", "i16"),
        ("integer_32", "i32"), ("integer_64", "i64"),
        ("unsigned_8", "u8"), ("unsigned_16", "u16"),
        ("unsigned_32", "u32"), ("unsigned_64", "u64"),
        ("float_32", "f32"), ("float_64", "f64"),
    ] {
        assert!(matches!(converter.type_mappings.get(classifier), Some(Type::Named(name)) if name == expected),
            "library classifier {classifier} must retain {expected}");
    }
}
