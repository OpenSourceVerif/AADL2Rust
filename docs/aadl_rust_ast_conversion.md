# AADL AST、Rust Light AST 与转换流程说明

本文说明本项目中三层核心结构的关系：

```text
AADL 源文件
  -> AADL parser
  -> src/ast.rs 中的 AADL AST
  -> src/aadl_ast2rust_code/* 中的 AADL -> Rust light AST 转换器
  -> crates/aadl_intermediate/src/intermediate_ast.rs 中的 Rust light AST
  -> crates/aadl_intermediate/src/intermediate_print.rs
  -> Rust 源码
```

这里的 Rust light AST 不是 `rustc` 的完整 AST，而是为了代码生成设计的一套轻量 Rust 中间表示。它只覆盖 AADL2Rust 当前需要生成的 Rust 结构。

## 1. AADL AST 结构

AADL AST 定义在：

```text
src/ast.rs
```

主要模块是：

```rust
pub mod aadl_ast_cj { ... }
```

整体层级可以概括为：

```text
Package
  -> visibility_decls: Vec<VisibilityDeclaration>
  -> public_section: Option<PackageSection>
  -> private_section: Option<PackageSection>
  -> properties: PropertyClause

PackageSection
  -> declarations: Vec<AadlDeclaration>

AadlDeclaration
  -> ComponentType
  -> ComponentImplementation
  -> ComponentTypeExtension
  -> ComponentImplementationExtension
  -> AnnexLibrary
```

也就是说，AADL 文件被解析后，最顶层是 `Package`。一个 `Package` 中有 `public` 和 `private` section，每个 section 里面放若干 `AadlDeclaration`。当前转换器主要处理两类声明：

```rust
AadlDeclaration::ComponentType(...)
AadlDeclaration::ComponentImplementation(...)
```

### 1.1 Package

`Package` 表示一个 AADL package：

```rust
pub struct Package {
    pub name: PackageName,
    pub visibility_decls: Vec<VisibilityDeclaration>,
    pub public_section: Option<PackageSection>,
    pub private_section: Option<PackageSection>,
    pub properties: PropertyClause,
}
```

字段含义：

- `name`：包名，例如 `aadlbook::PingPong`。
- `visibility_decls`：`with`、`renames` 等可见性声明。
- `public_section`：AADL `public` 区域。
- `private_section`：AADL `private` 区域。
- `properties`：package 级 property。

### 1.2 ComponentType

`ComponentType` 表示 AADL component type：

```rust
pub struct ComponentType {
    pub category: ComponentCategory,
    pub identifier: String,
    pub prototypes: PrototypeClause,
    pub features: FeatureClause,
    pub properties: PropertyClause,
    pub annexes: Vec<AnnexSubclause>,
}
```

它对应 AADL 中类似这样的定义：

```aadl
thread Sender
features
  out_port: out event data port Base_Types::Integer_32;
properties
  Dispatch_Protocol => Periodic;
end Sender;
```

关键字段：

- `category`：组件类别，比如 `Thread`、`Process`、`System`、`Data`、`Subprogram`。
- `identifier`：组件类型名。
- `features`：端口、data access、subprogram access 等特征。
- `properties`：周期、优先级、调度协议等属性。
- `annexes`：Behavior Annex 等扩展内容。

`ComponentCategory` 覆盖了 AADL 的主要 component 类别：

```rust
pub enum ComponentCategory {
    Abstract,
    Data,
    Subprogram,
    SubprogramGroup,
    Thread,
    ThreadGroup,
    Process,
    Memory,
    Processor,
    Bus,
    Device,
    VirtualProcessor,
    VirtualBus,
    System,
}
```

### 1.3 ComponentImplementation

`ComponentImplementation` 表示 AADL component implementation：

```rust
pub struct ComponentImplementation {
    pub category: ComponentCategory,
    pub name: ImplementationName,
    pub prototype_bindings: Option<PrototypeBindings>,
    pub prototypes: PrototypeClause,
    pub subcomponents: SubcomponentClause,
    pub calls: CallSequenceClause,
    pub connections: ConnectionClause,
    pub properties: PropertyClause,
    pub annexes: Vec<AnnexSubclause>,
}
```

它对应 AADL 中类似：

```aadl
process implementation Main.impl
subcomponents
  sender: thread Sender.impl;
  receiver: thread Receiver.impl;
connections
  c1: port sender.out_port -> receiver.in_port;
end Main.impl;
```

关键字段：

- `name`：实现名，由 `type_identifier` 和 `implementation_identifier` 组成，例如 `Main.impl`。
- `subcomponents`：子组件列表，例如 process 中的 thread，system 中的 process。
- `calls`：subprogram call sequence。
- `connections`：port、parameter、access connection。
- `properties`：implementation 级属性。
- `annexes`：implementation 级 annex。

### 1.4 Feature

AADL component type 的 features 被表示为：

```rust
pub enum Feature {
    Port(PortSpec),
    SubcomponentAccess(SubcomponentAccessSpec),
}
```

端口定义：

```rust
pub struct PortSpec {
    pub identifier: String,
    pub direction: PortDirection,
    pub port_type: PortType,
}
```

端口方向：

```rust
pub enum PortDirection {
    In,
    Out,
    InOut,
}
```

端口类型：

```rust
pub enum PortType {
    Data { classifier: Option<PortDataTypeReference> },
    EventData { classifier: Option<PortDataTypeReference> },
    Event,
}
```

这部分会被转换成 Rust struct 的字段。例如 AADL 的 input data port 通常会被转换成 Rust 中的 `Option<Receiver<T>>`，output data port 通常会被转换成 `Option<Sender<T>>`。

### 1.5 Subcomponent

`Subcomponent` 表示 implementation 中声明的子组件：

```rust
pub struct Subcomponent {
    pub identifier: String,
    pub category: ComponentCategory,
    pub classifier: SubcomponentClassifier,
    pub array_spec: Option<ArraySpec>,
    pub properties: Vec<Property>,
}
```

例如：

```aadl
subcomponents
  t1: thread Worker.impl;
```

会被表示为一个 `Subcomponent`，其中：

- `identifier = "t1"`
- `category = ComponentCategory::Thread`
- `classifier` 指向 `Worker.impl`

在转换阶段，process implementation 会根据这些 subcomponent 生成 Rust struct 字段，并在 `new()` 中构造对应子组件。

### 1.6 Connection

AADL connection 被表示为：

```rust
pub enum Connection {
    Port(PortConnection),
    Parameter(ParameterConnection),
    Access(AccessConnection),
}
```

其中 port connection：

```rust
pub struct PortConnection {
    pub identifier: String,
    pub source: PortEndpoint,
    pub destination: PortEndpoint,
    pub connection_direction: ConnectionSymbol,
}
```

endpoint 可以是组件端口、子组件端口、data access 等：

```rust
pub enum PortEndpoint {
    ComponentPort(String),
    SubcomponentPort {
        subcomponent: String,
        port: String,
    },
    ...
}
```

连接在 Rust 中主要会变成 channel 创建和字段赋值。例如：

```aadl
c1: port sender.out_port -> receiver.in_port;
```

大致会生成：

```rust
let c1 = crossbeam_channel::unbounded();
sender.out_port = Some(c1.0);
receiver.in_port = Some(c1.1);
```

在 light AST 中，这些语句会被表示为 `Statement::Let`、`Expr::Assign`、`Expr::Call` 等节点。

### 1.7 Property

AADL property 被表示为：

```rust
pub enum Property {
    BasicProperty(BasicPropertyAssociation),
    SubcomponentProperty(BasicPropertyAssociation),
    CallSequenceProperty(BasicPropertyAssociation),
}
```

基础 property：

```rust
pub struct BasicPropertyAssociation {
    pub identifier: PropertyIdentifier,
    pub operator: PropertyOperator,
    pub is_constant: bool,
    pub value: PropertyValue,
}
```

转换器会把一部分 property 解析成内部辅助类型 `StruPropertyValue`，例如：

```rust
Integer(i64)
Float(f64)
String(String)
Boolean(bool)
Range(i64, i64, Option<String>)
```

这些类型定义在：

```text
src/aadl_ast2rust_code/aadl_property.rs
```

注意：AADL property 不会保留到 Rust light AST 中。它会在 AADL converter 阶段被消费掉，变成 Rust 字段、初始化值、调度逻辑、CPU 绑定逻辑等。

### 1.8 Behavior Annex

Behavior Annex 相关 AST 也在 `src/ast.rs` 中，例如：

```rust
pub struct BehaviorAnnexContent {
    pub state_variables: Option<Vec<StateVariable>>,
    pub states: Option<Vec<State>>,
    pub transitions: Option<Vec<Transition>>,
}
```

核心结构：

```text
BehaviorAnnexContent
  -> state_variables
  -> states
  -> transitions

Transition
  -> source_states
  -> destination_state
  -> behavior_condition
  -> actions
```

Behavior Annex 的转换由：

```text
src/aadl_ast2rust_code/converter_annex.rs
```

负责。

## 2. Rust Light AST 结构

Rust light AST 定义在：

```text
crates/aadl_intermediate/src/intermediate_ast.rs
```

它的定位是通用 Rust 中间表示，不应该包含 AADL 专用概念。整体层级为：

```text
RustModule
  -> Vec<Item>
      -> StructDef / EnumDef / FunctionDef / ImplBlock / ConstDef / ...
          -> Type / Expr / Block / Statement / Field / Param / ...
```

### 2.1 RustModule

```rust
pub struct RustModule {
    pub name: String,
    pub docs: Vec<String>,
    pub items: Vec<Item>,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
}
```

`RustModule` 表示一个 Rust module。它包含：

- 模块名。
- 文档注释。
- module 级 attribute。
- module 级 visibility。
- module 内的所有 Rust item。

AADL package 会被转换成一个 `RustModule`：

```text
Package.name -> RustModule.name
Package declarations -> RustModule.items
```

### 2.2 Item

```rust
pub enum Item {
    Raw(String),
    Struct(StructDef),
    Enum(EnumDef),
    Union(UnionDef),
    Function(FunctionDef),
    Impl(ImplBlock),
    Const(ConstDef),
    TypeAlias(TypeAlias),
    Use(UseStatement),
    Mod(Box<RustModule>),
    LazyStatic(LazyStaticDef),
}
```

`Item` 表示 Rust 模块顶层可以出现的东西。

常见映射：

```text
AADL component type          -> Item::Struct / Item::Enum / Item::Union
AADL component implementation -> Item::Struct + Item::Impl
AADL with                   -> Item::Use
AADL runtime 依赖           -> Item::Use / Item::Raw
AADL processor schedule map -> Item::LazyStatic / Item::Function / Item::Raw
```

`Item::Raw(String)` 是一个 escape hatch，用来输出当前 light AST 尚未结构化表达的 Rust 代码，例如：

```rust
#![allow(unused_imports)]
include!(concat!(env!("OUT_DIR"), "/aadl_c_bindings.rs"));
```

### 2.3 StructDef / EnumDef / UnionDef

结构体：

```rust
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>,
    pub generics: Vec<GenericParam>,
    pub derives: Vec<String>,
    pub docs: Vec<String>,
    pub vis: Visibility,
}
```

AADL component type 和 implementation 经常会被转换成 `StructDef`。例如：

```aadl
thread Sender
features
  out_port: out event data port Base_Types::Integer_32;
end Sender;
```

可能转换成：

```rust
StructDef {
    name: "SenderThread",
    fields: vec![
        Field {
            name: "out_port",
            ty: Type::Generic(
                "Option",
                vec![Type::Generic("Sender", vec![Type::Named("i32")])]
            ),
            ...
        }
    ],
    derives: vec!["Debug"],
    vis: Visibility::Public,
    ...
}
```

枚举和 union 分别由 `EnumDef`、`UnionDef` 表示，主要用于 AADL data 类型转换。

### 2.4 Type

```rust
pub enum Type {
    Path(Vec<String>),
    Named(String),
    Generic(String, Vec<Type>),
    Reference(Box<Type>, bool, bool),
    Tuple(Vec<Type>),
    Slice(Box<Type>),
    Array(Box<Type>, usize),
    Unit,
    Never,
}
```

常见例子：

```text
i32                         -> Type::Named("i32")
std::time::Duration          -> Type::Path(["std", "time", "Duration"])
Option<Sender<i32>>          -> Type::Generic("Option", [Type::Generic("Sender", [Type::Named("i32")])])
&mut Foo                     -> Type::Reference(Box::new(Type::Named("Foo")), true, true)
()                           -> Type::Unit
```

AADL 数据类型到 Rust 类型的映射由 `AadlConverter::type_mappings` 管理。默认映射包括：

```text
boolean      -> bool
integer      -> i32
integer_8    -> i8
integer_16   -> i16
integer_32   -> i32
integer_64   -> i64
unsigned_8   -> u8
unsigned_16  -> u16
unsigned_32  -> u32
unsigned_64  -> u64
float        -> f32
float_32     -> f32
float_64     -> f64
character    -> char
string       -> String
```

### 2.5 FunctionDef / ImplBlock

函数：

```rust
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
```

`impl`：

```rust
pub struct ImplBlock {
    pub target: Type,
    pub generics: Vec<GenericParam>,
    pub items: Vec<ImplItem>,
    pub trait_impl: Option<Type>,
}
```

例如 thread implementation 通常会生成：

```text
impl Thread for FooThread {
    fn new(...) -> Self { ... }
    fn run(&mut self) -> () { ... }
}
```

在 light AST 中大致是：

```text
Item::Impl(
  ImplBlock {
    target: Type::Named("FooThread"),
    trait_impl: Some(Type::Named("Thread")),
    items: [
      ImplItem::Method(FunctionDef { name: "new", ... }),
      ImplItem::Method(FunctionDef { name: "run", ... }),
    ]
  }
)
```

### 2.6 Block / Statement / Expr

代码块：

```rust
pub struct Block {
    pub stmts: Vec<Statement>,
    pub expr: Option<Box<Expr>>,
}
```

语句：

```rust
pub enum Statement {
    Let(LetStmt),
    Expr(Expr),
    Item(Box<Item>),
    Continue,
    Break,
    Comment(String),
}
```

表达式：

```rust
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
    BuilderChain(Vec<BuilderMethod>),
    Unsafe(Box<Block>),
    If { ... },
    IfLet { ... },
    Match { ... },
    Reference(Box<Expr>, bool, bool),
    BinaryOp(Box<Expr>, String, Box<Expr>),
    UnaryOp(String, Box<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Parenthesized(Box<Expr>),
    Assign(Box<Expr>, Box<Expr>),
}
```

例如 Rust：

```rust
let c1 = crossbeam_channel::unbounded();
sender.out_port = Some(c1.0);
```

可以表示为：

```text
Statement::Let(
  LetStmt {
    name: "c1",
    init: Some(Expr::Call(
      Expr::Path(["crossbeam_channel", "unbounded"], Namespace),
      []
    ))
  }
)

Statement::Expr(
  Expr::Assign(
    Expr::Ident("sender.out_port"),
    Expr::Call(Expr::Path(["Some"], Member), [Expr::Ident("c1.0")])
  )
)
```

当前 AST 有些地方仍然用 `Expr::Ident(String)` 承载较复杂的 Rust 片段，例如 struct literal 或 `foo.bar` 形式。这是 light AST 的折中设计：在常用结构上保持结构化，遇到复杂但低风险的代码片段时允许字符串逃逸。

## 3. AADL AST 到 Rust Light AST 的转换流程

主转换器定义在：

```text
src/aadl_ast2rust_code/converter.rs
```

核心结构：

```rust
pub struct AadlConverter {
    pub type_mappings: HashMap<String, Type>,
    pub component_types: HashMap<String, ComponentType>,
    pub annex_converter: AnnexConverter,
    ...
}
```

主入口：

```rust
pub fn convert_package(&mut self, pkg: &Package) -> RustModule
```

### 3.1 总体流程

`convert_package` 的主要步骤：

```text
1. 收集 component type 信息
   collector::collect_component_types(...)

2. 收集 system/process/thread 层面的 broadcast connection 信息
   collector::collect_process_connections(...)
   collector::collect_thread_connections(...)

3. 创建 RustModule
   name = AADL package name 转 snake/lower module name
   docs = Auto-generated 注释
   items = runtime_prelude_items()

4. 把 AADL with 转成 Rust use
   convert_withs(pkg)

5. 遍历 public section declarations
   convert_declaration(...)

6. 遍历 private section declarations
   convert_declaration(...)

7. 添加 CPU scheduling 相关辅助代码
   collector::convert_cpu_schedule_mapping(...)
   collector::add_period_to_priority_function(...)

8. 返回 RustModule
```

简化代码形态：

```rust
pub fn convert_package(&mut self, pkg: &Package) -> RustModule {
    collector::collect_component_types(&mut self.component_types, pkg);
    collector::collect_process_connections(...);
    collector::collect_thread_connections(...);

    let mut module = RustModule {
        name: pkg.name.0.join("_").to_lowercase(),
        docs: vec![format!("// Auto-generated from AADL package: {}", ...)],
        items: Self::runtime_prelude_items(),
        attrs: Default::default(),
        vis: Visibility::Public,
    };

    module.items.extend(self.convert_withs(pkg));

    for decl in public_and_private_declarations {
        self.convert_declaration(decl, &mut module, pkg);
    }

    module
}
```

### 3.2 Declaration 分发

`convert_declaration` 根据 AADL declaration 类型分发：

```rust
fn convert_declaration(&mut self, decl: &AadlDeclaration, module: &mut RustModule, package: &Package) {
    match decl {
        AadlDeclaration::ComponentType(comp) => {
            module.items.extend(self.convert_component(comp, package));
        }
        AadlDeclaration::ComponentImplementation(impl_) => {
            module.items.extend(self.convert_implementation(impl_, package));
        }
        _ => {}
    }
}
```

当前主要处理：

```text
ComponentType           -> convert_component
ComponentImplementation -> convert_implementation
```

extension 和 annex library 当前大多未在主分发中展开处理。

### 3.3 ComponentType 转换

`convert_component` 根据 component category 分发到不同文件：

```rust
fn convert_component(&mut self, comp: &ComponentType, package: &Package) -> Vec<Item> {
    match comp.category {
        ComponentCategory::Data => conv_data_type::convert_data_component(...),
        ComponentCategory::Thread => conv_thread_type::convert_thread_component(...),
        ComponentCategory::Subprogram => conv_subprogram_type::convert_subprogram_component(...),
        ComponentCategory::System => conv_system_type::convert_system_component(...),
        ComponentCategory::Process => conv_process_type::convert_process_component(...),
        ComponentCategory::Device => conv_device_type::convert_device_component(...),
        _ => Vec::default(),
    }
}
```

对应文件：

```text
src/aadl_ast2rust_code/types/conv_data_type.rs
src/aadl_ast2rust_code/types/conv_thread_type.rs
src/aadl_ast2rust_code/types/conv_subprogram_type.rs
src/aadl_ast2rust_code/types/conv_system_type.rs
src/aadl_ast2rust_code/types/conv_process_type.rs
src/aadl_ast2rust_code/types/conv_device_type.rs
```

典型映射：

```text
data type       -> Rust struct / union / type alias / type mapping
thread type     -> FooThread struct
process type    -> FooProcess struct
system type     -> FooSystem struct
subprogram type -> Rust function-like component representation
device type     -> Rust struct
```

### 3.4 ComponentImplementation 转换

`convert_implementation` 根据 implementation category 分发：

```rust
fn convert_implementation(&mut self, impl_: &ComponentImplementation, package: &Package) -> Vec<Item> {
    match impl_.category {
        ComponentCategory::Process => conv_process_impl::convert_process_implementation(...),
        ComponentCategory::Thread => conv_thread_impl::convert_thread_implemenation(...),
        ComponentCategory::System => conv_system_impl::convert_system_implementation(...),
        ComponentCategory::Data => conv_data_impl::convert_data_implementation(...),
        ComponentCategory::Processor => conv_processor_impl::convert_processor_implementation(...),
        _ => Vec::default(),
    }
}
```

对应文件：

```text
src/aadl_ast2rust_code/implementations/conv_process_impl.rs
src/aadl_ast2rust_code/implementations/conv_thread_impl.rs
src/aadl_ast2rust_code/implementations/conv_system_impl.rs
src/aadl_ast2rust_code/implementations/conv_data_impl.rs
src/aadl_ast2rust_code/implementations/conv_processor_impl.rs
```

典型映射：

```text
thread implementation
  -> FooThread struct
  -> impl Thread for FooThread
       -> new()
       -> run()

process implementation
  -> FooProcess struct
  -> impl Process for FooProcess
       -> new()
       -> run()

system implementation
  -> FooSystem struct
  -> impl System for FooSystem

processor implementation
  -> CPU scheduling protocol mapping
```

## 4. 关键转换规则

### 4.1 AADL package -> RustModule

AADL:

```aadl
package Demo
public
  ...
end Demo;
```

Rust light AST：

```rust
RustModule {
    name: "demo".to_string(),
    docs: vec!["// Auto-generated from AADL package: Demo".to_string()],
    items: vec![...],
    attrs: vec![],
    vis: Visibility::Public,
}
```

### 4.2 AADL with -> Rust use

AADL:

```aadl
with Base_Types;
with My_Package;
```

转换为：

```rust
Item::Use(UseStatement {
    path: vec!["crate".to_string(), "base_types".to_string()],
    kind: UseKind::Glob,
})
```

最终打印：

```rust
use crate::base_types::*;
```

### 4.3 AADL component type -> Rust struct

以 thread type 为例，转换函数在：

```text
src/aadl_ast2rust_code/types/conv_thread_type.rs
```

主要逻辑：

```text
1. 读取 comp.features
2. 调用 convert_type_features 生成 Rust Field
3. 读取 comp.properties
4. 将部分 property 转成 Rust 字段
5. 添加 cpu_id 字段
6. 生成 StructDef
7. 包装成 Item::Struct
```

输出结构大致为：

```rust
Item::Struct(StructDef {
    name: "FooThread".to_string(),
    fields,
    generics: Vec::new(),
    derives: vec!["Debug".to_string()],
    docs,
    vis: Visibility::Public,
})
```

### 4.4 AADL feature port -> Rust field

转换函数：

```text
src/aadl_ast2rust_code/converter.rs
```

相关方法：

```rust
pub fn convert_type_features(&self, features: &FeatureClause, comp_identifier: String) -> Vec<Field>
pub fn convert_port_type(&self, port: &PortSpec, comp_identifier: String) -> Type
```

规则概括：

```text
in data/event data port
  -> Option<Receiver<T>>

out data/event data port
  -> Option<Sender<T>>

broadcast receive 场景
  -> Option<BcReceiver<T>>

broadcast send 场景
  -> Option<BcSender<T>>

event port without data
  -> T = ()
```

例如：

```aadl
input: in event data port Base_Types::Integer_32;
```

可能转换成：

```rust
Field {
    name: "input".to_string(),
    ty: Type::Generic(
        "Option".to_string(),
        vec![Type::Generic(
            "Receiver".to_string(),
            vec![Type::Named("i32".to_string())],
        )],
    ),
    ...
}
```

### 4.5 AADL data type -> Rust type

基础 AADL 类型通过 `AadlConverter::type_mappings` 映射：

```text
Base_Types::Integer_32 -> i32
Base_Types::Boolean    -> bool
Base_Types::Float_64   -> f64
```

自定义 data component 会在 data converter 中生成 Rust struct、union 或共享类型。共享 data 组件通常会被包装为：

```text
FooShared = Arc<Mutex<Foo>>
```

具体处理位于：

```text
src/aadl_ast2rust_code/types/conv_data_type.rs
src/aadl_ast2rust_code/implementations/conv_data_impl.rs
```

### 4.6 AADL properties -> Rust 字段 / 初始化 / 调度逻辑

AADL property 不作为 Rust light AST 的专用节点存在。转换器会在 AADL 层消费 property。

例如 thread type 中：

```aadl
properties
  Period => 100 ms;
  Priority => 10;
```

转换阶段会做几件事：

```text
1. parse_property_value 解析 PropertyValue
2. type_for_property 推断 Rust 字段类型
3. 在 Thread struct 中加入 period / priority 字段
4. 在 new() 中生成初始化值
5. 在 run() 中根据 period / priority 生成调度相关代码
```

相关位置：

```text
src/aadl_ast2rust_code/converter.rs
src/aadl_ast2rust_code/types/conv_thread_type.rs
src/aadl_ast2rust_code/implementations/conv_thread_impl.rs
```

### 4.7 AADL implementation subcomponents -> Rust struct fields

以 process implementation 为例：

```aadl
process implementation P.impl
subcomponents
  t1: thread Worker.impl;
  data1: data Pos.impl;
end P.impl;
```

会生成类似：

```rust
pub struct PProcess {
    pub t1: WorkerThread,
    pub data1: PosShared,
    pub cpu_id: isize,
}
```

对应转换位置：

```text
src/aadl_ast2rust_code/implementations/conv_process_impl.rs
```

其中：

```text
Thread subcomponent -> XxxThread 字段
Data subcomponent   -> XxxShared 字段
```

### 4.8 AADL connections -> Rust channel + assignment

连接转换主要在：

```text
src/aadl_ast2rust_code/converter.rs
```

相关函数：

```rust
pub fn create_channel_connection(&self, conn: &PortConnection, comp_name: String) -> Vec<Statement>
```

普通 port connection：

```text
1. 创建 channel
   let c = crossbeam_channel::unbounded();

2. source port 获得 sender
   source.port = Some(c.0);

3. destination port 获得 receiver
   destination.port = Some(c.1);
```

对应 light AST：

```text
Statement::Let(...)
Statement::Expr(Expr::Assign(...))
Statement::Expr(Expr::Assign(...))
```

broadcast connection 会使用：

```rust
tokio::sync::broadcast::channel
```

sender 使用 `channel.0.clone()`，receiver 使用 `channel.0.subscribe()`。

### 4.9 AADL calls -> Rust subprogram 调用

Thread implementation 中的 call sequence 会被转换成 `run()` 方法里的调用逻辑。

相关位置：

```text
src/aadl_ast2rust_code/implementations/conv_thread_impl.rs
```

典型来源：

```rust
CallSequenceClause
SubprogramCall
ParameterConnection
AccessConnection
```

典型输出：

```text
Expr::Call(...)
Expr::MethodCall(...)
Statement::Let(...)
Statement::Expr(...)
```

例如 subprogram execute 调用可能被降成：

```rust
SubprogramName::execute(...)
```

### 4.10 Behavior Annex -> Rust 状态机逻辑

Behavior Annex 相关转换由：

```text
src/aadl_ast2rust_code/converter_annex.rs
```

负责。输入是：

```text
AnnexSubclause
  -> AnnexContent::BehaviorAnnex(...)
      -> states
      -> transitions
      -> actions
```

输出通常是 Rust light AST 中的：

```text
EnumDef       表示状态枚举
Statement     表示状态变量和动作
Expr::Match   表示状态分派
Expr::If      表示条件
Expr::Assign  表示状态转换和变量赋值
```

## 5. Rust Light AST 到 Rust 源码

打印器定义在：

```text
crates/aadl_intermediate/src/intermediate_print.rs
```

入口：

```rust
pub fn generate_module_code(&mut self, module: &RustModule) -> String
```

打印流程：

```text
generate_module_code
  -> generate_items
      -> generate_item
          -> generate_struct
          -> generate_enum
          -> generate_function
          -> generate_impl
          -> generate_const
          -> generate_use
          -> generate_lazy_static
          -> generate_expr
          -> type_to_string
```

重要边界：

```text
converter 负责理解 AADL 语义
printer 只负责打印 Rust light AST
```

因此：

- AADL `with` 已经在 converter 中变成 `Item::Use`。
- AADL property 已经在 converter 中变成 Rust 字段、表达式或调度逻辑。
- AADL runtime imports 已经由 converter 显式插入 `RustModule.items`。
- printer 不应该知道 AADL 的概念。

## 6. 一个完整的小例子

AADL 输入概念：

```aadl
package Demo
public
  with Base_Types;

  thread Producer
  features
    out_data: out event data port Base_Types::Integer_32;
  properties
    Period => 1000 ms;
  end Producer;

  thread implementation Producer.impl
  calls
    ...
  end Producer.impl;
end Demo;
```

转换后的结构大致是：

```text
Package(Demo)
  -> RustModule(name = "demo")
      -> runtime prelude Item::Use / Item::Raw
      -> AADL with Base_Types -> Item::Use(crate::base_types::*)
      -> ComponentType Producer
          -> Item::Struct(ProducerThread)
              -> field out_data: Option<Sender<i32>>
              -> field period: u64
              -> field cpu_id: isize
      -> ComponentImplementation Producer.impl
          -> Item::Struct(ProducerThread)
          -> Item::Impl(
               impl Thread for ProducerThread {
                 fn new(cpu_id: isize) -> Self
                 fn run(&mut self) -> ()
               }
             )
```

然后 `RustCodeGenerator` 把这些节点打印成 Rust 代码。

## 7. 代码阅读建议

建议按下面顺序阅读：

```text
1. src/ast.rs
   先理解 AADL AST 的数据结构。

2. crates/aadl_intermediate/src/intermediate_ast.rs
   理解 Rust light AST 能表达什么。

3. src/aadl_ast2rust_code/converter.rs
   看 Package 到 RustModule 的总流程。

4. src/aadl_ast2rust_code/types/conv_thread_type.rs
   看 component type 如何生成 Rust struct。

5. src/aadl_ast2rust_code/implementations/conv_process_impl.rs
   看 implementation、subcomponents、connections 如何生成 new/run。

6. src/aadl_ast2rust_code/implementations/conv_thread_impl.rs
   看 thread run 逻辑、property、subprogram call 如何生成。

7. crates/aadl_intermediate/src/intermediate_print.rs
   看 Rust light AST 如何最终打印为 Rust 源码。
```

## 8. 职责边界总结

```text
src/ast.rs
  只描述 AADL 语法和语义结构。

src/aadl_ast2rust_code/*
  理解 AADL，负责把 AADL AST lowering 成 Rust light AST。

crates/aadl_intermediate/src/intermediate_ast.rs
  只描述通用 Rust 代码结构。

crates/aadl_intermediate/src/intermediate_print.rs
  只负责把 Rust light AST 打印成 Rust 源码。
```

最重要的设计原则：

```text
AADL 专用语义留在 AADL converter。
Rust 语言结构留在 Rust light AST。
代码格式化输出留在 Rust printer。
```

