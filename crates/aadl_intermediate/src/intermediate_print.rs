#![allow(clippy::only_used_in_recursion)]
use crate::intermediate_ast::*;

// Rust code generator
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

    // Main entry: generate full module code
    pub fn generate_module_code(&mut self, module: &RustModule) -> String {
        self.buffer.clear();

        for doc in &module.docs {
            self.writeln(doc);
        }
        for attr in &module.attrs {
            self.generate_attribute(attr);
        }
        if !module.docs.is_empty() || !module.attrs.is_empty() {
            self.writeln("");
        }

        // Generate module contents
        self.generate_items(&module.items);

        self.buffer.clone()
    }

    // Generate multiple items
    fn generate_items(&mut self, items: &[Item]) {
        for item in items {
            self.generate_item(item);
        }
    }

    // Generate a single item
    fn generate_item(&mut self, item: &Item) {
        match item {
            Item::Raw(raw) => self.generate_raw(raw),
            Item::Struct(s) => self.generate_struct(s),
            Item::Enum(e) => self.generate_enum(e),
            Item::Union(u) => self.generate_union(u), // New
            Item::Function(f) => self.generate_function(f),
            Item::Impl(i) => self.generate_impl(i),
            Item::Const(c) => self.generate_const(c),
            Item::TypeAlias(t) => self.generate_type_alias(t),
            Item::Use(u) => self.generate_use(u),
            Item::Mod(m) => self.generate_nested_module(m),
            Item::LazyStatic(l) => self.generate_lazy_static(l),
        }
    }

    fn generate_raw(&mut self, raw: &str) {
        for line in raw.lines() {
            self.writeln(line);
        }
    }

    fn generate_nested_module(&mut self, m: &RustModule) {
        // Generate the module declaration line
        match &m.vis {
            Visibility::Public => self.write("pub "),
            Visibility::Private => (), // Do not add a modifier for private modules
            Visibility::Restricted(paths) => self.write(&format!("pub(in {} ) ", paths.join("::"))),
            Visibility::None => (),
        }

        self.writeln(&format!("mod {} {{", m.name));
        self.indent();

        // Module-level docs and attributes
        for doc in &m.docs {
            self.writeln(doc);
        }
        for attr in &m.attrs {
            self.generate_attribute(attr);
        }

        // Module contents
        self.generate_items(&m.items);

        self.dedent();
        self.writeln("}");
        self.writeln("");
    }

    fn generate_struct(&mut self, s: &StructDef) {
        // Documentation comments
        for doc in &s.docs {
            self.writeln(doc);
        }

        // Derive attributes
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

        // Struct definition
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

        for field in &s.fields {
            self.generate_field(field);
        }

        self.dedent();
        self.writeln("}");
        self.writeln("");
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

        // Generic parameters
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

        // Trait implementation
        if let Some(trait_ty) = &i.trait_impl {
            self.write(&format!(" {} for", self.type_to_string(trait_ty)));
        }
        // Target type
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
        // Documentation comments
        for doc in &f.docs {
            self.writeln(doc);
        }

        // Attributes
        for attr in &f.attrs {
            self.generate_attribute(attr);
        }

        // Function signature
        self.write(&format!(
            "{}{}fn {}",
            self.visibility(&f.vis),
            if f.asyncness { "async " } else { "" },
            f.name
        ));

        // Parameters
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

        // Return type
        self.write(&format!(" -> {}", self.type_to_string(&f.return_type)));

        // Function body
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

    // Dedicated method for generating match arm bodies
    fn generate_match_arm_body(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.generate_statement(stmt);
        }

        if let Some(expr) = &block.expr {
            self.generate_expr(expr);
            // The last expression in a match arm should never end with a semicolon, since it is the return value
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
            Expr::Await(expr) => {
                self.generate_expr(expr);
                self.write(".await");
            }
            // The call chain for creating threads inside a process is currently hard-coded
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
                    // Add comments based on the arm pattern
                    if arm.pattern.starts_with("Ok(") {
                        self.writeln("// Message received → call handler function");
                    } else if arm.pattern.contains("TryRecvError::Empty") {
                        self.writeln("// No message; do not block, skip directly");
                    } else if arm.pattern.contains("TryRecvError::Disconnected") {
                        self.writeln("// Channel has been closed");
                    }
                    // Generate arm body, but do not add a semicolon to the final expression
                    self.generate_match_arm_body(&arm.body);
                    self.dedent();
                    self.writeln("},");
                }
                self.dedent();
                self.write("}");
            }
            Expr::Unsafe(block) => {
                self.write("unsafe ");
                // Choose formatting strategy based on block contents
                if block.stmts.len() == 1 && block.expr.is_none() {
                    // Compact formatting for a single-statement unsafe block
                    self.write("{ ");
                    self.generate_block(block);
                    self.write(" }");
                } else {
                    // Expanded formatting for a multi-statement unsafe block
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

        // Generate the path part (e.g., \"super\" or \"std::collections\")
        for (i, part) in u.path.iter().enumerate() {
            if i > 0 {
                self.write("::");
            }
            self.write(part);
        }

        // Generate different kinds of use statements
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
        // Documentation comments
        for doc in &l.docs {
            self.writeln(doc);
        }

        // Generate lazy_static! macro
        self.writeln("lazy_static! {");
        self.indent();

        // static ref NAME: TYPE = { ... };
        self.write("static ref ");
        self.write(&l.name);
        self.write(": ");
        self.write(&self.type_to_string(&l.ty));
        self.write(" = ");

        // Generate initializer block with braces
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
        // Documentation comments
        for doc in &u.docs {
            self.writeln(doc);
        }

        // Derive attributes
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

        // Union definition
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

        // Union fields
        for field in &u.fields {
            self.generate_field(field);
        }

        self.dedent();
        self.writeln("}");
        self.writeln("");
    }

    // Helper methods
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
