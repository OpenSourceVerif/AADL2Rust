// Convert the Pest parse output into aadlAst
#![allow(clippy::single_match, clippy::if_same_then_else)]
use super::ast::aadl_ast_cj::*;
use crate::aadlight_parser;
use crate::transform_annex::*;
use pest::iterators::Pair;

// Import the annex transformation module
// transform_annex is now declared in main.rs

// Port information management struct
#[derive(Debug, Clone)]
pub struct PortInfo {
    pub name: String,
    pub direction: PortDirection,
}

// Port information manager
pub struct PortManager {
    ports: Vec<PortInfo>,
}

impl Default for PortManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PortManager {
    pub fn new() -> Self {
        Self { ports: Vec::new() }
    }

    pub fn add_port(&mut self, name: String, direction: PortDirection) {
        self.ports.push(PortInfo { name, direction });
    }

    pub fn get_port_direction(&self, name: &str) -> Option<PortDirection> {
        self.ports
            .iter()
            .find(|port| port.name == name)
            .map(|port| port.direction)
    }

    pub fn is_outgoing_port(&self, name: &str) -> bool {
        if let Some(direction) = self.get_port_direction(name) {
            matches!(direction, PortDirection::Out | PortDirection::InOut)
        } else {
            false
        }
    }
}

// Global port manager
use once_cell::sync::Lazy;
use std::sync::Mutex;

static GLOBAL_PORT_MANAGER: Lazy<Mutex<PortManager>> = Lazy::new(|| Mutex::new(PortManager::new()));

pub fn get_global_port_manager() -> &'static Mutex<PortManager> {
    &GLOBAL_PORT_MANAGER
}

// Helper: extract an identifier from a Pair
pub fn extract_identifier(pair: Pair<aadlight_parser::Rule>) -> String {
    pair.as_str().trim().to_string()
}

// Helper: extract a package name from a Pair
pub fn extract_package_name(pair: Pair<aadlight_parser::Rule>) -> PackageName {
    PackageName(
        pair.as_str()
            .split("::")
            .map(|s| s.trim().to_string())
            .collect(),
    )
}

// Main transformation struct
pub struct AADLTransformer {
    _port_manager: PortManager,
}

#[warn(unused_mut)]
impl Default for AADLTransformer {
    fn default() -> Self {
        Self::new()
    }
}

impl AADLTransformer {
    pub fn new() -> Self {
        Self {
            _port_manager: PortManager::new(),
        }
    }

    pub fn transform_file(pairs: Vec<Pair<aadlight_parser::Rule>>) -> Vec<Package> {
        let mut transformer = Self::new();
        let mut packages = Vec::new();

        // for pair in pairs {
        //     println!("Processing rule: {:?}, content: {}", pair.as_rule(), pair.as_str());
        //     if pair.as_rule() == aadlight_parser::Rule::package_declaration { // check whether it is the package_declaration rule
        //         if let Some(pkg) = Self::transform_package(pair) {
        //         }
        //     }
        // }
        for pair in pairs {
            //println!("Top-level rule: {:?}, content: {}", pair.as_rule(), pair.as_str());
            //println!("  Inner rule: {:?}", pair.as_rule());

            // Enter the file rule and extract the actual package_declaration
            if pair.as_rule() == aadlight_parser::Rule::file {
                for inner in pair.into_inner() {
                    //println!("  Inner rule: {:?}, content: {}", inner.as_rule(), inner.as_str());
                    //println!("  Inner rule: {:?}", inner.as_rule());
                    if inner.as_rule() == aadlight_parser::Rule::package_declaration {
                        if let Some(pkg) = transformer.transform_package(inner) {
                            packages.push(pkg);
                        }
                    }
                }
            }
        }

        packages
    }

    pub fn transform_package(&mut self, pair: Pair<aadlight_parser::Rule>) -> Option<Package> {
        //println!("=== Debug package ===");
        //println!("pair = Rule::{:?}", pair.as_rule());
        // for (i, inner) in pair.clone().into_inner().enumerate() {
        //     //println!("  inner[{}]: Rule::{:?}, text = {}", i, inner.as_rule(), inner.as_str());
        //     println!("  inner[{}]: Rule::{:?}", i, inner.as_rule());
        // }

        let mut inner_iter = pair.into_inner();
        let package_name = extract_package_name(inner_iter.next().unwrap());
        let mut visibility_decls = Vec::new();
        let mut public_section = None;
        let mut private_section = None;
        let properties = PropertyClause::ExplicitNone;

        for inner in inner_iter {
            //println!("  Inner rule: {:?}, content: {}", inner.as_rule(), inner.as_str());
            match inner.as_rule() {
                aadlight_parser::Rule::visibility_declarations => {
                    visibility_decls.push(Self::transform_visibility_declaration(inner));
                }
                aadlight_parser::Rule::package_sections => {
                    let section = self.transform_package_section(inner);
                    if section.is_public {
                        public_section = Some(section);
                    } else {
                        private_section = Some(section);
                    }
                }
                _ => {}
            }
        }

        Some(Package {
            name: package_name,
            visibility_decls,
            public_section,
            private_section,
            properties,
        })
    }

    pub fn transform_visibility_declaration(
        pair: Pair<aadlight_parser::Rule>,
    ) -> VisibilityDeclaration {
        // First collect all inner items into a vector so we can iterate multiple times
        let items: Vec<_> = pair.into_inner().collect();
        // println!("🧩 Parsed {} items:", items.len());
        // for (i, item) in items.iter().enumerate() {
        //     println!("  [{}] Rule: {:?}, Text: {}", i, item.as_rule(), item.as_str());
        // }

        match items.first().unwrap().as_str() {
            "with" => {
                // Handle the with clause
                let mut packages = Vec::new();
                let mut property_sets = Vec::new();

                // Skip the first "with" item
                for item in items.iter().skip(1) {
                    match item.as_rule() {
                        aadlight_parser::Rule::package_name => {
                            // If it is base_types, data_model, etc., treat it as a property set name and ignore it.
                            // It cannot be distinguished (file name vs. property set name) here, so we enumerate known property set names.
                            match item.clone().as_str().to_lowercase().as_str() {
                                "base_types" | "data_model" => {}
                                _ => {
                                    packages.push(extract_package_name(item.clone()));
                                }
                            }
                            // if item.as_str().contains("::") {
                            //     packages.push(extract_package_name(item.clone()));
                            // }
                        }
                        aadlight_parser::Rule::property_set_name => {
                            property_sets.push(extract_identifier(item.clone()));
                        }
                        _ => {} // ignore commas and other tokens
                    }
                }

                VisibilityDeclaration::Import {
                    packages,
                    property_sets,
                }
            }
            _ => {
                let identifier = extract_identifier(items[0].clone());
                //println!("🔎 Trying to handle renames statement: {:?}", items);

                let original = extract_package_name(items[1].clone());

                VisibilityDeclaration::Alias {
                    new_name: identifier.clone(),
                    original: QualifiedName {
                        package_prefix: None,
                        identifier: original.0.join("::"),
                    },
                    is_package: true, // assume we currently only handle package renames
                }
            }
        }
    }

    pub fn transform_package_section(
        &mut self,
        pair: Pair<aadlight_parser::Rule>,
    ) -> PackageSection {
        // println!("=== Debug package_section ===");
        // println!("pair = Rule::{:?}", pair.as_rule());
        // for (i, inner) in pair.clone().into_inner().enumerate() {
        //     //println!("  inner[{}]: Rule::{:?}, text = {}", i, inner.as_rule(), inner.as_str());
        //     println!("  inner[{}]: Rule::{:?}", i, inner.as_rule());
        // }

        let mut is_public = true; // default is public
        let mut declarations = Vec::new();

        let mut inner_iter = pair.into_inner();

        // Check whether the first element is the public/private modifier
        if let Some(first) = inner_iter.next() {
            match first.as_str() {
                "public" => {
                    is_public = true;
                }
                "private" => {
                    is_public = false;
                }
                _ => {
                    // If it is not a modifier, treat it as a declaration
                    declarations.push(self.transform_declaration(first));
                }
            }
        }

        // Process the remaining declarations
        for inner in inner_iter {
            match inner.as_rule() {
                aadlight_parser::Rule::declaration => {
                    declarations.push(self.transform_declaration(inner));
                }
                _ => {} // ignore other rules
            }
        }

        PackageSection {
            is_public,
            declarations,
        }
    }

    pub fn transform_declaration(&mut self, pair: Pair<aadlight_parser::Rule>) -> AadlDeclaration {
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            aadlight_parser::Rule::component_type => {
                AadlDeclaration::ComponentType(self.transform_component_type(inner))
            }
            aadlight_parser::Rule::component_implementation => {
                AadlDeclaration::ComponentImplementation(Self::transform_component_implementation(
                    inner,
                ))
            }
            aadlight_parser::Rule::annex_library => AadlDeclaration::AnnexLibrary(AnnexLibrary {}),
            _ => panic!("Unsupported declaration type: {:?}", inner.as_rule()),
        }
    }

    pub fn transform_component_type(&mut self, pair: Pair<aadlight_parser::Rule>) -> ComponentType {
        let mut inner_iter = pair.into_inner();

        let category = match inner_iter.next().unwrap().as_str() {
            "system" => ComponentCategory::System,
            "process" => ComponentCategory::Process,
            "thread" => ComponentCategory::Thread,
            "data" => ComponentCategory::Data,
            "subprogram" => ComponentCategory::Subprogram,
            "processor" => ComponentCategory::Processor,
            "memory" => ComponentCategory::Memory,
            "device" => ComponentCategory::Device,
            "bus" => ComponentCategory::Bus,
            s => panic!("Unknown component category: {}", s),
        };

        let identifier = extract_identifier(inner_iter.next().unwrap());
        let mut prototypes = PrototypeClause::None;
        let mut features = FeatureClause::None;
        let mut properties = PropertyClause::ExplicitNone;
        let mut annexes = Vec::new();

        for inner in inner_iter {
            match inner.as_rule() {
                aadlight_parser::Rule::prototypes => {
                    prototypes = Self::transform_prototypes_clause(inner);
                }
                aadlight_parser::Rule::features => {
                    features = self.transform_features_clause(inner);
                }
                aadlight_parser::Rule::properties => {
                    properties = Self::transform_properties_clause(inner);
                }
                aadlight_parser::Rule::annex_subclause => {
                    if let Some(annex) = transform_annex_subclause(inner) {
                        annexes.push(annex);
                    }
                }
                aadlight_parser::Rule::extends => {
                    //TODO: handle extends
                    //println!("extends: {:?}", inner.as_str());
                }
                _ => {}
            }
        }

        ComponentType {
            category,
            identifier,
            prototypes,
            features,
            properties,
            annexes,
        }
    }

    pub fn transform_prototypes_clause(pair: Pair<aadlight_parser::Rule>) -> PrototypeClause {
        if pair.as_str().contains("none") {
            return PrototypeClause::Empty;
        }

        let mut prototypes = Vec::new();
        for inner in pair.into_inner() {
            if inner.as_rule() == aadlight_parser::Rule::prototype_declaration {
                prototypes.push(Self::transform_prototype_declaration(inner));
            }
        }

        if prototypes.is_empty() {
            PrototypeClause::None
        } else {
            PrototypeClause::Items(prototypes)
        }
    }

    pub fn transform_prototype_declaration(pair: Pair<aadlight_parser::Rule>) -> Prototype {
        let mut inner_iter = pair.into_inner();
        let _identifier = extract_identifier(inner_iter.next().unwrap());
        let _colon = inner_iter.next();
        let prototype_type = inner_iter.next().unwrap();

        match prototype_type.as_str() {
            "component" => {
                let category = match inner_iter.next().unwrap().as_str() {
                    "system" => ComponentCategory::System,
                    "process" => ComponentCategory::Process,
                    "thread" => ComponentCategory::Thread,
                    "data" => ComponentCategory::Data,
                    "subprogram" => ComponentCategory::Subprogram,
                    "processor" => ComponentCategory::Processor,
                    "memory" => ComponentCategory::Memory,
                    s => panic!("Unknown component prototype category: {}", s),
                };

                Prototype::Component(ComponentPrototype {
                    category,
                    classifier: None, // TODO: handle classifier
                    is_array: false,  // TODO: handle array spec
                })
            }
            "feature" => {
                Prototype::Feature(FeaturePrototype {
                    direction: None,  // TODO: handle direction
                    classifier: None, // TODO: handle classifier
                })
            }
            "feature group" => {
                Prototype::FeatureGroup(FeatureGroupPrototype {
                    classifier: None, // TODO: handle classifier
                })
            }
            _ => panic!("Unknown prototype type"),
        }
    }

    pub fn transform_features_clause(
        &mut self,
        pair: Pair<aadlight_parser::Rule>,
    ) -> FeatureClause {
        if pair.as_str().contains("none") {
            return FeatureClause::Empty;
        }

        let mut features = Vec::new();
        for inner in pair.into_inner() {
            if inner.as_rule() == aadlight_parser::Rule::feature_declaration {
                let feature = Self::transform_feature_declaration(inner);

                // Collect port information
                if let Feature::Port(port_spec) = &feature {
                    if let Ok(mut manager) = get_global_port_manager().lock() {
                        manager.add_port(port_spec.identifier.clone(), port_spec.direction);
                    }
                }

                features.push(feature);
            }
        }

        if features.is_empty() {
            FeatureClause::None
        } else {
            FeatureClause::Items(features)
        }
    }

    pub fn transform_feature_declaration(pair: Pair<aadlight_parser::Rule>) -> Feature {
        let mut inner_iter = pair.into_inner();

        let identifier = extract_identifier(inner_iter.next().unwrap()); // p
        let mut direction: Option<PortDirection> = None;
        let mut port_type_str: Option<&str> = None;
        let mut access_direction: Option<AccessDirection> = None;
        let mut access_type_str: Option<&str> = None; // "data" | "subprogram"
        let mut classifier_qname: Option<String> = None; // qualified_identifier or identifier

        for inner in inner_iter {
            match inner.as_rule() {
                aadlight_parser::Rule::direction => {
                    direction = match inner.as_str() {
                        "in" => Some(PortDirection::In),
                        "out" => Some(PortDirection::Out),
                        "in out" => Some(PortDirection::InOut),
                        _ => None,
                    };
                }
                aadlight_parser::Rule::port_type => {
                    port_type_str = Some(inner.as_str());
                }
                aadlight_parser::Rule::access_direction => {
                    access_direction = match inner.as_str() {
                        "provides" => Some(AccessDirection::Provides),
                        "requires" => Some(AccessDirection::Requires),
                        _ => None,
                    };
                }
                aadlight_parser::Rule::access_type => {
                    access_type_str = Some(inner.as_str());
                }
                aadlight_parser::Rule::qualified_identifier => {
                    classifier_qname = Some(inner.as_str().to_string());
                }
                aadlight_parser::Rule::identifier => {
                    // Be compatible with older syntax where identifier is used as the type name
                    if classifier_qname.is_none() {
                        classifier_qname = Some(inner.as_str().to_string());
                    }
                }
                _ => {}
            }
        }

        // If this is a port feature
        if let Some(pt) = port_type_str {
            let classifier = classifier_qname.clone().map(|qname| {
                // Parse package prefix and type name
                let parts: Vec<&str> = qname.split("::").collect();
                let (package_prefix, type_id) = if parts.len() > 1 {
                    let package_name = parts[0..parts.len() - 1].join("::");
                    let type_name = parts.last().unwrap().split(".").next().unwrap().to_string();
                    (Some(package_name), type_name)
                } else {
                    (None, qname.to_string())
                };

                PortDataTypeReference::Classifier(UniqueComponentClassifierReference::Type(
                    UniqueImplementationReference {
                        package_prefix: package_prefix
                            .map(|p| PackageName(p.split("::").map(|s| s.to_string()).collect())),
                        implementation_name: ImplementationName {
                            type_identifier: type_id,
                            implementation_identifier: String::new(),
                        },
                    },
                ))
            });

            let resolved_port_type = match pt {
                "data port" | "parameter" => PortType::Data {
                    classifier: classifier.clone(),
                },
                "event data port" => PortType::EventData {
                    classifier: classifier.clone(),
                },
                "event port" => PortType::Event,
                other => panic!("Unknown port type: {}", other),
            };

            return Feature::Port(PortSpec {
                identifier,
                direction: direction.unwrap_or(match resolved_port_type {
                    PortType::Data { .. } | PortType::EventData { .. } => PortDirection::InOut,
                    PortType::Event => PortDirection::In,
                }),
                port_type: resolved_port_type,
            });
        }

        // Access feature: data access / subprogram access
        if let Some(at) = access_type_str {
            let direction = access_direction.unwrap_or(AccessDirection::Provides);

            // Build classifier (if present)
            let map_classifier_to_component_classifier =
                || -> Option<UniqueComponentClassifierReference> {
                    classifier_qname.clone().map(|qname| {
                        let type_id = qname.split("::").last().unwrap_or(&qname).to_string();

                        // Heuristic: if it ends with .Impl, treat it as an implementation reference
                        if type_id.ends_with("Impl") {
                            UniqueComponentClassifierReference::Implementation(
                                UniqueImplementationReference {
                                    package_prefix: None,
                                    implementation_name: ImplementationName {
                                        type_identifier: type_id,
                                        implementation_identifier: String::new(),
                                    },
                                },
                            )
                        } else {
                            // Otherwise treat it as a type reference
                            UniqueComponentClassifierReference::Type(
                                UniqueImplementationReference {
                                    package_prefix: None,
                                    implementation_name: ImplementationName {
                                        type_identifier: type_id,
                                        implementation_identifier: String::new(),
                                    },
                                },
                            )
                        }
                    })
                };

            match at {
                "data" => {
                    let classifier = map_classifier_to_component_classifier()
                        .map(DataAccessReference::Classifier);
                    return Feature::SubcomponentAccess(SubcomponentAccessSpec::Data(
                        DataAccessSpec {
                            identifier,
                            direction,
                            classifier,
                        },
                    ));
                }
                "subprogram" => {
                    let classifier = map_classifier_to_component_classifier()
                        .map(SubprogramAccessReference::Classifier);
                    return Feature::SubcomponentAccess(SubcomponentAccessSpec::Subprogram(
                        SubprogramAccessSpec {
                            identifier,
                            direction,
                            classifier,
                        },
                    ));
                }
                other => panic!("Unknown access type: {}", other),
            }
        }

        panic!("Unsupported feature_declaration: missing port or access spec")
    }
    pub fn transform_properties_clause(pair: Pair<aadlight_parser::Rule>) -> PropertyClause {
        if pair.as_str().contains("none") {
            return PropertyClause::ExplicitNone;
        }

        let mut properties = Vec::new();
        for inner in pair.into_inner() {
            if inner.as_rule() == aadlight_parser::Rule::property_association {
                properties.push(Self::transform_property_association(inner));
            }
        }

        if properties.is_empty() {
            PropertyClause::ExplicitNone
        } else {
            PropertyClause::Properties(properties)
        }
    }

    pub fn transform_property_association(pair: Pair<aadlight_parser::Rule>) -> Property {
        // println!("=== Debug property ===");
        // println!("pair = Rule::{:?}, text = {}", pair.as_rule(), pair.as_str());
        // for (i, inner) in pair.clone().into_inner().enumerate() {
        //     println!("  inner[{}]: Rule::{:?}, text = {}", i, inner.as_rule(), inner.as_str());
        // }

        let mut inner_iter = pair.into_inner().peekable();

        // Check whether there is a property set prefix (property_set::property_name)
        let (property_set, identifier) = if inner_iter.peek().map(|p| p.as_rule())
            == Some(aadlight_parser::Rule::identifier)
        {
            let first_identifier = extract_identifier(inner_iter.next().unwrap());

            // Check whether the next element is identifier
            if inner_iter.peek().map(|p| p.as_rule()) == Some(aadlight_parser::Rule::identifier) {
                let second_identifier = extract_identifier(inner_iter.next().unwrap());
                (Some(first_identifier), second_identifier)
            } else {
                // No second identifier: the first identifier is the property name
                (None, first_identifier)
            }
        } else {
            panic!("Expected property identifier");
        };

        let operator_pair = inner_iter.next().expect("Expected property operator");
        let operator = match operator_pair.as_str() {
            "=>" => PropertyOperator::Assign,
            "+=>" => PropertyOperator::Append,
            _ => panic!("Unknown property operator"),
        };
        // === Handle constant marker ===
        let mut is_constant = false;
        if inner_iter.peek().map(|p| p.as_rule()) == Some(aadlight_parser::Rule::constant) {
            is_constant = true;
            inner_iter.next(); // consume constant
        }
        // Process property_value
        let value: PropertyValue = Self::transform_property_value(inner_iter.next().unwrap());
        let applies_to = inner_iter.next().map(|clause| {
            clause
                .into_inner()
                .map(extract_identifier)
                .collect::<Vec<_>>()
        });

        Property::BasicProperty(BasicPropertyAssociation {
            identifier: PropertyIdentifier {
                property_set,
                name: identifier,
            },
            operator,
            is_constant, // TODO: Handle constant
            value,
            applies_to,
        })
    }

    // Helper function
    pub fn strip_string_literal(s: &str) -> String {
        if s.starts_with('"') && s.ends_with('"') {
            s[1..s.len() - 1].to_string()
        } else if s.starts_with('(') && s.ends_with(')') {
            s[1..s.len() - 1].to_string()
        } else {
            s.to_string()
        }
    }

    pub fn transform_property_value(pair: Pair<aadlight_parser::Rule>) -> PropertyValue {
        // println!("=== Debug property_value ===");
        // println!("pair = Rule::{:?}, text = {}", pair.as_rule(), pair.as_str());
        // for (i, inner) in pair.clone().into_inner().enumerate() {
        //     println!("  inner[{}]: Rule::{:?}, text = {}", i, inner.as_rule(), inner.as_str());
        // }

        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            aadlight_parser::Rule::range_value => {
                // println!("=== Debug range_value ===");
                // println!("inner = Rule::{:?}, text = {}", inner.as_rule(), inner.as_str());
                // for (i, inner2) in inner.clone().into_inner().enumerate() {
                //     println!("  inner[{}]: Rule::{:?}, text = {}", i, inner2.as_rule(), inner2.as_str());
                // }

                let mut parts = inner.into_inner();
                let lower_val = extract_identifier(parts.next().unwrap());
                //let lower_unit = Some(parts.next().unwrap().as_str().trim().to_string());
                // Parse the lower bound unit (optional, e.g., priority has no unit)
                let lower_unit = if parts
                    .peek()
                    .is_some_and(|p| p.as_rule() == aadlight_parser::Rule::unit)
                {
                    Some(parts.next().unwrap().as_str().trim().to_string())
                } else {
                    None
                };

                let upper_val = extract_identifier(parts.next().unwrap());
                //let upper_unit = Some(parts.next().unwrap().as_str().trim().to_string());
                // Parse the upper bound unit (optional)
                let upper_unit = if parts
                    .peek()
                    .is_some_and(|p| p.as_rule() == aadlight_parser::Rule::unit)
                {
                    Some(parts.next().unwrap().as_str().trim().to_string())
                } else {
                    None
                };

                // PropertyValue::List(vec![
                //     PropertyListElement::Value(PropertyExpression::String(StringTerm::Literal(lower))),
                //     PropertyListElement::Value(PropertyExpression::String(StringTerm::Literal(upper))),
                // ])
                PropertyValue::List(vec![PropertyListElement::Value(
                    PropertyExpression::IntegerRange(IntegerRangeTerm {
                        lower: StringWithUnit {
                            value: lower_val,
                            unit: lower_unit,
                        },
                        upper: StringWithUnit {
                            value: upper_val,
                            unit: upper_unit,
                        },
                    }),
                )])
            }
            aadlight_parser::Rule::literal_value => {
                // let value = inner.as_str().trim().to_string();
                // PropertyValue::Single(PropertyExpression::String(StringTerm::Literal(value)))
                // println!("=== Debug literal_value ===");
                // println!("pair = Rule::{:?}, text = {}", inner.as_rule(), inner.as_str());
                // for (i, inner2) in inner.clone().into_inner().enumerate() {
                //     println!("  inner[{}]: Rule::{:?}, text = {}", i, inner2.as_rule(), inner2.as_str());
                // }

                let mut parts = inner.into_inner().peekable();

                let first = parts.next().unwrap();
                let unit = match parts.peek() {
                    Some(p) if p.as_rule() == aadlight_parser::Rule::unit => {
                        Some(extract_identifier(parts.next().unwrap()))
                    }
                    _ => None,
                };
                // println!("=== Debug first ===");
                // println!("first = Rule::{:?}, text = {}", first.as_rule(), first.as_str());
                // for (i, inner2) in first.clone().into_inner().enumerate() {
                //     println!("  innerfirst[{}]: Rule::{:?}, text = {}", i, inner2.as_rule(), inner2.as_str());
                // }

                match first.as_rule() {
                    aadlight_parser::Rule::number => {
                        let mut number_parts = first.into_inner().peekable();

                        // Parse the sign
                        let sign = match number_parts.peek() {
                            Some(p) if p.as_rule() == aadlight_parser::Rule::sign => {
                                match number_parts.next().unwrap().as_str() {
                                    "+" => Some(Sign::Plus),
                                    "-" => Some(Sign::Minus),
                                    _ => None,
                                }
                            }
                            _ => None,
                        };
                        // Main numeric part
                        let int_part = number_parts.next().unwrap().as_str().trim();

                        // Determine whether it is a floating-point number
                        let expr = if int_part.contains('.') {
                            let value = int_part.parse::<f64>().unwrap();
                            PropertyExpression::Real(SignedRealOrConstant::Real(SignedReal {
                                sign,
                                value,
                                unit: unit.clone(),
                            }))
                        } else {
                            let value = int_part.parse::<i64>().unwrap();
                            PropertyExpression::Integer(SignedIntergerOrConstant::Real(
                                SignedInteger {
                                    sign,
                                    value,
                                    unit: unit.clone(),
                                },
                            ))
                        };

                        PropertyValue::Single(expr)
                    }

                    aadlight_parser::Rule::string_literal => {
                        let raw = first.as_str();
                        let value = Self::strip_string_literal(raw);
                        PropertyValue::Single(PropertyExpression::String(StringTerm::Literal(
                            value,
                        )))
                    }

                    aadlight_parser::Rule::boolean => {
                        let val = match first.as_str() {
                            "true" => true,
                            "false" => false,
                            _ => panic!("Invalid boolean"),
                        };

                        PropertyValue::Single(PropertyExpression::Boolean(BooleanTerm::Literal(
                            val,
                        )))
                    }

                    aadlight_parser::Rule::enum_value => {
                        let value = first.as_str().to_string();

                        PropertyValue::Single(PropertyExpression::String(StringTerm::Literal(
                            value,
                        )))
                    }

                    _ => panic!("Unknown literal_value inner rule: {:?}", first.as_rule()),
                }
            }
            aadlight_parser::Rule::list_value => {
                let mut elements = Vec::new();
                for item in inner.into_inner() {
                    let property_value = Self::transform_property_value(item);
                    match property_value {
                        PropertyValue::Single(expr) => {
                            elements.push(PropertyListElement::Value(expr));
                        }
                        PropertyValue::List(nested_elements) => {
                            elements.push(PropertyListElement::NestedList(nested_elements));
                        }
                    }
                }
                PropertyValue::List(elements)
            }
            aadlight_parser::Rule::reference_value => {
                let mut ref_parts = inner.into_inner();
                let referenced_id = extract_identifier(ref_parts.next().unwrap());
                PropertyValue::Single(PropertyExpression::Reference(ReferenceTerm {
                    identifier: referenced_id,
                }))
            }
            aadlight_parser::Rule::component_classifier_value => {
                let mut inner_iter = inner.into_inner();
                let qualified_identifier = inner_iter.next().unwrap();
                let qname = qualified_identifier.as_str().to_string();

                // Parse package prefix and type name
                let parts: Vec<&str> = qname.split("::").collect();
                let (package_prefix, type_id) = if parts.len() > 1 {
                    let package_name = parts[0..parts.len() - 1].join("::");
                    let type_name = parts.last().unwrap().to_string();
                    (Some(package_name), type_name)
                } else {
                    (None, qname.to_string())
                };

                // Heuristic: if it ends with .Impl, treat it as an implementation reference
                let unique_ref = if type_id.ends_with("Impl") {
                    UniqueComponentClassifierReference::Implementation(
                        UniqueImplementationReference {
                            package_prefix: package_prefix.map(|p| {
                                PackageName(p.split("::").map(|s| s.to_string()).collect())
                            }),
                            implementation_name: ImplementationName {
                                type_identifier: type_id,
                                implementation_identifier: String::new(),
                            },
                        },
                    )
                } else {
                    // Otherwise treat it as a type reference
                    UniqueComponentClassifierReference::Type(UniqueImplementationReference {
                        package_prefix: package_prefix
                            .map(|p| PackageName(p.split("::").map(|s| s.to_string()).collect())),
                        implementation_name: ImplementationName {
                            type_identifier: type_id,
                            implementation_identifier: String::new(),
                        },
                    })
                };

                PropertyValue::Single(PropertyExpression::ComponentClassifier(
                    ComponentClassifierTerm {
                        unique_component_classifier_reference: unique_ref,
                    },
                ))
            }
            _ => {
                println!("Unknown property value type: {:?}", inner.as_rule());
                panic!("Unknown property value type");
            }
        }
    }

    // pub fn transform_annexes_clause(pair: Pair<aadlight_parser::Rule>) -> Vec<AnnexSubclause> {
    //     //use crate::transform_annex::transform_annexes_clause as transform_annexes;
    //     //transform_annexes(pair)
    // }

    pub fn transform_component_implementation(
        pair: Pair<aadlight_parser::Rule>,
    ) -> ComponentImplementation {
        // println!("=== Debug implementation ===");
        // println!("pair = Rule::{:?}------text = {}", pair.as_rule(),pair.as_str());
        // for (i, inner) in pair.clone().into_inner().enumerate() {
        //     //println!("  inner[{}]: Rule::{:?}, text = {}", i, inner.as_rule(), inner.as_str());
        //     println!("  inner[{}]: Rule::{:?} text = {}", i, inner.as_rule(),inner.as_str());
        // }

        let mut inner_iter = pair.into_inner();

        let category = match inner_iter.next().unwrap().as_str() {
            "system" => ComponentCategory::System,
            "process" => ComponentCategory::Process,
            "thread" => ComponentCategory::Thread,
            "processor" => ComponentCategory::Processor,
            "memory" => ComponentCategory::Memory,
            "data" => ComponentCategory::Data,
            s => panic!("Unknown component implementation category: {}", s),
        };

        // Skip "implementation" keyword
        //let _ = inner_iter.next();

        let name_str = extract_identifier(inner_iter.next().unwrap());
        let mut name_parts = name_str.split('.');
        let name = ImplementationName {
            type_identifier: name_parts.next().unwrap().to_string(),
            implementation_identifier: name_parts.next().unwrap_or("").to_string(),
        };

        let mut prototypes = PrototypeClause::None;
        let mut subcomponents = SubcomponentClause::None;
        let mut calls = CallSequenceClause::None;
        let mut connections = ConnectionClause::None;
        let mut properties = PropertyClause::ExplicitNone;
        let mut annexes = Vec::new();

        for inner in inner_iter {
            match inner.as_rule() {
                aadlight_parser::Rule::prototypes => {
                    prototypes = Self::transform_prototypes_clause(inner);
                }
                aadlight_parser::Rule::subcomponents => {
                    subcomponents = Self::transform_subcomponents_clause(inner);
                }
                aadlight_parser::Rule::calls => {
                    calls = Self::transform_calls_clause(inner);
                }
                aadlight_parser::Rule::connections => {
                    connections = Self::transform_connections_clause(inner);
                }
                aadlight_parser::Rule::properties => {
                    properties = Self::transform_properties_clause(inner);
                }
                aadlight_parser::Rule::annex_subclause => {
                    if let Some(annex) = transform_annex_subclause(inner) {
                        annexes.push(annex);
                    }
                }
                _ => {}
            }
        }

        ComponentImplementation {
            category,
            name,
            prototype_bindings: None,
            prototypes,
            subcomponents,
            calls,
            connections,
            properties,
            annexes,
        }
    }

    pub fn transform_subcomponents_clause(pair: Pair<aadlight_parser::Rule>) -> SubcomponentClause {
        // println!("=== Debug subcomponents ===");
        // println!("pair = Rule::{:?}------text = {}", pair.as_rule(),pair.as_str());
        // for (i, inner) in pair.clone().into_inner().enumerate() {
        //     //println!("  inner[{}]: Rule::{:?}, text = {}", i, inner.as_rule(), inner.as_str());
        //     println!("  inner[{}]: Rule::{:?} text = {}", i, inner.as_rule(),inner.as_str());
        // }

        if pair.as_str().contains("none") {
            return SubcomponentClause::Empty;
        }

        let mut subcomponents = Vec::new();
        for inner in pair.into_inner() {
            if inner.as_rule() == aadlight_parser::Rule::subcomponent {
                subcomponents.push(Self::transform_subcomponent(inner));
            }
        }

        if subcomponents.is_empty() {
            SubcomponentClause::None
        } else {
            SubcomponentClause::Items(subcomponents)
        }
    }

    pub fn transform_subcomponent(pair: Pair<aadlight_parser::Rule>) -> Subcomponent {
        let mut inner_iter = pair.into_inner();
        let identifier = extract_identifier(inner_iter.next().unwrap());
        //let _colon = inner_iter.next();

        let category = match inner_iter.next().unwrap().as_str() {
            "system" => ComponentCategory::System,
            "process" => ComponentCategory::Process,
            "thread" => ComponentCategory::Thread,
            "processor" => ComponentCategory::Processor,
            "memory" => ComponentCategory::Memory,
            "data" => ComponentCategory::Data,
            "subprogram" => ComponentCategory::Subprogram,
            "device" => ComponentCategory::Device,
            s => panic!("Unknown subcomponent category: {}", s),
        };

        // If qualified_identifier contains multiple identifiers, keep only the last one
        let qualified_identifier = inner_iter.next().unwrap();
        let name_str = if qualified_identifier.as_str().contains("::") {
            // If it contains the :: separator, keep only the last identifier (e.g., Base_Types::Float -> Float)
            qualified_identifier
                .as_str()
                .split("::")
                .last()
                .unwrap()
                .trim()
                .to_string()
        } else {
            // Otherwise use the original string
            extract_identifier(qualified_identifier)
        };
        let mut name_parts = name_str.split(".");
        let classifier = SubcomponentClassifier::ClassifierReference(
            UniqueComponentClassifierReference::Implementation(UniqueImplementationReference {
                package_prefix: None,
                implementation_name: ImplementationName {
                    type_identifier: name_parts.next().unwrap().to_string(),
                    implementation_identifier: name_parts.next().unwrap_or("").to_string(),
                },
            }),
        );

        Subcomponent {
            identifier,
            category,
            classifier,
            array_spec: None,       // TODO: Handle array spec
            properties: Vec::new(), // TODO: Handle properties
        }
    }

    pub fn transform_calls_clause(pair: Pair<aadlight_parser::Rule>) -> CallSequenceClause {
        if pair.as_str().contains("none") {
            return CallSequenceClause::Empty;
        }

        let mut call_sequences = Vec::new();
        for inner in pair.into_inner() {
            if inner.as_rule() == aadlight_parser::Rule::call_sequence {
                call_sequences.push(Self::transform_call_sequence(inner));
            }
        }

        if call_sequences.is_empty() {
            CallSequenceClause::None
        } else {
            CallSequenceClause::Items(call_sequences)
        }
    }

    pub fn transform_call_sequence(pair: Pair<aadlight_parser::Rule>) -> CallSequence {
        // println!("=== Debug calls_sequence ===");
        // println!("pair = Rule::{:?}------text = {}", pair.as_rule(),pair.as_str());
        // for (i, inner) in pair.clone().into_inner().enumerate() {
        //     //println!("  inner[{}]: Rule::{:?}, text = {}", i, inner.as_rule(), inner.as_str());
        //     println!("  inner[{}]: Rule::{:?} text = {}", i, inner.as_rule(),inner.as_str());
        // }

        let mut inner_iter = pair.into_inner();
        let identifier = extract_identifier(inner_iter.next().unwrap());
        //let _colon = inner_iter.next();
        //let _open_brace = inner_iter.next();

        let mut calls = Vec::new();
        for inner in inner_iter {
            if inner.as_rule() == aadlight_parser::Rule::subprogram_call {
                calls.push(Self::transform_subprogram_call(inner));
            }
        }

        CallSequence {
            identifier,
            calls,
            properties: Vec::new(), // TODO: Handle properties
            in_modes: None,         // TODO: Handle modes
        }
    }

    pub fn transform_subprogram_call(pair: Pair<aadlight_parser::Rule>) -> SubprogramCall {
        // println!("=== Debug subprogram_call ===");
        // println!("pair = Rule::{:?}------text = {}", pair.as_rule(),pair.as_str());
        // for (i, inner) in pair.clone().into_inner().enumerate() {
        //     //println!("  inner[{}]: Rule::{:?}, text = {}", i, inner.as_rule(), inner.as_str());
        //     println!("  inner[{}]: Rule::{:?} text = {}", i, inner.as_rule(),inner.as_str());
        // }

        let mut inner_iter = pair.into_inner();
        let identifier = extract_identifier(inner_iter.next().unwrap());
        //let _colon = inner_iter.next();
        //let _subprogram = inner_iter.next();

        let called = CalledSubprogram::Classifier(
            UniqueComponentClassifierReference::Implementation(UniqueImplementationReference {
                package_prefix: None,
                implementation_name: ImplementationName {
                    type_identifier: extract_identifier(inner_iter.next().unwrap()),
                    implementation_identifier: String::new(),
                },
            }),
        );

        SubprogramCall {
            identifier,
            called,
            properties: Vec::new(), // TODO: Handle properties
        }
    }

    pub fn transform_connections_clause(pair: Pair<aadlight_parser::Rule>) -> ConnectionClause {
        if pair.as_str().contains("none") {
            return ConnectionClause::Empty;
        }

        let mut connections = Vec::new();
        for inner in pair.into_inner() {
            if inner.as_rule() == aadlight_parser::Rule::connection {
                connections.push(Self::transform_connection(inner));
            }
        }

        if connections.is_empty() {
            ConnectionClause::None
        } else {
            ConnectionClause::Items(connections)
        }
    }

    pub fn transform_connection(pair: Pair<aadlight_parser::Rule>) -> Connection {
        // println!("=== Debug connection ===");
        // println!("pair = Rule::{:?}, text = {}", pair.as_rule(), pair.as_str());

        // for (i, inner) in pair.clone().into_inner().enumerate() {
        //     println!("  inner[{}]: Rule::{:?}, text = {}", i, inner.as_rule(), inner.as_str());
        // }

        let mut inner_iter = pair.into_inner();
        let identifier = extract_identifier(inner_iter.next().unwrap());
        //let _colon = inner_iter.next();

        let connection_type = inner_iter.next().unwrap();
        let connection_body = inner_iter.next().unwrap(); // port_connection or parameter_connection

        match connection_type.as_str() {
            "port" => {
                let mut port_iter = connection_body.into_inner();

                let source = Self::transform_port_reference(port_iter.next().unwrap());
                let direction = match port_iter.next().unwrap().as_str() {
                    "->" => ConnectionSymbol::Direct,
                    "<->" => ConnectionSymbol::Didirect,
                    _ => panic!("Unknown connection direction"),
                };
                let destination = Self::transform_port_reference(port_iter.next().unwrap());

                Connection::Port(PortConnection {
                    identifier,
                    source,
                    destination,
                    connection_direction: direction,
                })
            }
            "parameter" => {
                let mut port_iter = connection_body.into_inner();

                let source = Self::transform_parameterport_reference(port_iter.next().unwrap());
                let direction = match port_iter.next().unwrap().as_str() {
                    "->" => ConnectionSymbol::Direct,
                    "<->" => ConnectionSymbol::Didirect,
                    _ => panic!("Unknown connection direction"),
                };
                let destination =
                    Self::transform_parameterport_reference(port_iter.next().unwrap());
                Connection::Parameter(ParameterConnection {
                    source,
                    destination,
                    connection_direction: direction,
                })
            }
            "data access" | "subprogram access" => {
                let mut port_iter = connection_body.into_inner();

                let source = Self::transform_access_reference(port_iter.next().unwrap());
                let direction = match port_iter.next().unwrap().as_str() {
                    "->" => ConnectionSymbol::Direct,
                    "<->" => ConnectionSymbol::Didirect,
                    _ => panic!("Unknown connection direction"),
                };
                let destination = Self::transform_access_reference(port_iter.next().unwrap());

                Connection::Access(AccessConnection {
                    source,
                    destination,
                    connection_direction: direction,
                })
            }

            _ => panic!("Unknown connection type"),
        }
    }

    pub fn transform_port_reference(pair: Pair<aadlight_parser::Rule>) -> PortEndpoint {
        let reference = pair.as_str().trim();
        if reference.contains('.') {
            let mut parts = reference.split('.');
            PortEndpoint::SubcomponentPort {
                subcomponent: parts.next().unwrap().to_string(),
                port: parts.next().unwrap().to_string(),
            }
        } else {
            PortEndpoint::ComponentPort(reference.to_string())
        }
    }

    pub fn transform_parameterport_reference(
        pair: Pair<aadlight_parser::Rule>,
    ) -> ParameterEndpoint {
        let reference = pair.as_str().trim();
        if reference.contains('.') {
            let mut parts = reference.split('.');
            ParameterEndpoint::SubprogramCallParameter {
                call_identifier: parts.next().unwrap().to_string(),
                parameter: parts.next().unwrap().to_string(),
            }
        } else {
            ParameterEndpoint::ComponentParameter {
                parameter: reference.to_string(),
                data_subcomponent: (None),
            }
        }
    }

    pub fn transform_access_reference(pair: Pair<aadlight_parser::Rule>) -> AccessEndpoint {
        let reference = pair.as_str().trim();
        if reference.contains('.') {
            let mut parts = reference.split('.');
            AccessEndpoint::SubcomponentAccess {
                subcomponent: parts.next().unwrap().to_string(),
                access: parts.next().unwrap().to_string(),
            }
        } else {
            AccessEndpoint::ComponentAccess(reference.to_string())
        }
    }
}
