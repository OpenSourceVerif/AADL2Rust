# Rust 中间 IR 解耦重构说明

## 修改目标

本次重构的目标是：把通用 Rust 中间表示和 Rust 代码打印器从 AADL 专有语义中解耦出来。

修改前，`intermediate_ast.rs` 和 `intermediate_print.rs` 名义上分别是 Rust IR 和 Rust printer，但里面仍然混有 AADL 专有概念，例如：

- `RustWith`
- AADL `properties`
- AADL runtime 默认 imports
- AADL C binding 的 `include!`
- 根据 `send` / `receive` 方法名特殊打印连接赋值

修改后，这两个文件的定位变成：

```text
SysML AST      \
AADL AST        -> intermediate_ast.rs 中的 Rust IR -> RustCodeGenerator -> Rust 源码
Isabelle AST   /
```

也就是说，未来如果做 `sysml2rust` 或 `isabelle2rust`，只需要把各自的 AST 转换到这套 Rust IR，就可以复用 `intermediate_print.rs` 生成 Rust 代码。

前端语言相关逻辑应该放在各自 converter 里，而不是放在 Rust IR 或 printer 里。

## 修改文件总览

### `src/aadl_ast2rust_code/intermediate_ast.rs`

核心修改：让该文件只描述 Rust 语言结构。

#### 删除 `RustWith`

修改前：

```rust
pub struct RustModule {
    ...
    pub withs: Vec<RustWith>,
}

pub struct RustWith {
    pub path: Vec<String>,
    pub glob: bool,
}
```

问题：

`RustWith` 实际上对应的是 AADL 的：

```aadl
with aadlbook::Icd;
```

这不是 Rust AST 的概念，而是 AADL import 语义。

修改后：

AADL converter 直接把 AADL `with` 降低成普通 Rust `use`：

```rust
Item::Use(UseStatement {
    path: vec!["crate".to_string(), "aadlbook_icd".to_string()],
    kind: UseKind::Glob,
})
```

最后 printer 只负责打印：

```rust
use crate::aadlbook_icd::*;
```

这样 `intermediate_ast.rs` 不再需要知道 AADL `with`。

#### 删除 `StructDef.properties` 和 `UnionDef.properties`

修改前：

```rust
pub struct StructDef {
    pub properties: Vec<StruProperty>,
}

pub struct UnionDef {
    pub properties: Vec<StruProperty>,
}
```

问题：

Rust 的 `struct` 和 `union` 没有 AADL property 这个概念。

例如 AADL 中的：

```aadl
Period => 2000 ms;
```

如果它影响 Rust 代码，应该在 AADL converter 阶段被消费掉，然后变成真正的 Rust 字段、常量、函数或表达式，例如：

```rust
pub period_ms: u64
```

而不是继续保存在 `StructDef.properties` 里。

修改后：

`StructDef` 和 `UnionDef` 只保留 Rust 自身结构：

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

#### 新增 `Item::Raw(String)`

新增：

```rust
Item::Raw(String)
```

含义：

表示一段需要原样输出的 Rust 顶层代码。

为什么需要它：

当前 Rust IR 还没有完整覆盖所有 Rust 顶层语法，比如：

```rust
#![allow(unused_imports)]
include!(concat!(env!("OUT_DIR"), "/aadl_c_bindings.rs"));
```

这些以前是 `intermediate_print.rs` 直接硬编码输出的。这样会让 printer 被 AADL 工程绑定。

现在改成由 converter 显式插入：

```rust
Item::Raw("#![allow(unused_imports)]".to_string())
```

这是一种通用 Rust escape hatch，不是 AADL 专用设计。其他前端也可以按需使用，或者完全不用。

### `src/aadl_ast2rust_code/aadl_property.rs`

核心修改：把 AADL property 辅助类型从通用 Rust IR 中移出来。

新增文件：

```text
src/aadl_ast2rust_code/aadl_property.rs
```

内容：

```rust
pub struct StruProperty {
    pub name: String,
    pub value: StruPropertyValue,
    pub docs: Vec<String>,
}

pub enum StruPropertyValue {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Duration(u64, String),
    Range(i64, i64, Option<String>),
    None,
    Custom(String),
}
```

原因：

这些类型仍然对 AADL 转换有用。例如 thread 的周期、优先级、调度协议等属性，在 converter 阶段需要解析成 typed value。

但是它们不是 Rust IR 节点，所以不应该放在 `intermediate_ast.rs`。

移动后职责变成：

```text
intermediate_ast.rs  -> 通用 Rust IR
aadl_property.rs     -> AADL converter 使用的属性辅助类型
```

### `src/aadl_ast2rust_code/mod.rs`

核心修改：暴露新的 AADL property 模块。

新增：

```rust
pub mod aadl_property;
```

原因：

`converter.rs`、`conv_thread_type.rs`、`conv_thread_impl.rs` 仍然需要使用 `StruPropertyValue`，所以现在通过：

```rust
use crate::aadl_ast2rust_code::aadl_property::*;
```

来导入。

### `src/aadl_ast2rust_code/converter.rs`

核心修改：把 AADL 专用 lowering 责任放回 AADL converter。

#### 引入 AADL property 模块

新增：

```rust
use crate::aadl_ast2rust_code::aadl_property::*;
```

原因：

`StruProperty` 和 `StruPropertyValue` 已经从通用 Rust IR 中移走，AADL 属性解析应该明确属于 AADL converter 层。

#### 修改 `RustModule` 构造逻辑

修改前：

```rust
items: Default::default(),
withs: self.convert_withs(pkg),
```

修改后：

```rust
items: Self::runtime_prelude_items(),
...
module.items.extend(self.convert_withs(pkg));
```

含义：

AADL2Rust 所需的默认 runtime imports 不再由 printer 自动生成，而是由 AADL converter 显式插入到 `RustModule.items` 中。

#### 新增 `runtime_prelude_items`

新增函数：

```rust
fn runtime_prelude_items() -> Vec<Item>
```

它负责生成 AADL2Rust 当前需要的 runtime 依赖，例如：

```rust
use crossbeam_channel::{Receiver, Sender};
use crate::common_traits::*;
use crate::posix::*;
include!(concat!(env!("OUT_DIR"), "/aadl_c_bindings.rs"));
```

这些内容以前写死在 printer 里，现在属于 AADL converter 的输出。

这样做的好处：

- `intermediate_print.rs` 不再自动注入 AADL runtime
- 未来 `sysml2rust` 不需要继承这些 AADL 依赖
- 不同前端可以自己决定需要哪些 runtime imports

#### 修改 `convert_withs`

修改前：

```rust
fn convert_withs(&self, pkg: &Package) -> Vec<RustWith>
```

修改后：

```rust
fn convert_withs(&self, pkg: &Package) -> Vec<Item>
```

也就是说，AADL `with` 不再转换成专门的 `RustWith`，而是直接转换成普通 Rust `Item::Use`。

例子：

```aadl
with aadlbook::Icd;
```

转换为：

```rust
Item::Use(UseStatement {
    path: vec!["crate".to_string(), "aadlbook_icd".to_string()],
    kind: UseKind::Glob,
})
```

最终打印为：

```rust
use crate::aadlbook_icd::*;
```

#### 清理连接赋值逻辑

新增辅助函数：

```rust
fn some_expr(value: Expr) -> Expr
fn assign_statement(target: String, value: Expr) -> Statement
```

原因：

以前 printer 中有特殊逻辑：

```rust
if method == "send" || method == "receive" {
    // 打印成赋值
}
```

这不是通用 Rust printer 应该做的事情。普通 Rust 方法调用 `.send(...)` 不应该被 printer 根据名字改写。

现在改成 converter 显式生成赋值表达式：

```rust
Expr::Assign(...)
```

例如：

```rust
camera.picture = Some(channel.0);
```

printer 只负责正常打印 `Expr::Assign`。

### `src/aadl_ast2rust_code/intermediate_print.rs`

核心修改：printer 只打印 Rust IR，不再生成 AADL 专用内容。

#### 删除硬编码 AADL 输出

删除了原来自动输出的内容：

```rust
// Auto-generated from AADL package: ...
// Generation time: ...
use crate::common_traits::*;
use crate::posix::*;
include!(concat!(env!("OUT_DIR"), "/aadl_c_bindings.rs"));
```

原因：

这些都是 AADL2Rust 的工程选择，不是通用 Rust printer 的职责。

修改后：

printer 只打印 `RustModule` 自己携带的内容：

```rust
module.docs
module.attrs
module.items
```

#### 删除 `generate_withs`

删除：

```rust
fn generate_withs(...)
```

原因：

AADL `with` 已经在 `converter.rs` 中降低成普通 `Item::Use`，printer 不再需要认识 `with`。

#### 删除 property 打印函数

删除：

```rust
generate_properties_impl(...)
type_for_property(...)
```

原因：

`StructDef` 和 `UnionDef` 已经不再包含 AADL properties。AADL property 的处理现在发生在 converter 阶段。

#### 增加 `Item::Raw` 打印

新增：

```rust
Item::Raw(raw) => self.generate_raw(raw)
```

含义：

如果 Rust IR 中显式包含一段 raw Rust 代码，printer 就原样输出。

#### 删除 `send` / `receive` 特判

删除了类似逻辑：

```rust
if method == "send" || method == "receive" {
    ...
}
```

原因：

printer 不应该根据方法名推断业务语义。连接赋值现在由 AADL converter 生成 `Expr::Assign`。

### `src/aadl_ast2rust_code/merge_utils.rs`

核心修改：删除已经不存在的 property 合并逻辑。

修改前会合并：

```rust
target.properties
source.properties
```

修改后只合并 Rust struct 的通用部分：

```text
fields
generics
derives
visibility
```

原因：

`StructDef.properties` 已经删除，merge 工具不应该再处理 AADL property。

### `src/aadl_ast2rust_code/types/*.rs`

核心修改：更新所有 `StructDef` 和 `UnionDef` 构造代码。

删除类似字段：

```rust
properties: Vec::new(),
properties: vec![],
properties: temp_converter.convert_properties(...)
```

原因：

Rust IR 的 struct / union 不再保存 AADL property。

涉及的文件包括：

- `conv_data_type.rs`
- `conv_device_type.rs`
- `conv_process_type.rs`
- `conv_subprogram_type.rs`
- `conv_system_type.rs`
- `conv_thread_type.rs`

### `src/aadl_ast2rust_code/implementations/*.rs`

核心修改：同样更新 implementation 侧的 `StructDef` / `UnionDef` 构造代码。

删除类似字段：

```rust
properties: Vec::new(),
properties: vec![],
```

涉及的文件包括：

- `conv_data_impl.rs`
- `conv_process_impl.rs`
- `conv_system_impl.rs`
- `conv_thread_impl.rs`

其中 thread 相关 converter 仍然保留属性值逻辑，但不再通过 `StructDef.properties` 传给 Rust IR，而是保存在 `AadlConverter.thread_field_values` 等 AADL converter 内部结构中。

## 修改前后的逻辑差异

### 修改前

```text
intermediate_ast.rs
  - 保存 RustModule.withs
  - 定义 RustWith
  - 保存 StructDef.properties
  - 定义 StruProperty / StruPropertyValue

intermediate_print.rs
  - 自动输出 AADL runtime imports
  - 自动输出 aadl_c_bindings include
  - 理解 AADL with
  - 保留 AADL property 初始化逻辑
  - 根据 send/receive 方法名改写打印结果
```

问题：

`intermediate_ast.rs` 和 `intermediate_print.rs` 无法作为真正通用的 Rust IR / printer 复用。

如果未来接入 `sysml2rust` 或 `isabelle2rust`，它们会被迫继承 AADL 的 import、property、runtime 假设。

### 修改后

```text
intermediate_ast.rs
  - 只描述 Rust module/item/type/expr/stmt
  - 不知道 AADL with
  - 不知道 AADL property
  - 不知道 AADL runtime

intermediate_print.rs
  - 只打印 Rust IR
  - 不主动注入任何 AADL 代码
  - 不根据方法名猜业务语义
```

AADL 专用逻辑现在放在：

```text
aadl_property.rs
converter.rs
types/conv_*.rs
implementations/conv_*.rs
```

也就是说：

```text
AADL-specific lowering -> AadlConverter
Generic Rust structure  -> intermediate_ast.rs
Generic Rust printing   -> intermediate_print.rs
```

## 未来前端如何复用

未来如果实现 `sysml2rust`，推荐结构是：

```text
SysML source
  -> SysML AST
  -> SysMLConverter
  -> RustModule / Item / Type / Expr / Statement
  -> RustCodeGenerator
  -> Rust source
```

如果实现 `isabelle2rust`，也是类似：

```text
Isabelle source
  -> Isabelle AST
  -> IsabelleConverter
  -> RustModule / Item / Type / Expr / Statement
  -> RustCodeGenerator
  -> Rust source
```

关键原则：

如果某个逻辑只属于某个源语言，放在该语言 converter 中。

如果某个逻辑属于 Rust 语言结构，才放进 `intermediate_ast.rs`。

如果某个逻辑只是如何把 Rust IR 打印成 Rust 代码，才放进 `intermediate_print.rs`。

## 测试结果

本次修改后执行了：

```text
cargo check
```

结果：

```text
OK
```

执行了全案例生成 smoke test：

```text
cargo test --test all_aadl_models -- --nocapture
```

结果：

```text
57 cases passed
0 failed
```

还测试了重新生成 `car` 案例：

```text
cargo run -- --input car
```

结果：

```text
OK
```

额外尝试检查生成后的 `generate/project/car`：

```text
cargo check
```

该检查在 Windows 下失败，原因是生成 runtime 使用了 Unix/POSIX 专用的 `libc` 符号，例如：

```text
syscall
SYS_gettid
pthread_self
sched_setaffinity
```

这些符号在 Windows target 下不存在。这属于生成 runtime 的平台兼容性问题，不是本次 Rust IR / printer 解耦造成的问题。

## 总结

本次重构之后，整体边界变成：

```text
AADL AST
  -> AADL converter 负责处理 AADL with / property / runtime
  -> 生成通用 Rust IR
  -> Rust printer 只打印 Rust IR
```

这使得 `intermediate_ast.rs` 和 `intermediate_print.rs` 更接近真正的通用 Rust 后端，可以服务于后续更多源语言到 Rust 的编译工作。
