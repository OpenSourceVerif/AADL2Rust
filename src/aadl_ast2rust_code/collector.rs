#![allow(
    clippy::empty_line_after_doc_comments,
    clippy::if_same_then_else,
    clippy::collapsible_match,
    clippy::vec_init_then_push
)]
use aadl_intermediate::*;
use crate::aadl_ast2rust_code::tool;
use crate::ast::aadl_ast_cj::*;
use std::collections::HashMap;
/**
 * Collector: gathers component type information from the AADL model,
 * multi-connection relationships between processes,
 * and multi-connection relationships between threads.
 *
 * collect_component_types: collect all component type information
 * collect_process_connections: collect multi-connection relationships between processes within a system
 * collect_thread_connections: collect multi-connection relationships within a process and between threads
 */

// Collect all component type information
pub fn collect_component_types(
    component_types: &mut HashMap<String, ComponentType>,
    pkg: &Package,
) {
    // Handle component types in the public section
    if let Some(public_section) = &pkg.public_section {
        for decl in &public_section.declarations {
            if let AadlDeclaration::ComponentType(comp) = decl {
                component_types.insert(comp.identifier.clone(), comp.clone());
            }
        }
    }

    // Handle component types in the private section
    if let Some(private_section) = &pkg.private_section {
        for decl in &private_section.declarations {
            if let AadlDeclaration::ComponentType(comp) = decl {
                component_types.insert(comp.identifier.clone(), comp.clone());
            }
        }
    }
}

// Collect multi-connection relationships between processes within a system
pub fn collect_process_connections(
    process_broadcast_send: &mut Vec<(String, String)>,
    process_broadcast_receive: &mut HashMap<(String, String), Vec<(String, String)>>,
    system_subcomponent_identify_to_type: &mut HashMap<String, String>,
    pkg: &Package,
) {
    // Create a mapping to record already encountered ports:
    // key is the subcomponent name, value is the port name.
    // Used to determine whether a port appears multiple times.
    // let mut subcomponent_ports = HashMap::new();

    if let Some(public_section) = &pkg.public_section {
        for decl in &public_section.declarations {
            if let AadlDeclaration::ComponentImplementation(impl_) = decl {
                if impl_.category == ComponentCategory::System {
                    if let ConnectionClause::Items(connections) = &impl_.connections {
                        for conn in connections {
                            if let Connection::Port(port_conn) = conn {
                                if let PortEndpoint::SubcomponentPort { subcomponent, port } =
                                    &port_conn.source
                                {
                                    process_broadcast_send
                                        .push((subcomponent.clone(), port.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // Filter duplicated (sending) ports and keep those appearing at least twice
    let temp_dedup_with_min_two_unique = tool::dedup_with_min_two_unique(process_broadcast_send);
    // temp_dedup_with_min_two_unique now stores duplicated sending ports.
    // Traverse the connections again to find all corresponding receiving ports
    // and add them to process_broadcast_receive.
    for ele in &temp_dedup_with_min_two_unique {
        if let Some(public_section) = &pkg.public_section {
            for decl in &public_section.declarations {
                if let AadlDeclaration::ComponentImplementation(impl_) = decl {
                    if impl_.category == ComponentCategory::System {
                        if let ConnectionClause::Items(connections) = &impl_.connections {
                            for conn in connections {
                                if let Connection::Port(port_conn) = conn {
                                    if let PortEndpoint::SubcomponentPort { subcomponent, port } =
                                        &port_conn.source
                                    {
                                        if subcomponent.eq(&ele.0) && port.eq(&ele.1) {
                                            if let PortEndpoint::SubcomponentPort {
                                                subcomponent: d_subcomponent,
                                                port: d_port,
                                            } = &port_conn.destination
                                            {
                                                process_broadcast_receive
                                                    .entry(ele.clone())
                                                    .or_default()
                                                    .push((d_subcomponent.clone(), d_port.clone()));
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
    }
    // Traverse all keys and values in process_broadcast_receive.
    // For each involved component, inspect the system subcomponents
    // to determine its concrete type, and record it in
    // system_subcomponent_identify_to_type.
    for (sendcomp, vercport) in process_broadcast_receive {
        for (comp, _) in vercport {
            if let Some(public_section) = &pkg.public_section {
                for decl in &public_section.declarations {
                    if let AadlDeclaration::ComponentImplementation(impl_) = decl {
                        if impl_.category == ComponentCategory::System {
                            if let SubcomponentClause::Items(subcomponents) = &impl_.subcomponents {
                                for sub in subcomponents {
                                    if sub.identifier.eq(&comp.clone()) {
                                        if let SubcomponentClassifier::ClassifierReference(
                                            classifier,
                                        ) = &sub.classifier
                                        {
                                            if let UniqueComponentClassifierReference::Implementation(unirf) = classifier {
                                                system_subcomponent_identify_to_type.insert(sub.identifier.clone(), unirf.implementation_name.type_identifier.clone());
                                            }
                                        }
                                    } else if sub.identifier.eq(&sendcomp.0.clone()) {
                                        if let SubcomponentClassifier::ClassifierReference(
                                            classifier,
                                        ) = &sub.classifier
                                        {
                                            if let UniqueComponentClassifierReference::Implementation(unirf) = classifier {
                                                system_subcomponent_identify_to_type.insert(sub.identifier.clone(), unirf.implementation_name.type_identifier.clone());
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
    }
}

// Collect multi-connection relationships within a process and between threads
pub fn collect_thread_connections(
    thread_broadcast_receive: &mut HashMap<(String, String), Vec<(String, String)>>,
    process_subcomponent_identify_to_type: &mut HashMap<String, String>,
    pkg: &Package,
) {
    // List storing ports within a process that have multiple connections;
    // each entry is a port name.
    let mut process_forwarder_broadcast_send: Vec<String> = Vec::new();

    if let Some(public_section) = &pkg.public_section {
        for decl in &public_section.declarations {
            if let AadlDeclaration::ComponentImplementation(impl_) = decl {
                if impl_.category == ComponentCategory::Process {
                    if let ConnectionClause::Items(connections) = &impl_.connections {
                        for conn in connections {
                            if let Connection::Port(port_conn) = conn {
                                if let PortEndpoint::ComponentPort(proc_port) = &port_conn.source {
                                    process_forwarder_broadcast_send.push(proc_port.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // Filter duplicated ports and keep those appearing at least twice
    let temp_process_port =
        tool::dedup_with_min_two_unique_single_string(&mut process_forwarder_broadcast_send);
    for port in &temp_process_port {
        if let Some(public_section) = &pkg.public_section {
            for decl in &public_section.declarations {
                if let AadlDeclaration::ComponentImplementation(impl_) = decl {
                    if impl_.category == ComponentCategory::Process {
                        if let ConnectionClause::Items(connections) = &impl_.connections {
                            for conn in connections {
                                if let Connection::Port(port_conn) = conn {
                                    if let PortEndpoint::ComponentPort(proc_port) =
                                        &port_conn.source
                                    {
                                        if proc_port.eq(port) {
                                            if let PortEndpoint::SubcomponentPort {
                                                subcomponent,
                                                port: sub_port,
                                            } = &port_conn.destination
                                            {
                                                thread_broadcast_receive
                                                    .entry((
                                                        port.clone(),
                                                        impl_.name.type_identifier.clone(),
                                                    ))
                                                    .or_default()
                                                    .push((subcomponent.clone(), sub_port.clone()));
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
    }
    // Traverse all values in thread_broadcast_receive (ignoring the keys).
    // For each component name, find the corresponding subcomponent in the process,
    // determine its concrete type, and record it in
    // process_subcomponent_identify_to_type.
    for vercport in thread_broadcast_receive.values_mut() {
        for (comp, _) in vercport {
            if let Some(public_section) = &pkg.public_section {
                for decl in &public_section.declarations {
                    if let AadlDeclaration::ComponentImplementation(impl_) = decl {
                        if impl_.category == ComponentCategory::Process {
                            if let SubcomponentClause::Items(subcomponents) = &impl_.subcomponents {
                                for sub in subcomponents {
                                    if sub.identifier.eq(&comp.clone()) {
                                        if let SubcomponentClassifier::ClassifierReference(
                                            classifier,
                                        ) = &sub.classifier
                                        {
                                            if let UniqueComponentClassifierReference::Implementation(unirf) = classifier {
                                                process_subcomponent_identify_to_type.insert(sub.identifier.clone(), unirf.implementation_name.type_identifier.clone());
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
    }
}

// Generate static code for CPU scheduling policy mappings
pub fn convert_cpu_schedule_mapping(
    module: &mut RustModule,
    cpu_scheduling_protocols: &HashMap<String, String>,
    cpu_name_to_id_mapping: &HashMap<String, isize>,
) {
    // If there is no CPU mapping information, do not generate code
    if cpu_name_to_id_mapping.is_empty() {
        return;
    }

    // Generate map.insert statements
    let mut map_insertions = Vec::new();

    for (cpu_name, cpu_id) in cpu_name_to_id_mapping {
        // Obtain the scheduling protocol for this CPU
        let scheduling_protocol = cpu_scheduling_protocols
            .get(cpu_name)
            .map(|s| s.as_str())
            .unwrap_or("FIFO"); // Default to FIFO

        // Convert the scheduling protocol to the corresponding constant
        let sched_constant = match scheduling_protocol.to_uppercase().as_str() {
            "POSIX_1003_HIGHEST_PRIORITY_FIRST_PROTOCOL" | "HPF" => "SCHED_FIFO",
            "ROUND_ROBIN_PROTOCOL" | "RR" => "SCHED_RR",
            "EDF" | "EARLIEST_DEADLINE_FIRST_PROTOCOL" => "SCHED_DEADLINE",
            "RATE_MONOTONIC_PROTOCOL" | "RMS" | "RM" => "SCHED_FIFO",
            "DEADLINE_MONOTONIC_PROTOCOL" | "DM" | "DMS" => "SCHED_FIFO",
            _ => "SCHED_FIFO", // Default
        };

        // Generate map.insert(cpu_id, sched_constant);
        map_insertions.push(Statement::Expr(Expr::MethodCall(
            Box::new(Expr::Ident("map".to_string())),
            "insert".to_string(),
            vec![
                Expr::Literal(Literal::Int(*cpu_id as i64)),
                Expr::Path(vec![sched_constant.to_string()], PathType::Namespace),
            ],
        )));
    }

    // Build the initialization block
    let mut init_stmts = Vec::new();

    // let mut map = HashMap::new();
    init_stmts.push(Statement::Let(LetStmt {
        ifmut: true,
        name: "map".to_string(),
        ty: Some(Type::Generic(
            "HashMap".to_string(),
            vec![
                Type::Named("isize".to_string()),
                Type::Named("i32".to_string()),
            ],
        )),
        init: Some(Expr::Call(
            Box::new(Expr::Path(
                vec!["HashMap".to_string(), "new".to_string()],
                PathType::Namespace,
            )),
            Vec::new(),
        )),
    }));

    // Add map.insert statements
    init_stmts.extend(map_insertions);

    // map // return map
    init_stmts.push(Statement::Expr(Expr::Ident("return map".to_string())));

    // Create LazyStaticDef
    let lazy_static_def = LazyStaticDef {
        name: "CPU_ID_TO_SCHED_POLICY".to_string(),
        ty: Type::Generic(
            "HashMap".to_string(),
            vec![
                Type::Named("isize".to_string()),
                Type::Named("i32".to_string()),
            ],
        ),
        init: Block {
            stmts: init_stmts,
            expr: None,
        },
        vis: Visibility::Public,
        docs: vec!["// Mapping from CPU ID to scheduling policy".to_string()],
    };

    // Add LazyStatic to the module
    module.items.push(Item::LazyStatic(lazy_static_def));
}

/// Add the period_to_priority function to the module
/// This function computes priority from period:
/// prio(P)=max(1,min(99,99−⌊k⋅log10(P)⌋))
/// The function is generated only when RMS or DMS scheduling is detected
pub fn add_period_to_priority_function(
    module: &mut RustModule,
    cpu_scheduling_protocols: &HashMap<String, String>,
) {
    // Check whether RMS or DMS scheduling protocols are present
    let has_rms_or_dms = cpu_scheduling_protocols.values().any(|protocol| {
        let protocol_upper = protocol.to_uppercase();
        protocol_upper.contains("RATE_MONOTONIC")
            || protocol_upper.contains("RMS")
            || protocol_upper.contains("RM")
            || protocol_upper.contains("DEADLINE_MONOTONIC")
            || protocol_upper.contains("DMS")
            || protocol_upper.contains("DM")
    });

    // If no RMS or DMS protocol is present, do not generate the function
    if !has_rms_or_dms {
        return;
    }

    // Build the function body
    let mut body_stmts = Vec::new();

    // let k = 10.0; // priority decreases by 10 for each order of magnitude
    body_stmts.push(Statement::Let(LetStmt {
        ifmut: false,
        name: "k".to_string(),
        ty: Some(Type::Named("f64".to_string())),
        init: Some(Expr::Ident("10.0".to_string())),
    }));

    // let raw = 99.0 - (k * period_ms.log10()).floor();
    body_stmts.push(Statement::Let(LetStmt {
        ifmut: false,
        name: "raw".to_string(),
        ty: Some(Type::Named("f64".to_string())),
        init: Some(Expr::BinaryOp(
            Box::new(Expr::Ident("99.0".to_string())),
            "-".to_string(),
            Box::new(Expr::MethodCall(
                Box::new(Expr::BinaryOp(
                    Box::new(Expr::Ident("k".to_string())),
                    "*".to_string(),
                    Box::new(Expr::MethodCall(
                        Box::new(Expr::Ident("period_ms".to_string())),
                        "log10".to_string(),
                        Vec::new(),
                    )),
                )),
                "floor".to_string(),
                Vec::new(),
            )),
        )),
    }));

    // raw.max(1.0).min(99.0) as i32
    body_stmts.push(Statement::Expr(Expr::Ident(
        "return raw.max(1.0).min(99.0) as i32".to_string(),
    )));

    // Create the function definition
    let function_def = FunctionDef {
        name: "period_to_priority".to_string(),
        params: vec![Param {
            name: "period_ms".to_string(),
            ty: Type::Named("f64".to_string()),
        }],
        return_type: Type::Named("i32".to_string()),
        body: Block {
            stmts: body_stmts,
            expr: None,
        },
        asyncness: false,
        vis: Visibility::Public,
        docs: vec![
            "// prio(P)=max(1,min(99,99−⌊k⋅log10(P)⌋))".to_string(),
            "// Compute priority from period: shorter period yields higher priority".to_string(),
            "// Used for RMS (Rate Monotonic Scheduling) and DMS (Deadline Monotonic Scheduling)"
                .to_string(),
        ],
        attrs: Vec::new(),
    };

    // Add the function to the module
    module.items.push(Item::Function(function_def));
}
