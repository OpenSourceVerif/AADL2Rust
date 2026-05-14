// crates/aadl_intermediate/src/intermediate_ast.rs

/// Lightweight Rust abstract syntax tree (module level)
#[derive(Debug, Clone)]
pub struct RustModule {
    pub name: String,
    pub docs: Vec<String>,
    pub items: Vec<Item>,
    pub attrs: Vec<Attribute>, // #[attributes]
    pub vis: Visibility,       // controls the visibility of the module
}

/// Module item definitions
#[derive(Debug, Clone)]
pub enum Item {
    Raw(String), // raw Rust item or crate-level directive
    Struct(StructDef),
    Enum(EnumDef),
    Union(UnionDef),
    Function(FunctionDef),
    Impl(ImplBlock),
    Const(ConstDef),
    TypeAlias(TypeAlias),
    Use(UseStatement),
    Mod(Box<RustModule>),      // nested module
    LazyStatic(LazyStaticDef), // lazy_static! macro
}

/// Struct definition
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>,
    pub generics: Vec<GenericParam>,
    pub derives: Vec<String>, // #[derive(...)]
    pub docs: Vec<String>,
    pub vis: Visibility, // controls struct visibility
}

/// Union definition
#[derive(Debug, Clone)]
pub struct UnionDef {
    pub name: String,
    pub fields: Vec<Field>,            // union fields
    pub generics: Vec<GenericParam>,
    pub derives: Vec<String>, // #[derive(...)]
    pub docs: Vec<String>,
    pub vis: Visibility, // controls union visibility
}

/// Enum definition
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<Variant>,
    pub generics: Vec<GenericParam>,
    pub derives: Vec<String>,
    pub docs: Vec<String>,
    pub vis: Visibility,
}

/// Function definition
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub body: Block,
    pub asyncness: bool,
    pub vis: Visibility,
    pub docs: Vec<String>,
    pub attrs: Vec<Attribute>,
}

/// Implementation block
#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub target: Type,
    pub generics: Vec<GenericParam>,
    pub items: Vec<ImplItem>,
    pub trait_impl: Option<Type>, // implemented for which trait
}

/// Constant definition
#[derive(Debug, Clone)]
pub struct ConstDef {
    pub name: String,
    pub ty: Type,
    pub value: Expr,
    pub vis: Visibility,
    pub docs: Vec<String>,
}

/// Type alias
#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: String,
    pub target: Type,
    pub vis: Visibility,
    pub docs: Vec<String>,
}

// ========== Basic type definitions ========== //

/// Type representation
#[derive(Debug, Clone)]
pub enum Type {
    Path(Vec<String>),                // std::vec::Vec
    Named(String),                    // i32, String
    Generic(String, Vec<Type>),       // HashMap<K, V>
    Reference(Box<Type>, bool, bool), // &mut T, first bool indicates reference, second bool indicates mutability
    Tuple(Vec<Type>),                 // (T1, T2)
    Slice(Box<Type>),                 // [T]
    Array(Box<Type>, usize),          // [T; N] fixed-size array
    Unit,                             // ()
    Never,                            // !
}

#[derive(Debug, Clone)]
pub enum PathType {
    Namespace, // separated by :: (e.g., std::thread)
    Member,    // separated by . (e.g., self.field)
}

/// Expression
#[derive(Debug, Clone)]
pub enum Expr {
    Ident(String),
    Path(Vec<String>, PathType),
    Literal(Literal),
    Call(Box<Expr>, Vec<Expr>),
    MethodCall(Box<Expr>, String, Vec<Expr>),
    Block(Block),
    Loop(Box<Block>),
    Await(Box<Expr>),
    Closure(Vec<String>, Box<Expr>),
    BuilderChain(Vec<BuilderMethod>), // represents builder-style chained calls
    Unsafe(Box<Block>),               // unsafe expression support
    If {
        condition: Box<Expr>,
        then_branch: Block,
        else_branch: Option<Block>,
    }, // conditional expression
    IfLet {
        pattern: String,
        value: Box<Expr>,
        then_branch: Block,
        else_branch: Option<Block>,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    }, // match expression
    Reference(Box<Expr>, bool, bool), // &expr, first flag for &, second flag for mut
    BinaryOp(Box<Expr>, String, Box<Expr>), // left op right, e.g., a != 0
    UnaryOp(String, Box<Expr>),       // op expr, e.g., !x, -y
    Index(Box<Expr>, Box<Expr>),      // expr[index], e.g., array[i]
    Parenthesized(Box<Expr>),         // (expr), e.g., (a + b)
    Assign(Box<Expr>, Box<Expr>), // left = right, e.g., state = State::S0 // lazy_static! macro support
}

// Match expression arm
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: String,
    pub guard: Option<Expr>,
    pub body: Block,
}

// Distinguish different builder methods such as .name(), .stack_size(), etc.
#[derive(Debug, Clone)]
pub enum BuilderMethod {
    Named(String), // e.g., .name("thread_name")
    //StackSize(Box<Expr>), // TODO .stack_size(expr)
    Spawn {
        closure: Box<Expr>,
        move_kw: bool, // move semantics handled in BuilderMethod
    },
}

/// Literal
#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Char(char),
}

/// Code block
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Statement>,
    pub expr: Option<Box<Expr>>,
}

/// Statement
#[derive(Debug, Clone)]
pub enum Statement {
    Let(LetStmt),
    Expr(Expr),
    Item(Box<Item>),
    Continue,        // continue statement
    Break,           // break statement
    Comment(String), // comment statement
}

/// let binding
#[derive(Debug, Clone)]
pub struct LetStmt {
    pub ifmut: bool,
    pub name: String,
    pub ty: Option<Type>,
    pub init: Option<Expr>,
}

// ========== Auxiliary types ========== //

#[derive(Debug, Clone)]
pub enum Visibility {
    Public,
    Private,
    Restricted(Vec<String>), // pub(in path)
    None,                    // no visibility needed for trait implementations, follows the trait
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<AttributeArg>,
}

#[derive(Debug, Clone)]
pub enum AttributeArg {
    Ident(String),
    Literal(Literal),
    KeyValue(String, Literal),
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub docs: Vec<String>,
    pub attrs: Vec<Attribute>,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub data: Option<Vec<Type>>, // Some for tuple variant
    pub docs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<String>, // trait bounds
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub enum ImplItem {
    Method(FunctionDef),
    AssocConst(String, Type, Expr),
    AssocType(String, Type),
}

#[derive(Debug, Clone)]
pub struct UseStatement {
    pub path: Vec<String>,
    pub kind: UseKind,
}

#[derive(Debug, Clone)]
pub enum UseKind {
    Simple,
    Glob,                // {path}::*
    Nested(Vec<String>), // {path}::{a, b}
}

/// lazy_static! macro definition
#[derive(Debug, Clone)]
pub struct LazyStaticDef {
    pub name: String,
    pub ty: Type,
    pub init: Block,
    pub vis: Visibility,
    pub docs: Vec<String>,
}
