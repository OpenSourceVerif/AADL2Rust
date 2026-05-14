use aadl_intermediate::*;

use crate::aadl_ast2rust_code::converter::AadlConverter;
use crate::aadl_ast2rust_code::tool::*;
use crate::ast::aadl_ast_cj::*;

pub fn convert_system_component(_temp_converter: &AadlConverter, comp: &ComponentType) -> Vec<Item> {
    let mut items = Vec::new();

    // 1. Struct definition - system types contain no fields because fields are defined in the implementation
    let struct_def = StructDef {
        name: format!("{}System", to_upper_camel_case(&comp.identifier)),
        fields: vec![], // System types contain no fields
        generics: Vec::new(),
        derives: vec!["Debug".to_string()],
        docs: vec![format!("// AADL System: {}", comp.identifier)],
        vis: Visibility::Public,
    };
    items.push(Item::Struct(struct_def));

    items
}
