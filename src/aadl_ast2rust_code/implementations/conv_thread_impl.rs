#![allow(clippy::vec_init_then_push)]
#![allow(clippy::single_match)]
use crate::aadl_ast2rust_code::aadl_property::*;
use aadl_intermediate::*;
use crate::aadl_ast2rust_code::converter::AadlConverter;
use crate::aadl_ast2rust_code::converter_annex::AnnexConverter;

use crate::ast::aadl_ast_cj::*;
use std::collections::HashMap;
use crate::aadl_ast2rust_code::tool::*;


pub fn convert_thread_implemenation(temp_converter: &mut AadlConverter, impl_: &ComponentImplementation) -> Vec<Item> {
    let mut items = Vec::new();

    // 1. Struct definition
    let mut fields = Vec::new(); // For thread implementations, there are no features; fields are derived from properties here
    let struct_name = format!("{}Thread", to_upper_camel_case(&impl_.name.type_identifier));
    let mut field_values = HashMap::new();

    // Merge implementation-level properties into fields and store property values
    if let PropertyClause::Properties(props) = &impl_.properties {
        for prop in props {
            if let Property::BasicProperty(bp) = prop {
                if let Some(val) = temp_converter.parse_property_value(&bp.value) {
                    let field_name = bp.identifier.name.to_lowercase();
                    let ty_name = temp_converter.type_for_property(&val);
                    
                    // Store property values into thread_field_values
                    field_values.insert(field_name.clone(), val.clone());
                    fields.push(Field {
                        name: field_name,
                        ty: Type::Named(ty_name),
                        docs: vec![format!("// AADL property (impl): {}", bp.identifier.name)],
                        attrs: Vec::new(),
                    });
                }
            }
        }
    }
    
    // Append implementation-level property values into thread_field_values
    if !field_values.is_empty() {
        // Get existing field-value map; create a new one if absent
        let existing_values = temp_converter.thread_field_values.entry(struct_name.clone()).or_default();
        // Append new field values; overwrite if the field already exists
        // (implementation-level properties have higher priority)
        for (key, value) in field_values {
            existing_values.insert(key, value);
        }
    }
    // println!("!!!!!!!!!!!!thread_field_values: {:?}", self.thread_field_values);
    
    let struct_def = StructDef {
        name: format!("{}Thread", to_upper_camel_case(&impl_.name.type_identifier)),
        fields,
        generics: Vec::new(),
        derives: vec!["Debug".to_string()],
        docs: temp_converter.create_component_impl_docs(impl_),
        vis: Visibility::Public, // default public
    };
    items.push(Item::Struct(struct_def));

    // 2. Impl block (contains new and run methods)
    let mut impl_items = Vec::new();
    
    // Generate new() method
    let mut flag_need_shared_variable_param = false;
    let new_method = create_thread_new_method(temp_converter, impl_, &mut flag_need_shared_variable_param);
    impl_items.push(ImplItem::Method(new_method));
    
    // Add run method
    impl_items.push(ImplItem::Method(FunctionDef {
        name: "run".to_string(),
        params: vec![Param {
            name: "".to_string(),
            ty: Type::Reference(Box::new(Type::Named("self".to_string())), false, true),
        }],
        return_type: Type::Unit,
        body: create_thread_run_body(temp_converter, impl_),
        asyncness: false,
        vis: Visibility::None,
        docs: vec![
            "// Thread execution entry point".to_string(),
            format!(
                "// Period: {:?} ms",
                extract_property_value(temp_converter, impl_, "period")
            ),
        ],
        attrs: Vec::new(),
    }));

    let impl_block = ImplBlock {
        target: Type::Named(format!(
            "{}Thread",
            to_upper_camel_case(&impl_.name.type_identifier)
        )),
        generics: Vec::new(),
        items: impl_items,
        trait_impl: Some(Type::Named("Thread".to_string())),
    };
    items.push(Item::Impl(impl_block));

    // Add an extra impl block to generate methods not included in the trait,
    // i.e., a new() method variant that takes shared-variable parameters.
    if flag_need_shared_variable_param {
        let items_no_trait = vec![ImplItem::Method(create_thread_new_method(temp_converter, impl_, &mut flag_need_shared_variable_param))];
        let impl_block_no_trait = ImplBlock {
            target: Type::Named(format!("{}Thread", to_upper_camel_case(&impl_.name.type_identifier))),
            generics: Vec::new(),
            items: items_no_trait,
            trait_impl: None,
        };
        items.push(Item::Impl(impl_block_no_trait));
    }
    

    items
}

/// Create the thread new() method
/// Initialize thread struct fields using stored property values
fn create_thread_new_method(temp_converter: &mut AadlConverter, impl_: &ComponentImplementation, flag_need_shared_variable_param: &mut bool) -> FunctionDef {
    let struct_name = format!("{}Thread", to_upper_camel_case(&impl_.name.type_identifier));
    let key = struct_name.clone();
    
    // Load stored property values
    let field_values = temp_converter.thread_field_values.get(&key).cloned().unwrap_or_default();
    let field_types = temp_converter.thread_field_types.get(&key).cloned().unwrap_or_default();
    
    // Generate struct literal initialization string
    let mut field_initializations = Vec::new();
    let mut params = vec![Param {
        name: "cpu_id".to_string(),
        ty: Type::Named("isize".to_string()),
    }];
    
    // Generate initialization expression and comment for each field
    for (field_name, prop_value) in &field_values {
        let mut init_value = property_value_to_initializer(prop_value);
        let comment = String::new();

        // For field types ending with "Shared", add parameters (removed)
        // and adjust the initialization value
        if let Some(field_type) = field_types.get(field_name) {
            match field_type {
                Type::Named(type_name) => {
                    if type_name.ends_with("Shared") {
                        if !*flag_need_shared_variable_param {
                            *flag_need_shared_variable_param = true;

                             // Generate init value in the form Arc::new(Mutex::new(TypeName::new()))
                            let base_type_name = type_name.trim_end_matches("Shared");
                            init_value = format!("Arc::new(Mutex::new({}::new()))", base_type_name);
                        } else{
                            params.push(Param {
                                name: field_name.clone(),
                                ty: field_type.clone(),
                            });
                        }
                    }
                }
                _ => {
                    // Other types are not used as new() parameters for now
                }
            }
        }
        // Field assignment
        field_initializations.push(format!("            {}: {}, {}", field_name, init_value, comment));
    }
    
    // Add CPU ID field initialization
    field_initializations.push("            cpu_id: cpu_id, // CPU ID".to_string());
    
    // Create struct literal return statement
    let struct_literal = format!("Self {{\n{}\n        }} // finalize thread", field_initializations.join("\n"));
    
    // Create method body
    let body = Block {
        stmts: vec![Statement::Expr(Expr::Ident(struct_literal))],
        expr: None,
    };
    
    FunctionDef {
        name: "new".to_string(),
        params,
        return_type: Type::Named("Self".to_string()),
        body,
        asyncness: false,
        vis: Visibility::None,
        docs: vec!["// Create component and initialize AADL properties".to_string()],
        attrs: Vec::new(),
    }
}


/// Convert a property value into an initializer expression string
fn property_value_to_initializer(val: &StruPropertyValue) -> String {
    match val {
        StruPropertyValue::Boolean(b) => b.to_string(),
        StruPropertyValue::Integer(i) => i.to_string(),
        StruPropertyValue::Float(f) => {
            let s = f.to_string();
            if f.fract() == 0.0 && !s.contains('.') { format!("{s}.0") } else { s }
        }
        StruPropertyValue::String(s) => format!("\"{}\".to_string()", s),
        StruPropertyValue::Duration(v, _unit) => v.to_string(),
        StruPropertyValue::Range(min, max, _unit) => format!("({}, {})", min, max),
        StruPropertyValue::None => "None".to_string(),
        StruPropertyValue::Custom(s) => s.to_string(),
    }
}

/// Create the thread run() method body
/// This method generates the thread execution logic, including:
/// 1. Thread priority and CPU affinity setup
/// 2. Different execution logic per scheduling protocol
/// 3. Subprogram call handling (parameter ports, shared variables, normal calls)
fn create_thread_run_body(temp_converter: &AadlConverter, impl_: &ComponentImplementation) -> Block {
    let mut stmts = Vec::new();
    
    //======================= Thread priority setup ========================
    // Check whether a priority property exists
    let priority = extract_property_value(temp_converter, impl_, "priority");
    let period = extract_property_value(temp_converter, impl_, "period");
    
    // If the thread has a priority property, set thread priority
    if let Some(priority) = priority {
        // Add priority setup code - using unsafe and full error handling
        stmts.push(Statement::Expr(Expr::Unsafe(Box::new(Block {
            stmts: vec![
                // let mut param = sched_param { sched_priority: self.priority as i32 };
                Statement::Let(LetStmt {
                    ifmut: false,
                    name: "param".to_string(),
                    ty: Some(Type::Named("sched_param".to_string())),
                    init: Some(Expr::Ident(format!("sched_param {{ sched_priority: {} }}", priority as i32))),
                }),
                // let ret = pthread_setschedparam(pthread_self(), *, &param);
                Statement::Let(LetStmt {
                    ifmut: false,
                    name: "ret".to_string(),
                    ty: None,
                    init: Some(Expr::Call(
                        Box::new(Expr::Path(
                            vec!["pthread_setschedparam".to_string()],
                            PathType::Namespace,
                        )),
                        vec![
                            Expr::Call(
                                Box::new(Expr::Path(
                                    vec!["pthread_self".to_string()],
                                    PathType::Namespace,
                                )),
                                Vec::new(),
                            ),
                            Expr::MethodCall(
                                Box::new(Expr::MethodCall(
                                    Box::new(Expr::Path(
                                        vec!["*CPU_ID_TO_SCHED_POLICY".to_string()],
                                        PathType::Namespace,
                                    )),
                                    "get".to_string(),
                                    vec![Expr::Reference(
                                        Box::new(Expr::Path(
                                            vec!["self".to_string(), "cpu_id".to_string()],
                                            PathType::Member,
                                        )),
                                        true,
                                        false,
                                    )],
                                )),
                                "unwrap_or".to_string(),
                                vec![Expr::Reference(
                                    Box::new(Expr::Path(
                                        vec!["SCHED_FIFO".to_string()],
                                        PathType::Namespace,
                                    )),
                                    true,
                                    false,
                                )],
                            ),
                            Expr::Reference(
                                Box::new(Expr::Ident("param".to_string())),
                                true,
                                false,
                            ),
                        ],
                    )),
                }),
                // if ret != 0 { eprintln!("..."); }
                Statement::Expr(Expr::If {
                    condition: Box::new(Expr::BinaryOp(
                        Box::new(Expr::Ident("ret".to_string())),
                        "!=".to_string(),
                        Box::new(Expr::Literal(Literal::Int(0))),
                    )),
                    then_branch: Block {
                        stmts: vec![
                            Statement::Expr(Expr::Call(
                                Box::new(Expr::Path(
                                    vec!["eprintln!".to_string()],
                                    PathType::Namespace,
                                )),
                                vec![
                                    Expr::Literal(Literal::Str(format!("{}Thread: Failed to set thread priority: {{}}", to_upper_camel_case(&impl_.name.type_identifier)))),
                                    Expr::Ident("ret".to_string()),
                                ],
                            )),
                        ],
                        expr: None,
                    },
                    else_branch: None,
                }),
            ],
            expr: None,
        }))));
    } else if period.is_some() {
        // If there is no priority but there is a period, compute priority from period (RMS)
        // TODO: not sure why period is not used here
        stmts.push(Statement::Expr(Expr::Unsafe(Box::new(Block {
            stmts: vec![
                // let prio = period_to_priority(self.period as f64); not sure why this line is commented out
                Statement::Let(LetStmt {
                    ifmut: false,
                    name: "prio".to_string(),
                    ty: None,
                    init: Some(Expr::Call(
                        Box::new(Expr::Path(
                            vec!["period_to_priority".to_string()],
                            PathType::Namespace,
                        )),
                        vec![Expr::Ident("self.period as f64".to_string())],
                    )),
                }),
                // let mut param: sched_param = sched_param { sched_priority: prio };
                Statement::Let(LetStmt {
                    ifmut: true,
                    name: "param".to_string(),
                    ty: Some(Type::Named("sched_param".to_string())),
                    init: Some(Expr::Ident("sched_param { sched_priority: prio }".to_string())),
                }),
                // let ret = pthread_setschedparam(pthread_self(), *, &mut param);
                Statement::Let(LetStmt {
                    ifmut: false,
                    name: "ret".to_string(),
                    ty: None,
                    init: Some(Expr::Call(
                        Box::new(Expr::Path(
                            vec!["pthread_setschedparam".to_string()],
                            PathType::Namespace,
                        )),
                        vec![
                            Expr::Call(
                                Box::new(Expr::Path(
                                    vec!["pthread_self".to_string()],
                                    PathType::Namespace,
                                )),
                                Vec::new(),
                            ),
                            Expr::MethodCall(
                                Box::new(Expr::MethodCall(
                                    Box::new(Expr::Path(
                                        vec!["*CPU_ID_TO_SCHED_POLICY".to_string()],
                                        PathType::Namespace,
                                    )),
                                    "get".to_string(),
                                    vec![Expr::Reference(
                                        Box::new(Expr::Path(
                                            vec!["self".to_string(), "cpu_id".to_string()],
                                            PathType::Member,
                                        )),
                                        true,
                                        false,
                                    )],
                                )),
                                "unwrap_or".to_string(),
                                vec![Expr::Reference(
                                    Box::new(Expr::Path(
                                        vec!["SCHED_FIFO".to_string()],
                                        PathType::Namespace,
                                    )),
                                    true,
                                    false,
                                )],
                            ),
                            Expr::Reference(
                                Box::new(Expr::Ident("param".to_string())),
                                true,
                                true,
                            ),
                        ],
                    )),
                }),
                // if ret != 0 { eprintln!("..."); }
                Statement::Expr(Expr::If {
                    condition: Box::new(Expr::BinaryOp(
                        Box::new(Expr::Ident("ret".to_string())),
                        "!=".to_string(),
                        Box::new(Expr::Literal(Literal::Int(0))),
                    )),
                    then_branch: Block {
                        stmts: vec![
                            Statement::Expr(Expr::Call(
                                Box::new(Expr::Path(
                                    vec!["eprintln!".to_string()],
                                    PathType::Namespace,
                                )),
                                vec![
                                    Expr::Literal(Literal::Str(format!("{}Thread: Failed to set thread priority from period: {{}}", to_upper_camel_case(&impl_.name.type_identifier)))),
                                    Expr::Ident("ret".to_string()),
                                ],
                            )),
                        ],
                        expr: None,
                    },
                    else_branch: None,
                }),
            ],
            expr: None,
        }))));
    }

    // ==================== Step 0.5: CPU affinity setup ====================
    // If cpu_id > -1, bind the thread to the specified CPU
    stmts.push(Statement::Expr(Expr::If {
        condition: Box::new(Expr::BinaryOp(
            Box::new(Expr::Path(
                vec!["self".to_string(), "cpu_id".to_string()],
                PathType::Member,
            )),
            ">".to_string(),
            Box::new(Expr::Literal(Literal::Int(-1))),
        )),
        then_branch: Block {
            stmts: vec![
                Statement::Expr(Expr::Call(
                    Box::new(Expr::Path(
                        vec!["set_thread_affinity".to_string()],
                        PathType::Namespace,
                    )),
                    vec![Expr::Path(
                        vec!["self".to_string(), "cpu_id".to_string()],
                        PathType::Member,
                    )],
                )),
            ],
            expr: None,
        },
        else_branch: None,
    }));

    // ==================== Step 1: Retrieve dispatch protocol ====================
    let dispatch_protocol = extract_dispatch_protocol(temp_converter, impl_);
    
    // ==================== Step 2: Generate execution logic based on dispatch protocol ====================
    match dispatch_protocol.as_deref() {
        Some("Periodic") => {
            // Periodic dispatch: generate periodic execution loop
            stmts.extend(create_periodic_execution_logic(temp_converter, impl_));
        }
        Some("Aperiodic") => {
            // Aperiodic dispatch: generate event-driven execution logic
            stmts.extend(create_aperiodic_execution_logic(temp_converter, impl_));
        }
        Some("Sporadic") => {
            // Sporadic dispatch: generate sporadic execution logic
            stmts.extend(create_sporadic_execution_logic(temp_converter, impl_));
        }
        Some("Timed") => {
            // Timed dispatch: generate timed execution logic
            stmts.extend(create_timed_execution_logic(temp_converter, impl_));
        }
        _ => {
            // Default to periodic dispatch
            stmts.extend(create_periodic_execution_logic(temp_converter, impl_));
        }
    }

    Block { stmts, expr: None }
}

/// Create periodic execution logic
fn create_periodic_execution_logic(temp_converter: &AadlConverter, impl_: &ComponentImplementation) -> Vec<Statement> {
    let mut stmts = Vec::new();
    
    // Extract period value from AADL properties; default is 1000ms
    let period = extract_property_value(temp_converter, impl_, "period").unwrap_or(1000);
    // println!("{:?}period:{:?}",impl_.name,period);
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
            vec![Expr::Literal(Literal::Int(period as i64))],
        )),
    }));

    // Add: let mut next_release = Instant::now() + period;
    stmts.push(Statement::Let(LetStmt {
        ifmut: true,
        name: "next_release".to_string(),
        ty: None,
        init: Some(Expr::BinaryOp(
            Box::new(Expr::Ident("Instant::now()".to_string())),
            "+".to_string(),
            Box::new(Expr::Ident("period".to_string())),
        )),
    }));
    
    
    // Handle BA
    let mut annex_converter = AnnexConverter::default();
    // Check whether Behavior Annex exists
    let mut if_has_ba = false;
    
    if annex_converter.find_behavior_annex(impl_).is_some(){
        if_has_ba = true;
        stmts.extend(annex_converter.generate_ba_variables_states(impl_, annex_converter.find_behavior_annex(impl_).unwrap()));
    } 

    let mut ba_stmts = Vec::new();
    if if_has_ba {
        if let Some(transitions) = annex_converter.find_behavior_annex(impl_).unwrap().transitions.clone() {
            ba_stmts.extend(annex_converter.generate_state_machine_loop(&transitions));
        }
    };

    // Subprogram call handling code
    let subprogram_handling_stmts = create_subprogram_call_logic(temp_converter, impl_);


    // Build the statement list inside the loop: scheduling control + subprogram calls + BA execution
    let mut loop_stmts: Vec<Statement> = Vec::new();

    // 1. let now = Instant::now();
    loop_stmts.push(Statement::Let(LetStmt {
        ifmut: false,
        name: "now".to_string(),
        ty: None,
        init: Some(Expr::Call(
            Box::new(Expr::Path(
                vec!["Instant".to_string(), "now".to_string()],
                PathType::Namespace,
            )),
            Vec::new(),
        )),
    }));

    // 2. if now < next_release { std::thread::sleep(next_release - now); }
    loop_stmts.push(Statement::Expr(Expr::If {
        condition: Box::new(Expr::BinaryOp(
            Box::new(Expr::Ident("now".to_string())),
            "<".to_string(),
            Box::new(Expr::Ident("next_release".to_string())),
        )),
        then_branch: Block {
            stmts: vec![Statement::Expr(Expr::MethodCall(
                Box::new(Expr::Path(
                    vec![
                        "std".to_string(),
                        "thread".to_string(),
                        "sleep".to_string(),
                    ],
                    PathType::Namespace,
                )),
                "".to_string(),
                vec![Expr::Ident("next_release - now".to_string())],
            ))],
            expr: None,
        },
        else_branch: None,
    }));

    // 3. Port-handling block
    if !subprogram_handling_stmts.is_empty() {
        loop_stmts.push(Statement::Expr(Expr::Block(Block {
            stmts: subprogram_handling_stmts,
            expr: None,
        })));
    }

    // 4. If a Behavior Annex exists, insert BA execution block
    if !ba_stmts.is_empty() {
        loop_stmts.push(Statement::Expr(Expr::Block(Block {
            stmts: ba_stmts,
            expr: None,
        })));
    }

    // 5. next_release += period;
    loop_stmts.push(Statement::Expr(Expr::BinaryOp(
        Box::new(Expr::Ident("next_release".to_string())),
        "+=".to_string(),
        Box::new(Expr::Ident("period".to_string())),
    )));

    // 6. Build loop expression and push into outer stmts
    stmts.push(Statement::Expr(Expr::Loop(Box::new(Block {
        stmts: loop_stmts,
        expr: None,
    }))));


    stmts
}

/// Create aperiodic execution logic
fn create_aperiodic_execution_logic(temp_converter: &AadlConverter, impl_: &ComponentImplementation) -> Vec<Statement> {
    let mut stmts = Vec::new();

    // Get event port info (event ports or event-data ports)
    let event_ports = extract_event_ports(temp_converter, impl_);

    // Extract event port urgency (priority) info
    let port_urgency = extract_event_port_urgency(impl_);
    // println!("port_urgency: {:?}", port_urgency);
    
    // If no event port is found, get receive ports from parameter connections as a fallback
    let receive_ports = if !event_ports.is_empty() {
        event_ports
    } else {
        Vec::new()
        // let subprogram_calls = extract_subprogram_calls(temp_converter, impl_);
        // subprogram_calls.iter()
        //     .filter(|(_, _, _, is_send, _)| !is_send)
        //     .map(|(_, _, thread_port_name, _, _)| thread_port_name.clone())
        //     .collect()
    };

    // Check whether there are subprogram calls that require port data
    let subprogram_calls = extract_subprogram_calls(temp_converter, impl_);
    let has_receiving_subprograms = subprogram_calls.iter().any(|(_, _, _, is_send, _)| !is_send);

    // Define events outside the loop
    stmts.push(Statement::Let(LetStmt {
        ifmut: true,
        name: "events".to_string(),
        ty: None,
        init: Some(Expr::Call(
            Box::new(Expr::Path(vec!["Vec".to_string(), "new".to_string()], PathType::Namespace)),
            Vec::new(),
        )),
    }));

    // Generate aperiodic execution logic - handle events by priority
    stmts.push(Statement::Expr(Expr::Loop(Box::new(Block {
        stmts: {
            let mut loop_stmts = Vec::new();
            
            // Add event collection logic
            loop_stmts.extend(create_event_collection_logic(&port_urgency, &receive_ports));
            
            // If events exist, pick the highest-priority one to handle
            loop_stmts.push(Statement::Expr(Expr::IfLet {
                pattern: "Some((idx, (_val, _urgency, _ts)))".to_string(),
                value: Box::new(Expr::MethodCall(
                    Box::new(Expr::MethodCall(
                        Box::new(Expr::MethodCall(
                            Box::new(Expr::Ident("events".to_string())),
                            "iter".to_string(),
                            Vec::new(),
                        )),
                        "enumerate".to_string(),
                        Vec::new(),
                    )),
                    "max_by".to_string(),
                    vec![Expr::Ident("|a, b| match a.1.1.cmp(&b.1.1) {\n                        std::cmp::Ordering::Equal => b.1.2.cmp(&a.1.2),\n                        other => other,\n                    }".to_string())],
                )),
                then_branch: Block {
                    stmts: vec![
                        // Remove the handled event
                        Statement::Let(LetStmt {
                            ifmut: false,
                            name: "(val, _, _)".to_string(),
                            ty: None,
                            init: Some(Expr::MethodCall(
                                Box::new(Expr::Ident("events".to_string())),
                                "remove".to_string(),
                                vec![Expr::Ident("idx".to_string())],
                            )),
                        }),
                        // Execute subprogram call handling
                        Statement::Expr(Expr::Block(Block {
                            stmts: create_subprogram_call_logic_with_data(temp_converter, impl_, has_receiving_subprograms),
                            expr: None,
                        })),
                    ],
                    expr: None,
                },
                else_branch: Some(Block {
                    stmts: vec![
                        // If no event, sleep briefly to avoid busy-waiting
                        Statement::Expr(Expr::MethodCall(
                            Box::new(Expr::Path(
                                vec!["std".to_string(), "thread".to_string(), "sleep".to_string()],
                                PathType::Namespace,
                            )),
                            "".to_string(),
                            vec![Expr::Call(
                                Box::new(Expr::Path(
                                    vec!["Duration".to_string(), "from_millis".to_string()],
                                    PathType::Namespace,
                                )),
                                vec![Expr::Literal(Literal::Int(1))], // 1ms
                            )],
                        )),
                    ],
                    expr: None,
                }),
            }));
            
            loop_stmts
        },
        expr: None,
    }))));

    stmts
}

/// Create sporadic execution logic
fn create_sporadic_execution_logic(temp_converter: &AadlConverter, impl_: &ComponentImplementation) -> Vec<Statement> {
    let mut stmts = Vec::new();
    
    // Extract minimum inter-arrival time from AADL properties; default is 1000ms
    let min_interval = extract_property_value(temp_converter, impl_, "period").unwrap_or(1000);
    stmts.push(Statement::Let(LetStmt {
        ifmut: false,
        name: "min_interarrival".to_string(),
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
            vec![Expr::Literal(Literal::Int(min_interval as i64))],
        )),
    }));

    // Initialize last dispatch time
    stmts.push(Statement::Let(LetStmt {
        ifmut: true,
        name: "last_dispatch".to_string(),
        ty: Some(Type::Path(vec![
            "std".to_string(),
            "time".to_string(),
            "Instant".to_string(),
        ])),
        init: Some(Expr::Call(
            Box::new(Expr::Path(
                vec!["Instant".to_string(), "now".to_string()],
                PathType::Namespace,
            )),
            Vec::new(),
        )),
    }));

    // Get event port info (event ports or event-data ports)
    let event_ports = extract_event_ports(temp_converter, impl_);

    // Extract event port urgency (priority) info
    let port_urgency = extract_event_port_urgency(impl_);
    // println!("port_urgency: {:?}", port_urgency);
    
    // If no event port is found, get receive ports from parameter connections as a fallback
    let receive_ports = if !event_ports.is_empty() {
        event_ports
    } else {
        Vec::new()
        // let subprogram_calls = extract_subprogram_calls(temp_converter, impl_);
        // subprogram_calls.iter()
        //     .filter(|(_, _, _, is_send, _)| !is_send)
        //     .map(|(_, _, thread_port_name, _, _)| thread_port_name.clone())
        //     .collect()
    };

    // Check whether there are subprogram calls that require port data
    let subprogram_calls = extract_subprogram_calls(temp_converter, impl_);
    // println!("subprogram_calls{:?}",subprogram_calls);
    let has_receiving_subprograms = subprogram_calls.iter().any(|(_, _, _, is_send, _)| !is_send); // flag: whether any subprogram needs input data

    // Define events outside the loop
    stmts.push(Statement::Let(LetStmt {
        ifmut: true,
        name: "events".to_string(),
        ty: None,
        init: Some(Expr::Call(
            Box::new(Expr::Path(vec!["Vec".to_string(), "new".to_string()], PathType::Namespace)),
            Vec::new(),
        )),
    }));

    // Generate sporadic execution logic - handle events by priority
    stmts.push(Statement::Expr(Expr::Loop(Box::new(Block {
        stmts: {
            let mut loop_stmts = Vec::new();
            
            // Add event collection logic
            loop_stmts.extend(create_event_collection_logic(&port_urgency, &receive_ports));
            
            // If events exist, pick the highest-priority one to handle
            loop_stmts.push(Statement::Expr(Expr::IfLet {
                pattern: "Some((idx, (_val, _urgency, _ts)))".to_string(),
                value: Box::new(Expr::MethodCall(
                    Box::new(Expr::MethodCall(
                        Box::new(Expr::MethodCall(
                            Box::new(Expr::Ident("events".to_string())),
                            "iter".to_string(),
                            Vec::new(),
                        )),
                        "enumerate".to_string(),
                        Vec::new(),
                    )),
                    "max_by".to_string(),
                    vec![Expr::Ident("|a, b| match a.1.1.cmp(&b.1.1) {\n                        std::cmp::Ordering::Equal => b.1.2.cmp(&a.1.2),\n                        other => other,\n                    }".to_string())],
                )),
                then_branch: Block {
                    stmts: vec![
                        // Remove the handled event
                        Statement::Let(LetStmt {
                            ifmut: false,
                            name: "(val, _, _)".to_string(),
                            ty: None,
                            init: Some(Expr::MethodCall(
                                Box::new(Expr::Ident("events".to_string())),
                                "remove".to_string(),
                                vec![Expr::Ident("idx".to_string())],
                            )),
                        }),
                        // Record current time
                        Statement::Let(LetStmt {
                            ifmut: false,
                            name: "now".to_string(),
                            ty: None,
                            init: Some(Expr::Call(
                                Box::new(Expr::Path(
                                    vec!["Instant".to_string(), "now".to_string()],
                                    PathType::Namespace,
                                )),
                                Vec::new(),
                            )),
                        }),
                        // Compute elapsed time since last dispatch
                        Statement::Let(LetStmt {
                            ifmut: false,
                            name: "elapsed".to_string(),
                            ty: None,
                            init: Some(Expr::MethodCall(
                                Box::new(Expr::Ident("now".to_string())),
                                "duration_since".to_string(),
                                vec![Expr::Ident("last_dispatch".to_string())],
                            )),
                        }),
                        // If faster than the minimum inter-arrival, sleep to fill the gap
                        Statement::Expr(Expr::If {
                            condition: Box::new(Expr::BinaryOp(
                                Box::new(Expr::Ident("elapsed".to_string())),
                                "<".to_string(),
                                Box::new(Expr::Ident("min_interarrival".to_string())),
                            )),
                            then_branch: Block {
                                stmts: vec![
                                    Statement::Expr(Expr::MethodCall(
                                        Box::new(Expr::Path(
                                            vec!["std".to_string(), "thread".to_string(), "sleep".to_string()],
                                            PathType::Namespace,
                                        )),
                                        "".to_string(),
                                        vec![Expr::BinaryOp(
                                            Box::new(Expr::Ident("min_interarrival".to_string())),
                                            "-".to_string(),
                                            Box::new(Expr::Ident("elapsed".to_string())),
                                        )],
                                    )),
                                ],
                                expr: None,
                            },
                            else_branch: None,
                        }),
                        // Execute subprogram call handling, passing the already-read data
                        Statement::Expr(Expr::Block(Block {
                            stmts: create_subprogram_call_logic_with_data(temp_converter, impl_, has_receiving_subprograms),
                            expr: None,
                        })),
                        // Update last dispatch time
                        Statement::Expr(Expr::Assign(
                            Box::new(Expr::Ident("last_dispatch".to_string())),
                            Box::new(Expr::Call(
                                Box::new(Expr::Path(
                                    vec!["Instant".to_string(), "now".to_string()],
                                    PathType::Namespace,
                                )),
                                Vec::new(),
                            ))
                        )),
                    ],
                    expr: None,
                },
                else_branch: Some(Block {
                    stmts: vec![
                        // If no event, sleep briefly to avoid busy-waiting
                        Statement::Expr(Expr::MethodCall(
                            Box::new(Expr::Path(
                                vec!["std".to_string(), "thread".to_string(), "sleep".to_string()],
                                PathType::Namespace,
                            )),
                            "".to_string(),
                            vec![Expr::Call(
                                Box::new(Expr::Path(
                                    vec!["Duration".to_string(), "from_millis".to_string()],
                                    PathType::Namespace,
                                )),
                                vec![Expr::Literal(Literal::Int(1))], // 1ms
                            )],
                        )),
                    ],
                    expr: None,
                }),
            }));
            
            loop_stmts
        },
        expr: None,
    }))));

    stmts
}

/// Create timed execution logic
fn create_timed_execution_logic(temp_converter: &AadlConverter, impl_: &ComponentImplementation) -> Vec<Statement> {
    let mut stmts = Vec::new();
    
    // Extract period from AADL properties; default is 1000ms
    let period = extract_property_value(temp_converter, impl_, "period").unwrap_or(1000);
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
            vec![Expr::Literal(Literal::Int(period as i64))],
        )),
    }));

    // Record start time
    stmts.push(Statement::Let(LetStmt {
        ifmut: true,
        name: "start_time".to_string(),
        ty: Some(Type::Path(vec![
            "std".to_string(),
            "time".to_string(),
            "Instant".to_string(),
        ])),
        init: Some(Expr::Call(
            Box::new(Expr::Path(
                vec!["Instant".to_string(), "now".to_string()],
                PathType::Namespace,
            )),
            Vec::new(),
        )),
    }));

    // Get event port info (event ports or event-data ports)
    let event_ports = extract_event_ports(temp_converter, impl_);

    // Extract event port urgency (priority) info
    let port_urgency = extract_event_port_urgency(impl_);
    // println!("port_urgency: {:?}", port_urgency);
    
    // If no event port is found, get receive ports from parameter connections as a fallback
    let receive_ports = if !event_ports.is_empty() {
        event_ports
    } else {
        Vec::new()
        // let subprogram_calls = extract_subprogram_calls(temp_converter,impl_);
        // subprogram_calls.iter()
        //     .filter(|(_, _, _, is_send, _)| !is_send)
        //     .map(|(_, _, thread_port_name, _, _)| thread_port_name.clone())
        //     .collect()
    };

    // Check whether there are subprogram calls that require port data
    let subprogram_calls = extract_subprogram_calls(temp_converter,impl_);
    let has_receiving_subprograms = subprogram_calls.iter().any(|(_, _, _, is_send, _)| !is_send);

    // Define events outside the loop
    stmts.push(Statement::Let(LetStmt {
        ifmut: true,
        name: "events".to_string(),
        ty: None,
        init: Some(Expr::Call(
            Box::new(Expr::Path(vec!["Vec".to_string(), "new".to_string()], PathType::Namespace)),
            Vec::new(),
        )),
    }));

    // Generate timed execution logic - handle events by priority, with timeout support
    stmts.push(Statement::Expr(Expr::Loop(Box::new(Block {
        stmts: {
            let mut loop_stmts = Vec::new();
            
            // Add event collection logic
            loop_stmts.extend(create_event_collection_logic(&port_urgency, &receive_ports));
            
            // If events exist, pick the highest-priority one to handle
            loop_stmts.push(Statement::Expr(Expr::IfLet {
                pattern: "Some((idx, (_val, _urgency, _ts)))".to_string(),
                value: Box::new(Expr::MethodCall(
                    Box::new(Expr::MethodCall(
                        Box::new(Expr::MethodCall(
                            Box::new(Expr::Ident("events".to_string())),
                            "iter".to_string(),
                            Vec::new(),
                        )),
                        "enumerate".to_string(),
                        Vec::new(),
                    )),
                    "max_by".to_string(),
                    vec![Expr::Ident("|a, b| match a.1.1.cmp(&b.1.1) {\n                        std::cmp::Ordering::Equal => b.1.2.cmp(&a.1.2),\n                        other => other,\n                    }".to_string())],
                )),
                then_branch: Block {
                    stmts: vec![
                        // Remove the handled event
                        Statement::Let(LetStmt {
                            ifmut: false,
                            name: "(val, _, _)".to_string(),
                            ty: None,
                            init: Some(Expr::MethodCall(
                                Box::new(Expr::Ident("events".to_string())),
                                "remove".to_string(),
                                vec![Expr::Ident("idx".to_string())],
                            )),
                        }),
                        // --- Compute Entrypoint (normal trigger) ---
                        Statement::Expr(Expr::Block(Block {
                            stmts: create_subprogram_call_logic_with_data(temp_converter, impl_, has_receiving_subprograms),
                            expr: None,
                        })),
                    ],
                    expr: None,
                },
                else_branch: Some(Block {
                    stmts: vec![
                        // Check whether timeout has occurred
                        Statement::Let(LetStmt {
                            ifmut: false,
                            name: "now".to_string(),
                            ty: None,
                            init: Some(Expr::Call(
                                Box::new(Expr::Path(
                                    vec!["Instant".to_string(), "now".to_string()],
                                    PathType::Namespace,
                                )),
                                Vec::new(),
                            )),
                        }),
                        Statement::Let(LetStmt {
                            ifmut: false,
                            name: "elapsed".to_string(),
                            ty: None,
                            init: Some(Expr::MethodCall(
                                Box::new(Expr::Ident("now".to_string())),
                                "duration_since".to_string(),
                                vec![Expr::Ident("start_time".to_string())],
                            )),
                        }),
                        Statement::Expr(Expr::If {
                            condition: Box::new(Expr::BinaryOp(
                                Box::new(Expr::Ident("elapsed".to_string())),
                                ">".to_string(),
                                Box::new(Expr::Ident("period".to_string())),
                            )),
                            then_branch: Block {
                                stmts: vec![
                                    // Print timeout error message
                                    Statement::Expr(Expr::Call(
                                        Box::new(Expr::Path(
                                            vec!["eprintln!".to_string()],
                                            PathType::Namespace,
                                        )),
                                        vec![Expr::Literal(Literal::Str(format!("{}Thread: timeout dispatch → Recover_Entrypoint", to_upper_camel_case(&impl_.name.type_identifier))))],   
                                    )),
                                    // recover_entrypoint();
                                    Statement::Expr(Expr::Ident("// recover_entrypoint()".to_string())),
                                    Statement::Expr(Expr::Ident("start_time = now".to_string())),
                                ],
                                expr: None,
                            },
                            else_branch: None,
                        }),
                    ],
                    expr: None,
                }),
            }));
            
            loop_stmts
        },
        expr: None,
    }))));

    stmts
}

/// Create subprogram call handling logic (extract shared part)
fn create_subprogram_call_logic(temp_converter: &AadlConverter, impl_: &ComponentImplementation) -> Vec<Statement> {
    create_subprogram_call_logic_with_data(temp_converter, impl_, false)
}

/// Create subprogram call handling logic (data-parameter variant)
fn create_subprogram_call_logic_with_data(temp_converter: &AadlConverter, impl_: &ComponentImplementation, has_receiving_subprograms: bool) -> Vec<Statement> {
    let mut port_handling_stmts = Vec::new();

    // Extract subprogram call info with parameter ports
    let subprogram_calls = extract_subprogram_calls(temp_converter, impl_);
    
    // Extract subprogram call sequence from AADL calls section
    let mut mycalls_sequence = Vec::new();
    if let CallSequenceClause::Items(calls_clause) = &impl_.calls {
        for call_clause in calls_clause {
            for subprocall in &call_clause.calls {
                if let CalledSubprogram::Classifier(
                    UniqueComponentClassifierReference::Implementation(temp),
                ) = &subprocall.called
                {
                    let subprogram_name = temp.implementation_name.type_identifier.to_lowercase();
                    mycalls_sequence.push((subprocall.identifier.to_lowercase(), subprogram_name));
                }
            }
        }
    }
    
    // Extract shared-variable access info
    let data_access_calls = extract_data_access_calls(impl_);
    
    // Create subprogram-to-shared-var mapping
    let mut shared_var_subprograms = std::collections::HashMap::new();
    for (subprogram_name, _, shared_var_field) in &data_access_calls {
        shared_var_subprograms.insert(subprogram_name.clone(), shared_var_field.clone());
    }
    
    // Create a set of subprograms with ports
    let subprograms_with_ports: std::collections::HashSet<String> = subprogram_calls.iter()
        .map(|(_, spg_name, _, _, _)| spg_name.clone())
        .collect();
    // println!("subprograms_with_ports: {:?}", subprograms_with_ports);
    // Add call sequence comment
    if !mycalls_sequence.is_empty() {
        let call_sequence = mycalls_sequence.iter()
            .map(|(call_id, _)| format!("{}()", call_id))
            .collect::<Vec<_>>()
            .join(" -> ");
        
        port_handling_stmts.push(Statement::Expr(Expr::Ident(format!(
            "// --- Call sequence (equivalent to the AADL Wrapper)---\n                           // {}",
            call_sequence
        ))));
    }

    // Handle all subprogram calls according to Mycalls order
    for (call_id, subprogram_name) in mycalls_sequence {
        let has_parameter_ports = subprograms_with_ports.contains(&call_id);
        
        port_handling_stmts.push(Statement::Expr(Expr::Ident(format!("// {}", call_id))));
        
        if has_parameter_ports {
            // Subprogram with parameter ports
            if let Some((_, _, thread_port_name, is_send, port_type)) = subprogram_calls.iter()
                .find(|(_, spg_identifier, _, _, _)| spg_identifier == &call_id) {

                if *is_send {
                    // Send mode
                    let mut send_stmts = Vec::new();
                    
                    // Generate an appropriate default value based on port type
                    let default_value = temp_converter.generate_default_value_for_type(port_type);
                    send_stmts.push(Statement::Let(LetStmt {
                        name: "val".to_string(),
                        ty: None,
                        init: Some(default_value),
                        ifmut: true,
                    }));
                    
                    send_stmts.push(Statement::Expr(Expr::Call(
                        Box::new(Expr::Path(
                            vec![subprogram_name.clone(), "send".to_string()],
                            PathType::Namespace,
                        )),
                        vec![Expr::Reference(
                            Box::new(Expr::Ident("val".to_string())),
                            true,
                            true,
                        )],
                    )));
                    
                    send_stmts.push(Statement::Expr(Expr::MethodCall(
                        Box::new(Expr::MethodCall(
                            Box::new(Expr::Ident("sender".to_string())),
                            "send".to_string(),
                            vec![Expr::Ident("val".to_string())],
                        )),
                        "unwrap".to_string(),
                        Vec::new(),
                    )));
                    
                    port_handling_stmts.push(Statement::Expr(Expr::IfLet {
                        pattern: "Some(sender)".to_string(),
                        value: Box::new(Expr::Reference(
                            Box::new(Expr::Path(
                                vec!["self".to_string(), thread_port_name.clone()],
                                PathType::Member,
                            )),
                            true,
                            false,
                        )),
                        then_branch: Block {
                            stmts: send_stmts,
                            expr: None,
                        },
                        else_branch: None,
                    }));
                } else {
                    // Receive mode
                    if has_receiving_subprograms {
                        // If receiving subprograms exist and data has already been read, use the data directly
                        port_handling_stmts.push(Statement::Expr(Expr::Call(
                            Box::new(Expr::Path(
                                vec![subprogram_name.clone(), "receive".to_string()],
                                PathType::Namespace,
                            )),
                            vec![Expr::Ident("val".to_string())],
                        )));
                    } else {
                        // If no pre-read data is available, use the original try_recv logic
                        let mut receive_stmts = Vec::new();

                        let match_expr = Expr::Match {
                            expr: Box::new(Expr::MethodCall(
                                Box::new(Expr::Ident("receiver".to_string())),
                                "try_recv".to_string(),
                                Vec::new(),
                            )),
                            arms: vec![
                                MatchArm {
                                    pattern: "Ok(val)".to_string(),
                                    guard: None,
                                    body: Block {
                                        stmts: vec![Statement::Expr(Expr::Call(
                                            Box::new(Expr::Path(
                                                vec![subprogram_name.clone(), "receive".to_string()],
                                                PathType::Namespace,
                                            )),
                                            vec![Expr::Ident("val".to_string())],
                                        ))],
                                        expr: None,
                                    },
                                },
                                MatchArm{
                                    pattern: "_".to_string(),
                                    guard: None,
                                    body :Block { stmts: vec![], expr: None },
                                },
                                // MatchArm {
                                //     pattern: "Err(crossbeam_channel::TryRecvError::Empty)".to_string(),
                                //     guard: None,
                                //     body: Block { stmts: vec![], expr: None },
                                // },
                                // MatchArm {
                                //     pattern: "Err(crossbeam_channel::TryRecvError::Disconnected)".to_string(),
                                //     guard: None,
                                //     body: Block {
                                //         stmts: vec![Statement::Expr(Expr::Call(
                                //             Box::new(Expr::Path(
                                //                 vec!["eprintln!".to_string()],
                                //                 PathType::Namespace,
                                //             )),
                                //             vec![Expr::Literal(Literal::Str("channel closed".to_string()))],
                                //         ))],
                                //         expr: None,
                                //     },
                                // },
                            ],
                        };

                        receive_stmts.push(Statement::Expr(match_expr));

                        port_handling_stmts.push(Statement::Expr(Expr::IfLet {
                            pattern: "Some(receiver)".to_string(),
                            value: Box::new(Expr::Reference(
                                Box::new(Expr::Path(
                                    vec!["self".to_string(), thread_port_name.clone()],
                                    PathType::Member,
                                )),
                                true,
                                true,
                            )),
                            then_branch: Block {
                                stmts: receive_stmts,
                                expr: None,
                            },
                            else_branch: None,
                        }));
                    }
                }
            }
        } else if let Some(shared_var_field) = shared_var_subprograms.get(&subprogram_name) {
            // Subprogram that uses a shared variable
            let mut lock_stmts = Vec::new();
            
            lock_stmts.push(Statement::Expr(Expr::Block(Block {
                stmts: vec![
                    Statement::Expr(Expr::IfLet {
                        pattern: "Ok(mut guard)".to_string(),
                        value: Box::new(Expr::MethodCall(
                            Box::new(Expr::Path(
                                vec!["self".to_string(), shared_var_field.clone()],
                                PathType::Member,
                            )),
                            "lock".to_string(),
                            Vec::new(),
                        )),
                        then_branch: Block {
                            stmts: vec![
                                Statement::Expr(Expr::Call(
                                    Box::new(Expr::Path(
                                        vec!["guard".to_string(), subprogram_name.clone()],
                                        PathType::Member,
                                    )),
                                    vec![],
                                )),
                                // Changed from read_pos::call(&mut guard.field) to -> guard.read_pos();
                                // Statement::Expr(Expr::Call(
                                //     Box::new(Expr::Path(
                                //         vec![subprogram_name.clone(), "call".to_string()],
                                //         PathType::Namespace,
                                //     )),
                                //     vec![Expr::Reference(
                                //         Box::new(Expr::Ident("guard".to_string())),
                                //         true,
                                //         true,
                                //     )],
                                // )),
                            ],
                            expr: None,
                        },
                        else_branch: None,
                    }),
                ],
                expr: None,
            })));
            
            port_handling_stmts.push(Statement::Expr(Expr::Block(Block {
                stmts: lock_stmts,
                expr: None,
            })));
        } else {
            // Normal subprogram with no parameter ports
            port_handling_stmts.push(Statement::Expr(Expr::Call(
                Box::new(Expr::Path(
                    vec![subprogram_name.clone(), "execute".to_string()],
                    PathType::Namespace,
                )),
                Vec::new(),
            )));
        }
    }

    port_handling_stmts
}

// Helper: extract property value
fn extract_property_value(temp_converter: &AadlConverter, impl_: &ComponentImplementation, name: &str) -> Option<u64> {
    let target_name = name.to_lowercase();
    for prop in temp_converter.convert_properties(ComponentRef::Impl(impl_)) {
        if prop.name.to_lowercase() == target_name {
            match prop.value {
                StruPropertyValue::Integer(val) => {
                    return Some(val as u64);
                }
                StruPropertyValue::Duration(val, unit) => {
                    println!(
                        "Warning: Found duration {} {} for property {}, expected integer",
                        val, unit, name
                    );
                    return Some(val); // Assume the numeric part of duration is usable
                }
                _ => {
                    println!("Warning: Property {} has unsupported type", name);
                    return None;
                }
            }
        }
    }
    None
}

// Helper: extract dispatch protocol
fn extract_dispatch_protocol(temp_converter: &AadlConverter, impl_: &ComponentImplementation) -> Option<String> {
    let target_name = "dispatch_protocol";
    for prop in temp_converter.convert_properties(ComponentRef::Impl(impl_)) {
        if prop.name.to_lowercase() == target_name {
            match prop.value {
                StruPropertyValue::String(val) => return Some(val),
                _ => {
                    println!("Warning: Property {} has unsupported type", target_name);
                    return None;
                }
            }
        }
    }
    None
}

// Extract event ports and event-data ports
fn extract_event_ports(temp_converter: &AadlConverter, impl_: &ComponentImplementation) -> Vec<String> {
    let mut event_ports = Vec::new();
    
    // Get port definitions from the component type
    if let Some(comp_type) = temp_converter.get_component_type(impl_) {
        if let FeatureClause::Items(features) = &comp_type.features {
            for feature in features {
                if let Feature::Port(port) = feature {
                    // Check whether it is an event port or event-data port, and is an input port
                    if matches!(port.port_type, PortType::Event | PortType::EventData { .. }) 
                       && port.direction == PortDirection::In {
                        event_ports.push(port.identifier.clone());
                    }
                }
            }
        }
    }
    
    event_ports
}

// Helper: extract urgency (priority) info for event ports
fn extract_event_port_urgency(impl_: &ComponentImplementation) -> Vec<(String, u32)> {
    let mut port_priorities = Vec::new();
    
    // Get component properties
    let properties = match &impl_.properties {
        PropertyClause::Properties(props) => props,
        _ => return port_priorities,
    };
    
    // Find urgency properties
    for prop in properties {
        if let Property::BasicProperty(bp) = prop {
            if bp.identifier.name.to_lowercase() == "urgency" {
                if let PropertyValue::Single(PropertyExpression::Integer(
                    SignedIntergerOrConstant::Real(integer),
                )) = &bp.value
                {
                    if integer.sign != Some(Sign::Minus) {
                        if let (Ok(priority), Some(targets)) =
                            (u32::try_from(integer.value), bp.applies_to.as_ref())
                        {
                            for target in targets {
                                port_priorities.push((target.clone(), priority));
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Sort by priority descending (higher priority first)
    port_priorities.sort_by(|a, b| b.1.cmp(&a.1));
    port_priorities
}

fn extract_subprogram_calls(
    temp_converter: &AadlConverter,
    impl_: &ComponentImplementation,
) -> Vec<(String, String, String, bool, Type)> {
    let mut calls = Vec::new();

    // Parse the calls section, obtain the identifier used for each subprogram call
    if let CallSequenceClause::Items(calls_clause) = &impl_.calls {
        for call_clause in calls_clause {
            for subprocall in &call_clause.calls {
                if let CalledSubprogram::Classifier(
                    UniqueComponentClassifierReference::Implementation(temp),
                ) = &subprocall.called
                {
                    let subprogram_name =
                        temp.implementation_name.type_identifier.to_lowercase(); // actual subprogram name referenced by calls
                    let subprogram_identifier = subprocall.identifier.to_lowercase(); // call identifier, e.g., P_Spg

                    // Parse connections section
                    if let ConnectionClause::Items(connections) = &impl_.connections {
                        for conn in connections {
                            if let Connection::Parameter(port_conn) = conn {
                                // For send direction
                                if let ParameterEndpoint::SubprogramCallParameter {
                                    call_identifier,
                                    parameter,
                                } = &port_conn.source
                                // For "send" connections, check source endpoint info
                                {
                                    let sou_parameter = parameter.to_lowercase();
                                    if subprogram_identifier == call_identifier.to_lowercase() {
                                        if let ParameterEndpoint::ComponentParameter {
                                            parameter,
                                            data_subcomponent:_,
                                        } = &port_conn.destination
                                        {
                                            let thread_port_name = parameter.to_lowercase();
                                            let port_type = get_subprogram_port_type(temp_converter, &subprogram_name, &sou_parameter);
                                            calls.push((
                                                sou_parameter.to_lowercase(),    // subprogram port name
                                                //subprogram_name.to_lowercase(),  // subprogram name
                                                subprogram_identifier.to_lowercase(),  // subprogram identifier
                                                thread_port_name.to_lowercase(), // thread port name
                                                true,
                                                port_type,
                                            ));
                                        }
                                    }
                                }
                                // For receive direction
                                if let ParameterEndpoint::SubprogramCallParameter {
                                    call_identifier,
                                    parameter,
                                } = &port_conn.destination
                                // For "receive" connections, check destination endpoint info
                                {
                                    let des_parameter = parameter.to_lowercase();
                                    if subprogram_identifier == call_identifier.to_lowercase() {
                                        if let ParameterEndpoint::ComponentParameter {
                                            parameter,
                                            data_subcomponent:_,
                                        } = &port_conn.source
                                        {
                                            let thread_port_name = parameter.to_lowercase();
                                            let port_type = get_subprogram_port_type(temp_converter, &subprogram_name, &des_parameter);
                                            calls.push((
                                                des_parameter.to_lowercase(),
                                                //subprogram_name.to_lowercase(),
                                                subprogram_identifier.to_lowercase(),
                                                thread_port_name.to_lowercase(),
                                                false,
                                                port_type,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    calls
}

// Get port type by subprogram name and port name
fn get_subprogram_port_type(temp_converter: &AadlConverter, subprogram_name: &str, port_name: &str) -> Type {
    // Iterate all component types and find the subprogram type
    for comp_type in temp_converter.component_types.values() {
        if comp_type.identifier.to_lowercase() == subprogram_name.to_lowercase() {
            // Found the subprogram type; search ports within it
            if let FeatureClause::Items(features) = &comp_type.features {
                for feature in features {
                    if let Feature::Port(port) = feature {
                        if port.identifier.to_lowercase() == port_name.to_lowercase() {
                            // Found the matching port; return its type
                            return temp_converter.convert_paramport_type(port);
                        }
                    }
                }
            }
        }
    }
    // If not found, return default type
    Type::Named("i32".to_string())
}



    /// Create event collection code
    fn create_event_collection_logic(port_urgency: &[(String, u32)], receive_ports: &[String]) -> Vec<Statement> {
        let mut stmts = Vec::new();
        
        // Only try to receive new messages when the event queue is empty
        stmts.push(Statement::Expr(Expr::If {
            condition: Box::new(Expr::MethodCall(
                Box::new(Expr::Ident("events".to_string())),
                "is_empty".to_string(),
                Vec::new(),
            )),
            then_branch: Block {
                stmts: {
                    let mut collect_stmts = Vec::new();
                    
                    // Generate event collection code for each port that has urgency
                    for (port_name, urgency) in port_urgency {
                        let port_field_name = port_name.to_lowercase();
                        
                        // Generate: if let Some(rx) = &self.port_name
                        collect_stmts.push(Statement::Expr(Expr::IfLet {
                            pattern: "Some(rx)".to_string(),
                            value: Box::new(Expr::Reference(
                                Box::new(Expr::Path(
                                    vec!["self".to_string(), port_field_name.clone()],
                                    PathType::Member,
                                )),
                                true,
                                false,
                            )),
                            then_branch: Block {
                                stmts: vec![
                                    // Generate: if let Ok(val) = rx.try_recv()
                                    Statement::Expr(Expr::IfLet {
                                        pattern: "Ok(val)".to_string(),
                                        value: Box::new(Expr::MethodCall(
                                            Box::new(Expr::Ident("rx".to_string())),
                                            "try_recv".to_string(),
                                            Vec::new(),
                                        )),
                                        then_branch: Block {
                                            stmts: vec![
                                                // events.push((val, urgency, ts))
                                                Statement::Expr(Expr::MethodCall(
                                                    Box::new(Expr::Ident("events".to_string())),
                                                    "push".to_string(),
                                                    vec![Expr::Call(
                                                        Box::new(Expr::Ident("".to_string())), // empty ident indicates tuple construction
                                                        vec![
                                                            Expr::Ident("val".to_string()),
                                                            Expr::Literal(Literal::Int(*urgency as i64)),
                                                            Expr::Ident("Instant::now()".to_string()),
                                                        ],
                                                    )],
                                                    
                                                )),
                                            ],
                                            expr: None,
                                        },
                                        else_branch: None,
                                    }),
                                ],
                                expr: None,
                            },
                            else_branch: None,
                        }));
                    }
                    
                    // If there is no urgency info, fall back to the original logic for receive ports
                    if port_urgency.is_empty() && !receive_ports.is_empty() {
                        let port_field_name = receive_ports[0].to_lowercase();
                        collect_stmts.push(Statement::Expr(Expr::IfLet {
                            pattern: "Some(rx)".to_string(),
                            value: Box::new(Expr::Reference(
                                Box::new(Expr::Path(
                                    vec!["self".to_string(), port_field_name],
                                    PathType::Member,
                                )),
                                true,
                                false,
                            )),
                            then_branch: Block {
                                stmts: vec![
                                    Statement::Expr(Expr::IfLet {
                                        pattern: "Ok(val)".to_string(),
                                        value: Box::new(Expr::MethodCall(
                                            Box::new(Expr::Ident("rx".to_string())),
                                            "try_recv".to_string(),
                                            Vec::new(),
                                        )),
                                        then_branch: Block {
                                            stmts: vec![
                                                Statement::Expr(Expr::MethodCall(
                                                    Box::new(Expr::Ident("events".to_string())),
                                                    "push".to_string(),
                                                    vec![Expr::Call(
                                                        Box::new(Expr::Ident("".to_string())), // empty ident indicates tuple construction
                                                        vec![
                                                            Expr::Ident("val".to_string()),
                                                            Expr::Literal(Literal::Int(0)),
                                                            Expr::Ident("Instant::now()".to_string()),
                                                        ],
                                                    )],
                                                    
                                                )),
                                            ],
                                            expr: None,
                                        },
                                        else_branch: None,
                                    }),
                                ],
                                expr: None,
                            },
                            else_branch: None,
                        }));
                    }
                    
                    collect_stmts
                },
                expr: None,
            },
            else_branch: None,
        }));
        
        stmts
    }

    /// Extract data access connections and identify which subprograms use shared variables
    /// Returns: (subprogram_name, shared_var_name, shared_var_field_lowercase)
    fn extract_data_access_calls(impl_: &ComponentImplementation) -> Vec<(String, String, String)> {
        let mut data_access_calls = Vec::new();
        
        // First, build a mapping from call identifier to subprogram name from Mycalls
        let mut call_id_to_subprogram = std::collections::HashMap::new();
        if let CallSequenceClause::Items(calls_clause) = &impl_.calls {
            for call_clause in calls_clause {
                for subprocall in &call_clause.calls {
                    if let CalledSubprogram::Classifier(
                        UniqueComponentClassifierReference::Implementation(temp),
                    ) = &subprocall.called
                    {
                        let subprogram_name = temp.implementation_name.type_identifier.to_lowercase();
                        call_id_to_subprogram.insert(subprocall.identifier.to_lowercase(), subprogram_name);
                    }
                }
            }
        }
        
        if let ConnectionClause::Items(connections) = &impl_.connections {
            for conn in connections {
                if let Connection::Access(access_conn) = conn {
                    // Handle data access mapping:
                    // ComponentAccess(data) -> SubcomponentAccess{ subcomponent: thread, .. }
                    match (&access_conn.source, &access_conn.destination) {
                        (AccessEndpoint::ComponentAccess(data_name), AccessEndpoint::SubcomponentAccess { subcomponent: call_identifier, .. }) => {
                            // Extract subprogram name from call identifier
                            if let Some(subprogram_name) = call_id_to_subprogram.get(&call_identifier.to_lowercase()) {
                                // Extract lowercase shared-var field from data name
                                // TODO: shared_var_field is not necessarily the struct field name
                                let shared_var_field = data_name.to_lowercase();
                                
                                // Shared variable name (for comments)
                                let shared_var_name = data_name.clone();
                                
                                data_access_calls.push((subprogram_name.clone(), shared_var_name, shared_var_field));
                            }
                        }
                        _ => {} // Other directions are not handled for now
                    }
                }
            }
        }
        
        data_access_calls
    }
