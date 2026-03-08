#![allow(clippy::collapsible_match)]
use crate::aadl_ast2rust_code::intermediate_ast::*;

use crate::aadl_ast2rust_code::converter::AadlConverter;
use crate::ast::aadl_ast_cj::*;

pub fn convert_subprogram_component(
    temp_converter: &AadlConverter,
    comp: &ComponentType,
    package: &Package,
) -> Vec<Item> {
    let items = Vec::new();

    // 检查是否是C语言绑定的子程序
    if let Some(c_func_name) = extract_c_function_name(comp) {
        return generate_c_function_wrapper(temp_converter, comp, &c_func_name, package);
    }
    
    items
}

fn extract_c_function_name(comp: &ComponentType) -> Option<String> {
    if let PropertyClause::Properties(props) = &comp.properties {
        for prop in props {
            if let Property::BasicProperty(bp) = prop {
                if bp.identifier.name.to_lowercase() == "source_name" {
                    if let PropertyValue::Single(PropertyExpression::String(StringTerm::Literal(
                        name,
                    ))) = &bp.value
                    {
                        return Some(name.clone());
                    }
                }
            }
        }
    }
    None
}

fn generate_c_function_wrapper(
    temp_converter: &AadlConverter,
    comp: &ComponentType,
    c_func_name: &str,
    package: &Package,
) -> Vec<Item> {
    //获取C程序源文件文件名
    let source_files = extract_source_files(comp);

    let mut items = Vec::new();
    let mut functions = Vec::new();
    let mut types_to_import = std::collections::HashSet::new();

    // 处理每个特征
    if let FeatureClause::Items(features) = &comp.features {
        for feature in features {
            match feature {
                Feature::Port(port) => {
                    let (func_name, param_type) = match port.direction {
                        PortDirection::Out => (
                            "send",
                            Type::Reference(
                                Box::new(temp_converter.convert_paramport_type(port)),
                                true,
                                true,
                            ),
                        ),
                        PortDirection::In => (
                            "receive",
                            Type::Reference(
                                Box::new(temp_converter.convert_paramport_type(port)),
                                false,
                                false,
                            ),
                        ),
                        _ => continue, //
                    };

                    // 收集需要导入的类型
                    //println!("port: {:?}", port);
                    if let Type::Named(type_name) = &temp_converter.convert_paramport_type(port) {
                        //println!("type_name: {}", type_name);
                        if !is_rust_primitive_type(type_name) {
                            types_to_import.insert(type_name.clone());
                        }
                    }

                    // 创建包装函数
                    functions.push(FunctionDef {
                        name: func_name.to_string(),
                        params: vec![Param {
                            name: port.identifier.to_string().to_lowercase(),
                            ty: param_type,
                        }],
                        return_type: Type::Unit,
                        body: Block {
                            stmts: vec![Statement::Expr(Expr::Unsafe(Box::new(Block {
                                stmts: vec![Statement::Expr(Expr::Call(
                                    Box::new(Expr::Path(
                                        vec![c_func_name.to_string()],
                                        PathType::Namespace,
                                    )),
                                    vec![Expr::Ident(port.identifier.to_string().to_lowercase())],
                                ))],
                                expr: None,
                            })))],
                            expr: None,
                        },
                        asyncness: false,
                        vis: Visibility::Public,
                        docs: vec![
                            format!("// Wrapper for C function {}", c_func_name),
                            format!("// Original AADL port: {}", port.identifier),
                        ],
                        attrs: Vec::new(),
                    });
                }
                Feature::SubcomponentAccess(sub_access) => {
                    // 处理 requires data access 特征
                    if let SubcomponentAccessSpec::Data(data_access) = sub_access {
                        if data_access.direction == AccessDirection::Requires {
                            // 从 this : requires data access POS.Impl 中提取 POS.Impl
                            if let Some(classifier) = &data_access.classifier {
                                if let DataAccessReference::Classifier(unique_ref) = classifier {
                                    if let UniqueComponentClassifierReference::Implementation(
                                        impl_ref,
                                    ) = unique_ref
                                    {
                                        let data_component_name =
                                            &impl_ref.implementation_name.type_identifier;
                                        // 查找该数据组件实现中的具体数据类型
                                        let data_types = find_data_type_from_implementation(
                                            data_component_name,
                                            package,
                                            temp_converter,
                                        );
                                        // 将所有数据类型添加到导入列表中
                                        for data_type in &data_types {
                                            types_to_import.insert(data_type.clone());
                                        }

                                        // 生成一个 call 函数，参数包含全部的数据类型
                                        if !data_types.is_empty() {
                                            let mut params = Vec::new();
                                            let mut call_args = Vec::new();

                                            for (idx, data_type) in data_types.iter().enumerate() {
                                                let param_name = format!("arg{}", idx);
                                                params.push(Param {
                                                    name: param_name.clone(),
                                                    ty: Type::Reference(Box::new(Type::Named(data_type.clone())), true, true), // &mut DataType
                                                });
                                                call_args.push(Expr::Ident(param_name));
                                            }

                                            let call_function = FunctionDef {
                                                name: "call".to_string(),
                                                params,
                                                return_type: Type::Unit,
                                                body: Block {
                                                    stmts: vec![Statement::Expr(Expr::Unsafe(Box::new(Block {
                                                        stmts: vec![Statement::Expr(Expr::Call(
                                                            Box::new(Expr::Path(
                                                                vec![c_func_name.to_string()],
                                                                PathType::Namespace,
                                                            )),
                                                            call_args, // 传递所有参数
                                                        ))],
                                                        expr: None,
                                                    })))],
                                                    expr: None,
                                                },
                                                asyncness: false,
                                                vis: Visibility::Public,
                                                docs: vec![
                                                    format!("// Call C function {} with data access references", c_func_name),
                                                    "// Generated for requires data access feature".to_string(),
                                                    "// Note: Rust compiler will handle the reference to pointer conversion".to_string(),
                                                ],
                                                attrs: Vec::new(),
                                            };

                                            functions.push(call_function);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } //_ => {} // 忽略其他类型的特征
                Feature::FeatureGroup(fg) => {
                    if let Some(FeatureGroupReference::Classifier(ref c)) = fg.classifier {
                        // 解析类型名称
                        let type_name = match c {
                            UniqueComponentClassifierReference::Type(t) => t.implementation_name.type_identifier.clone(),
                            UniqueComponentClassifierReference::Implementation(i) => i.implementation_name.type_identifier.clone(),
                        };
                        
                        // 添加到导入列表
                        types_to_import.insert(type_name.clone());
                    }
                }

                Feature::Parameter(param) => {
                    // 解析参数类型
                    let mut param_type_name = String::new();
                    if let Some(PortDataTypeReference::Classifier(ref c)) = param.classifier {
                        param_type_name = match c {
                            UniqueComponentClassifierReference::Type(t) => t.implementation_name.type_identifier.clone(),
                            UniqueComponentClassifierReference::Implementation(i) => i.implementation_name.type_identifier.clone(),
                        };
                    }

                    // 只有非基本类型才需要导入
                    if !param_type_name.is_empty() && !is_rust_primitive_type(&param_type_name) {
                        types_to_import.insert(param_type_name.clone());
                    }
                    
                    let rust_type = if param_type_name.is_empty() { 
                        Type::Unit 
                    } else { 
                        Type::Named(param_type_name.clone()) 
                    };

                    // 假设这是一个 setter/getter 风格的接口
                    let func_name = format!("{}_{}", 
                        if param.direction == PortDirection::Out { "get" } else { "set" }, 
                        param.identifier.to_lowercase()
                    );

                    functions.push(FunctionDef {
                        name: func_name,
                        params: vec![Param {
                            name: "val".to_string(),
                            ty: if param.direction == PortDirection::In { 
                                rust_type.clone() // In 参数通常传值
                            } else { 
                                Type::Reference(Box::new(rust_type.clone()), true, true) // Out 参数传可变引用
                            },
                        }],
                        return_type: Type::Unit,
                        body: Block {
                            stmts: vec![
                                Statement::Expr(Expr::Unsafe(Box::new(Block {
                                    stmts: vec![], 
                                    expr: Some(Box::new(Expr::Ident(format!("/* TODO: Map parameter {} to C call arg */", param.identifier))))
                                })))
                            ],
                            expr: None,
                        },
                        asyncness: false,
                        vis: Visibility::Public,
                        docs: vec![format!("// Helper for parameter: {}", param.identifier)],
                        attrs: Vec::new(),
                    });
                }
            }
        }
    }

    // 如果没有通信端口，创建直接调用C函数的包装器
    if functions.is_empty() {
        functions.push(FunctionDef {
            name: "execute".to_string(),
            params: Vec::new(),
            return_type: Type::Unit,
            body: Block {
                stmts: vec![Statement::Expr(Expr::Unsafe(Box::new(Block {
                    stmts: vec![Statement::Expr(Expr::Call(
                        Box::new(Expr::Path(
                            vec![c_func_name.to_string()],
                            PathType::Namespace,
                        )),
                        Vec::new(),
                    ))],
                    expr: None,
                })))],
                expr: None,
            },
            asyncness: false,
            vis: Visibility::Public,
            docs: vec![
                format!("// Direct execution wrapper for C function {}", c_func_name),
                "// This component has no communication ports".to_string(),
            ],
            attrs: Vec::new(),
        });
    }
    // 创建模块
    //if !functions.is_empty()

    {
        let mut docs = vec![
            format!(
                "// Auto-generated from AADL subprogram: {}",
                comp.identifier
            ),
            format!("// C binding to: {}", c_func_name),
        ];
        //在注释中添加C程序源文件文件名
        if !source_files.is_empty() {
            docs.push(format!("// source_files: {}", source_files.join(", ")));
        }

        // 构建use语句
        let mut imports = vec![c_func_name.to_string()];
        //println!("types_to_import: {:?}", types_to_import);
        if !types_to_import.is_empty() {
            imports.extend(types_to_import);
        }

        let use_stmt = Item::Use(UseStatement {
            path: vec!["super".to_string()],
            kind: UseKind::Nested(imports),
        });

        // 构建模块内容：先添加use语句，再添加函数
        let mut module_items = vec![use_stmt];
        module_items.extend(functions.into_iter().map(Item::Function));

        let module = RustModule {
            name: comp.identifier.to_lowercase(),
            docs,
            //items: functions.into_iter().map(Item::Function).collect(),
            items: module_items,
            attrs: Default::default(),
            vis: Visibility::Public,
            withs: Vec::new(),
        };
        items.push(Item::Mod(Box::new(module)));
    }

    items
}

fn extract_source_files(comp: &ComponentType) -> Vec<String> {
    let mut source_files = Vec::new();

    if let PropertyClause::Properties(props) = &comp.properties {
        for prop in props {
            if let Property::BasicProperty(bp) = prop {
                if bp.identifier.name.to_lowercase() == "source_text" {
                    match &bp.value {
                        PropertyValue::Single(PropertyExpression::String(StringTerm::Literal(
                            text,
                        ))) => {
                            source_files.push(text.clone());
                        }
                        PropertyValue::List(arraylist) => {
                            for item in arraylist {
                                if let PropertyListElement::Value(PropertyExpression::String(
                                    StringTerm::Literal(text),
                                )) = item
                                {
                                    source_files.push(text.clone());
                                }
                            }
                        }
                        _ => {
                            println!("error in extract_source_files");
                        }
                    }
                }
            }
        }
    }

    source_files
}

// 辅助函数：判断是否为Rust原生类型
fn is_rust_primitive_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
            | "bool"
            | "char"
            | "str"
            | "String"
    )
}

/// 从数据组件实现名称中查找具体的数据类型
/// 例如：从 POS.Impl 中找到 Field : data POS_Internal_Type 中的 POS_Internal_Type
fn find_data_type_from_implementation(impl_name: &str, package: &Package, temp_converter:&AadlConverter) -> Vec<String> {
    let mut data_types = Vec::new();

    // 在 Package 中查找组件实现
    if let Some(public_section) = &package.public_section {
        for decl in &public_section.declarations {
            if let AadlDeclaration::ComponentImplementation(impl_) = decl {
                //println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!impl_.name.type_identifier: {}", impl_.name.type_identifier);
                // 检查实现名称是否匹配：impl_name 可能是 "POS.Impl"，而 type_identifier 是 "POS"
                // 所以需要检查 impl_name 是否以 type_identifier 开头
                if impl_name.starts_with(&impl_.name.type_identifier) {
                    // 找到匹配的组件实现，查找其中的数据子组件
                    if let SubcomponentClause::Items(subcomponents) = &impl_.subcomponents {
                        for sub in subcomponents {
                            if sub.category == ComponentCategory::Data {
                                // 从数据子组件中提取类型名
                                if let SubcomponentClassifier::ClassifierReference(
                                    UniqueComponentClassifierReference::Implementation(unirf),
                                ) = &sub.classifier
                                {
                                    // 先去查找type_mappings中是否有这个类型
                                    if let Some(type_name) = temp_converter.type_mappings.get(&unirf.implementation_name.type_identifier.to_lowercase()) {
                                        if let Type::Named(type_name_str) = type_name {
                                            data_types.push(type_name_str.clone());
                                        }
                                    } else {
                                        data_types.push(unirf.implementation_name.type_identifier.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 在私有部分也查找
    if let Some(private_section) = &package.private_section {
        for decl in &private_section.declarations {
            if let AadlDeclaration::ComponentImplementation(impl_) = decl {
                // 检查实现名称是否匹配：impl_name 可能是 "POS.Impl"，而 type_identifier 是 "POS"
                // 所以需要检查 impl_name 是否以 type_identifier 开头
                if impl_name.starts_with(&impl_.name.type_identifier) {
                    // 找到匹配的组件实现，查找其中的数据子组件
                    if let SubcomponentClause::Items(subcomponents) = &impl_.subcomponents {
                        for sub in subcomponents {
                            if sub.category == ComponentCategory::Data {
                                // 从数据子组件中提取类型名
                                if let SubcomponentClassifier::ClassifierReference(
                                    UniqueComponentClassifierReference::Implementation(unirf),
                                ) = &sub.classifier
                                {
                                    data_types.push(unirf.implementation_name.type_identifier.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    data_types
}