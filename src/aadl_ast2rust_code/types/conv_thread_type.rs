use aadl_intermediate::*;

use crate::aadl_ast2rust_code::aadl_property::*;
use crate::aadl_ast2rust_code::converter::AadlConverter;
use crate::aadl_ast2rust_code::tool::*;
use crate::ast::aadl_ast_cj::*;
use std::collections::HashMap;

pub fn convert_thread_component(
    temp_converter: &mut AadlConverter,
    comp: &ComponentType,
) -> Vec<Item> {
    let mut items = Vec::new();

    // 1. Struct definition
    let mut fields = temp_converter.convert_type_features(&comp.features, comp.identifier.clone()); // Feature list
                                                                                                    // Merge properties into fields as well (no longer stored separately in `properties`)
                                                                                                    // Collect properties: add typed fields into `fields` and record values into `thread_field_values`
    let mut value_map: HashMap<String, StruPropertyValue> = HashMap::new();
    let mut type_map: HashMap<String, Type> = HashMap::new();

    // Also add feature fields into thread_field_values and thread_field_types; decide values based on field types, and record the field types of Shared fields
    for field in &fields {
        let field_value = match &field.ty {
            Type::Named(type_name) => {
                if type_name.ends_with("Shared") {
                    StruPropertyValue::Custom(field.name.clone())
                } else {
                    StruPropertyValue::None
                }
            }
            _ => StruPropertyValue::None,
        };
        value_map.insert(field.name.clone(), field_value);
        type_map.insert(field.name.clone(), field.ty.clone());
    }
    if let PropertyClause::Properties(props) = &comp.properties {
        for prop in props {
            if let Property::BasicProperty(bp) = prop {
                if let Some(val) = temp_converter.parse_property_value(&bp.value) {
                    let name_lc = bp.identifier.name.to_lowercase();
                    let ty_name = temp_converter.type_for_property(&val);
                    fields.push(Field {
                        name: name_lc.clone(),
                        ty: Type::Named(ty_name),
                        docs: vec![format!("// AADL property: {}", bp.identifier.name)],
                        attrs: Vec::new(),
                    });
                    value_map.insert(name_lc, val);
                }
            }
        }
    }
    // Add CPU ID field
    fields.push(Field {
        name: "cpu_id".to_string(),
        ty: Type::Named("isize".to_string()),
        docs: vec!["// Struct adds CPU ID".to_string()],
        attrs: Vec::new(),
    });

    let struct_name = format!("{}Thread", to_upper_camel_case(&comp.identifier));
    // Save the property values corresponding to fields (only for fields that have values)
    if !value_map.is_empty() {
        temp_converter
            .thread_field_values
            .insert(struct_name.clone(), value_map);
    }
    // Save the types corresponding to fields (only for fields that have values)
    if !type_map.is_empty() {
        temp_converter
            .thread_field_types
            .insert(struct_name.clone(), type_map);
    }

    let struct_def = StructDef {
        name: struct_name,
        fields,                 // Feature list
        generics: Vec::new(),
        derives: vec!["Debug".to_string()],
        docs: temp_converter.create_component_type_docs(comp),
        vis: Visibility::Public, // Default: public
    };
    items.push(Item::Struct(struct_def));
    // 2. Impl block
    // if let Some(impl_block) = self.create_threadtype_impl(comp) {
    //     items.push(Item::Impl(impl_block));
    // }

    items
}
