// aadlAST2rustAST
use crate::aadl_ast2rust_code::aadl_property::*;
use aadl_intermediate::*;
use crate::aadl_ast2rust_code::converter_annex::AnnexConverter;

use crate::ast::aadl_ast_cj::*;
use std::collections::HashMap;
use crate::aadl_ast2rust_code::collector;
use crate::aadl_ast2rust_code::types::*;
use crate::aadl_ast2rust_code::implementations::*;

// Converter from AADL to the Rust intermediate representation
pub struct AadlConverter {
    pub type_mappings: HashMap<String, Type>, // initially built from the AADL library file Base_Types.aadl, mapping AADL Data component names to corresponding Rust types; later extended based on model files

    pub component_types: HashMap<String, ComponentType>, // stores component type information (used in some cases to obtain port information from a component implementation based on its type)
    pub annex_converter: AnnexConverter, // Behavior Annex converter
    cpu_scheduling_protocols: HashMap<String, String>, // stores scheduling protocol information for CPU implementations
    pub cpu_name_to_id_mapping: HashMap<String, isize>, // stores the mapping from CPU name to ID
    data_comp_type: HashMap<String, String>, // stores data component type info: key is the data component name, value is the data component kind; used when the data component is a struct/union and properties must be obtained from its impl
    
    pub thread_field_values: HashMap<String, HashMap<String, StruPropertyValue>>,// stores property values for thread-type fields: key is the thread struct name (e.g., fooThread), value maps field name -> property value
    pub thread_field_types: HashMap<String, HashMap<String, Type>>,// stores types for thread-type fields: key is the thread struct name (e.g., fooThread), value maps field name -> type; used as a basis when Shared fields are used as parameters

    // List stores multi-connection relationships between processes within a system; each entry is (component, port)
    pub process_broadcast_send: Vec<(String, String)>,
    // HashMap stores multi-connection relationships between processes within a system; key is (component, port), value is a list of receiving (component, port)
    process_broadcast_receive: HashMap<(String, String), Vec<(String, String)>>,
    // HashMap stores the mapping from a subcomponent's identify to its actual implementation type within a system
    system_subcomponent_identify_to_type: HashMap<String, String>,


    // HashMap stores multi-connection relationships between a process and its threads; key is (port name on process, process name), value is a list of receiving (component, port) on threads
    pub thread_broadcast_receive: HashMap<(String, String), Vec<(String, String)>>,
    // HashMap stores the mapping from a subcomponent (thread) identify to its actual implementation type within a process
    process_subcomponent_identify_to_type: HashMap<String, String>,
}


/// Implement Default for AadlConverter
/// Initializes the default type mappings, including mappings from AADL base types to Rust types
impl Default for AadlConverter {
    fn default() -> Self {
        let mut type_mappings = HashMap::new();
        type_mappings.insert("boolean".to_string(), Type::Named("bool".to_string()));

        type_mappings.insert("integer".to_string(), Type::Named("i32".to_string()));
        type_mappings.insert("integer_8".to_string(), Type::Named("i8".to_string()));
        type_mappings.insert("integer_16".to_string(), Type::Named("i16".to_string()));
        type_mappings.insert("integer_32".to_string(), Type::Named("i32".to_string()));
        type_mappings.insert("integer_64".to_string(), Type::Named("i64".to_string()));
        type_mappings.insert("unsigned_8".to_string(), Type::Named("u8".to_string()));
        type_mappings.insert("unsigned_16".to_string(), Type::Named("u16".to_string()));
        type_mappings.insert("unsigned_32".to_string(), Type::Named("u32".to_string()));
        type_mappings.insert("unsigned_64".to_string(), Type::Named("u64".to_string()));

        type_mappings.insert("natural".to_string(), Type::Named("usize".to_string()));

        type_mappings.insert("float".to_string(), Type::Named("f32".to_string()));
        type_mappings.insert("float_32".to_string(), Type::Named("f32".to_string()));
        type_mappings.insert("float_64".to_string(), Type::Named("f64".to_string()));

        type_mappings.insert("character".to_string(), Type::Named("char".to_string()));

        type_mappings.insert("string".to_string(), Type::Named("String".to_string()));

        Self {
            type_mappings,
            component_types: HashMap::new(),
            annex_converter: AnnexConverter::default(),
            cpu_scheduling_protocols: HashMap::new(),
            cpu_name_to_id_mapping: HashMap::new(),
            data_comp_type: HashMap::new(),
            thread_field_values: HashMap::new(),
            thread_field_types: HashMap::new(),
            process_broadcast_send: Vec::new(),
            process_broadcast_receive: HashMap::new(),
            system_subcomponent_identify_to_type: HashMap::new(),
            thread_broadcast_receive: HashMap::new(),
            process_subcomponent_identify_to_type: HashMap::new(),
        }
    }
}

impl AadlConverter {
    // Infer the Rust type from a property value (used when inferring types for thread property values)
    pub fn type_for_property(&self, value: &StruPropertyValue) -> String {
        match value {
            StruPropertyValue::Boolean(_) => "bool".to_string(),
            StruPropertyValue::Integer(_) => "u64".to_string(),
            StruPropertyValue::Float(_) => "f64".to_string(),
            StruPropertyValue::String(_) => "String".to_string(),
            StruPropertyValue::Duration(_, _) => "u64".to_string(),
            StruPropertyValue::Range(_, _, _) => "(u64, u64)".to_string(),
            StruPropertyValue::None => "None".to_string(),
            StruPropertyValue::Custom(s) => s.to_string(),
        }
    }
    // Main conversion entry
    pub fn convert_package(&mut self, pkg: &Package) -> RustModule {
        // First collect all component type information
        collector::collect_component_types(&mut self.component_types, pkg);

        // Collect multi-connection relationships between processes within a system
        collector::collect_process_connections(&mut self.process_broadcast_send,&mut self.process_broadcast_receive,&mut self.system_subcomponent_identify_to_type,pkg);
        // Collect multi-connection relationships between a process and its threads
        collector::collect_thread_connections(&mut self.thread_broadcast_receive,&mut self.process_subcomponent_identify_to_type,pkg);
        // println!("thread_broadcast_receive: {:?}", self.thread_broadcast_receive);
        // println!("process_subcomponent_identify_to_type: {:?}", self.process_subcomponent_identify_to_type);


        let mut module = RustModule {
            name: pkg.name.0.join("_").to_lowercase(),
            docs: vec![format!(
                "// Auto-generated from AADL package: {}",
                pkg.name.0.join("::")
            )],
            //..Default::default()
            items: Self::runtime_prelude_items(),
            attrs: Default::default(),
            vis: Visibility::Public,
        };
        module.items.extend(self.convert_withs(pkg));

        // Handle public declarations
        if let Some(public_section) = &pkg.public_section {
            for decl in &public_section.declarations {
                self.convert_declaration(decl, &mut module, pkg);
            }
        }

        // Handle private declarations
        if let Some(private_section) = &pkg.private_section {
            for decl in &private_section.declarations {
                self.convert_declaration(decl, &mut module, pkg);
            }
        }

        // Handle mapping between CPU and assigned ID; in the generated Rust code, initialize the <ID, scheduling protocol> mapping
        collector::convert_cpu_schedule_mapping(&mut module, &self.cpu_scheduling_protocols, &self.cpu_name_to_id_mapping);
        collector::add_period_to_priority_function(&mut module, &self.cpu_scheduling_protocols);
        //println!("cpu_scheduling_protocols: {:?}", self.cpu_scheduling_protocols);
        //println!("cpu_name_to_id_mapping: {:?}", self.cpu_name_to_id_mapping);
        module
    }

    fn runtime_prelude_items() -> Vec<Item> {
        vec![
            Item::Raw("#![allow(unused_imports)]".to_string()),
            Item::Raw("#![allow(non_camel_case_types)]".to_string()),
            Item::Raw("#![allow(non_snake_case)]".to_string()),
            Item::Raw("#![allow(unused_assignments)]".to_string()),
            Item::Use(UseStatement {
                path: vec!["crossbeam_channel".to_string()],
                kind: UseKind::Nested(vec!["Receiver".to_string(), "Sender".to_string()]),
            }),
            Item::Use(UseStatement {
                path: vec!["std".to_string(), "sync".to_string()],
                kind: UseKind::Nested(vec!["Arc".to_string(), "Mutex".to_string()]),
            }),
            Item::Use(UseStatement {
                path: vec!["std".to_string(), "thread".to_string()],
                kind: UseKind::Simple,
            }),
            Item::Use(UseStatement {
                path: vec!["std".to_string(), "time".to_string()],
                kind: UseKind::Nested(vec!["Duration".to_string(), "Instant".to_string()]),
            }),
            Item::Use(UseStatement {
                path: vec!["lazy_static".to_string(), "lazy_static".to_string()],
                kind: UseKind::Simple,
            }),
            Item::Use(UseStatement {
                path: vec!["std".to_string(), "collections".to_string(), "HashMap".to_string()],
                kind: UseKind::Simple,
            }),
            Item::Use(UseStatement {
                path: vec!["crate".to_string(), "common_traits".to_string()],
                kind: UseKind::Glob,
            }),
            Item::Use(UseStatement {
                path: vec!["crate".to_string(), "posix".to_string()],
                kind: UseKind::Glob,
            }),
            Item::Use(UseStatement {
                path: vec!["tokio".to_string(), "sync".to_string(), "broadcast".to_string()],
                kind: UseKind::Nested(vec![
                    "self".to_string(),
                    "Sender as BcSender".to_string(),
                    "Receiver as BcReceiver".to_string(),
                ]),
            }),
            Item::Use(UseStatement {
                path: vec!["libc".to_string()],
                kind: UseKind::Nested(vec![
                    "self".to_string(),
                    "syscall".to_string(),
                    "SYS_gettid".to_string(),
                ]),
            }),
            Item::Use(UseStatement {
                path: vec!["rand".to_string(), "Rng".to_string()],
                kind: UseKind::Simple,
            }),
            Item::Use(UseStatement {
                path: vec!["libc".to_string()],
                kind: UseKind::Nested(vec![
                    "pthread_self".to_string(),
                    "sched_param".to_string(),
                    "pthread_setschedparam".to_string(),
                    "SCHED_FIFO".to_string(),
                ]),
            }),
            Item::Raw("include!(concat!(env!(\"OUT_DIR\"), \"/aadl_c_bindings.rs\"));".to_string()),
        ]
    }

    fn convert_withs(&self, pkg: &Package) -> Vec<Item> {
        let mut items = Vec::new();
        for ele in pkg.visibility_decls.iter() {
            if let VisibilityDeclaration::Import { packages, property_sets: _ } = ele {
                        //println!("packages: {:?}", packages);
                        for pkg_name in packages.iter() {
                            // Key point: do not use to_string()
                            // print!("pkg0:{:?}",pkg_name.0.clone());
                            let module_name = pkg_name
                                .0
                                .iter()
                                .map(|s| s.to_ascii_lowercase())
                                .collect::<Vec<_>>()
                                .join("_");

                            items.push(Item::Use(UseStatement {
                                path: vec!["crate".to_string(), module_name],
                                kind: UseKind::Glob,
                            }));
                        }
                    }
        }
        items
    }
    // Get the component type from an implementation
    pub fn get_component_type(&self, impl_: &ComponentImplementation) -> Option<&ComponentType> {
        self.component_types.get(&impl_.name.type_identifier)
    }

    // Get the port direction by port name
    fn get_port_direction(&self, port_name: &str) -> PortDirection {
        // Traverse all component types to find one containing this port
        // TODO: if two components contain ports with the same name but different directions, this will break
        for comp_type in self.component_types.values() {
            if let FeatureClause::Items(features) = &comp_type.features {
                for feature in features {
                    if let Feature::Port(port) = feature {
                        if port.identifier.to_lowercase() == port_name.to_lowercase() {
                            return port.direction;
                        }
                    }
                }
            }
        }
        // If not found, default to Out
        PortDirection::Out
    }

    // Generate an appropriate default value for a type
    pub fn generate_default_value_for_type(&self, port_type: &Type) -> Expr {
        match port_type {
            Type::Named(type_name) => {
                // First check whether it is a native Rust type
                match type_name.as_str() {
                    "bool" => Expr::Literal(Literal::Bool(false)),
                    "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => Expr::Literal(Literal::Int(0)),
                    "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => Expr::Literal(Literal::Int(0)),
                    "f32" | "f64" => Expr::Literal(Literal::Float(0.0)),
                    "char" => Expr::Literal(Literal::Char('\0')),
                    "String" => Expr::Literal(Literal::Str("".to_string())),
                    _ => {
                        // Check whether it is a custom type; look up the corresponding Rust type via type_mappings
                        if let Some(mapped_type) = self.type_mappings.get(&type_name.to_string().to_lowercase()) {
                            // Recursive call using the mapped type
                            self.generate_default_value_for_type(mapped_type)
                        } else {
                            // If no mapping found, fall back to heuristic rules
                            if type_name.to_lowercase().contains("bool") {
                                Expr::Literal(Literal::Bool(false))
                            } else {
                                Expr::Literal(Literal::Int(0)) // default to 0
                            }
                        }
                    }
                }
            }
            _ => Expr::Literal(Literal::Int(0)), // for complex types, default to 0
        }
    }

    fn convert_declaration(&mut self, decl: &AadlDeclaration, module: &mut RustModule, package: &Package) {
        match decl {
            AadlDeclaration::ComponentType(comp) => {
                // Convert a component type declaration into the corresponding Rust struct or type definition
                module.items.extend(self.convert_component(comp, package));
            }
            AadlDeclaration::ComponentImplementation(impl_) => {
                // Convert a component implementation declaration into the corresponding Rust impl blocks
                module.items.extend(self.convert_implementation(impl_, package));
            }
            _ => {} // TODO: ignore other declaration kinds
        }
    }

    fn convert_component(&mut self, comp: &ComponentType, package: &Package) -> Vec<Item> {
        match comp.category {
            ComponentCategory::Data => conv_data_type::convert_data_component(&mut self.type_mappings, comp,&mut self.data_comp_type),
            ComponentCategory::Thread => conv_thread_type::convert_thread_component(self, comp),
            ComponentCategory::Subprogram => conv_subprogram_type::convert_subprogram_component(self,comp, package),
            ComponentCategory::System => conv_system_type::convert_system_component(self, comp),
            ComponentCategory::Process => conv_process_type::convert_process_component(self, comp),
            ComponentCategory::Device => conv_device_type::convert_device_component(self,comp),
            _ => Vec::default(), //TODO: other component categories still need handling
        }
    }

    fn convert_implementation(&mut self, impl_: &ComponentImplementation, package: &Package) -> Vec<Item> {
        match impl_.category {
            ComponentCategory::Process => conv_process_impl::convert_process_implementation(self,impl_),
            ComponentCategory::Thread => conv_thread_impl::convert_thread_implemenation(self,impl_),
            ComponentCategory::System => conv_system_impl::convert_system_implementation(self,impl_),
            ComponentCategory::Data => conv_data_impl::convert_data_implementation(&self.type_mappings,&self.data_comp_type,impl_,package),
            ComponentCategory::Processor => conv_processor_impl::convert_processor_implementation(&mut self.cpu_scheduling_protocols,impl_),
            _ => Vec::default(), // default implementation
        }
    }

    pub fn convert_type_features(&self, features: &FeatureClause, comp_identifier: String) -> Vec<Field> {
        let mut fields = Vec::new();

        if let FeatureClause::Items(feature_items) = features {
            for feature in feature_items {
                match feature {
                    Feature::Port(port) => {
                        fields.push(Field {
                            name: port.identifier.to_lowercase(),
                            ty: self.convert_port_type(port,comp_identifier.clone()),
                            docs: vec![format!("// Port: {} {:?}", port.identifier, port.direction)],
                            attrs: Vec::new(),
                        });
                    }
                    Feature::SubcomponentAccess(sub_access) => {
                        // Handle "requires data access" features
                        if let SubcomponentAccessSpec::Data(data_access) = sub_access {
                            if data_access.direction == AccessDirection::Requires {
                                // Generate field: pub GNC_POS : PosShared,
                                let field_name = data_access.identifier.to_lowercase();
                                
                                // Extract the component name from the classifier to generate the PosShared type
                                if let Some(classifier) = &data_access.classifier {
                                    if let DataAccessReference::Classifier(unique_ref) = classifier {
                                        let shared_type_name = match unique_ref {
                                            UniqueComponentClassifierReference::Implementation(impl_ref) => {
                                                // From POS.Impl generate pos_shared
                                                let base_name = &impl_ref.implementation_name.type_identifier;
                                                if base_name.ends_with(".Impl") {
                                                    let prefix = &base_name[..base_name.len() - 5]; // remove the ".Impl" suffix
                                                    format!("{}Shared", prefix)
                                                } else {
                                                    // If there is no Impl suffix, handle directly
                                                    format!("{}Shared", base_name)
                                                }
                                            }
                                            UniqueComponentClassifierReference::Type(type_ref) => {
                                                // From POS generate pos_shared
                                                let base_name = &type_ref.implementation_name.type_identifier;
                                                format!("{}Shared", base_name)
                                            }
                                        };
                                        
                                        fields.push(Field {
                                            name: field_name,
                                            ty: Type::Named(shared_type_name),
                                            docs: vec![format!("// AADL feature: {} : requires data access {}", 
                                                data_access.identifier, 
                                                match classifier {
                                                    DataAccessReference::Classifier(UniqueComponentClassifierReference::Implementation(impl_ref)) => 
                                                        impl_ref.implementation_name.type_identifier.clone(),
                                                    DataAccessReference::Classifier(UniqueComponentClassifierReference::Type(type_ref)) => 
                                                        type_ref.implementation_name.type_identifier.clone(),
                                                    _ => "Unknown".to_string(),
                                                }
                                            )],
                                            attrs: Vec::new(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        fields
    }

    pub fn convert_port_type(&self, port: &PortSpec, comp_identifier: String) -> Type {
        // Determine the channel type (Sender/Receiver)
        let mut channel_type = String::new();
        // If comp_identifier is non-empty, first check whether port.identifier appears in keys/values of process_broadcast_receive
        // If so, find its corresponding subcomponent identify in system_subcomponent_identify_to_type, and compare it with comp_identifier
        // If they match, this port is a broadcast port, so the channel type should be BcReceiver or BcSender; otherwise use Receiver or Sender
        if !comp_identifier.is_empty() {
            for (subcomponent_port, vercport) in &self.process_broadcast_receive {
                // First check whether the key (sender) contains port.identifier
                
                if subcomponent_port.1.eq(&port.identifier) {
                    if let Some(subcomponent_identify) = self.system_subcomponent_identify_to_type.get(&subcomponent_port.0.clone()) {
                        if subcomponent_identify.eq(&comp_identifier) {
                            channel_type = match port.direction {
                                PortDirection::Out => "BcSender".to_string(),
                                _ => panic!("error, In port is not allowed in broadcast send ports"),
                            };
                            continue;
                        }
                    }
                };
                // Then check whether the value (receiver) contains port.identifier
                for (comp, port_identifier) in vercport {
                    if port_identifier.eq(&port.identifier) {
                        if let Some(subcomponent_identify) = self.system_subcomponent_identify_to_type.get(&comp.clone()) {
                            if subcomponent_identify.eq(&comp_identifier) {
                                channel_type = match port.direction {
                                        PortDirection::In => "BcReceiver".to_string(),
                                        _ => panic!("error, Out port is not allowed in broadcast receive ports"),
                                };
                            }
                        };
                        
                    }
                }
            }

            // Handle the case where thread ports inside a process are broadcast types
            // This can only appear in the values of thread_broadcast_receive
            for vercport in self.thread_broadcast_receive.values() {
                for (comp, port_identifier) in vercport {
                    if port_identifier.eq(&port.identifier) {
                        if let Some(subcomponent_identify) = self.process_subcomponent_identify_to_type.get(&comp.clone()) {
                            if subcomponent_identify.eq(&comp_identifier) {
                                channel_type = match port.direction {
                                    PortDirection::In => "BcReceiver".to_string(),
                                    _ => panic!("error, Out port is not allowed in broadcast receive ports"),
                                };
                                continue;
                            }
                        }
                    }
                }
            }
        }
        if channel_type.is_empty() {
            channel_type = match port.direction {
                PortDirection::In => "Receiver".to_string(),
                PortDirection::Out => "Sender".to_string(),
                PortDirection::InOut => "Sender".to_string(), //TODO: bidirectional channels are not supported; keep as-is for now
            };
        }

        // Determine the inner data type
        let inner_type = match &port.port_type {
            PortType::Data { classifier } | PortType::EventData { classifier } => {
                classifier
                    .as_ref() //.as_ref() converts Option<T> to Option<&T>; it does not take ownership but borrows the inner value
                    .map(|c: &PortDataTypeReference| self.classifier_to_type(c)) // Apply a function to the wrapped value c inside Some(...) using .map() on Option
                    .unwrap_or(Type::Named("()".to_string()))
            }
            PortType::Event => Type::Named("()".to_string()), // TODO: event ports always use the unit type
        };

        // Compose the final type
        //Type::Generic(channel_type.to_string(), vec![inner_type])
        Type::Generic(
            "Option".to_string(),
            vec![Type::Generic(channel_type.to_string(), vec![inner_type])],
        )
    }

    pub fn classifier_to_type(&self, classifier: &PortDataTypeReference) -> Type {
        
        //println!("classifier: {:?}", classifier);
        //println!("-------------------------------");
        match classifier {
            PortDataTypeReference::Classifier(UniqueComponentClassifierReference::Type(
                ref type_ref,
            )) => {
                // Prefer our custom type mapping rules
                // println!("cjcjcjcj:{:?}",self.type_mappings);
                self.type_mappings
                    .get(&type_ref.implementation_name.type_identifier.to_lowercase())
                    .cloned()
                    .unwrap_or_else(|| {
                        //println!("Using named type for: {}", type_ref.implementation_name.type_identifier);
                        Type::Named(type_ref.implementation_name.type_identifier.clone())
                    })
            }
            _ => {  println!("Unsupported classifier type: {:?}", classifier);
                Type::Named("()".to_string())}
        }
    }

    // Convert AADL properties into a Property list
    pub fn convert_properties(&self, comp: ComponentRef<'_>) -> Vec<StruProperty> {
        let mut result = Vec::new();

        // Obtain properties via pattern matching
        let properties = match comp {
            ComponentRef::Type(component_type) => &component_type.properties,
            ComponentRef::Impl(component_impl) => &component_impl.properties,
        };

        // Existing processing logic
        if let PropertyClause::Properties(props) = properties {
            for prop in props {
                if let Some(converted) = self.convert_single_property(prop) {
                    result.push(converted);
                }
            }
        }
        result
        // properties
    }
    // Convert a single property
    fn convert_single_property(&self, prop: &Property) -> Option<StruProperty> {
        let Property::BasicProperty(bp) = prop else {
            return None; // skip non-basic properties
        };

        let docs = vec![format!("// AADL property: {}", bp.identifier.name)];

        Some(StruProperty {
            name: bp.identifier.name.clone(),
            value: self.parse_property_value(&bp.value)?,
            docs,
        })
    }

    // Parse an AADL property value into a Rust value type
    pub fn parse_property_value(&self, value: &PropertyValue) -> Option<StruPropertyValue> {
        match value {
            PropertyValue::Single(expr) => self.parse_property_expression(expr),
            _ => None, // ignore other complex property forms
        }
    }

    // Parse a property expression into StruPropertyValue
    fn parse_property_expression(&self, expr: &PropertyExpression) -> Option<StruPropertyValue> {
        match expr {
            // Basic types
            PropertyExpression::Boolean(boolean_term) => self.parse_boolean_term(boolean_term),
            PropertyExpression::Real(real_term) => self.parse_real_term(real_term),
            PropertyExpression::Integer(integer_term) => self.parse_integer_term(integer_term),
            PropertyExpression::String(string_term) => self.parse_string_term(string_term),

            // Range type
            PropertyExpression::IntegerRange(range_term) => Some(StruPropertyValue::Range(
                range_term.lower.value.parse().ok()?,
                range_term.upper.value.parse().ok()?,
                range_term.lower.unit.clone(),
            )),

            // Other complex types are not handled yet
            _ => None,
        }
    }

    // Boolean term parsing
    fn parse_boolean_term(&self, term: &BooleanTerm) -> Option<StruPropertyValue> {
        match term {
            BooleanTerm::Literal(b) => Some(StruPropertyValue::Boolean(*b)),
            BooleanTerm::Constant(_) => None, // constants require table lookup; simplified here
        }
    }

    // Real term parsing
    fn parse_real_term(&self, term: &SignedRealOrConstant) -> Option<StruPropertyValue> {
        match term {
            SignedRealOrConstant::Real(signed_real) => {
                let value = signed_real.sign.as_ref().map_or(1.0, |s| match s {
                    Sign::Plus => 1.0,
                    Sign::Minus => -1.0,
                }) * signed_real.value;
                Some(StruPropertyValue::Float(value))
            }
            SignedRealOrConstant::Constant { .. } => None, // TODO: constants require table lookup
        }
    }

    // Integer term parsing
    fn parse_integer_term(&self, term: &SignedIntergerOrConstant) -> Option<StruPropertyValue> {
        match term {
            SignedIntergerOrConstant::Real(signed_int) => {
                let value = signed_int.sign.as_ref().map_or(1, |s| match s {
                    Sign::Plus => 1,
                    Sign::Minus => -1,
                }) * signed_int.value;
                Some(StruPropertyValue::Integer(value))
            }
            SignedIntergerOrConstant::Constant { .. } => None, // constants require table lookup
        }
    }

    // String term parsing
    fn parse_string_term(&self, term: &StringTerm) -> Option<StruPropertyValue> {
        match term {
            StringTerm::Literal(s) => Some(StruPropertyValue::String(s.clone())),
            StringTerm::Constant(_) => None, // constants require table lookup
        }
    }


    fn some_expr(value: Expr) -> Expr {
        Expr::Call(
            Box::new(Expr::Path(vec!["Some".to_string()], PathType::Member)),
            vec![value],
        )
    }

    fn assign_statement(target: String, value: Expr) -> Statement {
        Statement::Expr(Expr::Assign(
            Box::new(Expr::Ident(target)),
            Box::new(value),
        ))
    }

    pub fn create_channel_connection(&self, conn: &PortConnection, comp_name: String) -> Vec<Statement> {
        let mut stmts = Vec::new();

        // Define a flag indicating whether a channel has been created
        let mut is_channel_created = false;

        // Create an appropriate channel depending on whether the connection is broadcast.
        // Currently this check only exists for connections between processes in a system.
        let mut is_broadcast = false;
        if let PortEndpoint::SubcomponentPort { subcomponent, port } = &conn.source {
            if self.process_broadcast_send.contains(&(subcomponent.clone(), port.clone())) {
                // Broadcast channels use tokio::sync::broadcast::channel::<>.
                is_broadcast = true;
                stmts.push(Statement::Let(LetStmt {
                    ifmut: false,
                    name: "channel".to_string(),
                    ty: None,
                    init: Some(Expr::Call(
                        Box::new(Expr::Path(vec!["broadcast".to_string(), "channel".to_string(), "<>".to_string()], PathType::Namespace)),
                        vec![Expr::Literal(Literal::Int(100))],
                    )),
                }));
                is_channel_created = true;
            }
        } else if let PortEndpoint::ComponentPort (proc_port) = &conn.source {
            if self.thread_broadcast_receive.contains_key(&(proc_port.clone(), comp_name.clone())){
                is_broadcast = true;
                stmts.push(Statement::Let(LetStmt {
                    ifmut: false,
                    name: "channel".to_string(),
                    ty: None,
                    init: Some(Expr::Call(
                        Box::new(Expr::Path(vec!["broadcast".to_string(), "channel".to_string(), "<>".to_string()], PathType::Namespace)),
                        vec![Expr::Literal(Literal::Int(100))],
                    )),
                }));
                is_channel_created = true;
            }
        }

        if !is_channel_created {
            // Non-broadcast channels use crossbeam_channel::unbounded.
            stmts.push(Statement::Let(LetStmt {
                ifmut: false,
                name: conn.identifier.clone(),
                ty: None, // channel type is inferred by the compiler
                init: Some(Expr::Call(
                    Box::new(Expr::Path(
                        vec!["crossbeam_channel".to_string(), "unbounded".to_string()],
                        PathType::Namespace,
                    )),
                    Vec::new(),
                )),
            }));
        }

        // Handle source and destination endpoints
        match (&conn.source, &conn.destination) {
            (
                PortEndpoint::SubcomponentPort {
                    subcomponent: src_comp,
                    port: src_port,
                },
                PortEndpoint::SubcomponentPort {
                    subcomponent: dst_comp,
                    port: dst_port,
                },
            ) => {
                let sender_value = if is_broadcast {
                    Expr::MethodCall(
                        Box::new(Expr::Ident("channel.0".to_string())),
                        "clone".to_string(),
                        Vec::new(),
                    )
                } else {
                    Expr::Ident(format!("{}.0", conn.identifier.clone()))
                };
                stmts.push(Self::assign_statement(
                    format!("{}.{}", src_comp.to_lowercase(), src_port.to_lowercase()),
                    Self::some_expr(sender_value),
                ));

                // Assign receiver side
                // Decide whether this is a broadcast port: if yes, skip for now; if no, generate channel.1
                if !is_broadcast {
                    stmts.push(Self::assign_statement(
                        format!("{}.{}", dst_comp.to_lowercase(), dst_port.to_lowercase()),
                        Self::some_expr(Expr::Ident(format!("{}.1", conn.identifier.clone()))),
                    ));
                }
                
            }
            (
                PortEndpoint::ComponentPort(port_name),
                PortEndpoint::SubcomponentPort {
                    subcomponent: dst_comp,
                    port: dst_port,
                },
            ) => {
                // Handle connections from a component port to a subcomponent port
                // Determine the internal port name based on port direction
                let internal_port_name = match self.get_port_direction(port_name) {
                    PortDirection::In => format!("{}Send", port_name.to_lowercase()),
                    PortDirection::Out => format!("{}Send", port_name.to_lowercase()), // output ports generate Send
                    PortDirection::InOut => format!("{}Send", port_name.to_lowercase()), // InOut is treated as In for now
                };
                
                // Assign directly to the internal port variable
                if is_broadcast {
                    stmts.push(Self::assign_statement(
                        internal_port_name,
                        Self::some_expr(Expr::MethodCall(
                            Box::new(Expr::Ident("channel.0".to_string())),
                            "clone".to_string(),
                            Vec::new(),
                        )),
                    ));
                } else {
                    stmts.push(Self::assign_statement(
                        internal_port_name,
                        Self::some_expr(Expr::Ident(format!("{}.0", conn.identifier.clone()))),
                    ));
                }
                
                if !is_broadcast {
                    stmts.push(Self::assign_statement(
                        format!("{}.{}", dst_comp, dst_port),
                        Self::some_expr(Expr::Ident(format!("{}.1", conn.identifier.clone()))),
                    ));
                }
            }
            (
                PortEndpoint::SubcomponentPort {
                    subcomponent: src_comp,
                    port: src_port,
                },
                PortEndpoint::ComponentPort(port_name),
            ) => {
                // Handle connections from a subcomponent port to a component port (e.g., th_c.evenement -> evenement)
                stmts.push(Self::assign_statement(
                    format!("{}.{}", src_comp, src_port),
                    Self::some_expr(Expr::Ident(format!("{}.0", conn.identifier.clone()))),
                ));

                // Receiver side to the internal port
                // It seems this assignment is unnecessary: it must be Rece
                // And get_port_direction() has a bug
                // let internal_port_name = match self.get_port_direction(port_name) {
                //     PortDirection::In => {println!("In port: {}", port_name);format!("{}Send", port_name.to_lowercase())},
                //     PortDirection::Out => {println!("Out port: {}", port_name); format!("{}Rece", port_name.to_lowercase())}, // output ports generate Send
                //     PortDirection::InOut => {println!("InOut port: {}", port_name);format!("{}Send", port_name.to_lowercase())}, // InOut is treated as In for now
                // };
                // Directly change to the following
                let internal_port_name = format!("{}Rece", port_name.to_lowercase());
                
                // Assign directly to the internal port variable
                stmts.push(Self::assign_statement(
                    internal_port_name,
                    Self::some_expr(Expr::Ident(format!("{}.1", conn.identifier.clone()))),
                ));
            }
            // Additional endpoint combinations can be added here
            _ => {
                // For unsupported connection types, generate a TODO comment
                stmts.push(Statement::Expr(Expr::Ident(format!(
                    "// TODO: Unsupported connection type: {:?} -> {:?}",
                    conn.source, conn.destination
                ))));
            }
        }

        // If is_broadcast is true, handle all subscriber subscriptions here in one place:
        // according to process_broadcast_receive, subscribe each receiver with channel.0.subscribe()
        if is_broadcast {
            if let PortEndpoint::SubcomponentPort { subcomponent, port } = &conn.source {
                if let Some(vercport) = self.process_broadcast_receive.get(&(subcomponent.clone(), port.clone())) {
                    for (subcomponent, port) in vercport {
                        stmts.push(Self::assign_statement(
                            format!("{}.{}", subcomponent, port),
                            Self::some_expr(Expr::MethodCall(
                                Box::new(Expr::Ident("channel.0".to_string())),
                                "subscribe".to_string(),
                                Vec::new(),
                            )),
                        ));
                    }
                }
            }
            if let PortEndpoint::ComponentPort (proc_port) = &conn.source {
                if self.thread_broadcast_receive.contains_key(&(proc_port.clone(), comp_name.clone())){
                    if let Some(vercport) = self.thread_broadcast_receive.get(&(proc_port.clone(), comp_name.clone())) {
                        for (subcomponent, port) in vercport {
                            stmts.push(Self::assign_statement(
                                format!("{}.{}", subcomponent, port),
                                Self::some_expr(Expr::MethodCall(
                                    Box::new(Expr::Ident("channel.0".to_string())),
                                    "subscribe".to_string(),
                                    Vec::new(),
                                )),
                            ));
                        }
                    }
                }
            }
            
            
        }
        
        stmts
    }

    pub fn create_component_type_docs(&self, comp: &ComponentType) -> Vec<String> {
        let docs = vec![format!(
            "// AADL {:?}: {}",
            comp.category,
            comp.identifier.to_lowercase()
        )];

        docs
    }

    pub fn create_component_impl_docs(&self, impl_: &ComponentImplementation) -> Vec<String> {
        let docs = vec![format!(
            "// AADL {:?}: {}",
            impl_.category,
            impl_.name.type_identifier.to_lowercase()
        )];

        docs
    }

    // TODO: due to parameter connections in subprogram features; currently still using port connections (parameter connection form is not defined in aadl_ast), so the parameter connection type is hard-coded here
    pub fn convert_paramport_type(&self, port: &PortSpec) -> Type {
        // Extract classifier type directly without any wrapping
        match &port.port_type {
            PortType::Data { classifier } | PortType::EventData { classifier } => {
                classifier
                    .as_ref()
                    .map(|c| self.classifier_to_type(c))
                    .unwrap_or_else(|| {
                        // Default type handling; adjust as needed
                        match port.direction {
                            PortDirection::Out => Type::Named("i32".to_string()),
                            _ => Type::Named("(error)".to_string()),
                        }
                    })
            }
            PortType::Event => Type::Named("()".to_string()),
            // Other kinds do not need handling since this function is only called for parameter connections
        }
    }

    

}
