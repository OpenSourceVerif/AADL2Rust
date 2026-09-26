#![allow(clippy::collapsible_match)]
use aadl_intermediate::*;
use crate::ast::aadl_ast_cj::*;
use std::collections::HashMap;

pub fn convert_data_component(
    type_mappings: &mut HashMap<String, Type>,
    comp: &ComponentType,
    data_comp_type: &mut HashMap<String, String>,
) -> Vec<Item> {
    let target_type = determine_data_type(type_mappings, comp);
    // println!("target_type:{:?}",target_type);
    // When determine_data_type returns a struct type, generate a struct definition
    // When determine_data_type returns a union type, generate a union definition
    // When determine_data_type returns an enum type, generate an enum definition
    // TODO: when a complex type is determined, the component identifier should be added into type_mappings
    if let Type::Named(unit_type) = &target_type {
        if unit_type.to_lowercase() == "struct" {
            // Extract the property list from component properties
            if let PropertyClause::Properties(props) = &comp.properties {
                let struct_def = determine_struct_type(type_mappings, comp, props, data_comp_type);

                // Only add into type_mappings if the component identifier does not exist in type_mappings
                // !Do not add, because "struct" is not useful; we need a type alias.
                // if !type_mappings.contains_key(&comp.identifier.to_lowercase()) {
                //     type_mappings.insert(comp.identifier.to_lowercase(), target_type.clone());
                // }

                if struct_def.fields.is_empty() {
                    // This means fields are obtained from subcomponents in the implementation,
                    // rather than from the type at this stage.
                    return Vec::new();
                } else {
                    return vec![Item::Struct(struct_def)];
                }
            }
            // } else { // This else branch is redundant: when comp.properties is empty,
            //           // determine_struct_type returns an empty result.
            //     // If there are no properties, return an empty struct
            //     return vec![Item::Struct(determine_struct_type(
            //         type_mappings,
            //         comp,
            //         &[],
            //         data_comp_type,
            //     ))];
            // }
        } else if unit_type.to_lowercase() == "union" {
            // Extract the property list from component properties
            if let PropertyClause::Properties(props) = &comp.properties {
                let union_def = determine_union_type(type_mappings, comp, props, data_comp_type);
                if union_def.fields.is_empty() {
                    // This means fields are obtained from subcomponents in the implementation,
                    // rather than from the type at this stage.
                    return Vec::new();
                } else {
                    return vec![Item::Union(union_def)];
                }
            }
            // This else branch is redundant: when comp.properties is empty,
            // determine_union_type returns an empty result.
            // else {
            //     // If there are no properties, return an empty union
            //     return vec![Item::Union(determine_union_type(
            //         type_mappings,
            //         comp,
            //         &[],
            //         data_comp_type,
            //     ))];
            // }
        } else if unit_type.to_lowercase() == "enum" {
            // Extract the property list from component properties
            if let PropertyClause::Properties(props) = &comp.properties {
                return vec![Item::Enum(determine_enum_type(comp, props))];
            }
            // } else {
            //     // If there are no properties, return an empty enum
            //     return vec![Item::Enum(determine_enum_type(comp, &[]))];
            // }
        } else if unit_type.to_lowercase() == "taggedunion" {
            // Extract the property list from component properties
            if let PropertyClause::Properties(props) = &comp.properties {
                let taggedunion_def =
                    determine_taggedunion_type(type_mappings, comp, props, data_comp_type);

                // Only add into type_mappings if the component identifier does not exist in type_mappings
                // if !type_mappings.contains_key(&comp.identifier.to_lowercase()) {
                //     type_mappings.insert(comp.identifier.to_lowercase(), target_type.clone());
                // }

                if taggedunion_def.variants.is_empty() {
                    // This means fields are obtained from subcomponents in the implementation,
                    // rather than from the type at this stage.
                    return Vec::new();
                } else {
                    return vec![Item::Enum(taggedunion_def)];
                }
            }
            // This else branch is redundant: when comp.properties is empty,
            // determine_taggedunion_type returns an empty result.
            // else {
            //     // If there are no properties, return an empty tagged union
            //     return vec![Item::Enum(determine_taggedunion_type(
            //         type_mappings,
            //         comp,
            //         &[],
            //         data_comp_type,
            //     ))];
            // }
        }
    }
    // Explicit properties describe the payload ABI, even when a classifier name
    // also occurs in the built-in mappings. Ports must use this same resolved type.
    type_mappings.insert(comp.identifier.to_lowercase(), target_type.clone());

    vec![Item::TypeAlias(TypeAlias {
        name: comp.identifier.clone(),
        target: target_type,
        // AADL data aliases currently lower to concrete Rust types.
        generics: Vec::new(),
        vis: Visibility::Public,
        docs: vec![format!("// AADL Data Type: {}", comp.identifier.clone())],
    })]
}

fn determine_data_type(type_mappings: &HashMap<String, Type>, comp: &ComponentType) -> Type {
    if let PropertyClause::Properties(props) = &comp.properties {
        if data_model_property(props, "data_representation").is_some() {
            return determine_complex_data_type(type_mappings, comp);
        }
    }
    // First check whether the component identifier already exists in type_mappings
    // println!("comp.identifier.to_lowercase():{:?}",comp.identifier.to_lowercase());
    if let Some(existing_type) = type_mappings.get(&comp.identifier.to_lowercase()) {
        return existing_type.clone();
    }

    // If not found, handle complex types
    determine_complex_data_type(type_mappings, comp)
}

/// Handle complex data types, including arrays, structs, unions, enums, etc.
fn determine_complex_data_type(
    type_mappings: &HashMap<String, Type>,
    comp: &ComponentType,
) -> Type {
    if let PropertyClause::Properties(props) = &comp.properties {
        for prop in props {
            if let Property::BasicProperty(bp) = prop {
                // Handle Data_Model::Data_Representation property
                // Check whether the property set is "Data_Model" and the property name is "Data_Representation"
                if let Some(property_set) = &bp.identifier.property_set {
                    if property_set.to_lowercase() == "data_model"
                        && bp.identifier.name.to_lowercase() == "data_representation"
                    {
                        if let PropertyValue::Single(PropertyExpression::String(
                            StringTerm::Literal(str_val),
                        )) = &bp.value
                        {
                            match str_val.to_lowercase().as_str() {
                                "integer" => {
                                    return determine_integer_type(comp, props);
                                }
                                "array" => {
                                    return determine_array_type(type_mappings, props);
                                }
                                "struct" => {
                                    return Type::Named("struct".to_string());
                                }
                                "union" => {
                                    return Type::Named("union".to_string());
                                }
                                "enum" => {
                                    return Type::Named("enum".to_string());
                                }
                                "taggedunion" => {
                                    return Type::Named("taggedunion".to_string());
                                }
                                _ => {
                                    // Look up the corresponding type in type_mappings; if not found, use the original value
                                    return type_mappings
                                        .get(&str_val.to_string().to_lowercase())
                                        .cloned()
                                        .unwrap_or_else(|| Type::Named(str_val.to_string()));
                                }
                            }
                        }
                    }
                }

                // Handle type_source_name property, used to specify the data type
                if bp.identifier.name.to_lowercase() == "type_source_name" {
                    if let PropertyValue::Single(PropertyExpression::String(StringTerm::Literal(
                        str_val,
                    ))) = &bp.value
                    {
                        return type_mappings
                            .get(&str_val.to_string().to_lowercase())
                            .cloned()
                            .unwrap_or_else(|| Type::Named(str_val.to_string()));
                    }
                }
            }
        }
    }
    Type::Named("()".to_string())
}

fn property_named<'a>(props: &'a [Property], name: &str) -> Option<&'a PropertyValue> {
    props.iter().find_map(|property| match property {
        Property::BasicProperty(association)
            if association.identifier.name.eq_ignore_ascii_case(name) =>
        {
            Some(&association.value)
        }
        _ => None,
    })
}

fn data_model_property<'a>(props: &'a [Property], name: &str) -> Option<&'a PropertyValue> {
    props.iter().find_map(|property| match property {
        Property::BasicProperty(association)
            if association.identifier.name.eq_ignore_ascii_case(name)
                && association.identifier.property_set.as_deref()
                    .is_some_and(|set| set.eq_ignore_ascii_case("data_model")) =>
        {
            Some(&association.value)
        }
        _ => None,
    })
}

/// Preserve the integer storage ABI instead of treating every Integer as i32.
/// Source_Data_Size takes precedence over the more general Data_Size property.
fn determine_integer_type(comp: &ComponentType, props: &[Property]) -> Type {
    let prefix = match data_model_property(props, "number_representation") {
        None => "i",
        Some(PropertyValue::Single(PropertyExpression::String(StringTerm::Literal(value))))
            if value.eq_ignore_ascii_case("signed") => "i",
        Some(PropertyValue::Single(PropertyExpression::String(StringTerm::Literal(value))))
            if value.eq_ignore_ascii_case("unsigned") => "u",
        Some(value) => panic!(
            "unsupported integer Number_Representation for {}: {value:?}", comp.identifier
        ),
    };
    let size = property_named(props, "source_data_size")
        .or_else(|| property_named(props, "data_size"));
    let bits = match size {
        None => 32,
        Some(PropertyValue::Single(PropertyExpression::Integer(
            SignedIntergerOrConstant::Real(value),
        ))) if value.sign != Some(Sign::Minus) && value.value > 0 => {
            match value.unit.as_deref().map(str::to_ascii_lowercase).as_deref() {
                Some("bits") => value.value,
                Some("bytes") => value.value.checked_mul(8)
                    .expect("integer storage size exceeds the supported range"),
                _ => panic!(
                    "unsupported integer storage size unit for {}: {:?}",
                    comp.identifier, value.unit
                ),
            }
        }
        Some(value) => panic!(
            "unsupported integer storage size for {}: {value:?}", comp.identifier
        ),
    };
    if !matches!(bits, 8 | 16 | 32 | 64) {
        panic!("unsupported integer storage width for {}: {bits} bits", comp.identifier);
    }
    Type::Named(format!("{prefix}{bits}"))
}

/// Base_Type is normally a one-element classifier list. Keep support for the
/// equivalent single expression used by older models without inventing an i32 base.
fn array_base_expression(value: &PropertyValue) -> Option<&PropertyExpression> {
    match value {
        PropertyValue::Single(expression) => Some(expression),
        PropertyValue::List(values) if values.len() == 1 => match &values[0] {
            PropertyListElement::Value(expression) => Some(expression),
            _ => None,
        },
        _ => None,
    }
}

/// Handle array types
fn determine_array_type(type_mappings: &HashMap<String, Type>, props: &[Property]) -> Type {
    let mut base_type = None;
    let mut dimensions = Vec::new();

    // Look up Base_Type property
    for prop in props {
        //println!("prop: {:?}", prop);
        if let Property::BasicProperty(bp) = prop {
            // Look up base_type property
            if bp.identifier.name.to_lowercase() == "base_type" {
                if let Some(PropertyExpression::ComponentClassifier(
                    ComponentClassifierTerm {
                        unique_component_classifier_reference: uccr,
                    },
                )) = array_base_expression(&bp.value)
                {
                    if let UniqueComponentClassifierReference::Type(impl_ref) = uccr {
                        let type_name = impl_ref.implementation_name.type_identifier.clone();
                        // println!("type_mappings:{:?}",type_mappings);
                        base_type = Some(type_mappings
                            .get(&type_name.to_lowercase())
                            .cloned()
                            // Rust aliases can refer to later declarations. Preserve
                            // that classifier instead of guessing its element type.
                            .unwrap_or_else(|| Type::Named(type_name)));
                    }
                }
            }

            // Look up Dimension property
            if bp.identifier.name.to_lowercase() == "dimension" {
                match &bp.value {
                    PropertyValue::Single(PropertyExpression::Integer(
                        SignedIntergerOrConstant::Real(int_val),
                    )) => {
                        dimensions.push(int_val.value as usize);
                    }
                    PropertyValue::List(dim_list) => {
                        for dim_item in dim_list {
                            if let PropertyListElement::Value(PropertyExpression::Integer(
                                SignedIntergerOrConstant::Real(int_val),
                            )) = dim_item
                            {
                                dimensions.push(int_val.value as usize);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // If no dimension info is found, default to a 1-D array
    if dimensions.is_empty() {
        dimensions.push(1);
    }

    // Construct the array type: build nested arrays from inner to outer
    let mut array_type = base_type.expect("unsupported or absent array Base_Type classifier");
    for &dim in dimensions.iter().rev() {
        array_type = Type::Array(Box::new(array_type), dim);
    }

    array_type
}

/// Handle struct types
fn determine_struct_type(
    type_mappings: &HashMap<String, Type>,
    comp: &ComponentType,
    props: &[Property],
    data_comp_type: &mut HashMap<String, String>,
) -> StructDef {
    let mut fields = Vec::new();
    let mut field_names = Vec::new();
    let mut field_types = Vec::new();

    // Parse field types and field names
    for prop in props {
        if let Property::BasicProperty(bp) = prop {
            // Parse Base_Type property to obtain field types
            if let Some(property_set) = &bp.identifier.property_set {
                if property_set.to_lowercase() == "data_model"
                    && bp.identifier.name.to_lowercase() == "base_type"
                {
                    if let PropertyValue::List(type_list) = &bp.value {
                        for type_item in type_list {
                            if let PropertyListElement::Value(
                                PropertyExpression::ComponentClassifier(ComponentClassifierTerm {
                                    unique_component_classifier_reference,
                                }),
                            ) = type_item
                            {
                                // Extract type name from the classifier reference
                                // There will be no implementation reference
                                // Use match, not if
                                let type_name = match unique_component_classifier_reference {
                                    UniqueComponentClassifierReference::Type(impl_ref) => {
                                        impl_ref.implementation_name.type_identifier.clone()
                                    }
                                    UniqueComponentClassifierReference::Implementation(
                                        _impl_ref,
                                    ) => "".to_string(),
                                };
                                // let type_name = if let UniqueComponentClassifierReference::Type(impl_ref) = unique_component_classifier_reference {
                                //     impl_ref.implementation_name.type_identifier.clone()
                                // } else {
                                //     "".to_string()
                                // };

                                // Map to Rust type
                                let rust_type = type_mappings
                                    .get(&type_name.to_string().to_lowercase())
                                    .cloned()
                                    .unwrap_or(Type::Named(type_name));

                                field_types.push(rust_type);
                            }
                        }
                    }
                }
            }

            // Parse Element_Names property to obtain field names
            if let Some(property_set) = &bp.identifier.property_set {
                if property_set.to_lowercase() == "data_model"
                    && bp.identifier.name.to_lowercase() == "element_names"
                {
                    if let PropertyValue::List(name_list) = &bp.value {
                        for name_item in name_list {
                            if let PropertyListElement::Value(PropertyExpression::String(
                                StringTerm::Literal(name),
                            )) = name_item
                            {
                                field_names.push(name.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    // Check whether field information is obtained; in theory both should be present or absent together
    if field_names.is_empty() || field_types.is_empty() {
        // No field information is obtained; need to retrieve property info from the component implementation (impl)
        // Store the info into the global data structure
        data_comp_type.insert(comp.identifier.clone(), "struct".to_string());
    }
    // Create fields
    for (name, ty) in field_names.iter().zip(field_types.iter()) {
        fields.push(Field {
            name: name.clone(),
            ty: ty.clone(),
            docs: vec!["".to_string()],
            attrs: vec![],
        });
    }

    // Create struct definition
    StructDef {
        name: comp.identifier.clone(),
        fields,
        generics: vec![],
        derives: vec!["Debug".to_string(), "Clone".to_string()],
        docs: vec![format!("// AADL Struct: {}", comp.identifier)],
        vis: Visibility::Public,
    }
}

/// Handle union types: unsafe unions
fn determine_union_type(
    type_mappings: &HashMap<String, Type>,
    comp: &ComponentType,
    props: &[Property],
    data_comp_type: &mut HashMap<String, String>,
) -> UnionDef {
    // Parse field types and field names
    let mut field_names = Vec::new();
    let mut field_types = Vec::new();

    for prop in props {
        if let Property::BasicProperty(bp) = prop {
            // Parse Base_Type property to obtain field types
            if let Some(property_set) = &bp.identifier.property_set {
                if property_set.to_lowercase() == "data_model"
                    && bp.identifier.name.to_lowercase() == "base_type"
                {
                    if let PropertyValue::List(type_list) = &bp.value {
                        for type_item in type_list {
                            if let PropertyListElement::Value(
                                PropertyExpression::ComponentClassifier(ComponentClassifierTerm {
                                    unique_component_classifier_reference,
                                }),
                            ) = type_item
                            {
                                // Extract type name from the classifier reference
                                // There will be no implementation reference
                                // Use match, not if
                                let type_name = match unique_component_classifier_reference {
                                    UniqueComponentClassifierReference::Type(impl_ref) => {
                                        impl_ref.implementation_name.type_identifier.clone()
                                    }
                                    UniqueComponentClassifierReference::Implementation(
                                        _impl_ref,
                                    ) => "".to_string(),
                                };
                                // let type_name = if let UniqueComponentClassifierReference::Type(impl_ref) = unique_component_classifier_reference {
                                //     impl_ref.implementation_name.type_identifier.clone()
                                // } else {
                                //     "".to_string()
                                // };

                                // Map to Rust type
                                let rust_type = type_mappings
                                    .get(&type_name.to_string().to_lowercase())
                                    .cloned()
                                    .unwrap_or(Type::Named(type_name));

                                field_types.push(rust_type);
                            }
                        }
                    }
                }
            }

            // Parse Element_Names property to obtain field names
            if let Some(property_set) = &bp.identifier.property_set {
                if property_set.to_lowercase() == "data_model"
                    && bp.identifier.name.to_lowercase() == "element_names"
                {
                    if let PropertyValue::List(name_list) = &bp.value {
                        for name_item in name_list {
                            if let PropertyListElement::Value(PropertyExpression::String(
                                StringTerm::Literal(name),
                            )) = name_item
                            {
                                field_names.push(name.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // Check whether field information is obtained; in theory both should be present or absent together
    if field_names.is_empty() || field_types.is_empty() {
        // No field information is obtained; need to retrieve property info from the component implementation (impl)
        // Store the info into the global data structure
        data_comp_type.insert(comp.identifier.clone(), "union".to_string());
    }

    // Create union fields
    let mut fields = Vec::new();
    for (name, ty) in field_names.iter().zip(field_types.iter()) {
        fields.push(Field {
            name: name.clone(),
            ty: ty.clone(),
            docs: vec!["".to_string()],
            attrs: vec![],
        });
    }

    // Create union definition
    UnionDef {
        name: comp.identifier.clone(),
        fields,
        generics: vec![],
        // Rust unions have no active-field discriminator, so `Debug` cannot be derived safely.
        // Deriving `Clone` for a union requires `Copy`, which is valid for union-compatible fields.
        derives: vec!["Copy".to_string(), "Clone".to_string()],
        docs: vec![format!("// AADL Union: {}", comp.identifier)],
        vis: Visibility::Public,
    }
}

/// Handle enum types
fn determine_enum_type(comp: &ComponentType, props: &[Property]) -> EnumDef {
    // Parse enumerator names
    let mut variant_names = Vec::new();

    for prop in props {
        if let Property::BasicProperty(bp) = prop {
            // Parse Enumerators property to obtain enumerator names
            if let Some(property_set) = &bp.identifier.property_set {
                if property_set.to_lowercase() == "data_model"
                    && bp.identifier.name.to_lowercase() == "enumerators"
                {
                    if let PropertyValue::List(name_list) = &bp.value {
                        for name_item in name_list {
                            if let PropertyListElement::Value(PropertyExpression::String(
                                StringTerm::Literal(name),
                            )) = name_item
                            {
                                variant_names.push(name.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // Create enum variants (without payload data)
    let mut variants = Vec::new();
    for name in variant_names {
        variants.push(Variant {
            name: name.clone(),
            data: None, // Enum variants contain no payload data
            docs: vec![],
        });
    }

    // Create enum definition
    EnumDef {
        name: comp.identifier.clone(),
        variants,
        generics: vec![],
        derives: vec!["Debug".to_string(), "Clone".to_string()],
        docs: vec![format!("// AADL Enum: {}", comp.identifier)],
        vis: Visibility::Public,
    }
}

/// Handle tagged union types: generate an enum with payload types
fn determine_taggedunion_type(
    type_mappings: &HashMap<String, Type>,
    comp: &ComponentType,
    props: &[Property],
    data_comp_type: &mut HashMap<String, String>,
) -> EnumDef {
    // Parse field names and field types
    let mut field_names = Vec::new();
    let mut field_types = Vec::new();

    for prop in props {
        if let Property::BasicProperty(bp) = prop {
            // Parse Base_Type property to obtain field types
            if let Some(property_set) = &bp.identifier.property_set {
                if property_set.to_lowercase() == "data_model"
                    && bp.identifier.name.to_lowercase() == "base_type"
                {
                    if let PropertyValue::List(type_list) = &bp.value {
                        for type_item in type_list {
                            if let PropertyListElement::Value(
                                PropertyExpression::ComponentClassifier(ComponentClassifierTerm {
                                    unique_component_classifier_reference,
                                }),
                            ) = type_item
                            {
                                // Extract type name from the classifier reference
                                // There will be no implementation reference

                                // Use match, not if
                                let type_name = match unique_component_classifier_reference {
                                    UniqueComponentClassifierReference::Type(impl_ref) => {
                                        impl_ref.implementation_name.type_identifier.clone()
                                    }
                                    UniqueComponentClassifierReference::Implementation(
                                        _impl_ref,
                                    ) => "".to_string(),
                                };
                                //  let type_name = if let UniqueComponentClassifierReference::Type(impl_ref) = unique_component_classifier_reference {
                                //     impl_ref.implementation_name.type_identifier.clone()
                                // } else {
                                //     "".to_string()
                                // };

                                // Map to Rust type
                                let rust_type = type_mappings
                                    .get(&type_name.to_string().to_lowercase())
                                    .cloned()
                                    .unwrap_or(Type::Named(type_name));

                                field_types.push(rust_type);
                            }
                        }
                    }
                }
            }

            // Parse Element_Names property to obtain field names
            if let Some(property_set) = &bp.identifier.property_set {
                if property_set.to_lowercase() == "data_model"
                    && bp.identifier.name.to_lowercase() == "element_names"
                {
                    if let PropertyValue::List(name_list) = &bp.value {
                        for name_item in name_list {
                            if let PropertyListElement::Value(PropertyExpression::String(
                                StringTerm::Literal(name),
                            )) = name_item
                            {
                                field_names.push(name.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // Check whether field information is obtained; in theory both should be present or absent together
    if field_names.is_empty() || field_types.is_empty() {
        // No field information is obtained; need to retrieve property info from the component implementation (impl)
        // Store the info into the global data structure
        data_comp_type.insert(comp.identifier.clone(), "taggedunion".to_string());
    }

    // Create enum variants (with payload types)
    let mut variants = Vec::new();
    for (name, ty) in field_names.iter().zip(field_types.iter()) {
        // Capitalize the first character of the field name, e.g., "f1" -> "F1"
        // (ignore the case where name is empty)

        let mut chars = name.chars();
        let variant_name = match chars.next() {
            None => "Default".to_string(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        };

        variants.push(Variant {
            name: variant_name,
            data: Some(vec![ty.clone()]), // Tagged union variants include payload types
            docs: vec![],
        });
    }

    // Create enum definition
    EnumDef {
        name: comp.identifier.clone(),
        variants,
        generics: vec![],
        derives: vec!["Debug".to_string(), "Clone".to_string()],
        docs: vec![format!("// AADL Tagged Union: {}", comp.identifier)],
        vis: Visibility::Public,
    }
}
