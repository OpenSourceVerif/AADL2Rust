#![allow(clippy::collapsible_match)]
use aadl_intermediate::*;

use crate::aadl_ast2rust_code::converter::AadlConverter;
use crate::ast::aadl_ast_cj::*;

pub fn convert_subprogram_component(
    temp_converter: &AadlConverter,
    comp: &ComponentType,
    package: &Package,
) -> Vec<Item> {
    let items = Vec::new();

    // Check whether this is a C-language bound subprogram
    if let Some(c_func_name) = extract_c_function_name(comp) {
        return generate_c_function_wrapper(temp_converter, comp, &c_func_name, package);
    }

    items
}

fn extract_c_function_name(comp: &ComponentType) -> Option<String> {
    if let PropertyClause::Properties(props) = &comp.properties {
        for prop in props {
            if let Property::BasicProperty(bp) = prop {
                if bp.identifier.name.to_lowercase() == "source_name" {
                    if let PropertyValue::Single(PropertyExpression::String(StringTerm::Literal(
                        name,
                    ))) = &bp.value
                    {
                        return Some(name.clone());
                    }
                }
            }
        }
    }
    None
}

fn generate_c_function_wrapper(
    temp_converter: &AadlConverter,
    comp: &ComponentType,
    c_func_name: &str,
    package: &Package,
) -> Vec<Item> {
    // Extract C source file names
    let source_files = extract_source_files(comp);

    let mut items = Vec::new();
    let mut functions = Vec::new();
    let mut types_to_import = std::collections::HashSet::new();

    // Process each feature
    if let FeatureClause::Items(features) = &comp.features {
        for feature in features {
            match feature {
                Feature::Port(port) => {
                    let (func_name, param_type) = match port.direction {
                        PortDirection::Out => (
                            "send",
                            Type::Reference(
                                Box::new(temp_converter.convert_paramport_type(port)),
                                true,
                                true,
                            ),
                        ),
                        PortDirection::In => (
                            "receive",
                            Type::Reference(
                                Box::new(temp_converter.convert_paramport_type(port)),
                                false,
                                false,
                            ),
                        ),
                        _ => continue, //
                    };

                    // Collect types that need to be imported
                    //println!("port: {:?}", port);
                    if let Type::Named(type_name) = &temp_converter.convert_paramport_type(port) {
                        //println!("type_name: {}", type_name);
                        if !is_rust_primitive_type(type_name) {
                            types_to_import.insert(type_name.clone());
                        }
                    }

                    // Create wrapper function
                    functions.push(FunctionDef {
                        name: func_name.to_string(),
                        params: vec![Param {
                            name: port.identifier.to_string().to_lowercase(),
                            ty: param_type,
                        }],
                        return_type: Type::Unit,
                        body: Block {
                            stmts: vec![Statement::Expr(Expr::Unsafe(Box::new(Block {
                                stmts: vec![Statement::Expr(Expr::Call(
                                    Box::new(Expr::Path(
                                        vec![c_func_name.to_string()],
                                        PathType::Namespace,
                                    )),
                                    vec![Expr::Ident(port.identifier.to_string().to_lowercase())],
                                ))],
                                expr: None,
                            })))],
                            expr: None,
                        },
                        asyncness: false,
                        vis: Visibility::Public,
                        docs: vec![
                            format!("// Wrapper for C function {}", c_func_name),
                            format!("// Original AADL port: {}", port.identifier),
                        ],
                        attrs: Vec::new(),
                    });
                }
                Feature::SubcomponentAccess(sub_access) => {
                    // Handle requires data access features
                    if let SubcomponentAccessSpec::Data(data_access) = sub_access {
                        if data_access.direction == AccessDirection::Requires {
                            // Extract POS.Impl from `this : requires data access POS.Impl`
                            if let Some(classifier) = &data_access.classifier {
                                if let DataAccessReference::Classifier(unique_ref) = classifier {
                                    if let UniqueComponentClassifierReference::Implementation(
                                        impl_ref,
                                    ) = unique_ref
                                    {
                                        let data_component_name =
                                            &impl_ref.implementation_name.type_identifier;
                                        // Find concrete data types from the data component implementation
                                        let data_types = find_data_type_from_implementation(
                                            data_component_name,
                                            package,
                                            temp_converter,
                                        );
                                        // Add all data types to the import list
                                        for data_type in &data_types {
                                            types_to_import.insert(data_type.clone());
                                        }

                                        // Generate a call function whose parameters include all data types
                                        if !data_types.is_empty() {
                                            let mut params = Vec::new();
                                            let mut call_args = Vec::new();

                                            for (idx, data_type) in data_types.iter().enumerate() {
                                                let param_name = format!("arg{}", idx);
                                                params.push(Param {
                                                    name: param_name.clone(),
                                                    ty: Type::Reference(
                                                        Box::new(Type::Named(data_type.clone())),
                                                        true,
                                                        true,
                                                    ), // &mut DataType
                                                });
                                                call_args.push(Expr::Ident(param_name));
                                            }

                                            let call_function = FunctionDef {
                                                name: "call".to_string(),
                                                params,
                                                return_type: Type::Unit,
                                                body: Block {
                                                    stmts: vec![Statement::Expr(Expr::Unsafe(Box::new(Block {
                                                        stmts: vec![Statement::Expr(Expr::Call(
                                                            Box::new(Expr::Path(
                                                                vec![c_func_name.to_string()],
                                                                PathType::Namespace,
                                                            )),
                                                            call_args, // Pass all parameters
                                                        ))],
                                                        expr: None,
                                                    })))],
                                                    expr: None,
                                                },
                                                asyncness: false,
                                                vis: Visibility::Public,
                                                docs: vec![
                                                    format!("// Call C function {} with data access references", c_func_name),
                                                    "// Generated for requires data access feature".to_string(),
                                                    "// Note: Rust compiler will handle the reference to pointer conversion".to_string(),
                                                ],
                                                attrs: Vec::new(),
                                            };

                                            functions.push(call_function);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } //_ => {} // Ignore other kinds of features
            }
        }
    }

    // If there are no communication ports, create a direct C function wrapper
    if functions.is_empty() {
        functions.push(FunctionDef {
            name: "execute".to_string(),
            params: Vec::new(),
            return_type: Type::Unit,
            body: Block {
                stmts: vec![Statement::Expr(Expr::Unsafe(Box::new(Block {
                    stmts: vec![Statement::Expr(Expr::Call(
                        Box::new(Expr::Path(
                            vec![c_func_name.to_string()],
                            PathType::Namespace,
                        )),
                        Vec::new(),
                    ))],
                    expr: None,
                })))],
                expr: None,
            },
            asyncness: false,
            vis: Visibility::Public,
            docs: vec![
                format!("// Direct execution wrapper for C function {}", c_func_name),
                "// This component has no communication ports".to_string(),
            ],
            attrs: Vec::new(),
        });
    }
    // Create module
    //if !functions.is_empty()

    {
        let mut docs = vec![
            format!(
                "// Auto-generated from AADL subprogram: {}",
                comp.identifier
            ),
            format!("// C binding to: {}", c_func_name),
        ];
        // Add C source file names to the documentation comments
        if !source_files.is_empty() {
            docs.push(format!("// source_files: {}", source_files.join(", ")));
        }

        // Build use statements
        let mut imports = vec![c_func_name.to_string()];
        // println!("types_to_import: {:?}", types_to_import);
        if !types_to_import.is_empty() {
            // Remove Rust primitive types from the import list
            types_to_import.retain(|type_name| !is_rust_primitive_type(type_name));
            //println!("types_to_import after filtering: {:?}", types_to_import);

            imports.extend(types_to_import);
        }

        let use_stmt = Item::Use(UseStatement {
            path: vec!["super".to_string()],
            kind: UseKind::Nested(imports),
        });

        // Build module contents: first add use statements, then functions
        let mut module_items = vec![use_stmt];
        module_items.extend(functions.into_iter().map(Item::Function));

        let module = RustModule {
            name: comp.identifier.to_lowercase(),
            docs,
            //items: functions.into_iter().map(Item::Function).collect(),
            items: module_items,
            attrs: Default::default(),
            vis: Visibility::Public,
        };
        items.push(Item::Mod(Box::new(module)));
    }

    items
}

fn extract_source_files(comp: &ComponentType) -> Vec<String> {
    let mut source_files = Vec::new();

    if let PropertyClause::Properties(props) = &comp.properties {
        for prop in props {
            if let Property::BasicProperty(bp) = prop {
                if bp.identifier.name.to_lowercase() == "source_text" {
                    match &bp.value {
                        PropertyValue::Single(PropertyExpression::String(StringTerm::Literal(
                            text,
                        ))) => {
                            source_files.push(text.clone());
                        }
                        PropertyValue::List(arraylist) => {
                            for item in arraylist {
                                if let PropertyListElement::Value(PropertyExpression::String(
                                    StringTerm::Literal(text),
                                )) = item
                                {
                                    source_files.push(text.clone());
                                }
                            }
                        }
                        _ => {
                            println!("error in extract_source_files");
                        }
                    }
                }
            }
        }
    }

    source_files
}

// Helper function: determine whether a type is a Rust primitive
fn is_rust_primitive_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
            | "bool"
            | "char"
            | "str"
            | "String"
    )
}

/// Find concrete data types from a data component implementation name
/// For example: from POS.Impl, find Field : data POS_Internal_Type
fn find_data_type_from_implementation(
    impl_name: &str,
    package: &Package,
    temp_converter: &AadlConverter,
) -> Vec<String> {
    let mut data_types = Vec::new();

    // Search component implementations in the public section
    if let Some(public_section) = &package.public_section {
        for decl in &public_section.declarations {
            if let AadlDeclaration::ComponentImplementation(impl_) = decl {
                //println!("impl_.name.type_identifier: {}", impl_.name.type_identifier);
                // impl_name may be "POS.Impl", while type_identifier is "POS"
                // Therefore, check whether impl_name starts with type_identifier
                if impl_name.starts_with(&impl_.name.type_identifier) {
                    // Matching component implementation found, search its data subcomponents
                    if let SubcomponentClause::Items(subcomponents) = &impl_.subcomponents {
                        for sub in subcomponents {
                            if sub.category == ComponentCategory::Data {
                                // Extract type name from the data subcomponent
                                if let SubcomponentClassifier::ClassifierReference(
                                    UniqueComponentClassifierReference::Implementation(unirf),
                                ) = &sub.classifier
                                {
                                    // First check whether the type exists in type_mappings
                                    if let Some(type_name) = temp_converter.type_mappings.get(
                                        &unirf.implementation_name.type_identifier.to_lowercase(),
                                    ) {
                                        if let Type::Named(type_name_str) = type_name {
                                            data_types.push(type_name_str.clone());
                                        }
                                    } else {
                                        data_types.push(
                                            unirf.implementation_name.type_identifier.clone(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Also search in the private section
    if let Some(private_section) = &package.private_section {
        for decl in &private_section.declarations {
            if let AadlDeclaration::ComponentImplementation(impl_) = decl {
                // impl_name may be "POS.Impl", while type_identifier is "POS"
                // Therefore, check whether impl_name starts with type_identifier
                if impl_name.starts_with(&impl_.name.type_identifier) {
                    // Matching component implementation found, search its data subcomponents
                    if let SubcomponentClause::Items(subcomponents) = &impl_.subcomponents {
                        for sub in subcomponents {
                            if sub.category == ComponentCategory::Data {
                                // Extract type name from the data subcomponent
                                if let SubcomponentClassifier::ClassifierReference(
                                    UniqueComponentClassifierReference::Implementation(unirf),
                                ) = &sub.classifier
                                {
                                    data_types
                                        .push(unirf.implementation_name.type_identifier.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    data_types
}
