//! Host parser regressions from AADL-BA-Rocq-light/Examples/Unit.
//! The fixture files are unmodified copies of the real source models. Only the
//! BA annex is delegated to the external compiler, as in the integration host.
use aadl_intermediate::RustCodeGenerator;
use compiler::aadl_ast2rust_code::converter::AadlConverter;
use compiler::aadl_ast2rust_code::merge_utils::merge_item_defs;
use compiler::aadlight_parser::{AADLParser, Rule};
use compiler::ast::aadl_ast_cj::*;
use compiler::transform::AADLTransformer;
use pest::Parser;

fn host_packages(source: &str) -> Vec<Package> {
    let annex_start = source.find("annex behavior_specification {**").unwrap();
    let annex_end = annex_start + source[annex_start..].find("**};").unwrap() + 4;
    let host = format!("{}{}", &source[..annex_start], &source[annex_end..]);
    let parsed = AADLParser::parse(Rule::file, &host).expect("real host model must parse completely");
    AADLTransformer::transform_file(parsed.collect())
}

fn port_specs(packages: &[Package]) -> Vec<&PortSpec> {
    let component = packages[0].public_section.as_ref().unwrap().declarations.iter()
        .find_map(|declaration| match declaration {
            AadlDeclaration::ComponentType(component) if component.identifier == "Worker" => Some(component),
            _ => None,
        }).unwrap();
    let FeatureClause::Items(features) = &component.features else { panic!("missing features") };
    features.iter().map(|feature| match feature {
        Feature::Port(port) => port,
        _ => panic!("expected port"),
    }).collect()
}

#[test]
fn port_freeze_times_retains_each_input_time_record() {
    let packages = host_packages(include_str!("fixtures/external_ba_parser/PortFreezeTimes.aadl"));
    let ports = port_specs(&packages);
    assert_eq!(ports.iter().map(|port| port.identifier.as_str()).collect::<Vec<_>>(),
        ["tick", "selected_input", "deferred_input"]);
    assert!(ports[0].properties.is_empty());
    for port in &ports[1..] {
        assert_eq!(port.direction, PortDirection::In);
        assert!(matches!(port.port_type, PortType::Data { classifier: Some(_) }));
        assert_eq!(port.properties.len(), 1);
        let Property::BasicProperty(association) = &port.properties[0] else { panic!("expected association") };
        assert_eq!(association.identifier.name, "Input_Time");
        assert_eq!(association.operator, PropertyOperator::Assign);
        let PropertyValue::List(values) = &association.value else { panic!("Input_Time must remain a list") };
        assert_eq!(values.len(), 1);
        let PropertyListElement::Value(PropertyExpression::RecordValue(record_value)) = &values[0]
            else { panic!("Input_Time must contain a structured record, never a string") };
        assert_eq!(record_value.fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>(),
            ["Time", "Time_Reference"]);
        assert!(matches!(&record_value.fields[0].value,
            PropertyValue::Single(PropertyExpression::Integer(SignedIntergerOrConstant::Real(value)))
            if value.value == 0 && value.sign.is_none() && value.unit.as_deref() == Some("ms")));
        assert!(matches!(&record_value.fields[1].value,
            PropertyValue::Single(PropertyExpression::String(StringTerm::Literal(value))) if value == "No_IO"));
    }

    // Exercise the real converter: metadata must neither remove the scalar ports
    // nor create extra runtime fields. This checks generation, not freezing semantics.
    let mut converter = AadlConverter::default();
    converter.set_available_packages(&packages);
    converter.set_external_ba_target("Worker.impl", &packages).unwrap();
    let module = merge_item_defs(converter.convert_package(&packages[0]));
    let generated = RustCodeGenerator::new().generate_module_code(&module);
    syn::parse_file(&generated).expect("generated real thread must be valid Rust syntax");
    assert!(generated.contains("pub tick: Option<Receiver<()>>"));
    for name in ["selected_input", "deferred_input"] {
        assert!(generated.contains(&format!("pub {name}: Option<Receiver<i32>>")), "{generated}");
        assert!(generated.contains(&format!("self.ba_context.enqueue_{name}(ba_value)")));
    }
    assert!(!generated.contains("pub input_time:"));
}

#[test]
fn background_is_parsed_and_uses_the_external_single_activation_entry() {
    let packages = host_packages(include_str!("fixtures/external_ba_parser/DispatchBackground.aadl"));
    let implementation = packages[0].public_section.as_ref().unwrap().declarations.iter()
        .find_map(|declaration| match declaration {
            AadlDeclaration::ComponentImplementation(implementation) => Some(implementation),
            _ => None,
        }).unwrap();
    let PropertyClause::Properties(properties) = &implementation.properties else { panic!("missing properties") };
    assert!(properties.iter().any(|property| matches!(property,
        Property::BasicProperty(association) if association.identifier.name == "Dispatch_Protocol"
            && matches!(&association.value,
                PropertyValue::Single(PropertyExpression::String(StringTerm::Literal(value))) if value == "Background"))));
    assert_eq!(port_specs(&packages).len(), 1);
    let mut converter = AadlConverter::default();
    converter.set_available_packages(&packages);
    converter.set_external_ba_target("Worker.impl", &packages).unwrap();
    let module = merge_item_defs(converter.convert_package(&packages[0]));
    let generated = RustCodeGenerator::new().generate_module_code(&module);
    syn::parse_file(&generated).expect("Background host must remain valid Rust");
    assert!(generated.contains("self.ba_poll_dispatch(std::time::Duration::ZERO)"));
    assert!(generated.contains("if self.ba_background_dispatched"));
}

#[test]
fn nested_record_fields_keep_their_lists_and_association_metadata() {
    let source = "sample : in data port Types::Signed32 { Timing::Input_Time +=> constant \
        ([Time => -2 ms; Time_Reference => No_IO; Details => [Values => (1, 2);];]) applies to sample; };";
    let parsed = AADLParser::parse(Rule::feature_declaration, source).unwrap().next().unwrap();
    assert_eq!(parsed.as_str(), source);
    let Feature::Port(port) = AADLTransformer::transform_feature_declaration(parsed) else { panic!("expected port") };
    let Property::BasicProperty(association) = &port.properties[0] else { panic!("expected association") };
    assert_eq!(association.identifier.property_set.as_deref(), Some("Timing"));
    assert_eq!(association.operator, PropertyOperator::Append);
    assert!(association.is_constant);
    assert_eq!(association.applies_to.as_ref().unwrap(), &["sample"]);
    let PropertyValue::List(values) = &association.value else { panic!("expected list") };
    let PropertyListElement::Value(PropertyExpression::RecordValue(record_value)) = &values[0] else { panic!("expected record") };
    assert!(matches!(&record_value.fields[0].value,
        PropertyValue::Single(PropertyExpression::Integer(SignedIntergerOrConstant::Real(value)))
        if value.sign == Some(Sign::Minus) && value.value == 2 && value.unit.as_deref() == Some("ms")));
    let PropertyValue::Single(PropertyExpression::RecordValue(nested)) = &record_value.fields[2].value else { panic!("expected nested record") };
    assert_eq!(nested.fields[0].name, "Values");
    let PropertyValue::List(numbers) = &nested.fields[0].value else { panic!("expected nested list") };
    assert_eq!(numbers.len(), 2);
}
