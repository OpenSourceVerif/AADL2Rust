use aadl_intermediate::*;
use crate::ast::aadl_ast_cj::*;
use std::collections::HashMap;

pub fn convert_data_implementation(
    type_mappings: &HashMap<String, Type>,
    data_comp_type: &HashMap<String, String>,
    impl_: &ComponentImplementation,
    package: &Package,
) -> Vec<Item> {
    let mut items = Vec::new();

    // Check subcomponents to determine whether this is a shared variable / complex data type; both have subcomponents
    if let SubcomponentClause::Items(subcomponents) = &impl_.subcomponents {
        // Filter out data subcomponents
        let data_subcomponents: Vec<_> = subcomponents
            .iter()
            .filter(|sub| sub.category == ComponentCategory::Data)
            .cloned()
            .collect();

        if data_comp_type.contains_key(&impl_.name.type_identifier) {
            // This indicates a complex data type
            let data_type_name = data_comp_type.get(&impl_.name.type_identifier).unwrap();
            if data_type_name == "struct" {
                items.push(Item::Struct(determine_struct_impl(
                    type_mappings,
                    impl_,
                    data_subcomponents.as_slice(),
                )));
            } else if data_type_name == "union" {
                items.push(Item::Union(determine_union_impl(
                    type_mappings,
                    impl_,
                    data_subcomponents.as_slice(),
                )));
            } else if data_type_name == "taggedunion" {
                items.push(Item::Enum(determine_taggedunion_impl(
                    type_mappings,
                    impl_,
                    data_subcomponents.as_slice(),
                )));
            }
        }

        // Filter out subprogram subcomponents
        let subprogram_subcomponents: Vec<_> = subcomponents
            .iter()
            .filter(|sub| sub.category == ComponentCategory::Subprogram)
            .cloned()
            .collect();
        // For each subprogram subcomponent, inspect its classifier reference
        let mut subprogram_methods = Vec::new();
        for sub in &subprogram_subcomponents {
            // Get the implementation reference of the subprogram
            if let SubcomponentClassifier::ClassifierReference(
                UniqueComponentClassifierReference::Implementation(impl_ref),
            ) = &sub.classifier
            {
                let subprogram_impl_name = &impl_ref.implementation_name.type_identifier;
                // println!(
                //     "Data component implementation {} contains a subprogram subcomponent: {}",
                //     impl_.name.type_identifier, subprogram_impl_name
                // );
                // Generate subprogram-related code as needed; the subprogram name is the lowercase of subprogram_impl_name

                // Generate the subprogram invocation method
                let method_name = subprogram_impl_name.to_lowercase();
                let mut method_body = Vec::new();

                // Generate a single call; arguments are all data_subcomponents
                let mut call_args = Vec::new();
                for data_sub in &data_subcomponents {
                    let field_name = data_sub.identifier.clone();
                    call_args.push(Expr::Reference(
                        Box::new(Expr::Path(
                            vec!["self".to_string(), field_name],
                            PathType::Member,
                        )),
                        true, // &
                        true, // mut
                    ));
                }

                let call_expr = Expr::Call(
                    Box::new(Expr::Path(
                        vec![subprogram_impl_name.to_lowercase(), "call".to_string()],
                        PathType::Namespace,
                    )),
                    call_args,
                );
                method_body.push(Statement::Expr(call_expr));

                let method = ImplItem::Method(FunctionDef {
                    name: method_name.clone(),
                    params: vec![Param {
                        name: "self".to_string(),
                        ty: Type::Reference(Box::new(Type::Named("Self".to_string())), true, true),
                    }],
                    return_type: Type::Unit,
                    body: Block {
                        stmts: method_body,
                        expr: None,
                    },
                    asyncness: false,
                    vis: Visibility::Public,
                    docs: vec![format!(
                        "/// {} : provides subprogram access {};",
                        method_name, subprogram_impl_name
                    )],
                    attrs: Vec::new(),
                });

                subprogram_methods.push(method);
            }
        }

        // Check whether this data is a shared variable (used by some process)
        let is_shared_data = is_shared_data_component(package, &impl_.name.type_identifier);
        // Only if it is shared data, generate a new() method to initialize fields
        if is_shared_data {
            let mut field_initializations = Vec::new();

            for sub in &data_subcomponents {
                let field_name = sub.identifier.clone();
                // For simplicity, use 0 as the default value; users can modify as needed
                field_initializations.push(format!("            {}: 0", field_name));
            }

            let struct_init_code = format!(
                "return {} {{\n{}\n        }}",
                impl_.name.type_identifier,
                field_initializations.join(",\n")
            );

            let new_method = ImplItem::Method(FunctionDef {
                name: "new".to_string(),
                params: vec![],
                return_type: Type::Named("Self".to_string()),
                body: Block {
                    stmts: vec![Statement::Expr(Expr::Ident(struct_init_code))],
                    expr: None,
                },
                asyncness: false,
                vis: Visibility::Public,
                docs: vec![format!(
                    "// Creates a new instance of {}",
                    impl_.name.type_identifier
                )],
                attrs: Vec::new(),
            });
            subprogram_methods.push(new_method);
        }

        // If there are subprogram methods, generate an impl block
        if !subprogram_methods.is_empty() {
            let impl_block = ImplBlock {
                target: Type::Named(impl_.name.type_identifier.clone()),
                generics: Vec::new(),
                items: subprogram_methods,
                trait_impl: None,
            };
            items.push(Item::Impl(impl_block));
        }

        if is_shared_data {
            // Generate the shared data type
            let shared_type_name = format!("{}Shared", impl_.name.type_identifier);
            let shared_type = Type::Generic(
                "Arc".to_string(),
                vec![Type::Generic(
                    "Mutex".to_string(),
                    vec![Type::Named(impl_.name.type_identifier.clone())],
                )],
            );
            let type_alias = TypeAlias {
                name: shared_type_name,
                target: shared_type,
                vis: Visibility::Public,
                docs: vec![
                    format!("// Shared data type for {}", impl_.name.type_identifier),
                    "// Auto-generated from AADL data implementation".to_string(),
                ],
            };
            items.push(Item::TypeAlias(type_alias));
        }

        // let subprogram_count = subcomponents
        //     .iter()
        //     .filter(|sub| sub.category == ComponentCategory::Subprogram)
        //     .count();

        // Incorrect assumption (fixed): if there are multiple subprograms, it indicates a shared variable;
        // currently, data with more than 1 shared data subcomponent is not supported
        // if subprogram_count > 1 {
        //     if data_subcomponents.len() == 1 {
        //         // Get the type name of the data subcomponent (used as T in Arc<Mutex<T>>)
        //         let data_type_name = match &data_subcomponents[0].classifier {
        //             SubcomponentClassifier::ClassifierReference(
        //                 UniqueComponentClassifierReference::Implementation(unirf),
        //             ) => {
        //                 format!("{}", unirf.implementation_name.type_identifier)
        //             }
        //             _ => "UnknownType".to_string(),
        //         };

        //         // Generate shared type definition: extract the name part from the data component implementation name (e.g., POS.Impl),
        //         // then append Shared
        //         let shared_type_name = {
        //             // Extract the implementation name from impl_.name.type_identifier (remove possible Impl suffix)
        //             let impl_name = &impl_.name.type_identifier;
        //             format!("{}Shared", impl_name)
        //         };

        //         // Generate Arc<Mutex<T>> type
        //         let shared_type = Type::Generic(
        //             "Arc".to_string(),
        //             vec![Type::Generic(
        //                 "Mutex".to_string(),
        //                 vec![Type::Named(data_type_name)],
        //             )],
        //         );

        //         let type_alias = TypeAlias {
        //             name: shared_type_name,
        //             target: shared_type,
        //             vis: Visibility::Public,
        //             docs: vec![
        //                 format!("// Shared data type for {}", impl_.name.type_identifier),
        //                 "// Auto-generated from AADL data implementation".to_string(),
        //             ],
        //         };

        //         items.push(Item::TypeAlias(type_alias));
        //     } else if data_subcomponents.len() > 1 {
        //         // Print an error message: multiple shared data subcomponents are not supported
        //         eprintln!(
        //             "Error: data component implementation {} has {} data subcomponents; multiple shared data are not supported yet",
        //             impl_.name.type_identifier,
        //             data_subcomponents.len()
        //         );
        //         eprintln!("Please check the AADL model to ensure each shared data component implementation has exactly one data subcomponent");
        //     }
        // }
    }

    items
}

/// Check whether a data component is shared (used by some process)
fn is_shared_data_component(package: &Package, data_impl_name: &str) -> bool {
    // Search in the public section
    if let Some(public_section) = &package.public_section {
        for decl in &public_section.declarations {
            if let AadlDeclaration::ComponentImplementation(process_impl) = decl {
                if process_impl.category == ComponentCategory::Process {
                    if let SubcomponentClause::Items(process_subcomponents) =
                        &process_impl.subcomponents
                    {
                        for sub in process_subcomponents {
                            if sub.category == ComponentCategory::Data {
                                if let SubcomponentClassifier::ClassifierReference(
                                    UniqueComponentClassifierReference::Implementation(data_ref),
                                ) = &sub.classifier
                                {
                                    if data_ref.implementation_name.type_identifier
                                        == data_impl_name
                                    {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Search in the private section as well
    if let Some(private_section) = &package.private_section {
        for decl in &private_section.declarations {
            if let AadlDeclaration::ComponentImplementation(process_impl) = decl {
                if process_impl.category == ComponentCategory::Process {
                    if let SubcomponentClause::Items(process_subcomponents) =
                        &process_impl.subcomponents
                    {
                        for sub in process_subcomponents {
                            if sub.category == ComponentCategory::Data {
                                if let SubcomponentClassifier::ClassifierReference(
                                    UniqueComponentClassifierReference::Implementation(data_ref),
                                ) = &sub.classifier
                                {
                                    if data_ref.implementation_name.type_identifier
                                        == data_impl_name
                                    {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    false
}

/// Handle struct types
fn determine_struct_impl(
    type_mappings: &HashMap<String, Type>,
    impl_: &ComponentImplementation,
    subcomponents: &[Subcomponent],
) -> StructDef {
    let mut fields = Vec::new();

    // Parse field types and names from subcomponents
    for sub in subcomponents {
        // Get field name (subcomponent identifier)
        let field_name = sub.identifier.clone();

        // Get field type
        let field_type = match &sub.classifier {
            SubcomponentClassifier::ClassifierReference(
                UniqueComponentClassifierReference::Implementation(impl_ref),
            ) => {
                // Extract type name from classifier reference
                let type_name = impl_ref.implementation_name.type_identifier.clone();

                // Map to Rust type
                type_mappings
                    .get(&type_name.to_lowercase())
                    .cloned()
                    .unwrap_or(Type::Named(type_name))
            }
            _ => Type::Named("UnknownType".to_string()),
            // SubcomponentClassifier::ClassifierReference(
            //     UniqueComponentClassifierReference::Type(type_ref),
            // ) => {
            //     // Extract type name from type reference
            //     let type_name = type_ref.implementation_name.type_identifier.clone();

            //     // Map to Rust type
            //     type_mappings
            //         .get(&type_name.to_lowercase())
            //         .cloned()
            //         .unwrap_or_else(|| Type::Named(type_name))
            // }
            // SubcomponentClassifier::Prototype(prototype_name) => {
            //     // Handle prototype reference
            //     Type::Named(prototype_name.clone())
            // }
        };

        // Create field
        fields.push(Field {
            name: field_name,
            ty: field_type,
            docs: vec![format!("// Subcomponent field: {}", sub.identifier)],
            attrs: vec![],
        });
    }

    // Create struct definition
    StructDef {
        name: impl_.name.type_identifier.clone(),
        fields,
        generics: vec![],
        derives: vec!["Debug".to_string(), "Clone".to_string()],
        docs: vec![format!("// AADL Struct: {}", impl_.name.type_identifier)],
        vis: Visibility::Public,
    }
}

/// Handle union types (unsafe)
fn determine_union_impl(
    type_mappings: &HashMap<String, Type>,
    impl_: &ComponentImplementation,
    subcomponents: &[Subcomponent],
) -> UnionDef {
    let mut fields = Vec::new();

    // Parse field types and names from subcomponents
    for sub in subcomponents {
        // Get field name (subcomponent identifier)
        let field_name = sub.identifier.clone();

        // Get field type
        let field_type = match &sub.classifier {
            SubcomponentClassifier::ClassifierReference(
                UniqueComponentClassifierReference::Implementation(impl_ref),
            ) => {
                // Extract type name from classifier reference
                let type_name = impl_ref.implementation_name.type_identifier.clone();

                // Map to Rust type
                type_mappings
                    .get(&type_name.to_lowercase())
                    .cloned()
                    .unwrap_or(Type::Named(type_name))
            }
            _ => Type::Named("UnknownType".to_string()),
            // SubcomponentClassifier::ClassifierReference(
            //     UniqueComponentClassifierReference::Type(type_ref),
            // ) => {
            //     // Extract type name from type reference
            //     let type_name = type_ref.implementation_name.type_identifier.clone();

            //     // Map to Rust type
            //     type_mappings
            //         .get(&type_name.to_lowercase())
            //         .cloned()
            //         .unwrap_or_else(|| Type::Named(type_name))
            // }
            // SubcomponentClassifier::Prototype(prototype_name) => {
            //     // Handle prototype reference
            //     Type::Named(prototype_name.clone())
            // }
        };

        // Create field
        fields.push(Field {
            name: field_name,
            ty: field_type,
            docs: vec![format!("// Union field: {}", sub.identifier)],
            attrs: vec![],
        });
    }

    // Create union definition
    UnionDef {
        name: impl_.name.type_identifier.clone(),
        fields,
        generics: vec![],
        derives: vec!["Debug".to_string(), "Clone".to_string()],
        docs: vec![format!("// AADL Union: {}", impl_.name.type_identifier)],
        vis: Visibility::Public,
    }
}

/// Handle tagged union types: generate a typed enum from subcomponents in the implementation
fn determine_taggedunion_impl(
    type_mappings: &HashMap<String, Type>,
    impl_: &ComponentImplementation,
    subcomponents: &[Subcomponent],
) -> EnumDef {
    let mut variants = Vec::new();

    // Parse field types and names from subcomponents
    for sub in subcomponents {
        // Get field name (subcomponent identifier)
        let field_name = sub.identifier.clone();

        // Get field type
        let field_type = match &sub.classifier {
            SubcomponentClassifier::ClassifierReference(
                UniqueComponentClassifierReference::Implementation(impl_ref),
            ) => {
                // Extract type name from classifier reference
                let type_name = impl_ref.implementation_name.type_identifier.clone();

                // Map to Rust type
                type_mappings
                    .get(&type_name.to_lowercase())
                    .cloned()
                    .unwrap_or(Type::Named(type_name))
            }
            _ => Type::Named("UnknownType".to_string()),
            // SubcomponentClassifier::ClassifierReference(
            //     UniqueComponentClassifierReference::Type(type_ref),
            // ) => {
            //     // Extract type name from type reference
            //     let type_name = type_ref.implementation_name.type_identifier.clone();

            //     // Map to Rust type
            //     type_mappings
            //         .get(&type_name.to_lowercase())
            //         .cloned()
            //         .unwrap_or_else(|| Type::Named(type_name))
            // }
            // SubcomponentClassifier::Prototype(prototype_name) => {
            //     // Handle prototype reference
            //     Type::Named(prototype_name.clone())
            // }
        };

        // Capitalize the first letter of the field name, e.g., "f1" -> "F1"
        let mut chars = field_name.chars();
        let variant_name = match chars.next() {
            None => "Default".to_string(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        };

        // Create enum variant (with payload type)
        variants.push(Variant {
            name: variant_name,
            data: Some(vec![field_type]), // Tagged union variants carry a payload type
            docs: vec![format!("// Tagged union field: {}", sub.identifier)],
        });
    }

    // Create enum definition
    EnumDef {
        name: impl_.name.type_identifier.clone(),
        variants,
        generics: vec![],
        derives: vec!["Debug".to_string(), "Clone".to_string()],
        docs: vec![format!(
            "// AADL Tagged Union: {}",
            impl_.name.type_identifier
        )],
        vis: Visibility::Public,
    }
}
