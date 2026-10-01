//! External BA hooks emitted on the existing generated thread implementation.
//! Transport fields and their Rust types still come from the AADL converter.
use super::*;

pub(super) fn uses_event_tickets(protocol: Option<&str>) -> bool {
    matches!(protocol, Some("Aperiodic" | "Sporadic" | "Timed" | "Hybrid"))
}

pub(super) fn context_fields(event_driven: bool) -> Vec<Field> {
    let mut fields = vec![
        Field {
            name: "ba_context".to_string(),
            ty: Type::Named("crate::ba_glue::GlueContext".to_string()),
            docs: vec!["// Runtime-owned queues and output records survive host dispatches.".to_string()],
            attrs: Vec::new(),
        },
        Field {
            name: "ba_initialized".to_string(),
            ty: Type::Named("bool".to_string()),
            docs: vec!["// C initialization occurs only after the host connects the ports.".to_string()],
            attrs: Vec::new(),
        },
    ];
    for (name, ty, explanation) in [
        ("ba_protocol_driven", "Option<bool>", "The first entry selects manual (false) or protocol (true) dispatch; mixing is rejected before side effects."),
        ("ba_last_poll", "std::time::Duration", "Logical elapsed time is monotonic; initialization completes at zero."),
        ("ba_last_dispatch", "Option<std::time::Duration>", "Timed and Sporadic measure from the previous activation, not completion."),
        ("ba_next_release", "std::time::Duration", "Periodic and Hybrid retain their release grid across event activations."),
        ("ba_background_dispatched", "bool", "Background has one activation after initialization, not a periodic loop."),
    ] {
        fields.push(Field { name: name.to_string(), ty: Type::Named(ty.to_string()),
            docs: vec![format!("// {explanation}")], attrs: Vec::new() });
    }
    if event_driven {
        fields.push(Field {
            name: "ba_pending_events".to_string(),
            ty: Type::Named("Vec<(&'static str, u32, std::time::Instant)>".to_string()),
            docs: vec!["// Arrival tickets survive collection during initialization and dispatch.".to_string()],
            attrs: Vec::new(),
        });
    }
    fields
}

pub(super) fn context_initializers(converter: &AadlConverter, implementation: &ComponentImplementation) -> Vec<String> {
    let mut initializers = vec![
        "ba_context: crate::ba_glue::GlueContext::default(),".to_string(),
        "ba_initialized: false,".to_string(),
        "ba_protocol_driven: None,".to_string(),
        "ba_last_poll: std::time::Duration::ZERO,".to_string(),
        "ba_last_dispatch: None,".to_string(),
        format!("ba_next_release: std::time::Duration::from_nanos({}u64),", converter.external_ba_period_nanoseconds(implementation).unwrap_or(0)),
        "ba_background_dispatched: false,".to_string(),
    ];
    if uses_event_tickets(converter.external_ba_protocol(implementation).as_deref()) {
        initializers.push("ba_pending_events: Vec::new(),".to_string());
    }
    initializers
}

fn ports(converter: &AadlConverter, implementation: &ComponentImplementation) -> Vec<PortSpec> {
    converter.get_component_type(implementation).and_then(|component| {
        if let FeatureClause::Items(features) = &component.features {
            Some(features.iter().filter_map(|feature| match feature {
                Feature::Port(port) => Some(port.clone()),
                _ => None,
            }).collect())
        } else { None }
    }).unwrap_or_default()
}

fn method(name: &str, extra_params: Vec<Param>, source: String, public: bool) -> ImplItem {
    method_with_result(name, extra_params, source, public, Type::Unit)
}

fn method_with_result(name: &str, extra_params: Vec<Param>, source: String, public: bool, return_type: Type) -> ImplItem {
    let mut params = vec![Param {
        name: String::new(),
        ty: Type::Reference(Box::new(Type::Named("self".to_string())), true, true),
    }];
    params.extend(extra_params);
    ImplItem::Method(FunctionDef {
        name: name.to_string(), params, return_type,
        generics: Vec::new(), asyncness: false,
        vis: if public { Visibility::Public } else { Visibility::None },
        docs: vec![format!("// Generated external BA lifecycle/transport method: {name}.")],
        attrs: Vec::new(),
        // RustLight currently has no while-let node. Preserve the typed transport
        // loop as source inside its method body, as existing constructors already do.
        body: Block { stmts: vec![Statement::Expr(Expr::Ident(source))], expr: None },
    })
}

pub(super) fn lifecycle_impl(converter: &AadlConverter, implementation: &ComponentImplementation) -> Item {
    let protocol = converter.external_ba_protocol(implementation).expect("selected external BA protocol");
    let event_driven = uses_event_tickets(Some(&protocol));
    let urgencies = extract_event_port_urgency(implementation);
    let mut collect = String::from("// Append newly available values; do not clear unconsumed runtime input.\n");
    let mut publish = String::from("// Deliver every recorded output in order, including initialization outputs.\n");
    let mut imported_tickets = String::new();
    for port in ports(converter, implementation) {
        let field = port.identifier.to_lowercase();
        match port.direction {
            PortDirection::In => {
                let ticket = if event_driven && matches!(port.port_type, PortType::Event | PortType::EventData { .. }) {
                    let urgency = urgencies.iter().find(|(name, _)| name.eq_ignore_ascii_case(&field)).map(|(_, value)| *value).unwrap_or(0);
                    imported_tickets.push_str(&format!("\"{field}\" => self.ba_pending_events.push((\"{field}\", {urgency}u32, std::time::Instant::now())),\n"));
                    format!("        self.ba_pending_events.push((\"{field}\", {urgency}u32, std::time::Instant::now()));\n")
                } else { String::new() };
                collect.push_str(&format!(
                    "if let Some(ba_receiver) = &self.{field} {{\n    self.ba_context.bind_receiver_{field}(ba_receiver);\n    while let Ok(ba_value) = ba_receiver.try_recv() {{\n        self.ba_context.enqueue_{field}(ba_value);\n{ticket}    }}\n}}\n"));
            },
            PortDirection::Out => publish.push_str(&format!(
                "if let Some(ba_sender) = &self.{field} {{\n    for ba_value in self.ba_context.drain_{field}() {{\n        ba_sender.send(ba_value).expect(\"external BA output receiver disconnected: {field}\");\n    }}\n}}\n")),
            PortDirection::InOut => unreachable!("external BA validation rejects in-out ports"),
        }
    }
    let import_arrivals = if event_driven {
        format!("// Port-service callbacks can receive real inputs while C executes.\n\
            // They report only new event arrivals, never values already enqueued by the host.\n\
            for ba_port in self.ba_context.take_arrival_ports() {{\n\
                match ba_port {{\n{imported_tickets}\n\
                    _ => panic!(\"unknown external BA callback arrival port: {{}}\", ba_port),\n\
                }}\n\
            }}")
    } else {
        "// Time-only and Background hosts keep payloads but do not accumulate event tickets.\nlet _ = self.ba_context.take_arrival_ports();".to_string()
    };
    let methods = vec![
        method("ba_collect_inputs", Vec::new(), collect, false),
        method("ba_publish_outputs", Vec::new(), publish, false),
        method("ba_import_arrivals", Vec::new(), import_arrivals, false),
        method("ba_initialize", Vec::new(), String::from(
            "assert!(!self.ba_initialized, \"external BA was already initialized\");\n\
             // new() creates the context; this call occurs after actual ports are connected.\n\
             self.ba_collect_inputs();\n\
             crate::ba_glue::initialize(&mut self.ba_context);\n\
             self.ba_import_arrivals();\n\
             self.ba_initialized = true;\n\
             self.ba_publish_outputs();"), true),
        method("ba_dispatch_once", vec![Param {
            name: "trigger_ports".to_string(), ty: Type::Named("&[&str]".to_string()),
        }], String::from(
            "assert!(self.ba_initialized, \"external BA dispatch requires initialization\");\n\
             assert!(self.ba_protocol_driven != Some(true), \"cannot mix manual dispatch and protocol polling\");\n\
             self.ba_protocol_driven = Some(false);\n\
             // Explicit lifecycle override retained for behavior-only tests; it does not advance host time.\n\
             self.ba_collect_inputs();\n\
             self.ba_dispatch_collected(trigger_ports);"), true),
        method("ba_dispatch_collected", vec![Param {
            name: "trigger_ports".to_string(), ty: Type::Named("&[&str]".to_string()),
        }], String::from(
             "// The caller has frozen the trigger batch. Never drain Rx again here.\n\
             self.ba_context.set_dispatch_ports(trigger_ports);\n\
             crate::ba_glue::dispatch(&mut self.ba_context);\n\
             // New callback arrivals belong to the next activation, not the frozen trigger batch.\n\
             self.ba_import_arrivals();\n\
             self.ba_publish_outputs();"), false),
        method_with_result("ba_poll_dispatch", vec![Param {
            name: "now".to_string(), ty: Type::Named("std::time::Duration".to_string()),
        }], activation_source(&protocol, converter.external_ba_period_nanoseconds(implementation)), true, Type::Named("bool".to_string())),
    ];
    Item::Impl(ImplBlock {
        target: Type::Named(format!("{}Thread", to_upper_camel_case(&implementation.name.type_identifier))),
        generics: Vec::new(), items: methods, trait_impl: None,
    })
}

/// Emit one bounded host decision. Both run() and deterministic tests call this
/// method; the BA still decides its transitions and consumes its own payloads.
fn activation_source(protocol: &str, period_ns: Option<u64>) -> String {
    let mut source = String::from(
        "assert!(self.ba_initialized, \"external BA poll requires initialization\");\n\
         assert!(self.ba_protocol_driven != Some(false), \"cannot mix manual dispatch and protocol polling\");\n\
         assert!(now >= self.ba_last_poll, \"external BA host clock moved backwards\");\n\
         self.ba_protocol_driven = Some(true);\n\
         self.ba_last_poll = now;\n\
         self.ba_collect_inputs();\n");
    if let Some(period) = period_ns {
        source.push_str(&format!("let ba_period = std::time::Duration::from_nanos({period}u64);\n"));
    }
    match protocol {
        "Periodic" => source.push_str(
            "if now < self.ba_next_release { return false; }\n\
             // One release per poll; late polls preserve, rather than shift, the release grid.\n\
             self.ba_next_release = self.ba_next_release.checked_add(ba_period).expect(\"BA release clock overflow\");\n"),
        "Aperiodic" => source.push_str("if self.ba_pending_events.is_empty() { return false; }\n"),
        "Sporadic" => source.push_str(
            "if self.ba_pending_events.is_empty() { return false; }\n\
             // The first event has no preceding activation to constrain it.\n\
             if self.ba_last_dispatch.is_some_and(|last| now - last < ba_period) { return false; }\n"),
        "Timed" => source.push_str(
            "// A timeout is measured since the last activation (or initialization at zero).\n\
             let ba_time_due = now - self.ba_last_dispatch.unwrap_or(std::time::Duration::ZERO) >= ba_period;\n\
             if self.ba_pending_events.is_empty() && !ba_time_due { return false; }\n"),
        "Hybrid" => source.push_str(
            "let ba_time_due = now >= self.ba_next_release;\n\
             if self.ba_pending_events.is_empty() && !ba_time_due { return false; }\n\
             // An event and a due release share this activation; events do not reset the grid.\n\
             if ba_time_due { self.ba_next_release = self.ba_next_release.checked_add(ba_period).expect(\"BA release clock overflow\"); }\n"),
        "Background" => source.push_str(
            "// A Background thread is dispatched once and executes until that call completes.\n\
             if self.ba_background_dispatched { return false; }\n\
             self.ba_background_dispatched = true;\n"),
        _ => unreachable!("external BA protocol was validated before generation"),
    }
    if uses_event_tickets(Some(protocol)) {
        source.push_str(
            "// One activation observes every distinct event port in the current arrival batch.\n\
             // Consume all batch tickets, including repeated arrivals, without consuming payloads.\n\
             let mut ba_batch = std::mem::take(&mut self.ba_pending_events);\n\
             ba_batch.sort_by(|left, right| right.1.cmp(&left.1).then(left.2.cmp(&right.2)));\n\
             let mut ba_trigger_ports = Vec::new();\n\
             for (port, _, _) in ba_batch {\n\
                 if !ba_trigger_ports.contains(&port) { ba_trigger_ports.push(port); }\n\
             }\n\
             self.ba_last_dispatch = Some(now);\n\
             self.ba_dispatch_collected(&ba_trigger_ports);\n");
    } else {
        source.push_str("self.ba_last_dispatch = Some(now);\nself.ba_dispatch_collected(&[]);\n");
    }
    source.push_str("return true;");
    source
}

pub(super) fn run_loop(converter: &AadlConverter, implementation: &ComponentImplementation) -> Vec<Statement> {
    let protocol = converter.external_ba_protocol(implementation).expect("selected external BA protocol");
    if protocol == "Background" {
        return vec![Statement::Expr(Expr::Ident(String::from(
            "// Background executes once after initialization; no artificial period or redispatch.\n\
             self.ba_poll_dispatch(std::time::Duration::ZERO);")))];
    }
    let pause = match protocol.as_str() {
        "Periodic" => "self.ba_next_release.saturating_sub(ba_clock.elapsed())".to_string(),
        "Hybrid" => "self.ba_next_release.saturating_sub(ba_clock.elapsed()).min(std::time::Duration::from_millis(1))".to_string(),
        "Timed" => format!("std::time::Duration::from_nanos({}u64).saturating_sub(ba_clock.elapsed().saturating_sub(self.ba_last_dispatch.unwrap_or(std::time::Duration::ZERO))).min(std::time::Duration::from_millis(1))", converter.external_ba_period_nanoseconds(implementation).unwrap()),
        _ => "std::time::Duration::from_millis(1)".to_string(),
    };
    vec![Statement::Expr(Expr::Ident(format!(
        "// The same bounded decision drives real execution and virtual-clock acceptance tests.\n\
         // OS sleeps only pace polling; they do not establish a wall-clock guarantee.\n\
         let ba_clock = std::time::Instant::now();\n\
         loop {{\n\
             if self.ba_poll_dispatch(ba_clock.elapsed()) {{ continue; }}\n\
             let ba_pause = {pause};\n\
             std::thread::sleep(ba_pause);\n\
         }}")))]
}

pub(super) fn initialize_call() -> Statement {
    Statement::Expr(Expr::MethodCall(Box::new(Expr::Ident("self".to_string())), "ba_initialize".to_string(), Vec::new()))
}

pub(super) fn dispatch_call(event_driven: bool) -> Statement {
    let triggers = if event_driven { vec![Expr::Ident("val".to_string())] } else { Vec::new() };
    Statement::Expr(Expr::MethodCall(
        Box::new(Expr::Ident("self".to_string())), "ba_dispatch_once".to_string(),
        vec![Expr::Reference(Box::new(Expr::Array(triggers)), true, false)],
    ))
}

/// Retain the existing event-loop ordering and timing, replacing only its
/// untyped payload slot with a port identity. Payloads enter the real Glue queue.
pub(super) fn event_collection(_converter: &AadlConverter, _implementation: &ComponentImplementation, _urgencies: &[(String, u32)]) -> Vec<Statement> {
    // Exactly one receiver-draining method owns both payload enqueue and ticket
    // recording. Thus initialization or dispatch cannot hide an arrival from run().
    let source = String::from("self.ba_collect_inputs();\nif events.is_empty() {\n    events.append(&mut self.ba_pending_events);\n}");
    vec![Statement::Expr(Expr::Ident(source))]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aadlight_parser::{AADLParser, Rule};
    use crate::aadl_ast2rust_code::merge_utils::merge_item_defs;
    use crate::transform::AADLTransformer;
    use pest::Parser;

    fn model(protocol: &str) -> Vec<Package> {
        model_with_period(protocol, None, Some("10 ms"))
    }

    fn model_with_period(protocol: &str, type_period: Option<&str>, implementation_period: Option<&str>) -> Vec<Package> {
        let type_property = type_period.map(|value| format!("properties Period => {value};")).unwrap_or_default();
        let implementation_property = implementation_period.map(|value| format!("Period => {value};")).unwrap_or_default();
        let source = format!(r#"package Hook_Example
public
  with Base_Types;
  thread Worker
    features
      tick : in event data port Base_Types::Integer_32;
      other : in event port;
      answer : out event data port Base_Types::Integer_32;
    {type_property}
  end Worker;
  thread implementation Worker.impl
    properties
      Dispatch_Protocol => {protocol};
      {implementation_property}
  end Worker.impl;
end Hook_Example;"#);
        let parsed = AADLParser::parse(Rule::file, &source).expect("valid hook fixture");
        AADLTransformer::transform_file(parsed.collect())
    }

    fn generated(protocol: &str, external: bool) -> String {
        let packages = model(protocol);
        let mut converter = AadlConverter::default();
        converter.set_available_packages(&packages);
        if external {
            converter.set_external_ba_target("Hook_Example::Worker.impl", &packages).unwrap();
        }
        let module = merge_item_defs(converter.convert_package(&packages[0]));
        let source = RustCodeGenerator::new().generate_module_code(&module);
        syn::parse_file(&source).expect("external hook output must remain valid Rust syntax");
        source
    }

    #[test]
    fn generated_thread_reuses_lifecycle_and_keeps_real_transport() {
        let source = generated("Sporadic", true);
        assert!(source.contains("pub tick: Option<Receiver<i32>>"));
        assert!(source.contains("pub answer: Option<Sender<i32>>"));
        assert!(source.contains("ba_context: crate::ba_glue::GlueContext::default()"));
        assert_eq!(source.matches("crate::ba_glue::initialize(").count(), 1);
        assert_eq!(source.matches("crate::ba_glue::dispatch(").count(), 1);
        assert!(source.contains("self.ba_initialize()"));
        assert!(source.contains("self.ba_poll_dispatch(ba_clock.elapsed())"));
        assert!(source.contains("self.ba_dispatch_collected(&ba_trigger_ports)"));
        assert!(source.contains("ba_pending_events.push((\"tick\", 0u32, std::time::Instant::now()))"));
        assert!(source.contains("ba_pending_events.push((\"other\", 0u32, std::time::Instant::now()))"));
        assert!(source.contains("std::mem::take(&mut self.ba_pending_events)"));
        assert!(source.contains("while let Ok(ba_value) = ba_receiver.try_recv()"));
        assert!(source.contains("for ba_value in self.ba_context.drain_answer()"));
        assert!(source.contains("assert!(!self.ba_initialized"));
        assert!(source.contains("assert!(self.ba_initialized"));
        // A minimal System/Process host has no CPU deployment metadata or helper.
        assert!(!source.contains("CPU_ID_TO_SCHED_POLICY"));
        assert!(!source.contains("period_to_priority("));
        assert!(!source.contains("set_thread_affinity(self.cpu_id)"));
        assert!(source.contains("leave OS priority and affinity unchanged"));
    }

    #[test]
    fn native_generation_stays_opt_in_and_periodic_uses_shared_entry() {
        assert!(!generated("Periodic", false).contains("ba_glue"));
        assert!(generated("Periodic", true).contains("self.ba_dispatch_collected(&[])"));
        assert!(!generated("Periodic", true).contains("ba_pending_events"));
        for protocol in ["Aperiodic", "Sporadic", "Timed", "Hybrid"] {
            assert!(generated(protocol, true).contains("self.ba_poll_dispatch(ba_clock.elapsed())"));
        }
        assert!(generated("Background", true).contains("self.ba_poll_dispatch(std::time::Duration::ZERO)"));
    }

    #[test]
    fn unsupported_protocol_and_unresolved_target_are_explicit() {
        for protocol in ["Event"] {
            let mut converter = AadlConverter::default();
            let error = converter.set_external_ba_target("Worker.impl", &model(protocol)).unwrap_err();
            assert!(error.contains("unsupported external BA dispatch protocol"));
        }
        let mut converter = AadlConverter::default();
        assert!(converter.set_external_ba_target("Missing.impl", &model("Periodic")).unwrap_err().contains("matched 0"));
    }

    #[test]
    fn period_units_inheritance_and_missing_values_are_explicit() {
        for (literal, expected_ns) in [
            ("10 ns", 10u64), ("10 us", 10_000), ("10 ms", 10_000_000),
            ("10 sec", 10_000_000_000), ("1 min", 60_000_000_000), ("1 hr", 3_600_000_000_000),
        ] {
            let packages = model_with_period("Timed", None, Some(literal));
            let mut converter = AadlConverter::default();
            converter.set_available_packages(&packages);
            converter.set_external_ba_target("Worker.impl", &packages).unwrap();
            let module = merge_item_defs(converter.convert_package(&packages[0]));
            let emitted = RustCodeGenerator::new().generate_module_code(&module);
            assert!(emitted.contains(&format!("let ba_period = std::time::Duration::from_nanos({expected_ns}u64)")), "Period {literal}: {emitted}");
        }
        for (type_value, implementation_value, expected_ns) in [
            (Some("2 sec"), None, 2_000_000_000),
            (Some("2 sec"), Some("5 us"), 5_000),
        ] {
            let packages = model_with_period("Hybrid", type_value, implementation_value);
            let mut converter = AadlConverter::default();
            converter.set_external_ba_target("Worker.impl", &packages).unwrap();
            let module = merge_item_defs(converter.convert_package(&packages[0]));
            assert!(RustCodeGenerator::new().generate_module_code(&module).contains(
                &format!("let ba_period = std::time::Duration::from_nanos({expected_ns}u64)")));
        }
        for protocol in ["Periodic", "Sporadic", "Timed", "Hybrid"] {
            let packages = model_with_period(protocol, None, None);
            let error = AadlConverter::default().set_external_ba_target("Worker.impl", &packages).unwrap_err();
            assert!(error.contains("without an explicit Period"));
        }
        for protocol in ["Aperiodic", "Background"] {
            let packages = model_with_period(protocol, None, None);
            AadlConverter::default().set_external_ba_target("Worker.impl", &packages).unwrap();
        }
        for unsupported in ["1", "0 ns", "-1 ms", "0.1 ns", "0.5 ms", "9007199254740993.0 ns", "100000000000 sec"] {
            let packages = model_with_period("Timed", None, Some(unsupported));
            assert!(AadlConverter::default().set_external_ba_target("Worker.impl", &packages).unwrap_err().contains("unsupported external BA Period"));
        }
    }

    #[test]
    fn generated_transport_keeps_arrival_tickets_across_lifecycle_calls() {
        let packages = model("Sporadic");
        let mut converter = AadlConverter::default();
        converter.set_available_packages(&packages);
        converter.set_external_ba_target("Worker.impl", &packages).unwrap();
        let mut module = merge_item_defs(converter.convert_package(&packages[0]));
        // Compile the actual generated thread, including its normal run method.
        // Standard channels provide the same transport operations for this isolated
        // emitter regression; BA/C semantics are tested by the integration suite.
        module.items.retain(|item| match item {
            Item::Struct(definition) => definition.name == "WorkerThread",
            Item::Impl(definition) => matches!(&definition.target, Type::Named(name) if name == "WorkerThread"),
            _ => false,
        });
        let emitted = RustCodeGenerator::new().generate_module_code(&module);
        let implementation = packages[0].public_section.as_ref().unwrap().declarations.iter().find_map(|declaration| {
            if let AadlDeclaration::ComponentImplementation(implementation) = declaration { Some(implementation) } else { None }
        }).unwrap();
        let collection = event_collection(&converter, implementation, &[]);
        let Statement::Expr(Expr::Ident(collection)) = &collection[0] else { panic!("expected generated collection source"); };
        let support = r#"
use std::sync::mpsc::{Sender, Receiver};
use std::time::{Duration, Instant};
trait Thread { fn new(cpu_id: isize) -> Self; fn run(self); }
mod ba_glue {
    #[derive(Debug, Default)]
    pub struct GlueContext { pub tick_values: Vec<i32> }
    impl GlueContext {
        pub fn bind_receiver_tick<T>(&mut self, _: &T) {}
        pub fn bind_receiver_other<T>(&mut self, _: &T) {}
        pub fn take_arrival_ports(&mut self) -> Vec<&'static str> { Vec::new() }
        pub fn enqueue_tick(&mut self, value: i32) { self.tick_values.push(value); }
        pub fn enqueue_other(&mut self, _: ()) {}
        pub fn drain_answer(&mut self) -> Vec<i32> { Vec::new() }
        pub fn set_dispatch_ports(&mut self, _: &[&str]) {}
    }
    pub fn initialize(_: &mut GlueContext) {}
    pub fn dispatch(_: &mut GlueContext) {}
}
"#;
        let driver = r#"
fn main() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut host = WorkerThread::new(-1);
    host.tick = Some(receiver);
    sender.send(10).unwrap();
    host.ba_initialize();
    assert_eq!(host.ba_pending_events.len(), 1);
    assert_eq!(host.ba_context.tick_values, vec![10]);

    sender.send(20).unwrap();
    host.ba_dispatch_once(&["tick"]);
    assert_eq!(host.ba_pending_events.len(), 2);
    let arrivals = host.collect_for_test();
    assert_eq!(arrivals.iter().map(|entry| entry.0).collect::<Vec<_>>(), vec!["tick", "tick"]);
    assert!(host.ba_pending_events.is_empty());
    assert_eq!(host.ba_context.tick_values, vec![10, 20]);

    sender.send(30).unwrap();
    assert_eq!(host.collect_for_test().len(), 1);
    assert!(host.collect_for_test().is_empty());
    assert_eq!(host.ba_context.tick_values, vec![10, 20, 30]);
}
"#;
        let source = format!("{support}\n{emitted}\nimpl WorkerThread {{ fn collect_for_test(&mut self) -> Vec<(&'static str, u32, Instant)> {{ let mut events = Vec::new();\n{collection}\nevents }} }}\n{driver}");
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let folder = std::env::temp_dir().join(format!("aadl2rust-ba-arrival-{}-{stamp}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let source_path = folder.join("arrival.rs");
        let binary_path = folder.join(if cfg!(windows) { "arrival.exe" } else { "arrival" });
        std::fs::write(&source_path, source).unwrap();
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
        let compilation = std::process::Command::new(rustc).arg("--edition=2021").arg(&source_path).arg("-o").arg(&binary_path).output().unwrap();
        assert!(compilation.status.success(), "generated transport compilation failed at {}: {}", source_path.display(), String::from_utf8_lossy(&compilation.stderr));
        let execution = std::process::Command::new(&binary_path).output().unwrap();
        assert!(execution.status.success(), "generated transport regression failed: {}", String::from_utf8_lossy(&execution.stderr));
    }

    fn clock_test_source(protocol: &str, driver: &str) -> String {
        let mut packages = model(protocol);
        // A data-only arrival must never impersonate an event-triggered dispatch.
        let component = packages[0].public_section.as_mut().unwrap().declarations.iter_mut().find_map(|declaration| {
            if let AadlDeclaration::ComponentType(component) = declaration { Some(component) } else { None }
        }).unwrap();
        let FeatureClause::Items(features) = &mut component.features else { panic!("fixture features") };
        let Feature::Port(mut sample) = features[0].clone() else { panic!("fixture event-data port") };
        sample.identifier = "sample".to_string();
        let PortType::EventData { classifier } = sample.port_type else { panic!("fixture classifier") };
        sample.port_type = PortType::Data { classifier };
        features.push(Feature::Port(sample));
        let mut converter = AadlConverter::default();
        converter.set_available_packages(&packages);
        converter.set_external_ba_target("Worker.impl", &packages).unwrap();
        let mut module = merge_item_defs(converter.convert_package(&packages[0]));
        module.items.retain(|item| match item {
            Item::Struct(definition) => definition.name == "WorkerThread",
            Item::Impl(definition) => matches!(&definition.target, Type::Named(name) if name == "WorkerThread"),
            _ => false,
        });
        let emitted = RustCodeGenerator::new().generate_module_code(&module);
        // This is a host selection/transport test. Mock Glue records calls and
        // payloads; it is deliberately not a BA interpreter or a semantic oracle.
        let support = r#"
use std::sync::mpsc::{Sender, Receiver};
use std::time::{Duration, Instant};
trait Thread { fn new(cpu_id: isize) -> Self; fn run(self); }
mod ba_glue {
    #[derive(Debug, Default)]
    pub struct GlueContext {
        pub tick_values: Vec<i32>, pub samples: Vec<i32>, pub initializations: usize,
        pub dispatches: Vec<Vec<String>>, triggers: Vec<String>, outputs: Vec<i32>,
        pub send_during_dispatch: Option<std::sync::mpsc::Sender<i32>>,
        pub callback_on_initialize: bool, pub callback_on_dispatch: bool,
        pub arrival_ports: Vec<&'static str>,
    }
    impl GlueContext {
        pub fn bind_receiver_tick<T>(&mut self, _: &T) {}
        pub fn bind_receiver_other<T>(&mut self, _: &T) {}
        pub fn bind_receiver_sample<T>(&mut self, _: &T) {}
        pub fn take_arrival_ports(&mut self) -> Vec<&'static str> { std::mem::take(&mut self.arrival_ports) }
        pub fn enqueue_tick(&mut self, value: i32) { self.tick_values.push(value); }
        pub fn enqueue_other(&mut self, _: ()) {}
        pub fn enqueue_sample(&mut self, value: i32) { self.samples.push(value); }
        pub fn drain_answer(&mut self) -> Vec<i32> { std::mem::take(&mut self.outputs) }
        pub fn set_dispatch_ports(&mut self, ports: &[&str]) { self.triggers = ports.iter().map(|name| name.to_string()).collect(); }
    }
    pub fn initialize(context: &mut GlueContext) {
        context.initializations += 1;
        if context.callback_on_initialize { context.arrival_ports.push("other"); }
    }
    pub fn dispatch(context: &mut GlueContext) {
        context.dispatches.push(context.triggers.clone());
        context.outputs.push(context.dispatches.len() as i32);
        if let Some(sender) = context.send_during_dispatch.take() { sender.send(99).unwrap(); }
        // Simulate only the Glue notification contract, not BA or receiver behavior.
        if std::mem::take(&mut context.callback_on_dispatch) { context.arrival_ports.push("other"); }
    }
}
fn setup() -> (WorkerThread, Sender<i32>, Sender<()>, Sender<i32>, Receiver<i32>) {
    let mut host = WorkerThread::new(-1);
    let (tick, tick_rx) = std::sync::mpsc::channel(); host.tick = Some(tick_rx);
    let (other, other_rx) = std::sync::mpsc::channel(); host.other = Some(other_rx);
    let (sample, sample_rx) = std::sync::mpsc::channel(); host.sample = Some(sample_rx);
    let (answer_tx, answer) = std::sync::mpsc::channel(); host.answer = Some(answer_tx);
    (host, tick, other, sample, answer)
}
fn ms(value: u64) -> Duration { Duration::from_millis(value) }
"#;
        format!("{support}\n{emitted}\nfn main() {{ {driver} }}")
    }

    fn run_generated_clock_test(protocol: &str, driver: &str) {
        let source = clock_test_source(protocol, driver);
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let folder = std::env::temp_dir().join(format!("aadl2rust-ba-clock-{}-{protocol}-{stamp}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let source_path = folder.join("clock.rs");
        let binary_path = folder.join(if cfg!(windows) { "clock.exe" } else { "clock" });
        std::fs::write(&source_path, source).unwrap();
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
        let compilation = std::process::Command::new(rustc).arg("--edition=2021").arg(&source_path).arg("-o").arg(&binary_path).output().unwrap();
        assert!(compilation.status.success(), "{protocol} generated host failed at {}: {}", source_path.display(), String::from_utf8_lossy(&compilation.stderr));
        let execution = std::process::Command::new(&binary_path).output().unwrap();
        assert!(execution.status.success(), "{protocol} generated host clock regression failed: {}", String::from_utf8_lossy(&execution.stderr));
    }

    #[test]
    fn generated_host_rejects_mixed_entries_before_transport_or_clock_changes() {
        for protocol in ["Periodic", "Sporadic", "Aperiodic", "Timed", "Hybrid", "Background"] {
            run_generated_clock_test(protocol, r#"
                let (mut host, tick, _, _, answer) = setup();
                host.ba_initialize();
                tick.send(7).unwrap();
                host.ba_dispatch_once(&["tick"]);
                tick.send(8).unwrap();
                assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| host.ba_poll_dispatch(ms(100)))).is_err());
                assert_eq!(host.ba_last_poll, ms(0));
                assert_eq!(host.ba_last_dispatch, None);
                assert_eq!(host.ba_context.tick_values, vec![7]);
                assert_eq!(host.ba_context.dispatches.len(), 1);
                assert_eq!(answer.try_recv().unwrap(), 1);
                assert!(answer.try_recv().is_err());
                host.ba_dispatch_once(&["tick"]);
                assert_eq!(host.ba_context.tick_values, vec![7, 8]);
                assert_eq!(host.ba_context.dispatches.len(), 2);

                let (mut host, tick, _, _, _answer) = setup();
                host.ba_initialize();
                host.ba_poll_dispatch(ms(0));
                tick.send(9).unwrap();
                let before = host.ba_context.dispatches.len();
                assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| host.ba_dispatch_once(&["tick"]))).is_err());
                assert_eq!(host.ba_context.dispatches.len(), before);
                assert!(host.ba_context.tick_values.is_empty());
                assert!(host.ba_poll_dispatch(ms(10)) || host.ba_background_dispatched);
                assert_eq!(host.ba_context.tick_values, vec![9]);
            "#);
        }
    }

    #[test]
    fn generated_periodic_clock_keeps_release_grid() {
        run_generated_clock_test("Periodic", r#"
            let (mut host, tick, _, sample, answer) = setup();
            tick.send(7).unwrap(); sample.send(8).unwrap();
            host.ba_initialize();
            assert!(!host.ba_poll_dispatch(ms(0)));
            assert!(!host.ba_poll_dispatch(Duration::from_nanos(9_999_999)));
            assert!(host.ba_poll_dispatch(ms(10)));
            assert!(!host.ba_poll_dispatch(ms(10)));
            assert_eq!(host.ba_context.dispatches, vec![Vec::<String>::new()]);
            assert_eq!(host.ba_context.tick_values, vec![7]);
            assert_eq!(host.ba_context.samples, vec![8]);
            assert_eq!(answer.try_recv().unwrap(), 1);
            // Late-poll catch-up is an explicit host policy, not reconstruction of past inputs.
            assert!(host.ba_poll_dispatch(ms(35)));
            assert!(host.ba_poll_dispatch(ms(35)));
            assert!(!host.ba_poll_dispatch(ms(35)));
            assert_eq!(host.ba_context.dispatches.len(), 3);
            assert_eq!(host.ba_next_release, ms(40));
        "#);
    }

    #[test]
    fn generated_sporadic_clock_retains_batch_until_eligible() {
        run_generated_clock_test("Sporadic", r#"
            let (mut host, tick, other, sample, _answer) = setup();
            tick.send(10).unwrap(); other.send(()).unwrap(); host.ba_initialize();
            assert!(host.ba_poll_dispatch(ms(0)));
            assert_eq!(host.ba_context.dispatches[0], vec!["tick", "other"]);
            tick.send(20).unwrap(); sample.send(30).unwrap();
            assert!(!host.ba_poll_dispatch(ms(1)));
            assert!(!host.ba_poll_dispatch(Duration::from_nanos(9_999_999)));
            assert_eq!(host.ba_pending_events.len(), 1);
            assert!(host.ba_poll_dispatch(ms(10)));
            assert_eq!(host.ba_context.dispatches[1], vec!["tick"]);
            assert!(host.ba_pending_events.is_empty());
            assert!(!host.ba_poll_dispatch(ms(10)));
            assert_eq!(host.ba_context.tick_values, vec![10, 20]);
            assert_eq!(host.ba_context.samples, vec![30]);
            // The approved batch policy consumes arrival notifications, not queued payloads.
            tick.send(40).unwrap(); tick.send(50).unwrap();
            assert!(host.ba_poll_dispatch(ms(20)));
            assert_eq!(host.ba_context.dispatches[2], vec!["tick"]);
            assert!(!host.ba_poll_dispatch(ms(30)));
            assert_eq!(host.ba_context.tick_values, vec![10, 20, 40, 50]);
        "#);
    }

    #[test]
    fn generated_timed_clock_resets_on_event_and_timeout() {
        run_generated_clock_test("Timed", r#"
            let (mut host, tick, _, sample, _answer) = setup(); host.ba_initialize();
            assert!(!host.ba_poll_dispatch(Duration::from_nanos(9_999_999)));
            assert!(host.ba_poll_dispatch(ms(10)));
            assert_eq!(host.ba_context.dispatches[0], Vec::<String>::new());
            tick.send(7).unwrap(); assert!(host.ba_poll_dispatch(ms(17)));
            assert_eq!(host.ba_context.dispatches[1], vec!["tick"]);
            assert!(!host.ba_poll_dispatch(ms(20)));
            assert!(!host.ba_poll_dispatch(Duration::from_nanos(26_999_999)));
            assert!(host.ba_poll_dispatch(ms(27)));
            // Data arrival does not reset the timeout or activate an event-driven host.
            sample.send(8).unwrap(); assert!(!host.ba_poll_dispatch(ms(30)));
            tick.send(9).unwrap(); assert!(host.ba_poll_dispatch(ms(37)));
            assert!(!host.ba_poll_dispatch(ms(37)));
            assert_eq!(host.ba_context.dispatches[3], vec!["tick"]);
            assert!(host.ba_poll_dispatch(ms(47)));
        "#);
    }

    #[test]
    fn generated_hybrid_clock_does_not_move_grid_on_event() {
        run_generated_clock_test("Hybrid", r#"
            let (mut host, tick, _, _, _answer) = setup(); host.ba_initialize();
            tick.send(7).unwrap(); assert!(host.ba_poll_dispatch(ms(7)));
            assert!(!host.ba_poll_dispatch(Duration::from_nanos(9_999_999)));
            assert!(host.ba_poll_dispatch(ms(10)));
            assert_eq!(host.ba_context.dispatches[1], Vec::<String>::new());
            tick.send(8).unwrap(); assert!(host.ba_poll_dispatch(ms(20)));
            assert!(!host.ba_poll_dispatch(ms(20)));
            assert_eq!(host.ba_context.dispatches[2], vec!["tick"]);
            assert!(host.ba_poll_dispatch(ms(30)));
            assert_eq!(host.ba_context.dispatches.len(), 4);
        "#);
    }

    #[test]
    fn generated_aperiodic_batch_and_during_dispatch_arrival() {
        run_generated_clock_test("Aperiodic", r#"
            let (mut host, tick, other, sample, _answer) = setup();
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| host.ba_poll_dispatch(ms(0)))).is_err());
            host.ba_initialize();
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| host.ba_initialize())).is_err());
            assert_eq!(host.ba_context.initializations, 1);
            sample.send(8).unwrap(); assert!(!host.ba_poll_dispatch(ms(0)));
            tick.send(7).unwrap(); other.send(()).unwrap();
            host.ba_context.send_during_dispatch = Some(tick.clone());
            assert!(host.ba_poll_dispatch(ms(0)));
            assert_eq!(host.ba_context.dispatches[0], vec!["tick", "other"]);
            assert_eq!(host.ba_context.tick_values, vec![7]);
            // Arrival during C execution belongs to the next poll, never the fixed current snapshot.
            assert!(host.ba_poll_dispatch(ms(0)));
            assert_eq!(host.ba_context.dispatches[1], vec!["tick"]);
            assert_eq!(host.ba_context.tick_values, vec![7, 99]);
            assert!(!host.ba_poll_dispatch(ms(1)));
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| host.ba_poll_dispatch(ms(0)))).is_err());
        "#);
    }

    #[test]
    fn generated_background_dispatches_once_in_poll_and_real_run() {
        run_generated_clock_test("Background", r#"
            let (mut host, _, _, _, _answer) = setup(); host.ba_initialize();
            assert!(host.ba_poll_dispatch(ms(0)));
            assert!(!host.ba_poll_dispatch(ms(0)));
            assert!(!host.ba_poll_dispatch(ms(100)));
            assert_eq!(host.ba_context.dispatches.len(), 1);
            // The actual generated Thread::run returns after the same one activation.
            let (running_host, _, _, _, answer) = setup(); running_host.run();
            assert_eq!(answer.try_recv().unwrap(), 1);
            assert!(answer.try_recv().is_err());
        "#);
    }

    #[test]
    fn generated_host_imports_callback_tickets_after_each_c_entry() {
        run_generated_clock_test("Aperiodic", r#"
            let (mut host, _, _, _, _answer) = setup();
            host.ba_context.callback_on_initialize = true;
            host.ba_initialize();
            assert!(host.ba_context.arrival_ports.is_empty());
            assert_eq!(host.ba_pending_events.len(), 1);
            host.ba_context.callback_on_dispatch = true;
            assert!(host.ba_poll_dispatch(ms(0)));
            assert_eq!(host.ba_context.dispatches[0], vec!["other"]);
            // Dispatch imports a new ticket after the current batch was consumed.
            assert_eq!(host.ba_pending_events.len(), 1);
            assert!(host.ba_poll_dispatch(ms(0)));
            assert_eq!(host.ba_context.dispatches[1], vec!["other"]);
            assert!(!host.ba_poll_dispatch(ms(0)));
        "#);
        run_generated_clock_test("Periodic", r#"
            let (mut host, _, _, _, _answer) = setup();
            host.ba_context.callback_on_initialize = true;
            host.ba_initialize();
            assert!(host.ba_context.arrival_ports.is_empty());
            host.ba_context.callback_on_dispatch = true;
            assert!(host.ba_poll_dispatch(ms(10)));
            assert!(host.ba_context.arrival_ports.is_empty());
            assert!(!host.ba_poll_dispatch(ms(10)));
        "#);
    }
}
