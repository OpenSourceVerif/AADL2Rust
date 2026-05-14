#![allow(clippy::single_match)]
use crate::aadl_ast2rust_code::converter::AadlConverter;
use aadl_intermediate::*;

use crate::aadl_ast2rust_code::tool::*;
use crate::ast::aadl_ast_cj::*;

pub fn convert_process_implementation(
    temp_converter: &mut AadlConverter,
    impl_: &ComponentImplementation,
) -> Vec<Item> {
    let mut items = Vec::new();

    // 0. If the process contains data subcomponents, treat them as shared variables
    // and generate _Shared = Arc<Mutex<_>>
    // generate_shared_data(impl_,&mut items);

    // 1. Generate the process struct
    // This is used to obtain the subcomponents of the process;
    // generation of internal ports is also handled here.
    let mut fields = get_process_fields(temp_converter, impl_);

    // Add CPU ID field
    fields.push(Field {
        name: "cpu_id".to_string(),
        ty: Type::Named("isize".to_string()),
        docs: vec!["// Added CPU ID".to_string()],
        attrs: Vec::new(),
    });

    let struct_def = StructDef {
        name: format! {"{}Process",to_upper_camel_case(&impl_.name.type_identifier)},
        fields,                 // Used to obtain the subcomponents of the process
        generics: Vec::new(),
        derives: vec!["Debug".to_string()],
        docs: vec![
            format!("// Process implementation: {}", impl_.name.type_identifier),
            "// Auto-generated from AADL".to_string(),
        ],
        vis: Visibility::Public,
    };
    items.push(Item::Struct(struct_def));

    // 2. Generate impl block
    items.push(Item::Impl(create_process_impl_block(temp_converter, impl_)));

    items
}

// Add forwarding ports (internal ports used to forward data to subcomponents);
// handle subcomponents (thread + data)
fn get_process_fields(
    temp_converter: &mut AadlConverter,
    impl_: &ComponentImplementation,
) -> Vec<Field> {
    let mut fields = Vec::new();

    // 1. Add process port fields (external ports + internal ports)
    if let Some(comp_type) = temp_converter.get_component_type(impl_) {
        if let FeatureClause::Items(features) = &comp_type.features {
            for feature in features {
                if let Feature::Port(port) = feature {
                    // Add external port
                    fields.push(Field {
                        name: port.identifier.to_lowercase(),
                        ty: temp_converter.convert_port_type(port, "".to_string()),
                        docs: vec![format!("// Port: {} {:?}", port.identifier, port.direction)],
                        attrs: Vec::new(),
                    });

                    // Add corresponding internal port
                    let internal_port_name = match port.direction {
                        PortDirection::In => format!("{}Send", port.identifier.to_lowercase()),
                        PortDirection::Out => format!("{}Rece", port.identifier.to_lowercase()),
                        PortDirection::InOut => {
                            format!("{}Send", port.identifier.to_lowercase())
                        } // InOut is temporarily treated as In
                    };

                    let internal_port_type = match port.direction {
                        PortDirection::In => {
                            // External is a receive port; internally we need a send port
                            match temp_converter.convert_port_type(port, "".to_string()) {
                                Type::Generic(option_name, inner_types)
                                    if option_name == "Option" =>
                                {
                                    if let Type::Generic(channel_name, channel_args) =
                                        &inner_types[0]
                                    {
                                        let mut send_type = "Sender".to_string();
                                        // Convert from Option<Receiver<T>> to Option<BcSender<T>>
                                        if temp_converter.thread_broadcast_receive.contains_key(&(
                                            port.identifier.clone(),
                                            impl_.name.type_identifier.clone(),
                                        )) {
                                            send_type = "BcSender".to_string();
                                        }

                                        if channel_name == "Receiver" {
                                            Type::Generic(
                                                "Option".to_string(),
                                                vec![Type::Generic(
                                                    send_type.clone(),
                                                    channel_args.clone(),
                                                )],
                                            )
                                        } else {
                                            Type::Generic(
                                                "Option".to_string(),
                                                vec![Type::Generic(
                                                    channel_name.clone(),
                                                    channel_args.clone(),
                                                )],
                                            )
                                        }
                                    } else {
                                        Type::Generic(
                                            "Option".to_string(),
                                            vec![Type::Generic(
                                                "Sender".to_string(),
                                                vec![inner_types[0].clone()],
                                            )],
                                        )
                                    }
                                }
                                _ => {
                                    // If not an Option type, create Option<BcSender<T>>
                                    Type::Generic(
                                        "Option".to_string(),
                                        vec![Type::Generic(
                                            "Sender".to_string(),
                                            vec![temp_converter
                                                .convert_port_type(port, "".to_string())],
                                        )],
                                    )
                                }
                            }
                        }
                        PortDirection::Out => {
                            // External is a send port; internally we need a receive port
                            match temp_converter.convert_port_type(port, "".to_string()) {
                                Type::Generic(option_name, inner_types)
                                    if option_name == "Option" =>
                                {
                                    if let Type::Generic(channel_name, channel_args) =
                                        &inner_types[0]
                                    {
                                        if channel_name == "Sender" {
                                            // Convert from Option<Sender<T>> to Option<Receiver<T>>
                                            Type::Generic(
                                                "Option".to_string(),
                                                vec![Type::Generic(
                                                    "Receiver".to_string(),
                                                    channel_args.clone(),
                                                )],
                                            )
                                        } else {
                                            Type::Generic(
                                                "Option".to_string(),
                                                vec![Type::Generic(
                                                    channel_name.clone(),
                                                    channel_args.clone(),
                                                )],
                                            )
                                        }
                                    } else {
                                        Type::Generic(
                                            "Option".to_string(),
                                            vec![Type::Generic(
                                                "Receiver".to_string(),
                                                vec![inner_types[0].clone()],
                                            )],
                                        )
                                    }
                                }
                                _ => {
                                    // If not an Option type, create Option<Receiver<T>>
                                    Type::Generic(
                                        "Option".to_string(),
                                        vec![Type::Generic(
                                            "Receiver".to_string(),
                                            vec![temp_converter
                                                .convert_port_type(port, "".to_string())],
                                        )],
                                    )
                                }
                            }
                        }
                        PortDirection::InOut => {
                            // InOut is temporarily treated as In
                            match temp_converter.convert_port_type(port, "".to_string()) {
                                Type::Generic(option_name, inner_types)
                                    if option_name == "Option" =>
                                {
                                    if let Type::Generic(channel_name, channel_args) =
                                        &inner_types[0]
                                    {
                                        if channel_name == "Receiver" {
                                            // Convert from Option<Receiver<T>> to Option<BcSender<T>>
                                            Type::Generic(
                                                "Option".to_string(),
                                                vec![Type::Generic(
                                                    "BcSender".to_string(),
                                                    channel_args.clone(),
                                                )],
                                            )
                                        } else {
                                            Type::Generic(
                                                "Option".to_string(),
                                                vec![Type::Generic(
                                                    channel_name.clone(),
                                                    channel_args.clone(),
                                                )],
                                            )
                                        }
                                    } else {
                                        Type::Generic(
                                            "Option".to_string(),
                                            vec![Type::Generic(
                                                "BcSender".to_string(),
                                                vec![inner_types[0].clone()],
                                            )],
                                        )
                                    }
                                }
                                _ => {
                                    // If not an Option type, create Option<BcSender<T>>
                                    Type::Generic(
                                        "Option".to_string(),
                                        vec![Type::Generic(
                                            "BcSender".to_string(),
                                            vec![temp_converter
                                                .convert_port_type(port, "".to_string())],
                                        )],
                                    )
                                }
                            }
                        }
                    };

                    fields.push(Field {
                        name: internal_port_name,
                        ty: internal_port_type,
                        docs: vec![format!(
                            "// Internal port: {} {:?}",
                            port.identifier, port.direction
                        )],
                        attrs: Vec::new(),
                    });
                }
            }
        }
    }

    // 2. Add subcomponent fields
    if let SubcomponentClause::Items(subcomponents) = &impl_.subcomponents {
        for sub in subcomponents {
            let type_name = match &sub.classifier {
                SubcomponentClassifier::ClassifierReference(
                    UniqueComponentClassifierReference::Implementation(unirf),
                ) => {
                    // Directly use subcomponent identifier + "Thread"
                    unirf.implementation_name.type_identifier.to_string()
                }
                _ => "UnsupportedComponent".to_string(),
            };

            // Determine field type based on category
            let field_ty = match sub.category {
                ComponentCategory::Thread => {
                    // Save binding relationship between thread and process
                    Type::Named(format!("{}Thread", to_upper_camel_case(&type_name)))
                }
                ComponentCategory::Data => {
                    // Directly use the original type name without case conversion
                    Type::Named(format!("{}Shared", type_name))
                }
                _ => Type::Named(format!("{}Thread", to_upper_camel_case(&type_name))),
            };

            let doc = match sub.category {
                ComponentCategory::Thread => {
                    format!(
                        "// Subcomponent thread ({} : thread {})",
                        sub.identifier, type_name
                    )
                }
                ComponentCategory::Data => {
                    // Directly use the original type name
                    format!("// Shared data ({} : data {})", sub.identifier, type_name)
                }
                _ => format!("// Subcomponent: {}", sub.identifier),
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

    fields
}

fn create_process_impl_block(
    temp_converter: &mut AadlConverter,
    impl_: &ComponentImplementation,
) -> ImplBlock {
    let mut items = Vec::new();

    // Add new method
    items.push(ImplItem::Method(FunctionDef {
        name: "new".to_string(),
        params: vec![Param {
            name: "cpu_id".to_string(),
            ty: Type::Named("isize".to_string()),
        }],
        return_type: Type::Named("Self".to_string()),
        body: create_process_new_body(temp_converter, impl_),
        asyncness: false,
        vis: Visibility::None,
        docs: vec!["// Creates a new process instance".to_string()],
        attrs: Vec::new(),
    }));

    // Add start method
    items.push(ImplItem::Method(FunctionDef {
        name: "run".to_string(),
        params: vec![Param {
            name: "self".to_string(),
            ty: Type::Named("Self".to_string()),
        }],
        return_type: Type::Unit,
        body: create_process_start_body(temp_converter, impl_),
        asyncness: false,
        vis: Visibility::None,
        docs: vec!["// Starts all threads in the process".to_string()],
        attrs: Vec::new(),
    }));

    ImplBlock {
        target: Type::Named(format! {"{}Process",to_upper_camel_case(&impl_.name.type_identifier)}),
        generics: Vec::new(),
        items,
        trait_impl: Some(Type::Named("Process".to_string())),
    }
}

fn create_process_new_body(
    temp_converter: &mut AadlConverter,
    impl_: &ComponentImplementation,
) -> Block {
    let mut stmts = Vec::new();
    // For each thread, collect extra shared variable arguments
    // to be injected into new() (e.g., data access mappings)
    let mut thread_extra_args: std::collections::HashMap<String, Vec<Expr>> =
        std::collections::HashMap::new();

    if let ConnectionClause::Items(connections) = &impl_.connections {
        for conn in connections {
            if let Connection::Access(access_conn) = conn {
                // Only handle data access mappings:
                // ComponentAccess(data) -> SubcomponentAccess{ subcomponent: thread, .. }
                match (&access_conn.source, &access_conn.destination) {
                    (
                        AccessEndpoint::ComponentAccess(data_name),
                        AccessEndpoint::SubcomponentAccess {
                            subcomponent: thread_name,
                            ..
                        },
                    ) => {
                        let thread_key = thread_name.to_lowercase();
                        let data_var = data_name.to_lowercase();
                        let entry = thread_extra_args.entry(thread_key).or_default();
                        // Pass clone: pos_data.clone()
                        // entry.push(Expr::MethodCall(
                        //     Box::new(Expr::Ident(data_var)),
                        //     "clone".to_string(),
                        //     Vec::new(),
                        // ));

                        // Modification: explicitly pass Arc::clone(&pos_data),
                        // i.e., Arc::clone(&data_var)
                        entry.push(Expr::MethodCall(
                            Box::new(Expr::Path(
                                vec!["Arc".to_string(), "clone".to_string()],
                                PathType::Namespace,
                            )),
                            "".to_string(),
                            vec![Expr::Reference(
                                Box::new(Expr::Ident(data_var)),
                                true,
                                false,
                            )],
                        ));
                    }
                    // Other directions are not handled for now
                    _ => {}
                }
            }
        }
    }

    // 1. Create subcomponent instances
    // (Data first, then Thread, to avoid threads referencing
    // undeclared shared variables in new())
    if let SubcomponentClause::Items(subcomponents) = &impl_.subcomponents {
        let mut data_inits: Vec<Statement> = Vec::new();
        let mut thread_inits: Vec<Statement> = Vec::new();

        for sub in subcomponents {
            let type_name = match &sub.classifier {
                SubcomponentClassifier::ClassifierReference(
                    UniqueComponentClassifierReference::Type(type_ref),
                ) => type_ref.implementation_name.type_identifier.clone(),
                SubcomponentClassifier::ClassifierReference(
                    UniqueComponentClassifierReference::Implementation(impl_ref),
                ) => impl_ref.implementation_name.type_identifier.clone(),
                SubcomponentClassifier::Prototype(_) => "UnsupportedPrototype".to_string(),
            };

            let var_name = sub.identifier.to_lowercase();
            // Initialize subcomponents by category:
            // threads call FooThread::new(cpu_id + shared clones),
            // data uses PosShared::new()
            match sub.category {
                ComponentCategory::Data => {
                    // Directly use the original type name without case conversion
                    let shared_ty = format!("{}Shared", type_name);
                    // Generate: let pos: POS.ImplShared = Arc::new(Mutex::new(PosShared::new()));
                    let init_expr = Expr::Call(
                        Box::new(Expr::Path(
                            vec!["Arc".to_string(), "new".to_string()],
                            PathType::Namespace,
                        )),
                        vec![Expr::Call(
                            Box::new(Expr::Path(
                                vec!["Mutex".to_string(), "new".to_string()],
                                PathType::Namespace,
                            )),
                            vec![Expr::Ident(format!("{}::new()", type_name))],
                        )],
                    );
                    data_inits.push(Statement::Let(LetStmt {
                        ifmut: false,
                        name: var_name.to_string(),
                        ty: Some(Type::Named(shared_ty.clone())),
                        init: Some(init_expr),
                    }));
                }
                ComponentCategory::Thread => {
                    // Assemble new() arguments: cpu_id + shared variable clone list
                    // derived from access connections
                    let mut args = vec![Expr::Ident("cpu_id".to_string())];
                    if let Some(extra) = thread_extra_args.get(&sub.identifier.to_lowercase()) {
                        args.extend(extra.clone());
                    }
                    thread_inits.push(Statement::Let(LetStmt {
                        ifmut: true,
                        name: var_name.to_string(),
                        ty: Some(Type::Named(format!(
                            "{}Thread",
                            to_upper_camel_case(&type_name)
                        ))),
                        init: Some(Expr::Call(
                            Box::new(Expr::Path(
                                vec![
                                    format!("{}Thread", to_upper_camel_case(&type_name)),
                                    "new".to_string(),
                                ],
                                PathType::Namespace,
                            )),
                            args,
                        )),
                    }));
                }
                _ => {
                    // Other categories are temporarily treated as threads
                    thread_inits.push(Statement::Let(LetStmt {
                        ifmut: false,
                        name: format!("mut {}", var_name),
                        ty: Some(Type::Named(format!(
                            "{}Thread",
                            to_upper_camel_case(&type_name)
                        ))),
                        init: Some(Expr::Call(
                            Box::new(Expr::Path(
                                vec![
                                    format!("{}Thread", to_upper_camel_case(&type_name)),
                                    "new".to_string(),
                                ],
                                PathType::Namespace,
                            )),
                            vec![Expr::Ident("cpu_id".to_string())],
                        )),
                    }));
                }
            }
        }

        // Shared data first, then threads
        stmts.extend(data_inits);
        stmts.extend(thread_inits);
    }

    // 2. Create internal port variables
    if let Some(comp_type) = temp_converter.get_component_type(impl_) {
        if let FeatureClause::Items(features) = &comp_type.features {
            for feature in features {
                if let Feature::Port(port) = feature {
                    let internal_port_name = match port.direction {
                        PortDirection::In => format!("{}Send", port.identifier.to_lowercase()),
                        PortDirection::Out => format!("{}Rece", port.identifier.to_lowercase()),
                        PortDirection::InOut => format!("{}Send", port.identifier.to_lowercase()),
                    };

                    // Create internal port variable, initialized to None
                    stmts.push(Statement::Let(LetStmt {
                        ifmut: true,
                        name: internal_port_name.clone(),
                        ty: None,
                        init: Some(Expr::Ident("None".to_string())),
                    }));
                }
            }
        }
    }

    // 3. Establish connections
    // Store processed broadcast connections to avoid duplicate handling.
    let mut processed_broadcast_connections = Vec::new();
    // println!("thread_broadcast_receive:{:?}",temp_converter.thread_broadcast_receive);

    if let ConnectionClause::Items(connections) = &impl_.connections {
        for conn in connections {
            if let Connection::Port(port_conn) = conn {
                if let PortEndpoint::ComponentPort(proc_port) = &port_conn.source {
                    if temp_converter
                        .thread_broadcast_receive
                        .contains_key(&(proc_port.clone(), impl_.name.type_identifier.clone()))
                    {
                        if processed_broadcast_connections
                            .contains(&(proc_port.clone(), impl_.name.type_identifier.clone()))
                        {
                            continue;
                        } else {
                            processed_broadcast_connections
                                .push((proc_port.clone(), impl_.name.type_identifier.clone()));
                        }
                    }
                }
                stmts.extend(
                    temp_converter
                        .create_channel_connection(port_conn, impl_.name.type_identifier.clone()),
                );
            }
        }
    }

    // 3. Return struct instance
    let mut field_inits = Vec::new();

    // Add port field initialization
    // (external ports initialized to None, internal ports use variables)
    if let Some(comp_type) = temp_converter.get_component_type(impl_) {
        if let FeatureClause::Items(features) = &comp_type.features {
            for feature in features {
                if let Feature::Port(port) = feature {
                    // External port initialized to None
                    field_inits.push(format!("{}: None", port.identifier.to_lowercase()));

                    // Internal port uses variable name
                    // (assigned during connection handling)
                    let internal_port_name = match port.direction {
                        PortDirection::In => format!("{}Send", port.identifier.to_lowercase()),
                        PortDirection::Out => format!("{}Rece", port.identifier.to_lowercase()),
                        PortDirection::InOut => format!("{}Send", port.identifier.to_lowercase()),
                    };

                    field_inits.push(internal_port_name.to_string());
                }
            }
        }
    }

    // Add subcomponent fields
    if let SubcomponentClause::Items(subcomponents) = &impl_.subcomponents {
        for sub in subcomponents {
            field_inits.push(sub.identifier.to_lowercase());
        }
    }

    // Add cpu_id field
    field_inits.push("cpu_id".to_string());

    let all_fields = field_inits.join(", ");

    stmts.push(Statement::Expr(Expr::Ident(format!(
        "Self {{ {} }}  // finalize process",
        all_fields
    ))));

    Block { stmts, expr: None }
}

fn create_process_start_body(
    temp_converter: &mut AadlConverter,
    impl_: &ComponentImplementation,
) -> Block {
    let mut stmts = Vec::new();

    // 1. Destructure self to obtain all required fields
    let mut destructure_fields = Vec::new();
    let mut thread_fields = Vec::new();
    let mut port_fields = Vec::new();

    // 1.1 Add port fields (from features)
    if let Some(comp_type) = temp_converter.get_component_type(impl_) {
        if let FeatureClause::Items(features) = &comp_type.features {
            for feature in features {
                if let Feature::Port(port) = feature {
                    // Add external port
                    let port_name = port.identifier.to_lowercase();
                    destructure_fields.push(port_name.clone());
                    port_fields.push(port_name);

                    // Add internal port
                    let internal_port_name = match port.direction {
                        PortDirection::In => format!("{}Send", port.identifier.to_lowercase()),
                        PortDirection::Out => format!("{}Rece", port.identifier.to_lowercase()),
                        PortDirection::InOut => format!("{}Send", port.identifier.to_lowercase()),
                    };
                    destructure_fields.push(internal_port_name.clone());
                    port_fields.push(internal_port_name);
                }
            }
        }
    }

    // 1.2 Add subcomponent fields
    if let SubcomponentClause::Items(subcomponents) = &impl_.subcomponents {
        for sub in subcomponents {
            let var_name = sub.identifier.to_lowercase();
            destructure_fields.push(var_name.clone());

            match sub.category {
                ComponentCategory::Thread => {
                    thread_fields.push(var_name);
                }
                ComponentCategory::Data => {
                    // Data components may be used as ports
                    port_fields.push(var_name);
                }
                _ => {}
            }
        }
    }

    // 1.3 Add cpu_id field
    // destructure_fields.push("cpu_id".to_string());

    // Create destructuring statement:
    // let Self { port1, port1Send, th_c, cpu_id, .. } = self;
    let destructure_stmt = Statement::Let(LetStmt {
        ifmut: false,
        name: format!("Self {{ {}, .. }}", destructure_fields.join(", ")),
        ty: None,
        init: Some(Expr::Ident("self".to_string())),
    });
    stmts.push(destructure_stmt);

    // 2. Start all thread subcomponents (using destructured variables)
    for thread_name in thread_fields {
        // Build thread closure (using move semantics)
        let closure = Expr::Closure(
            Vec::new(), // no parameters
            Box::new(Expr::MethodCall(
                Box::new(Expr::Ident(thread_name.clone())),
                "run".to_string(),
                Vec::new(),
            )),
        );

        // Build thread builder expression chain
        let builder_chain = vec![
            BuilderMethod::Named(format!("\"{}\".to_string()", thread_name)),
            BuilderMethod::Spawn {
                closure: Box::new(closure),
                move_kw: true,
            },
        ];

        stmts.push(Statement::Expr(Expr::MethodCall(
            Box::new(Expr::BuilderChain(builder_chain)),
            "unwrap".to_string(),
            Vec::new(),
        )));
    }

    // 3. Start data forwarding loops (using destructured variables)
    let mut forwarding_tasks = create_data_forwarding_tasks(impl_);
    forwarding_tasks.sort();
    forwarding_tasks.dedup();

    for (src_field, dst_field) in forwarding_tasks {
        // Create receiver variable: let evenementRece_rx = evenementRece.unwrap();
        let rx_var_name = format!("{}_rx", src_field);
        stmts.push(Statement::Let(LetStmt {
            ifmut: true,
            name: rx_var_name.clone(),
            ty: None,
            init: Some(Expr::MethodCall(
                Box::new(Expr::Ident(src_field.clone())),
                "unwrap".to_string(),
                Vec::new(),
            )),
        }));

        // Create forwarding thread
        let forwarding_loop = create_single_forwarding_thread(&rx_var_name, &dst_field);
        let closure = Expr::Closure(
            Vec::new(),
            Box::new(Expr::Block(Block {
                stmts: forwarding_loop,
                expr: None,
            })),
        );

        // Build thread builder expression chain
        let builder_chain = vec![
            BuilderMethod::Named(format!("\"data_forwarder_{}\".to_string()", src_field)),
            BuilderMethod::Spawn {
                closure: Box::new(closure),
                move_kw: true, // add move keyword
            },
        ];

        stmts.push(Statement::Expr(Expr::MethodCall(
            Box::new(Expr::BuilderChain(builder_chain)),
            "unwrap".to_string(),
            Vec::new(),
        )));
    }

    Block { stmts, expr: None }
}

/// Create data forwarding task list
fn create_data_forwarding_tasks(impl_: &ComponentImplementation) -> Vec<(String, String)> {
    let mut forwarding_tasks = Vec::new();

    if let ConnectionClause::Items(connections) = &impl_.connections {
        for conn in connections {
            if let Connection::Port(port_conn) = conn {
                // Resolve source and destination ports
                let (src_field, dst_field) = match (&port_conn.source, &port_conn.destination) {
                    // Process port to subcomponent port
                    (
                        PortEndpoint::ComponentPort(src_port),
                        PortEndpoint::SubcomponentPort {
                            subcomponent: _dst_comp,
                            port: _dst_port,
                        },
                    ) => {
                        // For process ports, use internal port field name (e.g., evenementSend)
                        let src_field = src_port.to_lowercase().to_string();
                        let dst_field = format!("{}Send", src_port.to_lowercase());
                        (src_field, dst_field)
                    }
                    // Subcomponent port to process port
                    (
                        PortEndpoint::SubcomponentPort {
                            subcomponent: _src_comp,
                            port: _src_port,
                        },
                        PortEndpoint::ComponentPort(dst_port),
                    ) => {
                        let src_field = format!("{}Rece", dst_port.to_lowercase());
                        // For process ports, use internal port field name (e.g., evenementRece)
                        let dst_field = dst_port.to_lowercase().to_string();
                        (src_field, dst_field)
                    }
                    _ => continue,
                };

                forwarding_tasks.push((src_field, dst_field));
            }
        }
    }

    forwarding_tasks
}

/// Create code for a single forwarding thread
fn create_single_forwarding_thread(rx_var_name: &str, dst_field: &str) -> Vec<Statement> {
    let mut stmts = Vec::new();

    // Create forwarding loop:
    // loop { if let Ok(msg) = rx_var_name.try_recv() { ... } }
    let loop_body = vec![
        Statement::Expr(Expr::IfLet {
            pattern: "Ok(msg)".to_string(),
            value: Box::new(Expr::MethodCall(
                Box::new(Expr::Ident(rx_var_name.to_string())),
                "try_recv".to_string(),
                Vec::new(),
            )),
            then_branch: Block {
                stmts: vec![Statement::Expr(Expr::IfLet {
                    pattern: "Some(tx)".to_string(),
                    value: Box::new(Expr::Reference(
                        Box::new(Expr::Ident(dst_field.to_string())),
                        true,
                        false,
                    )),
                    then_branch: Block {
                        stmts: vec![Statement::Let(LetStmt {
                            ifmut: false,
                            name: "_".to_string(),
                            ty: None,
                            init: Some(Expr::MethodCall(
                                Box::new(Expr::Ident("tx".to_string())),
                                "send".to_string(),
                                vec![Expr::Ident("msg".to_string())],
                            )),
                        })],
                        expr: None,
                    },
                    else_branch: None,
                })],
                expr: None,
            },
            else_branch: None,
        }),
        // Add sleep to avoid excessive CPU usage
        Statement::Expr(Expr::MethodCall(
            Box::new(Expr::Path(
                vec!["std".to_string(), "thread".to_string(), "sleep".to_string()],
                PathType::Namespace,
            )),
            "".to_string(),
            vec![Expr::MethodCall(
                Box::new(Expr::Path(
                    vec![
                        "std".to_string(),
                        "time".to_string(),
                        "Duration".to_string(),
                        "from_millis".to_string(),
                    ],
                    PathType::Namespace,
                )),
                "".to_string(),
                vec![Expr::Literal(Literal::Int(1))],
            )],
        )),
    ];

    // Create infinite loop
    stmts.push(Statement::Expr(Expr::Loop(Box::new(Block {
        stmts: loop_body,
        expr: None,
    }))));

    stmts
}
