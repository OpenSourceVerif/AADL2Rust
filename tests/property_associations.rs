use compiler::aadlight_parser::{AADLParser, Rule};
use compiler::ast::aadl_ast_cj::{
    Property, PropertyExpression, PropertyListElement, PropertyValue,
};
use compiler::transform::AADLTransformer;
use pest::Parser;

#[test]
fn parses_reference_list_with_multiple_applies_to_targets() {
    let input = "Actual_Processor_Binding => \
        (reference (cpu1), reference (cpu2)) applies to process1, process2;";
    let pair = AADLParser::parse(Rule::property_association, input)
        .expect("property association should parse")
        .next()
        .unwrap();

    let Property::BasicProperty(property) = AADLTransformer::transform_property_association(pair)
    else {
        panic!("expected a basic property association");
    };

    assert_eq!(
        property.applies_to,
        Some(vec!["process1".to_string(), "process2".to_string()])
    );

    let PropertyValue::List(elements) = property.value else {
        panic!("expected a property list value");
    };
    let references: Vec<_> = elements
        .iter()
        .filter_map(|element| match element {
            PropertyListElement::Value(PropertyExpression::Reference(reference)) => {
                Some(reference.identifier.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(references, vec!["cpu1", "cpu2"]);
}

#[test]
fn parses_empty_property_list() {
    let pair = AADLParser::parse(Rule::property_association, "Source_Text => ();")
        .expect("empty property list should parse")
        .next()
        .unwrap();

    let Property::BasicProperty(property) = AADLTransformer::transform_property_association(pair)
    else {
        panic!("expected a basic property association");
    };

    assert!(matches!(property.value, PropertyValue::List(elements) if elements.is_empty()));
}
