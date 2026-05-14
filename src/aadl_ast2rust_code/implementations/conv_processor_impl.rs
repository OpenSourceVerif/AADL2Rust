use aadl_intermediate::*;

use crate::ast::aadl_ast_cj::*;
use std::collections::HashMap;

// Convert CPU implementation
pub fn convert_processor_implementation(
    cpu_scheduling_protocols: &mut HashMap<String, String>,
    impl_: &ComponentImplementation,
) -> Vec<Item> {
    // Extract the Scheduling_Protocol property from the CPU implementation and store it
    let cpu_name = impl_.name.type_identifier.clone();

    if let PropertyClause::Properties(props) = &impl_.properties {
        for prop in props {
            if let Property::BasicProperty(bp) = prop {
                if bp.identifier.name.to_lowercase() == "scheduling_protocol" {
                    if let PropertyValue::Single(PropertyExpression::String(StringTerm::Literal(
                        scheduling_protocol,
                    ))) = &bp.value
                    {
                        cpu_scheduling_protocols
                            .insert(cpu_name.clone(), scheduling_protocol.clone());
                        return Vec::new(); // CPU implementations do not generate code; only store information
                    }
                }
            }
        }
    }

    // If Scheduling_Protocol is not found, use the default value
    cpu_scheduling_protocols.insert(cpu_name.clone(), "FIFO".to_string());
    println!(
        "CPU implementation {} does not specify a scheduling protocol; using default: FIFO",
        cpu_name
    );
    Vec::new() // CPU implementations do not generate code; only store information
}
