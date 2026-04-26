// src/ir/intermediate_ast.rs

/// 轻量级Rust抽象语法树（模块级）
#[derive(Debug,Clone)]
pub struct RustModule {
    pub name: String,
    pub docs: Vec<String>,
    pub items: Vec<Item>,
    pub attrs: Vec<Attribute>, // #[attributes]
    pub vis: Visibility, //控制结构体的可见性
    pub withs: Vec<RustWith>, // with声明
}
/// with声明
#[derive(Debug, Clone)]
pub struct RustWith {
    /// e.g. crate::aadlbook_devices
    pub path: Vec<String>,
    /// 是否 glob 引入（*）
    pub glob: bool,
}

/// 模块项定义
#[derive(Debug,Clone)]
pub enum Item {
    Struct(StructDef),
    Enum(EnumDef),
    Union(UnionDef),  
    Function(FunctionDef),
    Impl(ImplBlock),
    Const(ConstDef),
    TypeAlias(TypeAlias),
    Use(UseStatement),
    Mod(Box<RustModule>), // 嵌套模块
    LazyStatic(LazyStaticDef), // lazy_static! 宏
}

/// 结构体定义
#[derive(Debug,Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>, //(对应aadl端口)
    pub properties: Vec<StruProperty>, //存储属性
    pub generics: Vec<GenericParam>,
    pub derives: Vec<String>, // #[derive(...)]
    pub docs: Vec<String>,
    pub vis: Visibility, //控制结构体的可见性
}

/// 联合体定义
#[derive(Debug,Clone)]
pub struct UnionDef {
    pub name: String,
    pub fields: Vec<Field>, // 联合体字段
    pub properties: Vec<StruProperty>, // 存储属性
    pub generics: Vec<GenericParam>,
    pub derives: Vec<String>, // #[derive(...)]
    pub docs: Vec<String>,
    pub vis: Visibility, // 控制联合体的可见性
}

/// 枚举定义
#[derive(Debug,Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<Variant>,
    pub generics: Vec<GenericParam>,
    pub derives: Vec<String>,
    pub docs: Vec<String>,
    pub vis: Visibility,
}

/// 函数定义
#[derive(Debug,Clone)]
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

/// 实现块
#[derive(Debug,Clone)]
pub struct ImplBlock {
    pub target: Type,
    pub generics: Vec<GenericParam>,
    pub items: Vec<ImplItem>,
    pub trait_impl: Option<Type>, // 为哪个trait实现
}

/// 常量定义
#[derive(Debug,Clone)]
pub struct ConstDef {
    pub name: String,
    pub ty: Type,
    pub value: Expr,
    pub vis: Visibility,
    pub docs: Vec<String>,
}

/// 类型别名
#[derive(Debug,Clone)]
pub struct TypeAlias {
    pub name: String,
    pub target: Type,
    pub vis: Visibility,
    pub docs: Vec<String>,
}

#[derive(Debug,Clone)]
pub struct StruProperty {
    pub name: String,
    pub value: StruPropertyValue,
    pub docs: Vec<String>, // 属性文档
}
// ========== 基础类型定义 ========== //

/// 类型表示
#[derive(Debug, Clone)]
pub enum Type {
    Path(Vec<String>),           // std::vec::Vec
    Named(String),               // i32, String
    Generic(String, Vec<Type>),  // HashMap<K, V>
    Reference(Box<Type>, bool, bool),  // &mut T ,第一个bool代表是否是引用，第二个bool代码是否可变mut
    Tuple(Vec<Type>),            // (T1, T2)
    Slice(Box<Type>),            // [T]
    Array(Box<Type>, usize),     // [T; N] 固定大小数组
    Unit,                        // ()
    Never,                       // !
}

#[derive(Debug,Clone)]
pub enum PathType {
    Namespace,  // 用 :: 分隔 (如 std::thread)
    Member,     // 用 . 分隔 (如 self.field)
}

/// 表达式
#[derive(Debug,Clone)]
pub enum Expr {
    Ident(String),
    Path(Vec<String>,PathType),
    Literal(Literal),
    Call(Box<Expr>, Vec<Expr>),
    MethodCall(Box<Expr>, String, Vec<Expr>),
    Block(Block),
    Loop(Box<Block>),
    Break,
    Await(Box<Expr>),
    Closure(Vec<String>, Box<Expr>),
    BuilderChain(Vec<BuilderMethod>), // 新增：表示(进程在创建线程时)构建器链式调用
    Unsafe(Box<Block>),  // 新增 unsafe 表达式支持
    If {
        condition: Box<Expr>,
        then_branch: Block,
        else_branch: Option<Block>,
    },//条件表达式
    IfLet {
        pattern: String,
        value: Box<Expr>,
        then_branch: Block,
        else_branch: Option<Block>,
    },
    While {
        condition: Box<Expr>,
        body: Block,
    },
    For {
        var: String,
        range: String,
        body: Block,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    }, // match 表达式
    Reference(Box<Expr>,bool,bool),    // &expr,第一位标志&,第二位标志mut
    BinaryOp(Box<Expr>, String, Box<Expr>), // left op right, 如 a != 0
    UnaryOp(String, Box<Expr>), // op expr, 如 !x, -y
    Index(Box<Expr>, Box<Expr>), // expr[index], 如 array[i]
    Parenthesized(Box<Expr>), // (expr), 如 (a + b)
    Assign(Box<Expr>, Box<Expr>), // left = right, 如 state = State::S0 // 新增：lazy_static! 宏支持
}

// Match 表达式的分支
#[derive(Debug,Clone)]
pub struct MatchArm {
    pub pattern: String,
    pub guard: Option<Expr>,
    pub body: Block,
}

//区分.name()、.stack_size()等不同构建器方法
#[derive(Debug,Clone)]
pub enum BuilderMethod {
    Named(String), // 如.name("thread_name")
    //StackSize(Box<Expr>), // TODO .stack_size(expr)
    Spawn {
        closure: Box<Expr>,
        move_kw: bool, // 将move语义放在BuilderMethod中
    },
}


/// 字面量
#[derive(Debug,Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Char(char),
}


#[derive(Debug,Clone)]
pub enum StruPropertyValue {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Duration(u64, String),      // (值, 单位)
    Range(i64, i64, Option<String>), // (最小值, 最大值, 单位)
    None,                       // 表示没有属性值
    Custom(String),             //自定义类型(如 Shared)
}

/// 代码块
#[derive(Debug,Clone)]
pub struct Block {
    pub stmts: Vec<Statement>,
    pub expr: Option<Box<Expr>>,
}

/// 语句
#[derive(Debug,Clone)]
pub enum Statement {
    Let(LetStmt),
    Expr(Expr),
    Item(Box<Item>),
    Continue, // 新增：continue 语句
    Break,    // 新增：break 语句
    Comment(String), // 新增：注释语句
}

/// let绑定
#[derive(Debug,Clone)]
pub struct LetStmt {
    pub ifmut: bool,
    pub name: String,
    pub ty: Option<Type>,
    pub init: Option<Expr>,
}

// ========== 辅助类型 ========== //

#[derive(Debug,Clone)]
pub enum Visibility {
    Public,
    Private,
    Restricted(Vec<String>), // pub(in path)
    None, //trait实现时不需要可见性，跟trait保持一致
}

#[derive(Debug,Clone)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<AttributeArg>,
}

#[derive(Debug,Clone)]
pub enum AttributeArg {
    Ident(String),
    Literal(Literal),
    KeyValue(String, Literal),
}

#[derive(Debug,Clone)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub docs: Vec<String>,
    pub attrs: Vec<Attribute>,
}

#[derive(Debug,Clone)]
pub struct Variant {
    pub name: String,
    pub data: Option<Vec<Type>>, // Some for tuple variant
    pub docs: Vec<String>,
}

#[derive(Debug,Clone)]
pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<String>, // trait bounds
}

#[derive(Debug,Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug,Clone)]
pub enum ImplItem {
    Method(FunctionDef),
    AssocConst(String, Type, Expr),
    AssocType(String, Type),
}

#[derive(Debug,Clone)]
pub struct UseStatement {
    pub path: Vec<String>,
    pub kind: UseKind,
}

#[derive(Debug,Clone)]
pub enum UseKind {
    Simple,
    Glob,    // {path}::*
    Nested(Vec<String>),  // {path}::{a, b}
}

/// lazy_static! 宏定义
#[derive(Debug, Clone)]
pub struct LazyStaticDef {
    pub name: String,
    pub ty: Type,
    pub init: Block,
    pub vis: Visibility,
    pub docs: Vec<String>,
}