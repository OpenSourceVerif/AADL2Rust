//pest解析后的结果转换为aadlAst
#![allow(
    clippy::single_match,
    clippy::if_same_then_else,
)]
use crate::aadlight_parser;
use super::ast::aadl_ast_cj::*;
use pest::{iterators::Pair};
use crate::transform_annex::*;

// 引入 annex 转换模块
// transform_annex 现在在 main.rs 中声明

// 端口信息管理结构体
#[derive(Debug, Clone)]
pub struct PortInfo {
    pub name: String,
    pub direction: PortDirection,
}

// 端口信息管理器
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
        self.ports.iter()
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

// 全局端口管理器
use std::sync::Mutex;
use once_cell::sync::Lazy;

static GLOBAL_PORT_MANAGER: Lazy<Mutex<PortManager>> = Lazy::new(|| {
    Mutex::new(PortManager::new())
});

pub fn get_global_port_manager() -> &'static Mutex<PortManager> {
    &GLOBAL_PORT_MANAGER
}

// 辅助函数：从 Pair 中提取标识符
pub fn extract_identifier(pair: Pair<aadlight_parser::Rule>) -> String {
    pair.as_str().trim().to_string()
}

// 辅助函数：从 Pair 中提取包名
pub fn extract_package_name(pair: Pair<aadlight_parser::Rule>) -> PackageName {
    PackageName(
        pair.as_str()
            .split("::")
            .map(|s| s.trim().to_string())
            .collect(),
    )
}

// 主转换结构体
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
        //     println!("处理规则: {:?}, 内容: {}", pair.as_rule(), pair.as_str());
        //     if pair.as_rule() == aadlight_parser::Rule::package_declaration { //检查是否是package_declaration规则
        //         if let Some(pkg) = Self::transform_package(pair) {
        //         }
        //     }
        // }
        for pair in pairs {
            //println!("顶层规则: {:?}, 内容: {}", pair.as_rule(), pair.as_str());
            //println!("  内部规则: {:?}", pair.as_rule());

            // 进入 file 规则内部，提取出真正的 package_declaration
            if pair.as_rule() == aadlight_parser::Rule::file {
                for inner in pair.into_inner() {
                    //println!("  内部规则: {:?}, 内容: {}", inner.as_rule(), inner.as_str());
                    //println!("  内部规则: {:?}", inner.as_rule());
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
        //println!("=== 调试 package ===");
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
            //println!("  内部规则: {:?}, 内容: {}", inner.as_rule(), inner.as_str());
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
    
    pub fn transform_visibility_declaration(pair: Pair<aadlight_parser::Rule>) -> VisibilityDeclaration {
        // 首先收集所有内部项到向量中，这样我们可以多次遍历
        let items: Vec<_> = pair.into_inner().collect();
        // println!("🧩 解析到 {} 个 item:", items.len());
        // for (i, item) in items.iter().enumerate() {
        //     println!("  [{}] Rule: {:?}, Text: {}", i, item.as_rule(), item.as_str());
        // }


        match items.first().unwrap().as_str() {
            "with" => {
                // 处理 with 声明
                let mut packages = Vec::new();
                let mut property_sets = Vec::new();
                
                // 跳过第一个"with"项
                for item in items.iter().skip(1) {
                    match item.as_rule() {
                        aadlight_parser::Rule::package_name => {
                            //如果这里是base_types、data_model...，则说明是属性集名，不做处理。无法区分开，文件名还是属性集名，现在的方法是穷举属性集名。
                            match item.clone().as_str().to_lowercase().as_str() {
                                "base_types" | "data_model" => {}
                                _ =>{
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
                        _ => {} // 忽略逗号等其他符号
                    }
                }
                
                VisibilityDeclaration::Import {
                    packages,
                    property_sets,
                }
            }
            _ => {
                let identifier = extract_identifier(items[0].clone());
                //println!("🔎 尝试处理 renames 语句: {:?}", items);

                let original = extract_package_name(items[1].clone());

                VisibilityDeclaration::Alias {
                    new_name: identifier.clone(),
                    original: QualifiedName {
                        package_prefix: None,
                        identifier: original.0.join("::"),
                    },
                    is_package: true, // 假设现在只处理 package rename
                }
            }
        }
    }
    
    pub fn transform_package_section(&mut self, pair: Pair<aadlight_parser::Rule>) -> PackageSection {
        // println!("=== 调试 package_section ===");
        // println!("pair = Rule::{:?}", pair.as_rule());
        // for (i, inner) in pair.clone().into_inner().enumerate() {
        //     //println!("  inner[{}]: Rule::{:?}, text = {}", i, inner.as_rule(), inner.as_str());
        //     println!("  inner[{}]: Rule::{:?}", i, inner.as_rule());
        // }

        let mut is_public = true; //默认值是public
        let mut declarations = Vec::new();
        
        let mut inner_iter = pair.into_inner();
        
        // 检查第一个元素是否是 public/private 修饰符
        if let Some(first) = inner_iter.next() {
            match first.as_str() {
                "public" => {
                    is_public = true;
                }
                "private" => {
                    is_public = false;
                }
                _ => {
                    // 如果不是修饰符，则是说明其是一个声明
                    declarations.push(self.transform_declaration(first));
                }
            }
        }
        
        // 处理剩余的声明
        for inner in inner_iter {
            match inner.as_rule() {
                aadlight_parser::Rule::declaration => {
                    declarations.push(self.transform_declaration(inner));
                }
                _ => {} // 忽略其他规则
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
                // AadlDeclaration::ComponentType(self.transform_component_type(inner))
                let component_def = self.transform_component_type(inner);
                let component_type = match component_def {
                    ComponentDef::Type(ct) => ct,
                    ComponentDef::Extension(ext) => ComponentType {
                        category: ext.category,
                        identifier: ext.identifier,
                        prototypes: ext.prototypes,
                        features: ext.features,
                        properties: ext.properties,
                        annexes: ext.annexes,
                    },
                };
                AadlDeclaration::ComponentType(component_type)
            }
            aadlight_parser::Rule::component_implementation => {
                AadlDeclaration::ComponentImplementation(Self::transform_component_implementation(inner))
            }
            aadlight_parser::Rule::annex_library => {
                AadlDeclaration::AnnexLibrary(AnnexLibrary {})
            }
            _ => panic!("Unsupported declaration type: {:?}", inner.as_rule()),
        }
    }
    
    // 提取限定标识符（从pest的qualified_identifier节点转换）
    pub fn extract_qualified_identifier(pair: Pair<aadlight_parser::Rule>) -> QualifiedIdentifier {
        // `qualified_identifier` 的Pest规则是 `(identifier ~ "::")* ~ identifier`
        let parts: Vec<String> = pair.into_inner()
            .filter(|inner_pair| inner_pair.as_rule() == aadlight_parser::Rule::identifier)
            .map(|id_pair| id_pair.as_str().to_string())
            .collect();
        QualifiedIdentifier { parts }
    }
    
    pub fn transform_component_type(&mut self, pair: Pair<aadlight_parser::Rule>) -> ComponentDef {
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
        let mut extends: Option<UniqueComponentReference> = None;
        let mut prototype_bindings: Option<PrototypeBindings> = None;

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
                    //TODO: 处理extends（已解决）
                    //println!("extends: {:?}", inner.as_str());
                    let qual_id_pair = inner.into_inner().next().expect("extends必须指定组件类型引用");
                    let qual_id = Self::extract_qualified_identifier(qual_id_pair);
                    extends = Some(UniqueComponentReference {
                        package_prefix: None, // 或根据 qual_id 拆分出包前缀
                        identifier: qual_id.parts.join("::"),  // 将 qual_id 赋值给 identifier 字段
                    });
                }
                _ => {}
            }
        }
        
        // ComponentType {
        //     category,
        //     identifier,
        //     prototypes,
        //     features,
        //     properties,
        //     annexes,
        // }

        // 根据是否存在extends，返回不同的组件定义
        if let Some(extends) = extends {
            // 存在extends → 返回扩展组件类型
            ComponentDef::Extension(ComponentTypeExtension {
                category,
                identifier,
                extends,
                prototype_bindings,
                prototypes,
                features,
                // pub flows: FlowClause,
                // pub modes: Option<ModesClause>,
                properties,
                annexes,
            })
        } else {
            // 不存在extends → 返回普通组件类型
            ComponentDef::Type(ComponentType {
                category,
                identifier,
                prototypes,
                features,
                // pub flows: FlowClause,
                // pub modes: Option<ModesClause>,
                properties,
                annexes,
            })
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
                
                // 处理 classifier 和 is_array
                let mut classifier: Option<UniqueComponentClassifierReference> = None;
                let mut is_array = false;

                // 遍历剩余节点，匹配classifier和[]
                while let Some(node) = inner_iter.next() {
                    match node.as_str() {
                        "classifier" => {
                            // 提取classifier对应的qualified_identifier
                            let qual_id = Self::extract_qualified_identifier(inner_iter.next().unwrap());
                            let impl_ref = Self::qualified_id_to_impl_ref(qual_id);
                            classifier = Some(UniqueComponentClassifierReference::Type(impl_ref));
                        }
                        "[" => {
                            // 匹配数组语法[]，标记is_array为true
                            is_array = true;
                        }
                        _ => {}
                    }
                }

                Prototype::Component(ComponentPrototype {
                    category,
                    // classifier: None, // TODO: Handle classifier（已解决）
                    // is_array: false,  // TODO: Handle array spec（已解决）
                    classifier,
                    is_array,
                })
            }
            
            "feature" => {
                // 处理 direction 和 classifier
                let mut direction: Option<PortDirection> = None;
                let mut classifier: Option<UniqueComponentClassifierReference> = None;

                // 遍历剩余节点，匹配direction和classifier
                while let Some(node) = inner_iter.next() {
                    match node.as_str() {
                        // 匹配方向关键字
                        "in" => direction = Some(PortDirection::In),
                        "out" => direction = Some(PortDirection::Out),
                        "in out" => direction = Some(PortDirection::InOut),
                        // 匹配classifier
                        "classifier" => {
                            let qual_id = Self::extract_qualified_identifier(inner_iter.next().unwrap());
                            let impl_ref = Self::qualified_id_to_impl_ref(qual_id);
                            classifier = Some(UniqueComponentClassifierReference::Type(impl_ref));
                        }
                        _ => {}
                    }
                }

                Prototype::Feature(FeaturePrototype {
                    // direction: None, // TODO: Handle direction（已解决）
                    // classifier: None, // TODO: Handle classifier（已解决）
                    direction,
                    classifier,
                })
            }
            "feature group" => {
                // 处理 classifier
                let mut classifier: Option<UniqueFeatureGroupTypeReference> = None;

                // 遍历剩余节点，匹配classifier
                while let Some(node) = inner_iter.next() {
                    if node.as_str() == "classifier" {
                        let qual_id = Self::extract_qualified_identifier(inner_iter.next().unwrap());
                        classifier = Some(Self::qualified_id_to_feature_group_ref(qual_id));
                    }
                }

                Prototype::FeatureGroup(FeatureGroupPrototype {
                    // classifier: None, // TODO: Handle classifier（已解决）
                    classifier,
                })
            }
            _ => panic!("Unknown prototype type"),
        }
    }
    
    // ========== 新增辅助函数：QualifiedIdentifier 转 UniqueImplementationReference ==========
    fn qualified_id_to_impl_ref(qual_id: QualifiedIdentifier) -> UniqueImplementationReference {
        // 处理空 parts 情况
        if qual_id.parts.is_empty() {
            return UniqueImplementationReference {
                package_prefix: None,
                implementation_name: ImplementationName {
                    type_identifier: "unknown_type".to_string(),
                    implementation_identifier: "unknown_impl".to_string(),
                },
            };
        }

        // 拆分逻辑（适配 AADL 语法：package::type.impl → 包前缀=package，类型=type，实现=impl）
        let (package_prefix, type_id, impl_id) = if qual_id.parts.len() >= 3 {
            // 格式：pkg::type::impl → 包前缀=pkg，类型=type，实现=impl
            let pkg_parts = qual_id.parts[0..qual_id.parts.len()-2].to_vec();
            let type_identifier = qual_id.parts[qual_id.parts.len()-2].clone();
            let implementation_identifier = qual_id.parts.last().unwrap().clone();
            (Some(PackageName(pkg_parts)), type_identifier, implementation_identifier)
        } else if qual_id.parts.len() == 2 {
            // 格式：type::impl → 无包前缀，类型=type，实现=impl
            (None, qual_id.parts[0].clone(), qual_id.parts[1].clone())
        } else {
            // 格式：type → 无包前缀，类型=type，实现=默认值（比如 "impl"）
            (None, qual_id.parts[0].clone(), "impl".to_string())
        };

        UniqueImplementationReference {
            package_prefix,
            implementation_name: ImplementationName {
                type_identifier: type_id,
                implementation_identifier: impl_id,
            },
        }
    }

    // ========== 新增辅助函数：QualifiedIdentifier 转 UniqueFeatureGroupTypeReference ==========
    fn qualified_id_to_feature_group_ref(qual_id: QualifiedIdentifier) -> UniqueFeatureGroupTypeReference {
        if qual_id.parts.is_empty() {
            return UniqueFeatureGroupTypeReference {
                package_prefix: None,
                identifier: "unknown".to_string(),
            };
        }

        let (package_prefix, identifier) = if qual_id.parts.len() > 1 {
            let pkg_parts = qual_id.parts[0..qual_id.parts.len()-1].to_vec();
            let id = qual_id.parts.last().unwrap().clone();
            (Some(PackageName(pkg_parts)), id)
        } else {
            (None, qual_id.parts[0].clone())
        };

        UniqueFeatureGroupTypeReference {
            package_prefix,
            identifier,
        }
    }

    pub fn transform_features_clause(&mut self, pair: Pair<aadlight_parser::Rule>) -> FeatureClause {
        if pair.as_str().contains("none") {
            return FeatureClause::Empty;
        }
        
        let mut features = Vec::new();
        for inner in pair.into_inner() {
            if inner.as_rule() == aadlight_parser::Rule::feature_declaration {
                let feature = Self::transform_feature_declaration(inner);
                
                // 收集端口信息
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
                    // 兼容老语法中使用 identifier 作为类型名
                    if classifier_qname.is_none() {
                        classifier_qname = Some(inner.as_str().to_string());
                    }
                }
                _ => {}
            }
        }

        // 如果是端口类特征
        if let Some(pt) = port_type_str {
            let classifier = classifier_qname.clone().map(|qname| {
                // 解析包前缀和类型名
                let parts: Vec<&str> = qname.split("::").collect();
                let (package_prefix, type_id) = if parts.len() > 1 {
                    let package_name = parts[0..parts.len()-1].join("::");
                    let type_name = parts.last().unwrap().split(".").next().unwrap().to_string();
                    (Some(package_name), type_name)
                } else {
                    (None, qname.to_string())
                };
                
                PortDataTypeReference::Classifier(
                    UniqueComponentClassifierReference::Type(UniqueImplementationReference {
                        package_prefix: package_prefix.map(|p| PackageName(p.split("::").map(|s| s.to_string()).collect())),
                        implementation_name: ImplementationName {
                            type_identifier: type_id,
                            implementation_identifier: String::new(),
                        },
                    }),
                )
            });

            let resolved_port_type = match pt {
                "data port" | "parameter" => PortType::Data { classifier: classifier.clone() },
                "event data port" => PortType::EventData { classifier: classifier.clone() },
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

        // 访问特征：data access / subprogram access
        if let Some(at) = access_type_str {
            let direction = access_direction.unwrap_or(AccessDirection::Provides);

            // 构造分类器（若存在）
            let map_classifier_to_component_classifier = || -> Option<UniqueComponentClassifierReference> {
                classifier_qname.clone().map(|qname| {
                    let type_id = qname.split("::").last().unwrap_or(&qname).to_string();
                    
                    // 智能判断：如果以.Impl结尾，认为是实现引用
                    if type_id.ends_with("Impl") {
                        UniqueComponentClassifierReference::Implementation(UniqueImplementationReference {
                            package_prefix: None,
                            implementation_name: ImplementationName {
                                type_identifier: type_id,
                                implementation_identifier: String::new(),
                            },
                        })
                    } else {
                        // 否则认为是类型引用
                        UniqueComponentClassifierReference::Type(UniqueImplementationReference {
                            package_prefix: None,
                            implementation_name: ImplementationName {
                                type_identifier: type_id,
                                implementation_identifier: String::new(),
                            },
                        })
                    }
                })
            };

            match at {
                "data" => {
                    let classifier = map_classifier_to_component_classifier()
                        .map(DataAccessReference::Classifier);
                    return Feature::SubcomponentAccess(SubcomponentAccessSpec::Data(DataAccessSpec {
                        identifier,
                        direction,
                        classifier,
                    }));
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
        // println!("=== 调试 property ===");
        // println!("pair = Rule::{:?}, text = {}", pair.as_rule(), pair.as_str());
        // for (i, inner) in pair.clone().into_inner().enumerate() {
        //     println!("  inner[{}]: Rule::{:?}, text = {}", i, inner.as_rule(), inner.as_str());
        // }

        let mut inner_iter = pair.into_inner().peekable();

        // 检查是否有属性集前缀 (property_set::property_name)
        let (property_set, identifier) = if inner_iter.peek().map(|p| p.as_rule()) == Some(aadlight_parser::Rule::identifier) {
            let first_identifier = extract_identifier(inner_iter.next().unwrap());
            
            // 检查下一个元素是否是identifier
            if inner_iter.peek().map(|p| p.as_rule()) == Some(aadlight_parser::Rule::identifier) {
                let second_identifier = extract_identifier(inner_iter.next().unwrap());
                (Some(first_identifier), second_identifier)
            } else {
                // 没有identifier，说明第一个 identifier 就是属性名
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
        // === 处理 constant 标记 ===
        let mut is_constant = false;
        if inner_iter.peek().map(|p| p.as_rule()) == Some(aadlight_parser::Rule::constant) {
            is_constant = true;
            inner_iter.next(); // 消耗 constant
        }
        // 处理 property_value
        let value: PropertyValue = Self::transform_property_value(inner_iter.next().unwrap());
        
        Property::BasicProperty(BasicPropertyAssociation {
            identifier: PropertyIdentifier {
                property_set,
                name: identifier,
            },
            operator,
            is_constant, // TODO: Handle constant（已正确处理）
            value,
        })
    }
    
    //辅助函数
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
        // println!("=== 调试 property_value ===");
        // println!("pair = Rule::{:?}, text = {}", pair.as_rule(), pair.as_str());
        // for (i, inner) in pair.clone().into_inner().enumerate() {
        //     println!("  inner[{}]: Rule::{:?}, text = {}", i, inner.as_rule(), inner.as_str());
        // }

        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            aadlight_parser::Rule::apply_value => {
                let mut parts = inner.into_inner();
                let number = parts.next().unwrap().as_str().trim().to_string();
                let applies_to = parts.next().unwrap().as_str().trim().to_string();
                PropertyValue::Single(PropertyExpression::Apply(ApplyTerm {
                    number,
                    applies_to,
                }))
            }
            aadlight_parser::Rule::range_value => {
                // println!("=== 调试 range_value ===");
                // println!("inner = Rule::{:?}, text = {}", inner.as_rule(), inner.as_str());
                // for (i, inner2) in inner.clone().into_inner().enumerate() {
                //     println!("  inner[{}]: Rule::{:?}, text = {}", i, inner2.as_rule(), inner2.as_str());
                // }

                let mut parts = inner.into_inner();
                let lower_val = extract_identifier(parts.next().unwrap());
                //let lower_unit = Some(parts.next().unwrap().as_str().trim().to_string());
                // 解析下限单位（变为可选，例如优先级它没有单位）
                let lower_unit = if parts.peek().is_some_and(|p| p.as_rule() == aadlight_parser::Rule::unit) {
                    Some(parts.next().unwrap().as_str().trim().to_string())
                } else {
                    None
                };
                
                let upper_val = extract_identifier(parts.next().unwrap());
                //let upper_unit = Some(parts.next().unwrap().as_str().trim().to_string());
                // 解析上限单位（可选）
                let upper_unit = if parts.peek().is_some_and(|p| p.as_rule() == aadlight_parser::Rule::unit) {
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
                // println!("=== 调试 literal_value ===");
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
                // println!("=== 调试 first ===");
                // println!("first = Rule::{:?}, text = {}", first.as_rule(), first.as_str());
                // for (i, inner2) in first.clone().into_inner().enumerate() {
                //     println!("  innerfirst[{}]: Rule::{:?}, text = {}", i, inner2.as_rule(), inner2.as_str());
                // }

                match first.as_rule() {
                    aadlight_parser::Rule::number => {
                        let mut number_parts = first.into_inner().peekable();

                        // 解析符号
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
                        // 主数字部分
                        let int_part = number_parts.next().unwrap().as_str().trim();

                        // 判断是否为浮点数
                        let expr = if int_part.contains('.') {
                            let value = int_part.parse::<f64>().unwrap();
                            PropertyExpression::Real(SignedRealOrConstant::Real(SignedReal {
                                sign,
                                value,
                                unit: unit.clone(),
                            }))
                        } else {
                            let value = int_part.parse::<i64>().unwrap();
                            PropertyExpression::Integer(SignedIntergerOrConstant::Real(SignedInteger {
                                sign,
                                value,
                                unit: unit.clone(),
                            }))
                        };

                        PropertyValue::Single(expr)
                    }

                    aadlight_parser::Rule::string_literal => {
                        let raw = first.as_str();
                        let value = Self::strip_string_literal(raw);
                        PropertyValue::Single(PropertyExpression::String(
                            StringTerm::Literal(value)
                        ))

                    }

                    aadlight_parser::Rule::boolean => {
                        let val = match first.as_str() {
                            "true" => true,
                            "false" => false,
                            _ => panic!("Invalid boolean"),
                        };

                        PropertyValue::Single(PropertyExpression::Boolean(BooleanTerm::Literal(val)))
                    }

                    aadlight_parser::Rule::enum_value => {
                        let value = first.as_str().to_string();

                        PropertyValue::Single(PropertyExpression::String(
                            StringTerm::Literal(value)
                        ))
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
                
                // 检查是否有 applies to 子句
                let mut applies_to = None;
                for part in ref_parts {
                    if part.as_rule() == aadlight_parser::Rule::qualified_identifier {
                        applies_to = Some(extract_identifier(part));
                        break;
                    }
                }
                
                PropertyValue::Single(PropertyExpression::Reference(ReferenceTerm { 
                    identifier: referenced_id,
                    applies_to,
                }))
            }
            aadlight_parser::Rule::component_classifier_value => {
                let mut inner_iter = inner.into_inner();
                let qualified_identifier = inner_iter.next().unwrap();
                let qname = qualified_identifier.as_str().to_string();
                
                // 解析包前缀和类型名
                let parts: Vec<&str> = qname.split("::").collect();
                let (package_prefix, type_id) = if parts.len() > 1 {
                    let package_name = parts[0..parts.len()-1].join("::");
                    let type_name = parts.last().unwrap().to_string();
                    (Some(package_name), type_name)
                } else {
                    (None, qname.to_string())
                };
                
                // 智能判断：如果以.Impl结尾，认为是实现引用
                let unique_ref = if type_id.ends_with("Impl") {
                    UniqueComponentClassifierReference::Implementation(UniqueImplementationReference {
                        package_prefix: package_prefix.map(|p| PackageName(p.split("::").map(|s| s.to_string()).collect())),
                        implementation_name: ImplementationName {
                            type_identifier: type_id,
                            implementation_identifier: String::new(),
                        },
                    })
                } else {
                    // 否则认为是类型引用
                    UniqueComponentClassifierReference::Type(UniqueImplementationReference {
                        package_prefix: package_prefix.map(|p| PackageName(p.split("::").map(|s| s.to_string()).collect())),
                        implementation_name: ImplementationName {
                            type_identifier: type_id,
                            implementation_identifier: String::new(),
                        },
                    })
                };
                
                PropertyValue::Single(PropertyExpression::ComponentClassifier(ComponentClassifierTerm {
                    unique_component_classifier_reference: unique_ref,
                }))
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
    
    pub fn transform_component_implementation(pair: Pair<aadlight_parser::Rule>) -> ComponentImplementation {
        // println!("=== 调试 implementation ===");
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
        // println!("=== 调试 subcomponents ===");
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
    
    fn transform_array_spec(pair: Pair<aadlight_parser::Rule>) -> ArraySpec {
        let mut dimensions = Vec::new();
        
        for inner in pair.into_inner() {
            if inner.as_rule() == aadlight_parser::Rule::dimension {
                let mut dim_inner = inner.into_inner();
                
                // 尝试获取 number
                let size = if let Some(number_pair) = dim_inner.next() {
                    // number_pair 对应 number 规则
                    // 因为 AST 中是用 u32 存储 Fixed 大小，需要转换
                    // 处理可能存在的 "+10" 情况
                    let val_str = number_pair.as_str().trim().replace("+", "");
                    
                    // 这里假设数组维度必然是整数，如果解析失败默认为 1 或者 panic
                    let val: u32 = val_str.parse().expect("Array dimension must be a valid u32");
                    
                    Some(ArrayDimensionSize::Fixed(val))
                } else {
                    // 空维度 []
                    None
                };
                
                dimensions.push(ArrayDimension { size });
            }
        }

        ArraySpec {
            dimensions,
            element_implementations: None,
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
        
        // 处理 qualified_identifier，如果包含多个标识符就只取最后一个
        let qualified_identifier = inner_iter.next().unwrap();
        let name_str = if qualified_identifier.as_str().contains("::") {
            // 如果包含 :: 分隔符，只取最后一个标识符（Base_Types::Float，只需要Float）
            qualified_identifier.as_str().split("::").last().unwrap().trim().to_string()
        } else {
            // 否则直接使用原字符串
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
        
        // 初始化
        let mut array_spec = None;
        let mut properties = Vec::new();

        // 遍历剩余的 token
        for part in inner_iter {
            match part.as_rule() {
                aadlight_parser::Rule::array_spec => {
                    // 调用上面新写的辅助函数
                    array_spec = Some(Self::transform_array_spec(part));
                },
                aadlight_parser::Rule::properties => {
                    // 语法规则: properties = { "properties" ~ (property_association+) }
                    // 所以我们需要进入 properties 规则内部，找到所有的 property_association
                    for prop_inner in part.into_inner() {
                        if prop_inner.as_rule() == aadlight_parser::Rule::property_association {
                            // 复用现有的 transform_property_association 函数
                            properties.push(Self::transform_property_association(prop_inner));
                        }
                    }
                },
                _ => {
                    // 忽略分号或其他不相关的 token
                }
            }
        }

        Subcomponent {
            identifier,
            category,
            classifier,
            // array_spec: None, // TODO: Handle array spec（已解决）
            // properties: Vec::new(), // TODO: Handle properties（已解决）
            array_spec,
            properties,
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
    
    fn transform_in_modes(pair: Pair<aadlight_parser::Rule>) -> Vec<String> {
        let mut modes = Vec::new();
        // 遍历内部，提取所有 identifier
        for inner in pair.into_inner() {
            if inner.as_rule() == aadlight_parser::Rule::identifier {
                modes.push(extract_identifier(inner));
            }
        }
        modes
    }

    pub fn transform_call_sequence(pair: Pair<aadlight_parser::Rule>) -> CallSequence {
        // println!("=== 调试 calls_sequence ===");
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
        let mut properties = Vec::new();
        let mut in_modes = None;
    
        // for inner in inner_iter {
        //     if inner.as_rule() == aadlight_parser::Rule::subprogram_call {
        //         calls.push(Self::transform_subprogram_call(inner));
        //     }
        // }
        
        for part in inner_iter {
            match part.as_rule() {
                aadlight_parser::Rule::subprogram_call => {
                    // 处理子程序调用
                    calls.push(Self::transform_subprogram_call(part));
                },
                aadlight_parser::Rule::properties => {
                    // 处理属性集
                    for prop_inner in part.into_inner() {
                        if prop_inner.as_rule() == aadlight_parser::Rule::property_association {
                            properties.push(Self::transform_property_association(prop_inner));
                        }
                    }
                },
                aadlight_parser::Rule::in_modes => {
                    // 处理模式约束
                    in_modes = Some(Self::transform_in_modes(part));
                },
                _ => {
                    // 忽略冒号、分号或其他无关 token
                }
            }
        }

        CallSequence {
            identifier,
            calls,
            // properties: Vec::new(), // TODO: Handle properties（已解决）
            // in_modes: None, // TODO: Handle modes（已解决）
            properties,
            in_modes,
        }
    }
    
    pub fn transform_subprogram_call(pair: Pair<aadlight_parser::Rule>) -> SubprogramCall {
        // println!("=== 调试 subprogram_call ===");
        // println!("pair = Rule::{:?}------text = {}", pair.as_rule(),pair.as_str());
        // for (i, inner) in pair.clone().into_inner().enumerate() {
        //     //println!("  inner[{}]: Rule::{:?}, text = {}", i, inner.as_rule(), inner.as_str());
        //     println!("  inner[{}]: Rule::{:?} text = {}", i, inner.as_rule(),inner.as_str());
        // }

        let mut inner_iter = pair.into_inner();
        let identifier = extract_identifier(inner_iter.next().unwrap());
        //let _colon = inner_iter.next();
        //let _subprogram = inner_iter.next();
        
        let classifier_pair = inner_iter.next().unwrap();
        let type_name = if classifier_pair.as_str().contains("::") {
            classifier_pair.as_str().split("::").last().unwrap().trim().to_string()
        } else {
            extract_identifier(classifier_pair)
        };

        let called = CalledSubprogram::Classifier(
            UniqueComponentClassifierReference::Implementation(UniqueImplementationReference {
                package_prefix: None,
                implementation_name: ImplementationName {
                    type_identifier: extract_identifier(inner_iter.next().unwrap()),
                    implementation_identifier: String::new(),
                },
            }),
        );
        
        // 3. 处理属性
        let mut properties = Vec::new();

        // 遍历剩余的 token
        for part in inner_iter {
            if part.as_rule() == aadlight_parser::Rule::property_association {
                properties.push(Self::transform_property_association(part));
            }
        }

        SubprogramCall {
            identifier,
            called,
            // properties: Vec::new(), // TODO: Handle properties（已解决）
            properties,
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
        // println!("=== 调试 connection ===");
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
                let destination = Self::transform_parameterport_reference(port_iter.next().unwrap());
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

    pub fn transform_parameterport_reference(pair: Pair<aadlight_parser::Rule>) -> ParameterEndpoint {
        let reference = pair.as_str().trim();
        if reference.contains('.') {
            let mut parts = reference.split('.');
            ParameterEndpoint::SubprogramCallParameter { 
                call_identifier: parts.next().unwrap().to_string(), 
                parameter: parts.next().unwrap().to_string() } 
        } else {
            ParameterEndpoint::ComponentParameter { 
                parameter: reference.to_string(), 
                data_subcomponent: (None) }
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
