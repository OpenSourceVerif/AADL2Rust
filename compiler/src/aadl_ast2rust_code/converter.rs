// aadlAST2rustAST
use crate::aadl_ast2rust_code::intermediate_ast::*;
use crate::aadl_ast2rust_code::converter_annex::AnnexConverter;

use crate::ast::aadl_ast_cj::*;
use std::collections::HashMap;
use crate::aadl_ast2rust_code::collector;
use crate::aadl_ast2rust_code::types::*;
use crate::aadl_ast2rust_code::implementations::*;

// AADL到Rust中间表示的转换器
pub struct AadlConverter {
    pub type_mappings: HashMap<String, Type>, //初始是根据AADL库文件Base_Types.aadl，将AADL Data组件名称映射到对应的Rust类型，后续会根据AADL模型文件，添加新的映射关系

    pub component_types: HashMap<String, ComponentType>, // 存储组件类型信息，（为了有些情况下，需要在组件实现中，根据组件类型来获取端口信息）
    pub annex_converter: AnnexConverter, // Behavior Annex 转换器
    cpu_scheduling_protocols: HashMap<String, String>, // 存储CPU实现的调度协议信息
    pub cpu_name_to_id_mapping: HashMap<String, isize>, // 存储CPU名称到ID的映射关系
    data_comp_type: HashMap<String, String>, // 存储数据组件类型信息，key是数据组件名称，value是数据组件类型。是为了处理数据组件类型为结构体、联合体时，需要根据组件实现impl来获取属性信息
    
    pub thread_field_values: HashMap<String, HashMap<String, StruPropertyValue>>,// 存储线程类型字段对应的属性值，key为线程结构体名(如 fooThread)，value为字段名到属性值的映射
    pub thread_field_types: HashMap<String, HashMap<String, Type>>,// 存储线程类型字段对应的类型，key为线程结构体名(如 fooThread)，value为字段名到类型的映射。为了Shared的字段作为参数的依据

    //List列表存储system内process之间的多连接关系，每条数据是组件和端口
    pub process_broadcast_send: Vec<(String, String)>,
    //HashMap存储system内process之间的多连接关系，key为组件和端口，value为接收(组件和端口)列表
    process_broadcast_receive: HashMap<(String, String), Vec<(String, String)>>,
    //HashMap存储system内subcomponent的identify和真实实现类型的映射关系
    system_subcomponent_identify_to_type: HashMap<String, String>,


    //HashMap存储process内和thread之间的多连接关系，key为process上的(端口名,process名称），value为接收线程上的(组件和端口)列表
    pub thread_broadcast_receive: HashMap<(String, String), Vec<(String, String)>>,
    //HashMap存储process内subcomponent(thread)的identify和真实实现类型的映射关系
    process_subcomponent_identify_to_type: HashMap<String, String>,
}


/// 为AadlConverter实现Default trait
/// 初始化默认的类型映射关系，包括AADL基础类型到Rust类型的映射
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
    // 根据属性值推断Rust类型（使用在为thread的属性值推断类型）
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
    // 主转换入口
    pub fn convert_package(&mut self, pkg: &Package) -> RustModule {
        // 首先收集所有组件类型信息
        collector::collect_component_types(&mut self.component_types, pkg);

        //收集system内process之间的多连接关系
        collector::collect_process_connections(&mut self.process_broadcast_send,&mut self.process_broadcast_receive,&mut self.system_subcomponent_identify_to_type,pkg);
        //收集process内和thread之间的多连接关系
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
            items: Default::default(),
            attrs: Default::default(),
            vis: Visibility::Public,
            withs: self.convert_withs(pkg),
        };

        // 处理公共声明
        if let Some(public_section) = &pkg.public_section {
            for decl in &public_section.declarations {
                self.convert_declaration(decl, &mut module, pkg);
            }
        }

        // 处理私有声明
        if let Some(private_section) = &pkg.private_section {
            for decl in &private_section.declarations {
                self.convert_declaration(decl, &mut module, pkg);
            }
        }

        //处理CPU和分配ID的映射关系，生成的Rust代码中，初始化<ID,调度协议>的映射关系
        collector::convert_cpu_schedule_mapping(&mut module, &self.cpu_scheduling_protocols, &self.cpu_name_to_id_mapping);
        collector::add_period_to_priority_function(&mut module, &self.cpu_scheduling_protocols);
        //println!("cpu_scheduling_protocols: {:?}", self.cpu_scheduling_protocols);
        //println!("cpu_name_to_id_mapping: {:?}", self.cpu_name_to_id_mapping);
        module
    }

    fn convert_withs(&self, pkg: &Package) -> Vec<RustWith> {
        let mut withs = Vec::new();
        for ele in pkg.visibility_decls.iter() {
            if let VisibilityDeclaration::Import { packages, property_sets: _ } = ele {
                        //println!("packages: {:?}", packages);
                        //withs.push(RustWith { path: packages.iter().map(|p| p.to_string()).collect(), glob: true });
                        for pkg_name in packages.iter() {
                            // 关键点：不使用 to_string()
                             print!("pkg0:{:?}",pkg_name.0.clone());
                            let segments = pkg_name.0.iter().map(|s| s.to_ascii_lowercase()).collect();
            
                            withs.push(RustWith {
                                path: segments,
                                glob: true,
                            });
                        }
                    }
        }
        //println!("withs: {:?}", withs);
        withs
    }
    // 根据实现获取组件类型
    pub fn get_component_type(&self, impl_: &ComponentImplementation) -> Option<&ComponentType> {
        self.component_types.get(&impl_.name.type_identifier)
    }

    // 根据端口名称获取端口方向
    fn get_port_direction(&self, port_name: &str) -> PortDirection {
        // 遍历所有组件类型，查找包含该端口的组件
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
        // 如果找不到，默认返回 Out
        PortDirection::Out
    }

    // 根据类型生成合适的默认值
    pub fn generate_default_value_for_type(&self, port_type: &Type) -> Expr {
        match port_type {
            Type::Named(type_name) => {
                // 首先检查是否是Rust原生类型
                match type_name.as_str() {
                    "bool" => Expr::Literal(Literal::Bool(false)),
                    "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => Expr::Literal(Literal::Int(0)),
                    "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => Expr::Literal(Literal::Int(0)),
                    "f32" | "f64" => Expr::Literal(Literal::Float(0.0)),
                    "char" => Expr::Literal(Literal::Char('\0')),
                    "String" => Expr::Literal(Literal::Str("".to_string())),
                    _ => {
                        // 检查是否是自定义类型，通过type_mappings查找对应的Rust类型
                        if let Some(mapped_type) = self.type_mappings.get(&type_name.to_string().to_lowercase()) {
                            // 递归调用，使用映射后的类型
                            self.generate_default_value_for_type(mapped_type)
                        } else {
                            // 如果没有找到映射，使用启发式规则
                            if type_name.to_lowercase().contains("bool") {
                                Expr::Literal(Literal::Bool(false))
                            } else {
                                Expr::Literal(Literal::Int(0)) // 默认使用0
                            }
                        }
                    }
                }
            }
            _ => Expr::Literal(Literal::Int(0)), // 对于复杂类型，默认使用0
        }
    }

    fn convert_declaration(&mut self, decl: &AadlDeclaration, module: &mut RustModule, package: &Package) {
        match decl {
            AadlDeclaration::ComponentType(comp) => {
                // 转换组件类型声明，生成对应的Rust结构体或类型定义
                module.items.extend(self.convert_component(comp, package));
            }
            AadlDeclaration::ComponentImplementation(impl_) => {
                // 转换组件实现声明，生成对应的Rust实现块
                module.items.extend(self.convert_implementation(impl_, package));
            }
            _ => {} // TODO:忽略其他声明类型
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
            _ => Vec::default(), //TODO:其他组件类型还需要处理
        }
    }

    fn convert_implementation(&mut self, impl_: &ComponentImplementation, package: &Package) -> Vec<Item> {
        match impl_.category {
            ComponentCategory::Process => conv_process_impl::convert_process_implementation(self,impl_),
            ComponentCategory::Thread => conv_thread_impl::convert_thread_implemenation(self,impl_),
            ComponentCategory::System => conv_system_impl::convert_system_implementation(self,impl_),
            ComponentCategory::Data => conv_data_impl::convert_data_implementation(&self.type_mappings,&self.data_comp_type,impl_,package),
            ComponentCategory::Processor => conv_processor_impl::convert_processor_implementation(&mut self.cpu_scheduling_protocols,impl_),
            _ => Vec::default(), // 默认实现
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
                        // 处理 requires data access 特征
                        if let SubcomponentAccessSpec::Data(data_access) = sub_access {
                            if data_access.direction == AccessDirection::Requires {
                                // 生成字段：pub GNC_POS : PosShared,
                                let field_name = data_access.identifier.to_lowercase();
                                
                                // 从分类器中提取组件名称，用于生成PosShared类型
                                if let Some(classifier) = &data_access.classifier {
                                    if let DataAccessReference::Classifier(unique_ref) = classifier {
                                        let shared_type_name = match unique_ref {
                                            UniqueComponentClassifierReference::Implementation(impl_ref) => {
                                                // 从 POS.Impl 生成 pos_shared
                                                let base_name = &impl_ref.implementation_name.type_identifier;
                                                if base_name.ends_with(".Impl") {
                                                    let prefix = &base_name[..base_name.len() - 5]; // 去掉".Impl"后缀
                                                    format!("{}Shared", prefix)
                                                } else {
                                                    // 如果没有Impl后缀，直接处理
                                                    format!("{}Shared", base_name)
                                                }
                                            }
                                            UniqueComponentClassifierReference::Type(type_ref) => {
                                                // 从 POS 生成 pos_shared
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
                    Feature::FeatureGroup(fg) => {
                        // 获取 Feature Group 的类型名称
                        let fg_type_name = match &fg.classifier {
                            Some(FeatureGroupReference::Classifier(unique_ref)) => {
                                match unique_ref {
                                    UniqueComponentClassifierReference::Type(t) => t.implementation_name.type_identifier.clone(),
                                    UniqueComponentClassifierReference::Implementation(i) => {
                                        i.implementation_name.type_identifier.clone() 
                                    }
                                }
                            },
                            Some(FeatureGroupReference::Prototype(p)) => p.clone(),
                            None => "UnspecifiedFeatureGroup".to_string(),
                        };

                        // 生成字段: pub feature_name: FeatureGroupType
                        fields.push(Field {
                            name: fg.identifier.to_lowercase(),
                            ty: Type::Named(fg_type_name.clone()),
                            docs: vec![format!("// Feature Group: {}", fg.identifier)],
                            attrs: Vec::new(),
                        });
                    }

                    Feature::Parameter(param) => {
                        // 获取参数的数据类型
                        let param_type = match &param.classifier {
                            Some(PortDataTypeReference::Classifier(unique_ref)) => {
                                match unique_ref {
                                    UniqueComponentClassifierReference::Type(t) => Type::Named(t.implementation_name.type_identifier.clone()),
                                    UniqueComponentClassifierReference::Implementation(i) => Type::Named(i.implementation_name.type_identifier.clone()),
                                }
                            },
                            Some(PortDataTypeReference::Prototype(p)) => Type::Named(p.clone()),
                            None => Type::Named("()".to_string()), // 默认 Unit 类型
                        };

                        // 生成字段: pub param_name: ParamType
                        fields.push(Field {
                            name: param.identifier.to_lowercase(),
                            ty: param_type,
                            docs: vec![format!("// Parameter: {} ({:?})", param.identifier, param.direction)],
                            attrs: Vec::new(),
                        });
                    }
                }
            }
        }

        fields
    }

    pub fn convert_port_type(&self, port: &PortSpec, comp_identifier: String) -> Type {
        // 确定通道类型（Sender/Receiver）
        let mut channel_type = String::new();
        //如果comp_identifier不为空，首先需要查看process_broadcast_receive中的键值对中的键或值中是否包含port.identifier
        //如果有，则根据该项的键，在system_subcomponent_identify_to_type中找到它对应的组件identify值，判断该值和comp_identifier是否相同
        //如果相同，则说明该端口是广播端口，通道类型需要设置为BcReceiver或BcSender，否则设置为Receiver或Sender
        if !comp_identifier.is_empty() {
            for (subcomponent_port, vercport) in &self.process_broadcast_receive {
                //先判断键（发送）是否包含port.identifier
                
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
                //再判断值（接收）是否包含port.identifier
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

            //针对process中线程端口是广播类型的情况进行处理
            //只可能在thread_broadcast_receive的值中存在
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
                PortDirection::InOut => "Sender".to_string(), //TODO:不支持双向通道，暂时这样写
            };
        }

        // 确定内部数据类型
        let inner_type = match &port.port_type {
            PortType::Data { classifier } | PortType::EventData { classifier } => {
                classifier
                    .as_ref() //.as_ref() 的作用是把 Option<T> 变成 Option<&T>。它不会取得其中值的所有权，而只是"借用"里面的值。
                    .map(|c: &PortDataTypeReference| self.classifier_to_type(c)) //对 Option 类型调用 .map() 方法，用于在 Some(...) 中包裹的值c上应用一个函数。
                    .unwrap_or(Type::Named("()".to_string()))
            }
            PortType::Event => Type::Named("()".to_string()), // TODO:事件端口固定使用单元类型
        };

        // 组合成最终类型
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
                // 优先查找我们所自定义类型映射规则
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

    // 转换AADL属性为Property列表
    pub fn convert_properties(&self, comp: ComponentRef<'_>) -> Vec<StruProperty> {
        let mut result = Vec::new();

        // 通过模式匹配获取属性
        let properties = match comp {
            ComponentRef::Type(component_type) => &component_type.properties,
            ComponentRef::Impl(component_impl) => &component_impl.properties,
        };

        // 原有处理逻辑
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
    // 转换单个属性
    fn convert_single_property(&self, prop: &Property) -> Option<StruProperty> {
        let Property::BasicProperty(bp) = prop else {
            return None; // 跳过非基础属性
        };

        let docs = vec![format!("// AADL属性: {}", bp.identifier.name)];

        Some(StruProperty {
            name: bp.identifier.name.clone(),
            value: self.parse_property_value(&bp.value)?,
            docs,
        })
    }

    // 解析AADL属性值到Rust类型
    pub fn parse_property_value(&self, value: &PropertyValue) -> Option<StruPropertyValue> {
        match value {
            PropertyValue::Single(expr) => self.parse_property_expression(expr),
            _ => None, // 忽略其他复杂属性
        }
    }

    // 解析属性表达式为StruPropertyValue
    fn parse_property_expression(&self, expr: &PropertyExpression) -> Option<StruPropertyValue> {
        match expr {
            // 基础类型处理
            PropertyExpression::Boolean(boolean_term) => self.parse_boolean_term(boolean_term),
            PropertyExpression::Real(real_term) => self.parse_real_term(real_term),
            PropertyExpression::Integer(integer_term) => self.parse_integer_term(integer_term),
            PropertyExpression::String(string_term) => self.parse_string_term(string_term),

            // 范围类型处理
            PropertyExpression::IntegerRange(range_term) => Some(StruPropertyValue::Range(
                range_term.lower.value.parse().ok()?,
                range_term.upper.value.parse().ok()?,
                range_term.lower.unit.clone(),
            )),

            // 其他复杂类型暂不处理
            _ => None,
        }
    }

    // 布尔项解析
    fn parse_boolean_term(&self, term: &BooleanTerm) -> Option<StruPropertyValue> {
        match term {
            BooleanTerm::Literal(b) => Some(StruPropertyValue::Boolean(*b)),
            BooleanTerm::Constant(_) => None, // 常量需要查表解析，此处简化
        }
    }

    // 实数项解析
    fn parse_real_term(&self, term: &SignedRealOrConstant) -> Option<StruPropertyValue> {
        match term {
            SignedRealOrConstant::Real(signed_real) => {
                let value = signed_real.sign.as_ref().map_or(1.0, |s| match s {
                    Sign::Plus => 1.0,
                    Sign::Minus => -1.0,
                }) * signed_real.value;
                Some(StruPropertyValue::Float(value))
            }
            SignedRealOrConstant::Constant { .. } => None, // TODO:常量需要查表
        }
    }

    // 整数项解析
    fn parse_integer_term(&self, term: &SignedIntergerOrConstant) -> Option<StruPropertyValue> {
        match term {
            SignedIntergerOrConstant::Real(signed_int) => {
                let value = signed_int.sign.as_ref().map_or(1, |s| match s {
                    Sign::Plus => 1,
                    Sign::Minus => -1,
                }) * signed_int.value;
                Some(StruPropertyValue::Integer(value))
            }
            SignedIntergerOrConstant::Constant { .. } => None, // 常量需要查表
        }
    }

    // 字符串项解析
    fn parse_string_term(&self, term: &StringTerm) -> Option<StruPropertyValue> {
        match term {
            StringTerm::Literal(s) => Some(StruPropertyValue::String(s.clone())),
            StringTerm::Constant(_) => None, // 常量需要查表
        }
    }


    pub fn create_channel_connection(&self, conn: &PortConnection, comp_name: String) -> Vec<Statement> {
        let mut stmts = Vec::new();

        // 定义标志位，标志是否创建了通道
        let mut is_channel_created = false;

        // 根据连接是否是广播，创建适当的channel。
        // 目前这种检查只存在于针对system中的连接。
        let mut is_broadcast = false;
        if let PortEndpoint::SubcomponentPort { subcomponent, port } = &conn.source {
            if self.process_broadcast_send.contains(&(subcomponent.clone(), port.clone())) {
                //广播的channel使用tokio::sync::broadcast::channel::<>。
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
            //非广播的channel使用crossbeam_channel::unbounded。
            stmts.push(Statement::Let(LetStmt {
                ifmut: false,
                name: conn.identifier.clone(),
                ty: None, //这里的通道类型由编译器自动推导
                init: Some(Expr::Call(
                    Box::new(Expr::Path(
                        vec!["crossbeam_channel".to_string(), "unbounded".to_string()],
                        PathType::Namespace,
                    )),
                    Vec::new(),
                )),
            }));
        }

        // 处理源端和目标端
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
                // 分配发送端
                stmts.push(Statement::Expr(Expr::MethodCall(
                    Box::new(Expr::Ident(format!(
                        "{}.{}",
                        src_comp.to_lowercase(),
                        src_port.to_lowercase()
                    ))),
                    "send".to_string(), //这个关键字的固定的，例如cnx: port the_sender.p -> the_receiver.p;，前者发送，后者接收
                    //vec![Expr::Ident("channel.0".to_string())],
                    //需要根据标志位if_broadcast判断是否是广播端口，它的语法是channel.0.clone()
                    vec![Expr::Call(
                        Box::new(Expr::Path(vec!["Some".to_string()], PathType::Member)),
                        vec![if is_broadcast { Expr::MethodCall(Box::new(Expr::Ident("channel.0.clone".to_string())), "".to_string(), Vec::new()) } 
                            else{ Expr::Ident(format!("{}.0", conn.identifier.clone())) }],
                    )],
                )));

                // 分配接收端
                //需要根据标志位if_broadcast判断是否是广播端口，如果是，暂不处理，如果不是，生成channel.1
                if !is_broadcast {
                    stmts.push(Statement::Expr(Expr::MethodCall(
                        Box::new(Expr::Ident(format!(
                            "{}.{}",
                            dst_comp.to_lowercase(),
                            dst_port.to_lowercase()
                        ))),
                        "receive".to_string(),
                        //vec![Expr::Ident("channel.1".to_string())],
                        
                        vec![Expr::Call(
                            Box::new(Expr::Path(vec!["Some".to_string()], PathType::Member)),
                            vec![Expr::Ident(format!("{}.1", conn.identifier.clone()))],
                        )],
                    )));
                }
                
            }
            (
                PortEndpoint::ComponentPort(port_name),
                PortEndpoint::SubcomponentPort {
                    subcomponent: dst_comp,
                    port: dst_port,
                },
            ) => {
                // 处理组件端口到子组件端口的连接
                // 根据端口方向确定内部端口名称
                let internal_port_name = match self.get_port_direction(port_name) {
                    PortDirection::In => format!("{}Send", port_name.to_lowercase()),
                    PortDirection::Out => format!("{}Send", port_name.to_lowercase()), // 输出端口生成 Send
                    PortDirection::InOut => format!("{}Send", port_name.to_lowercase()), // InOut 暂时按 In 处理
                };
                
                // 直接赋值给内部端口变量
                if is_broadcast {
                    stmts.push(Statement::Expr(Expr::BinaryOp(
                        Box::new(Expr::Ident(internal_port_name)),
                        "=".to_string(),
                        Box::new(Expr::Call(
                            Box::new(Expr::Path(vec!["Some".to_string()], PathType::Member)),
                            vec![Expr::Ident("channel.0.clone()".to_string())],
                        )),
                    )));
                } else {
                    stmts.push(Statement::Expr(Expr::BinaryOp(
                        Box::new(Expr::Ident(internal_port_name)),
                        "=".to_string(),
                        Box::new(Expr::Call(
                            Box::new(Expr::Path(vec!["Some".to_string()], PathType::Member)),
                            vec![Expr::Ident(format!("{}.0", conn.identifier.clone()))],
                        )),
                    )));
                }
                
                if !is_broadcast {
                    stmts.push(Statement::Expr(Expr::MethodCall(
                        Box::new(Expr::Ident(format!("{}.{}", dst_comp, dst_port))),
                        "receive".to_string(),
                        vec![Expr::Call(
                            Box::new(Expr::Path(vec!["Some".to_string()], PathType::Member)),
                            vec![Expr::Ident(format!("{}.1", conn.identifier.clone()))],
                        )],
                    )));
                }
            }
            (
                PortEndpoint::SubcomponentPort {
                    subcomponent: src_comp,
                    port: src_port,
                },
                PortEndpoint::ComponentPort(port_name),
            ) => {
                // 处理子组件端口到组件端口的连接（如 th_c.evenement -> evenement）
                // 发送端给线程
                stmts.push(Statement::Expr(Expr::MethodCall(
                    Box::new(Expr::Ident(format!("{}.{}", src_comp, src_port))),
                    "send".to_string(),
                    vec![Expr::Call(
                        Box::new(Expr::Path(vec!["Some".to_string()], PathType::Member)),
                        vec![Expr::Ident(format!("{}.0", conn.identifier.clone()))],
                    )],
                )));

                // 接收端给内部端口
                //似乎这种的分配没必要，一定是Rece
                let internal_port_name = match self.get_port_direction(port_name) {
                    PortDirection::In => format!("{}Send", port_name.to_lowercase()),
                    PortDirection::Out => format!("{}Rece", port_name.to_lowercase()), // 输出端口生成 Send
                    PortDirection::InOut => format!("{}Send", port_name.to_lowercase()), // InOut 暂时按 In 处理
                };
                
                // 直接赋值给内部端口变量
                stmts.push(Statement::Expr(Expr::BinaryOp(
                    Box::new(Expr::Ident(internal_port_name)),
                    "=".to_string(),
                    Box::new(Expr::Call(
                        Box::new(Expr::Path(vec!["Some".to_string()], PathType::Member)),
                        vec![Expr::Ident(format!("{}.1", conn.identifier.clone()))],
                    )),
                )));
            }
            // 可以继续添加其他端点类型的组合处理
            _ => {
                // 对于不支持的连接类型，生成TODO注释
                stmts.push(Statement::Expr(Expr::Ident(format!(
                    "// TODO: Unsupported connection type: {:?} -> {:?}",
                    conn.source, conn.destination
                ))));
            }
        }

        //如果is_broadcast为true，则在此处一次性解决所有它的订阅者的订阅操作，根据process_broadcast_receive中的记录，依次订阅channel.0.subscribe()
        if is_broadcast {
            if let PortEndpoint::SubcomponentPort { subcomponent, port } = &conn.source {
                if let Some(vercport) = self.process_broadcast_receive.get(&(subcomponent.clone(), port.clone())) {
                    for (subcomponent, port) in vercport {
                        stmts.push(Statement::Expr(Expr::MethodCall(
                            Box::new(Expr::Ident(format!("{}.{}", subcomponent, port))),
                            "receive".to_string(),
                            vec![Expr::Call(
                                Box::new(Expr::Path(vec!["Some".to_string()], PathType::Member)),
                                vec![Expr::Ident("channel.0.subscribe()".to_string())],
                            )],
                        )));
                    }
                }
            }
            if let PortEndpoint::ComponentPort (proc_port) = &conn.source {
                if self.thread_broadcast_receive.contains_key(&(proc_port.clone(), comp_name.clone())){
                    if let Some(vercport) = self.thread_broadcast_receive.get(&(proc_port.clone(), comp_name.clone())) {
                        for (subcomponent, port) in vercport {
                            stmts.push(Statement::Expr(Expr::MethodCall(
                                Box::new(Expr::Ident(format!("{}.{}", subcomponent, port))),
                                "receive".to_string(),
                                vec![Expr::Call(
                                    Box::new(Expr::Path(vec!["Some".to_string()], PathType::Member)),
                                    vec![Expr::Ident("channel.0.subscribe()".to_string())],
                                )],
                            )));
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

    //TODO:这是由于subprogram的feature中的参数连接，暂时还是使用端口连接（在aadl_ast中未定义参数连接方式），这里写死参数链接的类型
    pub fn convert_paramport_type(&self, port: &PortSpec) -> Type {
        // 直接提取分类器类型，不加任何包装
        match &port.port_type {
            PortType::Data { classifier } | PortType::EventData { classifier } => {
                classifier
                    .as_ref()
                    .map(|c| self.classifier_to_type(c))
                    .unwrap_or_else(|| {
                        // 默认类型处理，可以根据需要调整
                        match port.direction {
                            PortDirection::Out => Type::Named("i32".to_string()),
                            _ => Type::Named("(error)".to_string()),
                        }
                    })
            }
            PortType::Event => Type::Named("()".to_string()),
            // 其他类型不需要处理，因为此函数仅在参数连接时调用
        }
    }

    

}
