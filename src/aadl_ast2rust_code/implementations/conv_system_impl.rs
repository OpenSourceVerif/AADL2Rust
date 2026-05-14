use crate::aadl_ast2rust_code::converter::AadlConverter;
use aadl_intermediate::*;

use crate::aadl_ast2rust_code::tool::*;
use crate::ast::aadl_ast_cj::*;

pub fn convert_system_implementation(
    temp_converter: &mut AadlConverter,
    impl_: &ComponentImplementation,
) -> Vec<Item> {
    let mut items = Vec::new();

    // 1. Generate the system struct
    let fields = get_system_fields(impl_); // Get system subcomponents

    let struct_def = StructDef {
        name: format!("{}System", to_upper_camel_case(&impl_.name.type_identifier)),
        fields,                 // System subcomponents
        generics: Vec::new(),
        derives: vec!["Debug".to_string()],
        docs: vec![
            format!("// System implementation: {}", impl_.name.type_identifier),
            "// Auto-generated from AADL".to_string(),
        ],
        vis: Visibility::Public,
    };
    items.push(Item::Struct(struct_def));

    // 2. Generate impl block
    items.push(Item::Impl(create_system_impl_block(temp_converter, impl_)));

    items
}

fn get_system_fields(impl_: &ComponentImplementation) -> Vec<Field> {
    let mut fields = Vec::new();

    if let SubcomponentClause::Items(subcomponents) = &impl_.subcomponents {
        for sub in subcomponents {
            if matches!(
                sub.category,
                ComponentCategory::Process | ComponentCategory::Device
            ) {
                let type_name = match &sub.classifier {
                    SubcomponentClassifier::ClassifierReference(
                        UniqueComponentClassifierReference::Implementation(unirf),
                    ) => unirf.implementation_name.type_identifier.clone(),
                    _ => "UnsupportedComponent".to_string(),
                };

                let type_suffix = match sub.category {
                    ComponentCategory::Process => "Process",
                    ComponentCategory::Device => "Device",
                    _ => unreachable!("Filtered above"),
                };

                let field_ty = Type::Named(format!(
                    "{}{}",
                    to_upper_camel_case(&type_name),
                    type_suffix
                ));
                let doc = match sub.category {
                    ComponentCategory::Process => {
                        format!(
                            "// Subcomponent process ({} : process {})",
                            sub.identifier, type_name
                        )
                    }
                    ComponentCategory::Device => {
                        format!(
                            "// Subcomponent device ({} : device {})",
                            sub.identifier, type_name
                        )
                    }
                    _ => unreachable!("Filtered above"),
                };

                fields.push(Field {
                    name: sub.identifier.to_lowercase(),
                    ty: field_ty,
                    docs: vec![doc],
                    // attrs: vec![Attribute {
                    //     name: "allow".to_string(),
                    //     args: vec![AttributeArg::Ident("dead_code".to_string())],
                    // }],
                    attrs: Vec::new(),
                });
            }
        }
    }

    fields
}

fn create_system_impl_block(
    temp_converter: &mut AadlConverter,
    impl_: &ComponentImplementation,
) -> ImplBlock {
    let mut items = Vec::new();

    // Add new method
    items.push(ImplItem::Method(FunctionDef {
        name: "new".to_string(),
        params: vec![],
        return_type: Type::Named("Self".to_string()),
        body: create_system_new_body(temp_converter, impl_),
        asyncness: false,
        vis: Visibility::None,
        docs: vec!["// Creates a new system instance".to_string()],
        attrs: Vec::new(),
    }));

    // Add run method
    items.push(ImplItem::Method(FunctionDef {
        name: "run".to_string(),
        params: vec![Param {
            name: "self".to_string(),
            ty: Type::Named("Self".to_string()),
        }],
        return_type: Type::Unit,
        body: create_system_run_body(impl_),
        asyncness: false,
        vis: Visibility::None,
        docs: vec!["// Runs the system, starts all processes".to_string()],
        attrs: Vec::new(),
    }));

    ImplBlock {
        target: Type::Named(format!(
            "{}System",
            to_upper_camel_case(&impl_.name.type_identifier)
        )),
        generics: Vec::new(),
        items,
        trait_impl: Some(Type::Named("System".to_string())),
    }
}

// Extract processor binding information from the system implementation
fn extract_processor_bindings(impl_: &ComponentImplementation) -> Vec<(String, String)> {
    let mut bindings = Vec::new();

    if let PropertyClause::Properties(properties) = &impl_.properties {
        for property in properties {
            if let Property::BasicProperty(basic_prop) = property {
                if basic_prop.identifier.name.to_lowercase() == "actual_processor_binding" {
                    if let PropertyValue::Single(PropertyExpression::Reference(ref_term)) =
                        &basic_prop.value
                    {
                        if let Some(applies_to) = &ref_term.applies_to {
                            // Format: (process_name, CPU_identifier)
                            bindings.push((applies_to.clone(), ref_term.identifier.clone()));
                        }
                    }
                }
            }
        }
    }

    bindings
}
// Create the new() method body for the system instance
fn create_system_new_body(
    temp_converter: &mut AadlConverter,
    impl_: &ComponentImplementation,
) -> Block {
    let mut stmts = Vec::new();

    // 1. Extract processor binding information and create CPU mapping
    let processor_bindings = extract_processor_bindings(impl_);

    // Assign an ID to each unique CPU name (if not already assigned)
    for (_, cpu_name) in &processor_bindings {
        if !temp_converter.cpu_name_to_id_mapping.contains_key(cpu_name) {
            let next_id: isize = temp_converter
                .cpu_name_to_id_mapping
                .len()
                .try_into()
                .expect("length does not fit into isize");
            temp_converter
                .cpu_name_to_id_mapping
                .insert(cpu_name.clone(), next_id);
        }
    }

    // If there is no processor binding, default to CPU 0
    // if temp_converter.cpu_name_to_id_mapping.is_empty() {
    //     temp_converter
    //         .cpu_name_to_id_mapping
    //         .insert("default".to_string(), 0);
    // }

    // 2. Create subcomponent instances - handle process and device subcomponents
    if let SubcomponentClause::Items(subcomponents) = &impl_.subcomponents {
        for sub in subcomponents {
            let var_name = sub.identifier.to_lowercase();
            let type_name = match &sub.classifier {
                SubcomponentClassifier::ClassifierReference(
                    UniqueComponentClassifierReference::Implementation(unirf),
                ) => unirf.implementation_name.type_identifier.clone(),
                _ => "UnsupportedComponent".to_string(),
            };

            match sub.category {
                ComponentCategory::Process => {
                    // Look up the CPU binding for this process
                    let cpu_id = processor_bindings
                        .iter()
                        .find(|(process_name, _)| process_name == &sub.identifier)
                        .and_then(|(_, cpu_name)| {
                            temp_converter.cpu_name_to_id_mapping.get(cpu_name).copied()
                        })
                        .unwrap_or(-1); // Default to CPU -1

                    let creation_stmt = format!(
                        "let mut {}: {}Process = {}Process::new({})",
                        var_name,
                        to_upper_camel_case(&type_name),
                        to_upper_camel_case(&type_name),
                        cpu_id
                    );
                    stmts.push(Statement::Expr(Expr::Ident(creation_stmt)));
                }
                ComponentCategory::Device => {
                    let creation_stmt = format!(
                        "let mut {}: {}Device = {}Device::new()",
                        var_name,
                        to_upper_camel_case(&type_name),
                        to_upper_camel_case(&type_name)
                    );
                    stmts.push(Statement::Expr(Expr::Ident(creation_stmt)));
                }
                _ => {}
            }
        }
    }

    // 2. Build connections (if any)
    // Store processed broadcast connections within this function to avoid duplicate handling.
    let mut processed_broadcast_connections = Vec::new();

    if let ConnectionClause::Items(connections) = &impl_.connections {
        for conn in connections {
            match conn {
                Connection::Port(port_conn) => {
                    // Check whether the connection is a broadcast that has already been processed.
                    // If so, skip it.
                    if let PortEndpoint::SubcomponentPort { subcomponent, port } = &port_conn.source
                    {
                        if temp_converter
                            .process_broadcast_send
                            .contains(&(subcomponent.clone(), port.clone()))
                        {
                            if processed_broadcast_connections
                                .contains(&(subcomponent.clone(), port.clone()))
                            {
                                continue;
                            } else {
                                processed_broadcast_connections
                                    .push((subcomponent.clone(), port.clone()));
                            }
                        }
                    }
                    // Handle port connection using the same logic as in processes
                    stmts.extend(
                        temp_converter.create_channel_connection(
                            port_conn,
                            impl_.name.type_identifier.clone(),
                        ),
                    );
                }
                _ => {
                    // For other connection types, generate a TODO comment
                    stmts.push(Statement::Expr(Expr::Ident(format!(
                        "// TODO: Unsupported connection type in system: {:?}",
                        conn
                    ))));
                }
            }
        }
    }

    // 3. Build return statement
    let mut field_names = Vec::new();
    if let SubcomponentClause::Items(subcomponents) = &impl_.subcomponents {
        for sub in subcomponents {
            if matches!(
                sub.category,
                ComponentCategory::Process | ComponentCategory::Device
            ) {
                field_names.push(sub.identifier.to_lowercase());
            }
        }
    }

    let fields_str = field_names.join(", ");
    stmts.push(Statement::Expr(Expr::Ident(format!(
        "Self {{ {} }}  // finalize system ",
        fields_str
    ))));

    Block { stmts, expr: None }
}

// Create the run() method body for the system instance
fn create_system_run_body(impl_: &ComponentImplementation) -> Block {
    let mut stmts = Vec::new();

    if let SubcomponentClause::Items(subcomponents) = &impl_.subcomponents {
        for sub in subcomponents {
            let var_name = sub.identifier.to_lowercase();
            match sub.category {
                ComponentCategory::Process => {
                    let start_stmt = format!("self.{}.run()", var_name);
                    stmts.push(Statement::Expr(Expr::Ident(start_stmt)));
                }
                ComponentCategory::Device => {
                    // Build thread closure (capture self with move semantics)
                    let closure = Expr::Closure(
                        Vec::new(), // no parameters
                        Box::new(Expr::MethodCall(
                            Box::new(Expr::Path(
                                vec!["self".to_string(), var_name.clone()],
                                PathType::Member,
                            )),
                            "run".to_string(),
                            Vec::new(),
                        )),
                    );

                    // Build thread builder expression chain
                    let builder_chain = vec![
                        BuilderMethod::Named(format!("\"{}\".to_string()", var_name)),
                        BuilderMethod::Spawn {
                            closure: Box::new(closure),
                            move_kw: true, // Use move to capture self
                        },
                    ];

                    stmts.push(Statement::Expr(Expr::MethodCall(
                        Box::new(Expr::BuilderChain(builder_chain)),
                        "unwrap".to_string(),
                        Vec::new(),
                    )));
                }
                _ => {}
            }
        }
    }

    Block { stmts, expr: None }
}
