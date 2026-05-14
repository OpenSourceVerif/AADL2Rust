use aadl_intermediate::*;

use crate::aadl_ast2rust_code::converter::AadlConverter;
use crate::aadl_ast2rust_code::tool::*;
use crate::ast::aadl_ast_cj::*;

pub fn convert_device_component(temp_converter: &AadlConverter, comp: &ComponentType) -> Vec<Item> {
    let mut items = Vec::new();

    // 1. Struct definition
    let mut fields = temp_converter.convert_type_features(&comp.features, comp.identifier.clone()); // Feature list (port fields)

    // Add period field
    fields.push(Field {
        name: "period_ms".to_string(),
        ty: Type::Named("u64".to_string()),
        docs: vec![format!(
            "// Period: {}ms",
            extract_period(comp).unwrap_or(2000)
        )],
        attrs: Vec::new(),
    });

    let struct_name = format!("{}Device", to_upper_camel_case(&comp.identifier));
    let struct_def = StructDef {
        name: struct_name.clone(),
        fields,
        generics: Vec::new(),
        derives: vec!["Debug".to_string()],
        docs: vec![format!("// AADL Device: {}", comp.identifier)],
        vis: Visibility::Public,
    };
    items.push(Item::Struct(struct_def));

    // 2. Impl block (including new and run methods)
    let mut impl_items = Vec::new();

    // Generate new() method
    let period_ms = extract_period(comp).unwrap_or(2000);
    let new_method = create_device_new_method(comp, period_ms);
    impl_items.push(ImplItem::Method(new_method));

    // Generate run() method
    let run_method = create_device_run_method(temp_converter, comp);
    impl_items.push(ImplItem::Method(run_method));

    let impl_block = ImplBlock {
        target: Type::Named(struct_name),
        generics: Vec::new(),
        items: impl_items,
        trait_impl: Some(Type::Named("Device".to_string())),
    };
    items.push(Item::Impl(impl_block));

    items
}

/// Create the device new() method
fn create_device_new_method(comp: &ComponentType, period_ms: u64) -> FunctionDef {
    let mut field_initializations = Vec::new();

    // Initialize all port fields to None
    if let FeatureClause::Items(features) = &comp.features {
        for feature in features {
            if let Feature::Port(port) = feature {
                let port_name = port.identifier.to_lowercase();
                field_initializations.push(format!("            {}: None,", port_name));
            }
        }
    }

    // Initialize the period field
    field_initializations.push(format!("            period_ms: {},", period_ms));

    // Create the struct literal return statement
    let struct_literal = format!(
        "return Self {{\n{}\n        }}",
        field_initializations.join("\n")
    );

    // Create method body
    let body = Block {
        stmts: vec![Statement::Expr(Expr::Ident(struct_literal))],
        expr: None,
    };

    FunctionDef {
        name: "new".to_string(),
        params: Vec::new(),
        return_type: Type::Named("Self".to_string()),
        body,
        asyncness: false,
        vis: Visibility::None,
        docs: vec!["// Creates a new device instance".to_string()],
        attrs: Vec::new(),
    }
}

/// Create the device run() method
fn create_device_run_method(temp_converter: &AadlConverter, comp: &ComponentType) -> FunctionDef {
    let mut stmts = Vec::new();

    // Collect all input ports
    let mut input_ports = Vec::new();
    if let FeatureClause::Items(features) = &comp.features {
        for feature in features {
            if let Feature::Port(port) = feature {
                if port.direction == PortDirection::In {
                    input_ports.push(port.clone());
                }
            }
        }
    }

    // Collect all output ports
    let mut output_ports = Vec::new();
    if let FeatureClause::Items(features) = &comp.features {
        for feature in features {
            if let Feature::Port(port) = feature {
                if port.direction == PortDirection::Out {
                    output_ports.push(port.clone());
                }
            }
        }
    }

    // Create periodic Duration
    stmts.push(Statement::Let(LetStmt {
        ifmut: false,
        name: "period".to_string(),
        ty: Some(Type::Path(vec![
            "std".to_string(),
            "time".to_string(),
            "Duration".to_string(),
        ])),
        init: Some(Expr::Call(
            Box::new(Expr::Path(
                vec!["Duration".to_string(), "from_millis".to_string()],
                PathType::Namespace,
            )),
            vec![Expr::Path(
                vec!["self".to_string(), "period_ms".to_string()],
                PathType::Member,
            )],
        )),
    }));

    // Create random number generator
    if !output_ports.is_empty() {
        stmts.push(Statement::Let(LetStmt {
            ifmut: true,
            name: "rng".to_string(),
            ty: None,
            init: Some(Expr::Call(
                Box::new(Expr::Path(
                    vec!["rand".to_string(), "thread_rng".to_string()],
                    PathType::Namespace,
                )),
                Vec::new(),
            )),
        }));
    }

    // Create main loop
    let mut loop_stmts = Vec::new();

    // Record start time
    loop_stmts.push(Statement::Let(LetStmt {
        ifmut: false,
        name: "start".to_string(),
        ty: None,
        init: Some(Expr::Call(
            Box::new(Expr::Path(
                vec!["Instant".to_string(), "now".to_string()],
                PathType::Namespace,
            )),
            Vec::new(),
        )),
    }));
    //println!("type_mappings: {:?}", self.type_mappings);

    if !input_ports.is_empty() {
        loop_stmts.push(Statement::Comment(
            "// --- Receive data from input ports ---".to_string(),
        ));
    }

    // Try receiving data for each input port
    for port in &input_ports {
        let port_name = port.identifier.to_lowercase();
        let received_var = format!("{}_in_val", port_name.clone());
        let log_port_name = port_name.clone();
        let field_port_name = port_name.clone();

        let inner_then_block = Block {
            stmts: vec![
                Statement::Expr(Expr::Call(
                    Box::new(Expr::Path(
                        vec!["println!".to_string()],
                        PathType::Namespace,
                    )),
                    vec![
                        Expr::Literal(Literal::Str(format!(
                            "[{}] Received {}: {{:?}}",
                            comp.identifier.to_lowercase(),
                            log_port_name
                        ))),
                        Expr::Ident(received_var.clone()),
                    ],
                )),
                Statement::Comment("// TODO: Add execution logic here".to_string()),
            ],
            expr: None,
        };

        let receive_block = Block {
            stmts: vec![Statement::Expr(Expr::IfLet {
                pattern: format!("Ok({})", received_var),
                value: Box::new(Expr::MethodCall(
                    Box::new(Expr::Ident("rx".to_string())),
                    "try_recv".to_string(),
                    Vec::new(),
                )),
                then_branch: inner_then_block,
                else_branch: None,
            })],
            expr: None,
        };

        loop_stmts.push(Statement::Expr(Expr::IfLet {
            pattern: "Some(rx)".to_string(),
            value: Box::new(Expr::Reference(
                Box::new(Expr::Path(
                    vec!["self".to_string(), field_port_name],
                    PathType::Member,
                )),
                true,
                false,
            )),
            then_branch: receive_block,
            else_branch: None,
        }));
    }

    // Generate data and send for each output port
    for port in &output_ports {
        let port_name = port.identifier.to_lowercase();

        // Determine the data type of the port
        let data_type = match &port.port_type {
            PortType::Data { classifier } | PortType::EventData { classifier } => classifier
                .as_ref()
                .map(|c| temp_converter.classifier_to_type(c))
                .unwrap_or(Type::Named("error_type".to_string())),
            PortType::Event => Type::Named("()".to_string()),
        };

        // Generate random data value (by type)
        // println!("Generating random value for port '{}' of type '{:?}'", port_name, data_type);
        let random_value = match &data_type {
            Type::Named(type_name) => {
                // Special handling for the car case, TODO: adjust type checks and generation logic as needed
                match type_name.as_str() {
                    "i32" | "i64" | "i16" | "i8" | "speed" => {
                        // Generate a random integer in 0-200
                        Expr::MethodCall(
                            Box::new(Expr::Ident("rng".to_string())),
                            "gen_range".to_string(),
                            vec![
                                Expr::Literal(Literal::Int(0)),
                                Expr::Literal(Literal::Int(201)),
                            ],
                        )
                    }
                    "u32" | "u64" | "u16" | "u8" | "pressure" | "contacts" => Expr::MethodCall(
                        Box::new(Expr::Ident("rng".to_string())),
                        "gen_range".to_string(),
                        vec![
                            Expr::Literal(Literal::Int(0)),
                            Expr::Literal(Literal::Int(127)),
                        ],
                    ),
                    "f32" | "f64" => Expr::MethodCall(
                        Box::new(Expr::Ident("rng".to_string())),
                        "gen_range".to_string(),
                        vec![
                            Expr::Literal(Literal::Float(0.0)),
                            Expr::Literal(Literal::Float(200.0)),
                        ],
                    ),
                    "bool" | "obstacle_position" | "music" => {
                        Expr::MethodCall(
                            Box::new(Expr::Ident("rng".to_string())),
                            "gen_bool".to_string(),
                            vec![Expr::Literal(Literal::Float(0.9))], // 90% probability of being true
                        )
                    }
                    "picture" => {
                        // Generate {core::array::from_fn(|_| { core::array::from_fn(|_| rng.gen_range(0..=100)) }) };
                        // Keep it simple and wrap directly in Ident()
                        Expr::Ident("{core::array::from_fn(|_| { core::array::from_fn(|_| rng.gen_range(0,200)) }) }".to_string())
                    }
                    "error_type" => Expr::Ident("// please customize".to_string()),
                    _ => Expr::Ident("// please customize".to_string()), // Default
                }
            }
            _ => Expr::Literal(Literal::Int(0)),
        };

        // Create a variable to store the generated value
        let value_var = format!("{}_val", port_name);
        loop_stmts.push(Statement::Let(LetStmt {
            ifmut: false,
            name: value_var.clone(),
            ty: None,
            init: Some(random_value),
        }));

        // Send data (if the port exists)
        let send_block = Block {
            stmts: vec![
                Statement::Let(LetStmt {
                    ifmut: false,
                    name: "_".to_string(),
                    ty: None,
                    init: Some(Expr::MethodCall(
                        Box::new(Expr::Ident("tx".to_string())),
                        "send".to_string(),
                        vec![Expr::Ident(value_var.clone())],
                    )),
                }),
                Statement::Expr(Expr::Call(
                    Box::new(Expr::Path(
                        vec!["println!".to_string()],
                        PathType::Namespace,
                    )),
                    vec![
                        Expr::Literal(Literal::Str(format!(
                            "[{}] send {} = {{:?}}",
                            comp.identifier.to_lowercase(),
                            port_name
                        ))),
                        Expr::Ident(value_var),
                    ],
                )),
            ],
            expr: None,
        };

        loop_stmts.push(Statement::Expr(Expr::IfLet {
            pattern: "Some(tx)".to_string(),
            value: Box::new(Expr::Reference(
                Box::new(Expr::Path(
                    vec!["self".to_string(), port_name.clone()],
                    PathType::Member,
                )),
                true,
                false,
            )),
            then_branch: send_block,
            else_branch: None,
        }));
    }

    // Compute elapsed time and sleep for the remaining time
    loop_stmts.push(Statement::Let(LetStmt {
        ifmut: false,
        name: "elapsed".to_string(),
        ty: None,
        init: Some(Expr::MethodCall(
            Box::new(Expr::Ident("start".to_string())),
            "elapsed".to_string(),
            Vec::new(),
        )),
    }));

    // Sleep for the remaining time
    loop_stmts.push(Statement::Expr(Expr::If {
        condition: Box::new(Expr::BinaryOp(
            Box::new(Expr::Ident("elapsed".to_string())),
            "<".to_string(),
            Box::new(Expr::Ident("period".to_string())),
        )),
        then_branch: Block {
            stmts: vec![Statement::Expr(Expr::MethodCall(
                Box::new(Expr::Path(
                    vec!["std".to_string(), "thread".to_string(), "sleep".to_string()],
                    PathType::Namespace,
                )),
                "".to_string(),
                vec![Expr::MethodCall(
                    Box::new(Expr::Ident("period".to_string())),
                    "saturating_sub".to_string(),
                    vec![Expr::Ident("elapsed".to_string())],
                )],
            ))],
            expr: None,
        },
        else_branch: None,
    }));

    // Add loop statements into the main statement list
    stmts.push(Statement::Expr(Expr::Loop(Box::new(Block {
        stmts: loop_stmts,
        expr: None,
    }))));

    FunctionDef {
        name: "run".to_string(),
        params: vec![Param {
            name: "self".to_string(),
            ty: Type::Named("Self".to_string()),
        }],
        return_type: Type::Unit,
        body: Block { stmts, expr: None },
        asyncness: false,
        vis: Visibility::None,
        docs: vec![
            "// Device execution entry point - periodically generates and sends data".to_string(),
        ],
        attrs: Vec::new(),
    }
}

fn extract_period(comp: &ComponentType) -> Option<u64> {
    if let PropertyClause::Properties(props) = &comp.properties {
        for prop in props {
            if let Property::BasicProperty(bp) = prop {
                if bp.identifier.name.to_lowercase() == "period" {
                    if let PropertyValue::Single(PropertyExpression::Integer(
                        SignedIntergerOrConstant::Real(int_val),
                    )) = &bp.value
                    {
                        return Some(int_val.value as u64);
                    }
                }
            }
        }
    }
    None
}
