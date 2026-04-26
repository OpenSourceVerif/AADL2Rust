#![allow(clippy::only_used_in_recursion)]
use super::intermediate_ast::*;
use chrono::Local;

// Rust代码生成器
pub struct RustCodeGenerator {
    buffer: String,
    indent_level: usize,
}

impl Default for RustCodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl RustCodeGenerator {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            indent_level: 0,
        }
    }

    // 主入口：生成完整模块代码
    pub fn generate_module_code(&mut self, module: &RustModule) -> String {
        self.buffer.clear();

        // 文件头
        self.writeln(&format!("// Auto-generated from AADL package: {}", module.name));
        //self.writeln("// 自动生成的 Rust 代码 - 来自 AADL 模型");
        self.writeln(&format!(
            "// 生成时间: {}",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        ));
        self.writeln("");
        self.writeln("#![allow(unused_imports)]");
        self.writeln("#![allow(non_camel_case_types)]");
        self.writeln("#![allow(non_snake_case)]");
        self.writeln("#![allow(unused_assignments)]");
        self.writeln("use crossbeam_channel::{Receiver, Sender};");
        self.writeln("use std::sync::{Arc,Mutex};");
        self.writeln("use std::thread;");
        self.writeln("use std::time::{Duration, Instant};");
        self.writeln("use lazy_static::lazy_static;");
        self.writeln("use std::collections::HashMap;");
        self.writeln("use crate::common_traits::*;");
        self.writeln("use crate::posix::*;");
        self.writeln("use tokio::sync::broadcast::{self,Sender as BcSender, Receiver as BcReceiver};");
        self.writeln("use libc::{self, syscall, SYS_gettid};");
        self.writeln("use rand::{Rng};");
        self.writeln("use libc::{");
        self.writeln("    pthread_self, sched_param, pthread_setschedparam, SCHED_FIFO,");
        // self.writeln("    cpu_set_t, CPU_SET, CPU_ZERO, sched_setaffinity,");
        self.writeln("};");
        self.writeln("include!(concat!(env!(\"OUT_DIR\"), \"/aadl_c_bindings.rs\"));"); //绑定的函数通过 include! 注入到根模块

        self.writeln("");
        self.generate_withs(&module.withs);

        // 添加CPU亲和性设置函数
        // self.writeln("// ---------------- cpu ----------------");
        // self.writeln("fn set_thread_affinity(cpu: isize) {");
        // self.writeln("    unsafe {");
        // self.writeln("        let mut cpuset: cpu_set_t = std::mem::zeroed();");
        // self.writeln("        CPU_ZERO(&mut cpuset);");
        // self.writeln("        CPU_SET(cpu as usize, &mut cpuset);");
        // self.writeln("        sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &cpuset);");
        // self.writeln("    }");
        // self.writeln("}");
        // self.writeln("");

        // self.writeln("// ---------------- System ----------------");
        // self.writeln("pub trait System {");
        // self.writeln("    fn new() -> Self");
        // self.writeln("        where Self: Sized;");
        // self.writeln("    fn run(self);");
        // self.writeln("}");
        // self.writeln("");
        
        // self.writeln("// ---------------- Process ----------------");
        // self.writeln("pub trait Process {");
        // self.writeln("    fn new(cpu_id: isize) -> Self");
        // self.writeln("        where Self: Sized;");
        // self.writeln("    fn start(self);");
        // self.writeln("}");
        // self.writeln("");
        
        // self.writeln("// ---------------- Thread ----------------");
        // self.writeln("pub trait Thread {");
        // self.writeln("    fn new(cpu_id: isize) -> Self");
        // self.writeln("        where Self: Sized;");
        // self.writeln("    fn run(self);");
        // self.writeln("}");
        // self.writeln("");

        // 生成模块内容
        self.generate_items(&module.items);

        self.buffer.clone()
    }

    // 生成with声明
    fn generate_withs(&mut self, withs: &[RustWith]) {
        
        for with in withs {
            self.writeln(&format!("use crate::{}{};", with.path.join("_"), if with.glob { "::*" } else { "" }));
        }
    }

    // 生成多个项
    fn generate_items(&mut self, items: &[Item]) {
        for item in items {
            self.generate_item(item);
        }
    }

    // 生成单个项
    fn generate_item(&mut self, item: &Item) {
        match item {
            Item::Struct(s) => self.generate_struct(s),
            Item::Enum(e) => self.generate_enum(e),
            Item::Union(u) => self.generate_union(u),  // 新增
            Item::Function(f) => self.generate_function(f),
            Item::Impl(i) => self.generate_impl(i),
            Item::Const(c) => self.generate_const(c),
            Item::TypeAlias(t) => self.generate_type_alias(t),
            Item::Use(u) => self.generate_use(u),
            Item::Mod(m) => self.generate_nested_module(m),
            Item::LazyStatic(l) => self.generate_lazy_static(l),
        }
    }

    fn generate_nested_module(&mut self, m: &RustModule) {
        // 生成模块声明行
        match &m.vis {
            Visibility::Public => self.write("pub "),
            Visibility::Private => (), // 私有模块不添加修饰符
            Visibility::Restricted(paths) => self.write(&format!("pub(in {} ) ", paths.join("::"))),
            Visibility::None => (),
        }

        self.writeln(&format!("mod {} {{", m.name));
        self.indent();

        // 模块级文档和属性
        for doc in &m.docs {
            self.writeln(doc);
        }
        for attr in &m.attrs {
            self.generate_attribute(attr);
        }

        // 模块内容
        self.generate_items(&m.items);

        self.dedent();
        self.writeln("}");
        self.writeln("");
    }

    fn generate_struct(&mut self, s: &StructDef) {
        // 文档注释
        for doc in &s.docs {
            self.writeln(doc);
        }

        // 派生属性
        if !s.derives.is_empty() {
            self.write("#[derive(");
            for (i, derive) in s.derives.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(derive);
            }
            self.writeln(")]");
        }

        // 结构体定义
        self.write(&format!("{}struct {} ", self.visibility(&s.vis), s.name));

        if s.generics.is_empty() {
            self.writeln("{");
        } else {
            self.write("<");
            for (i, generic) in s.generics.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(&generic.name);
                if !generic.bounds.is_empty() {
                    self.write(": ");
                    for (j, bound) in generic.bounds.iter().enumerate() {
                        if j > 0 {
                            self.write(" + ");
                        }
                        self.write(bound);
                    }
                }
            }
            self.writeln("> {");
        }

        self.indent();

        // 1. 生成端口字段
        for field in &s.fields {
            self.generate_field(field);
        }

        // 如果是进程结构体则不生成属性字段
        // if !s.name.ends_with("Process") && !s.properties.is_empty() {
        //     self.writeln("\n    // --- AADL属性 ---");
        //     for prop in &s.properties {
        //         self.writeln(&format!(
        //             "pub {}: {}, {}",
        //             prop.name.to_lowercase(),
        //             self.type_for_property(&prop.value),
        //             prop.docs.join("\n")
        //         ));
        //     }
        // }
        self.dedent();
        self.writeln("}");
        self.writeln("");

        //self.generate_properties_impl(s);
    }

    // 生成属性初始化impl块(似乎默认process没有属性，实际按标准是可以有的，TODO)
    // 2025.10.10 将这部分功能移动到converter.rs中，此处废弃
    #[allow(unused)]
    fn generate_properties_impl(&mut self, s: &StructDef) {
        if s.properties.is_empty() {
            return;
        }

        self.writeln(&format!("impl {} {{", s.name));
        self.writeln("    // 创建组件并初始化AADL属性");
        self.write("    pub fn new(cpu_id: isize");

        // 为以"Shared"结尾的字段添加参数
        for field in &s.fields {
            if let Type::Named(type_name) = &field.ty {
                if type_name.ends_with("Shared") {
                    self.write(&format!(", {}: {}", field.name, self.type_to_string(&field.ty)));
                }
            }
        }

        self.writeln(") -> Self {");
        self.writeln("        Self {");

        // 1端口字段初始化，新增了针对cpu_id的特殊处理，将其作为特性
        for field in &s.fields {
            //println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!field.name: {:?}", field.ty);
            if field.name == "cpu_id" {
                self.writeln("            cpu_id: cpu_id,");
            } else if let Type::Named(type_name) = &field.ty {
                if type_name.ends_with("Shared") {
                    // 共享变量字段使用传入的参数初始化
                    self.writeln(&format!("            {}: {},", field.name, field.name));
                } else {
                    // 其他字段初始化为None
                    self.writeln(&format!("            {}: None,", field.name));
                }
            } else {
                // 其他类型的字段初始化为None
                self.writeln(&format!("            {}: None,", field.name));
            }
        }

        // 2属性字段初始化
        for prop in &s.properties {
            let init_value = match &prop.value {
                StruPropertyValue::Boolean(b) => b.to_string(),
                StruPropertyValue::Integer(i) => i.to_string(),
                StruPropertyValue::Float(f) => f.to_string(),
                StruPropertyValue::String(s) => format!("\"{}\".to_string()", s),
                StruPropertyValue::Duration(val, _) => val.to_string(),
                StruPropertyValue::Range(min, max, _) => format!("({}, {})", min, max),
                StruPropertyValue::None => "Default::default()".to_string(),
                StruPropertyValue::Custom(s) => s.to_string(),
            };
            self.writeln(&format!(
                "            {}: {}, // {}",
                prop.name.to_lowercase(),
                init_value,
                prop.docs[0].trim_start_matches("// ")
            ));
        }

        self.writeln("        }");
        self.writeln("    }");
        self.writeln("}");
    }

    // 根据属性值推断Rust类型
    // 2025.10.10 将这部分功能移动到converter.rs中，此处废弃
    #[allow(unused)]
    fn type_for_property(&self, value: &StruPropertyValue) -> String {
        match value {
            StruPropertyValue::Boolean(_) => "bool".to_string(),
            StruPropertyValue::Integer(_) => "u64".to_string(), //基本上都是正数，就不采用i64
            StruPropertyValue::Float(_) => "f64".to_string(),
            StruPropertyValue::String(_) => "String".to_string(),
            StruPropertyValue::Duration(_, _) => "u64".to_string(),
            StruPropertyValue::Range(_, _, _) => "(u64, u64)".to_string(),
            StruPropertyValue::None => "Default".to_string(),
            StruPropertyValue::Custom(s) => s.to_string(),
        }
    }

    fn generate_field(&mut self, field: &Field) {
        for attr in &field.attrs {
            self.generate_attribute(attr);
        }
        self.write(&format!(
            "pub {}: {},",
            field.name,
            self.type_to_string(&field.ty)
        ));
        for doc in &field.docs {
            self.writeln(doc);
        }
    }

    fn generate_impl(&mut self, i: &ImplBlock) {
        self.write("impl");

        // 泛型参数
        if !i.generics.is_empty() {
            self.write("<");
            for (idx, generic) in i.generics.iter().enumerate() {
                if idx > 0 {
                    self.write(", ");
                }
                self.write(&generic.name);
                if !generic.bounds.is_empty() {
                    self.write(": ");
                    for (j, bound) in generic.bounds.iter().enumerate() {
                        if j > 0 {
                            self.write(" + ");
                        }
                        self.write(bound);
                    }
                }
            }
            self.write(">");
        }

        // trait实现
        if let Some(trait_ty) = &i.trait_impl {
            self.write(&format!(" {} for", self.type_to_string(trait_ty)));
        }
        // 目标类型
        self.write(&format!(" {} ", self.type_to_string(&i.target)));

        

        self.writeln("{");
        self.indent();

        for item in &i.items {
            match item {
                ImplItem::Method(m) => self.generate_function(m),
                ImplItem::AssocConst(name, ty, expr) => {
                    self.writeln(&format!("const {}: {} = ", name, self.type_to_string(ty)));
                    self.generate_expr(expr);
                    self.writeln(";");
                }
                ImplItem::AssocType(name, ty) => {
                    self.writeln(&format!("type {} = {};", name, self.type_to_string(ty)));
                }
            }
        }

        self.dedent();
        self.writeln("}");
        self.writeln("");
    }

    fn generate_function(&mut self, f: &FunctionDef) {
        // 文档注释
        for doc in &f.docs {
            self.writeln(doc);
        }

        // 属性
        for attr in &f.attrs {
            self.generate_attribute(attr);
        }

        // 函数签名
        self.write(&format!(
            "{}{}fn {}",
            self.visibility(&f.vis),
            if f.asyncness { "async " } else { "" },
            f.name
        ));

        // 参数
        self.write("(");
        for (i, param) in f.params.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            if param.name.is_empty() {
                self.write(&self.type_to_string(&param.ty));
            } else {
                self.write(&format!(
                    "{}: {}",
                    param.name,
                    self.type_to_string(&param.ty)
                ));
            }
        }
        self.write(")");

        // 返回类型
        self.write(&format!(" -> {}", self.type_to_string(&f.return_type)));

        // 函数体
        self.writeln(" {");
        self.indent();
        self.generate_block(&f.body);
        self.dedent();
        self.writeln("}");
        self.writeln("");
    }

    fn generate_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.generate_statement(stmt);
        }

        if let Some(expr) = &block.expr {
            self.generate_expr(expr);
            self.writeln(";");
        }
    }

    // 专门用于生成 match 分支体的方法
    fn generate_match_arm_body(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.generate_statement(stmt);
        }

        if let Some(expr) = &block.expr {
            self.generate_expr(expr);
            // match 分支的最后一个表达式永远不应该有分号，因为它是返回值
        }
    }

    fn generate_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let(ls) => {
                self.write(&format!(
                    "{} {}",
                    if ls.ifmut { "let mut" } else { "let" },
                    ls.name
                ));
                if let Some(ty) = &ls.ty {
                    self.write(&format!(": {}", self.type_to_string(ty)));
                }
                if let Some(init) = &ls.init {
                    self.write(" = ");
                    self.generate_expr(init);
                }
                self.writeln(";");
            }
            Statement::Expr(expr) => {
                // 处理连接建立的表达式 TODO
                if let Expr::MethodCall(receiver, method, args) = expr {
                    if method == "send" || method == "receive" {
                        self.writeln("// build connection: ");
                        self.write("    ");
                        self.generate_expr(receiver);
                        self.write(" = ");

                        for (i, arg) in args.iter().enumerate() {
                            if i > 0 {
                                self.write(", ");
                            }
                            self.generate_expr(arg);
                        }
                        self.writeln(";");

                        return;
                    }
                }
                // 普通表达式处理
                self.generate_expr(expr);
                self.writeln(";");
            }
            Statement::Item(item) => self.generate_item(item),
            Statement::Continue => {
                self.writeln("continue;");
            }
            Statement::Break => {
                self.writeln("break;");
            }
            Statement::Comment(comment) => {
                self.writeln(&format!("// {}", comment));
            }
        }
    }

    fn generate_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Ident(id) => self.write(id),
            Expr::Path(path, path_type) => {
                let separator = match path_type {
                    PathType::Namespace => "::",
                    PathType::Member => ".",
                };

                for (i, part) in path.iter().enumerate() {
                    if i > 0 {
                        self.write(separator);
                    }
                    self.write(part);
                }
            }
            Expr::Literal(lit) => self.generate_literal(lit),
            Expr::Call(callee, args) => {
                self.generate_expr(callee);
                self.write("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.generate_expr(arg);
                }
                self.write(")");
            }
            Expr::MethodCall(receiver, method, args) => {
                self.generate_expr(receiver);
                if !method.is_empty() {
                    self.write(&format!(".{}", method));
                }
                self.write("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.generate_expr(arg);
                }
                self.write(")");
            }
            Expr::Block(block) => {
                self.writeln("{");
                self.indent();
                self.generate_block(block);
                self.dedent();
                self.write("}");
            }
            Expr::Loop(block) => {
                self.writeln("loop {");
                self.indent();
                self.generate_block(block);
                self.dedent();
                self.write("}");
            }
            Expr::Break => {
                self.write("break");
            }
            Expr::Await(expr) => {
                self.generate_expr(expr);
                self.write(".await");
            }
            //进程中创建线程的调用链，暂时写死
            Expr::BuilderChain(methods) => {
                self.writeln("thread::Builder::new()");
                for method in methods {
                    match method {
                        BuilderMethod::Named(name) => {
                            self.writeln(&format!("    .name({})", name));
                        }
                        // BuilderMethod::StackSize(expr) => {
                        //     self.write("    .stack_size(");
                        //     self.generate_expr(expr);
                        //     self.writeln(" as usize)");
                        // },
                        BuilderMethod::Spawn { closure, move_kw } => {
                            self.write("    .spawn(");
                            if *move_kw {
                                self.write("move ");
                            }
                            self.generate_expr(closure);
                            self.write(")");
                        }
                    }
                }
            }
            Expr::Closure(params, body) => {
                self.write("|");
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(param);
                }
                self.write("| ");
                match body.as_ref() {
                    Expr::Block(_) => self.generate_expr(body),
                    _ => {
                        self.write("{ ");
                        self.generate_expr(body);
                        self.write(" }");
                    }
                }
            }
            Expr::Match { expr, arms } => {
                self.write("match ");
                self.generate_expr(expr);
                self.writeln(" {");
                self.indent();
                for arm in arms {
                    self.write(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.write(" if ");
                        self.generate_expr(guard);
                    }
                    self.writeln(" => {");
                    self.indent();
                    // 根据分支模式添加注释
                    if arm.pattern.starts_with("Ok(") {
                        self.writeln("// 收到消息 → 调用处理函数");
                    } else if arm.pattern.contains("TryRecvError::Empty") {
                        self.writeln("// 没有消息，不阻塞，直接跳过");
                    } else if arm.pattern.contains("TryRecvError::Disconnected") {
                        self.writeln("// 通道已关闭");
                    }
                    // 生成分支体，但不为最后的表达式添加分号
                    self.generate_match_arm_body(&arm.body);
                    self.dedent();
                    self.writeln("},");
                }
                self.dedent();
                self.write("}");
            }
            Expr::Unsafe(block) => {
                self.write("unsafe ");
                // 根据块的内容决定格式化方式
                if block.stmts.len() == 1 && block.expr.is_none() {
                    // 单语句的 unsafe 块，使用紧凑格式
                    self.write("{ ");
                    self.generate_block(block);
                    self.write(" }");
                } else {
                    // 多语句的 unsafe 块，使用展开格式
                    self.writeln("{");
                    self.indent();
                    self.generate_block(block);
                    self.dedent();
                    self.write("}");
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.write("if ");
                self.generate_expr(condition);
                self.write(" ");
                self.writeln("{");
                self.indent();
                self.generate_block(then_branch);
                self.dedent();
                self.write("}");
                

                if let Some(else_branch) = else_branch {
                    if else_branch.stmts.len() == 1 && else_branch.expr.is_none() {
                        if let Statement::Expr(inner_expr) = &else_branch.stmts[0] {
                            if matches!(inner_expr, Expr::If { .. }) {
                                self.write(" else ");
                                self.generate_expr(inner_expr);
                                return;
                            }
                        }
                    }

                    self.write(" else ");
                    self.writeln("{");
                    self.indent();
                    self.generate_block(else_branch);
                    self.dedent();
                    self.write("}");
                }
            }
            Expr::IfLet {
                pattern,
                value,
                then_branch,
                else_branch,
            } => {
                self.write("if let ");
                self.write(pattern);
                self.write(" = ");
                self.generate_expr(value);
                self.write(" {\n");
                self.indent();
                self.generate_block(then_branch);
                self.dedent();
                self.write("}");

                if let Some(else_branch) = else_branch {
                    self.write(" else {\n");
                    self.indent();
                    self.generate_block(else_branch);
                    self.dedent();
                    self.write("}");
                }
            }
            Expr::While { condition, body } => {
                self.write("while ");
                self.generate_expr(condition);
                self.writeln(" {");
                self.indent();
                self.generate_block(body);
                self.dedent();
                self.write("}");
            }
            Expr::For { var, range, body } => {
                self.write(&format!("for {} in {} ", var, range));
                self.writeln("{");
                self.indent();
                self.generate_block(body);
                self.dedent();
                self.write("}");
            }
            Expr::Reference(inner_expr, is_reference, mutable) => {
                if *is_reference {
                    self.write("&");
                }
                if *mutable {
                    self.write("mut ");
                }
                self.generate_expr(inner_expr);
            }
            Expr::BinaryOp(left, op, right) => {
                self.generate_expr(left);
                self.write(" ");
                self.write(op);
                self.write(" ");
                self.generate_expr(right);
            }
            Expr::Assign(left, right) => {
                self.generate_expr(left);
                self.write(" = ");
                self.generate_expr(right);
            }
            Expr::UnaryOp(op, expr) => {
                self.write(op);
                self.generate_expr(expr);
            }
            Expr::Index(array, index) => {
                self.generate_expr(array);
                self.write("[");
                self.generate_expr(index);
                self.write("]");
            }
            Expr::Parenthesized(expr) => {
                self.write("(");
                self.generate_expr(expr);
                self.write(")");
            }
        }
    }

    fn generate_literal(&mut self, lit: &Literal) {
        match lit {
            Literal::Int(i) => self.write(&i.to_string()),
            Literal::Float(f) => self.write(&f.to_string()),
            Literal::Str(s) => self.write(&format!("\"{}\"", s)),
            Literal::Bool(b) => self.write(&b.to_string()),
            Literal::Char(c) => self.write(&format!("'{}'", c)),
        }
    }

    fn generate_type_alias(&mut self, t: &TypeAlias) {
        for doc in &t.docs {
            self.writeln(doc);
        }
        self.writeln(&format!(
            "{}type {} = {};",
            self.visibility(&t.vis),
            t.name,
            self.type_to_string(&t.target)
        ));
        self.writeln("");
    }

    fn generate_enum(&mut self, e: &EnumDef) {
        for doc in &e.docs {
            self.writeln(doc);
        }

        if !e.derives.is_empty() {
            self.write("#[derive(");
            for (i, derive) in e.derives.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(derive);
            }
            self.writeln(")]");
        }

        self.write(&format!("{}enum {} ", self.visibility(&e.vis), e.name));

        if e.generics.is_empty() {
            self.writeln("{");
        } else {
            self.write("<");
            for (i, generic) in e.generics.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(&generic.name);
                if !generic.bounds.is_empty() {
                    self.write(": ");
                    for (j, bound) in generic.bounds.iter().enumerate() {
                        if j > 0 {
                            self.write(" + ");
                        }
                        self.write(bound);
                    }
                }
            }
            self.writeln("> {");
        }

        self.indent();
        for variant in &e.variants {
            for doc in &variant.docs {
                self.writeln(doc);
            }
            self.write(&variant.name);
            if let Some(types) = &variant.data {
                self.write("(");
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&self.type_to_string(ty));
                }
                self.write(")");
            }
            self.writeln(",");
        }
        self.dedent();
        self.writeln("}");
        self.writeln("");
    }

    fn generate_const(&mut self, c: &ConstDef) {
        for doc in &c.docs {
            self.writeln(doc);
        }
        self.write(&format!(
            "{}const {}: {} = ",
            self.visibility(&c.vis),
            c.name,
            self.type_to_string(&c.ty)
        ));
        self.generate_expr(&c.value);
        self.writeln(";");
        self.writeln("");
    }

    fn generate_use(&mut self, u: &UseStatement) {
        self.write("use ");

        // 生成路径部分 (如 "super" 或 "std::collections")
        for (i, part) in u.path.iter().enumerate() {
            if i > 0 {
                self.write("::");
            }
            self.write(part);
        }

        // 生成不同种类的use语句
        match &u.kind {
            UseKind::Simple => self.writeln(";"),
            UseKind::Glob => self.writeln("::*;"),
            UseKind::Nested(items) => {
                self.write("::{");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(item);
                }
                self.writeln("};");
            }
        }
    }

    fn generate_attribute(&mut self, attr: &Attribute) {
        self.write(&format!("#[{}", attr.name));
        if !attr.args.is_empty() {
            self.write("(");
            for (i, arg) in attr.args.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                match arg {
                    AttributeArg::Ident(id) => self.write(id),
                    AttributeArg::Literal(lit) => self.generate_literal(lit),
                    AttributeArg::KeyValue(k, v) => {
                        self.write(&format!("{} = ", k));
                        self.generate_literal(v);
                    }
                }
            }
            self.write(")");
        }
        self.writeln("]");
    }

    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::Path(path) => path.join("::"),
            Type::Named(name) => name.clone(),
            Type::Generic(name, params) => {
                let mut s = name.clone();
                s.push('<');
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&self.type_to_string(param));
                }
                s.push('>');
                s
            }
            Type::Reference(inner, is_reference, mutable) => {
                format!(
                    "{}{}{}",
                    if *is_reference { "&" } else { "" },
                    if *mutable { "mut " } else { "" },
                    self.type_to_string(inner)
                )
            }
            Type::Tuple(types) => {
                let mut s = "(".to_string();
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&self.type_to_string(ty));
                }
                if types.len() == 1 {
                    s.push(',');
                }
                s.push(')');
                s
            }
            Type::Slice(inner) => format!("[{}]", self.type_to_string(inner)),
            Type::Array(inner, size) => format!("[{}; {}]", self.type_to_string(inner), size),
            Type::Unit => "()".to_string(),
            Type::Never => "!".to_string(),
        }
    }

    fn visibility(&self, vis: &Visibility) -> String {
        match vis {
            Visibility::Public => "pub ".to_string(),
            Visibility::Private => "".to_string(),
            Visibility::Restricted(path) => format!("pub(in {}) ", path.join("::")),
            Visibility::None => "".to_string(),
        }
    }

    fn generate_lazy_static(&mut self, l: &LazyStaticDef) {
        // 文档注释
        for doc in &l.docs {
            self.writeln(doc);
        }

        // 生成 lazy_static! 宏
        self.writeln("lazy_static! {");
        self.indent();
        
        // static ref NAME: TYPE = { ... };
        self.write("static ref ");
        self.write(&l.name);
        self.write(": ");
        self.write(&self.type_to_string(&l.ty));
        self.write(" = ");
        
        // 生成初始化块，添加花括号
        self.writeln("{");
        self.indent();
        self.generate_block(&l.init);
        self.dedent();
        self.write("}");
        self.write(";");
        self.writeln("");
        
        self.dedent();
        self.writeln("}");
        self.writeln("");
    }

    fn generate_union(&mut self, u: &UnionDef) {
        // 文档注释
        for doc in &u.docs {
            self.writeln(doc);
        }

        // 派生属性
        if !u.derives.is_empty() {
            self.write("#[derive(");
            for (i, derive) in u.derives.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(derive);
            }
            self.writeln(")]");
        }

        // 联合体定义
        self.write(&format!("{}union {} ", self.visibility(&u.vis), u.name));

        if u.generics.is_empty() {
            self.writeln("{");
        } else {
            self.write("<");
            for (i, generic) in u.generics.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(&generic.name);
                if !generic.bounds.is_empty() {
                    self.write(": ");
                    for (j, bound) in generic.bounds.iter().enumerate() {
                        if j > 0 {
                            self.write(" + ");
                        }
                        self.write(bound);
                    }
                }
            }
            self.writeln("> {");
        }

        self.indent();

        // 联合体字段
        for field in &u.fields {
            self.generate_field(field);
        }

        self.dedent();
        self.writeln("}");
        self.writeln("");
    }

    // 辅助方法
    fn writeln(&mut self, s: &str) {
        self.write(s);
        self.buffer.push('\n');
    }

    fn write(&mut self, s: &str) {
        if self.buffer.ends_with('\n') || self.buffer.is_empty() {
            self.buffer.push_str(&"    ".repeat(self.indent_level));
        }
        self.buffer.push_str(s);
    }

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }
}
