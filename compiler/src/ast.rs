#![allow(clippy::all)]
#[allow(dead_code)]
pub mod aadl_ast_cj {

    /* ========== 4.2 Package ========== */
    // 包名（双冒号分隔的标识符序列）
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct PackageName(pub Vec<String>);

    //双冒号分隔的包名
    impl PackageName {
        pub fn to_string(&self) -> String {
            self.0.join("::")
        }
    }

    // 包可见性声明（with/renames）
    #[derive(Debug, Clone)]
    pub enum VisibilityDeclaration {
        // with package1, package2, property_set;对应 `with` 语法
        Import {
            packages: Vec<PackageName>,
            property_sets: Vec<String>,
        },
        // renames package::component;对应 `renames` 语法
        Alias {
            new_name: String,
            original: QualifiedName,
            is_package: bool, // 区分包别名和组件别名
        },
        // renames package::all;
        ImportAll(PackageName),
    }

    // 可能带命名空间前缀的引用
    #[derive(Debug, Clone)]
    pub struct QualifiedName {
        pub package_prefix: Option<PackageName>,
        pub identifier: String,
    }

    // 包声明部分（公共/私有）
    #[derive(Debug, Clone)]
    pub struct PackageSection {
        pub is_public: bool,
        pub declarations: Vec<AadlDeclaration>,
    }

    // 包属性声明
    #[derive(Debug, Clone)]
    pub enum PropertyClause {
        ExplicitNone, // none;
        Properties(Vec<Property>),
    }

    // 完整包定义
    #[derive(Debug, Clone)]
    pub struct Package {
        pub name: PackageName,
        pub visibility_decls: Vec<VisibilityDeclaration>, //声明当前包与其他包或属性集之间的依赖关系
        pub public_section: Option<PackageSection>,
        pub private_section: Option<PackageSection>,
        pub properties: PropertyClause, //暂时例子中，为空
    }

    #[derive(Debug, Clone)]
    pub enum AadlDeclaration {
        ComponentType(ComponentType),
        ComponentTypeExtension(ComponentTypeExtension),
        ComponentImplementation(ComponentImplementation),
        ComponentImplementationExtension(ComponentImplementationExtension),
        AnnexLibrary(AnnexLibrary), //...
    }

    #[derive(Debug)]
    //合并类型，为了在某些函数的入口参数处起到“多态”的效果
    pub enum ComponentRef<'a> {
        Type(&'a ComponentType),
        Impl(&'a ComponentImplementation),
    }

    /* ========== 4.3 Component Types ========== */
    // 组件类型定义
    #[derive(Debug, Clone)]
    pub struct ComponentType {
        pub category: ComponentCategory,
        pub identifier: String,
        pub prototypes: PrototypeClause, //“原型”，暂没见过
        pub features: FeatureClause,
        //pub flows: FlowClause,
        //pub modes: Option<ModesClause>,
        pub properties: PropertyClause,
        pub annexes: Vec<AnnexSubclause>,
    }

    //sTODO 4.8 Annex Subclauses and Annex Libraries
    #[derive(Debug, Clone)]
    pub struct AnnexLibrary {}
    
    /// AADL 扩展附件子句
    /// 对应标准中的 `annex_subclause`
    #[derive(Debug, Clone)]
    pub struct AnnexSubclause {
        /// 附件标识符 (annex_identifier)
        pub identifier: AnnexIdentifier,
        /// 附件内容 (annex_specific_language_constructs | none)
        pub content: AnnexContent,
    }
    
    /// 附件标识符
    /// 目前只支持 behavior_specification 和 EMV2
    #[derive(Debug, Clone, PartialEq)]
    pub enum AnnexIdentifier {
        BehaviorSpecification,
        EMV2,
    }
    
    /// 附件内容
    #[derive(Debug, Clone)]
    pub enum AnnexContent {
        /// 显式声明 none
        None,
        /// 附件特定语言构造
        // 对应 {** annex_specific_language_constructs **}
        // LanguageConstructs(String),
        /// Behavior Annex 特定内容
        BehaviorAnnex(BehaviorAnnexContent),
    }
    
    /// Behavior Annex 内容结构
    /// 对应标准中的 BA 语法结构
    #[derive(Debug, Clone)]
    pub struct BehaviorAnnexContent {
        // 状态变量声明 (可选)
        pub state_variables: Option<Vec<StateVariable>>,
        // 初始化部分 (可选)
        // pub initialization: Option<Initialization>,
        // 状态定义 (可选)
        pub states: Option<Vec<State>>,
        // 转换定义 (可选)
        pub transitions: Option<Vec<Transition>>,
        // 连接定义 (可选)
        // pub connections: Option<Vec<BAConnection>>,
        // 复合声明 (可选，可多个)
        // pub composite_declarations: Vec<CompositeDeclaration>,
    }
    
    /// 状态变量声明
    #[derive(Debug, Clone)]
    pub struct StateVariable {
        pub identifier: String,
        pub data_type: String, // 数据类型
        pub initial_value: Option<String>, // 初始值（可选）
    }
    
    /// 初始化部分
    // #[derive(Debug, Clone)]
    // pub struct Initialization {
    //     pub statements: Vec<String>, // 初始化语句列表
    // }
    
    /// 状态定义
    /// 对应标准中的 states 语法
    #[derive(Debug, Clone)]
    pub struct State {
        /// 状态标识符列表 (identifier (, identifier)*)
        pub identifiers: Vec<String>,
        /// 状态修饰符 (initial | complete | return | urgent | composite | exit)*
        pub modifiers: Vec<StateModifier>,
    }
    
    /// 状态修饰符
    #[derive(Debug, Clone, PartialEq)]
    pub enum StateModifier {
        Initial,    // initial
        Complete,   // complete
        //Return,     // return
        //Urgent,     // urgent
        //Composite,  // composite
        //Exit,       // exit
        Final,      // final
    }
    
    /// 转换定义
    /// 对应标准中的 behavior_transition 语法
    #[derive(Debug, Clone)]
    pub struct Transition {
        /// 转换标识符 (可选)
        pub transition_identifier: Option<String>,
        /// 转换优先级 (可选)
        pub priority: Option<String>,
        /// 源状态标识符列表 (支持多个源状态)
        pub source_states: Vec<String>,
        /// 目标状态标识符
        pub destination_state: String,
        /// 行为条件 (可选)
        pub behavior_condition: Option<BehaviorCondition>,
        /// 动作列表 (可选)
        pub actions: Option<BehaviorActionBlock>,
    }
    
    /// 行为条件
    /// 对应标准中的 behavior_condition 语法
    #[derive(Debug, Clone)]
    pub enum BehaviorCondition {
        /// dispatch_condition,特指“on dispatch”
        Dispatch(DispatchCondition),
        /// execute_condition,特指not + identifier端口的情况,自定义了not
        Execute(DispatchConjunction),
    }
    
    /// 执行条件
    /// 对应标准中的 execute_condition 语法
    #[derive(Debug, Clone)]
    pub enum ExecuteCondition {
        /// logical_value_expression
        LogicalExpression(BehaviorExpression),
        /// behavior_action_block_timeout_catch (暂时忽略timeout相关)
        ActionBlockTimeoutCatch,
        /// otherwise
        Otherwise,
    }
    
    /// 分发条件
    /// 对应标准中的 dispatch_condition 语法
    #[derive(Debug, Clone)]
    pub struct DispatchCondition {
        /// dispatch_trigger_condition
        pub trigger_condition: Option<DispatchTriggerCondition>,
        /// frozen_ports (可选)
        pub frozen_ports: Option<Vec<String>>,
    }
    
    /// 分发触发条件
    /// 对应标准中的 dispatch_trigger_condition 语法
    #[derive(Debug, Clone)]
    pub enum DispatchTriggerCondition {
        /// dispatch_trigger_logical_expression
        LogicalExpression(DispatchTriggerLogicalExpression),
        /// provides_subprogram_access_identifier
        SubprogramAccess(String),
        /// stop
        Stop,
        /// completion_relative_timeout_condition_and_catch (暂时忽略timeout相关)
        CompletionTimeout,
        /// dispatch_relative_timeout_catch (暂时忽略timeout相关)
        DispatchTimeout,
    }
    
    /// 分发触发逻辑表达式
    /// 对应标准中的 dispatch_trigger_logical_expression 语法
    #[derive(Debug, Clone)]
    pub struct DispatchTriggerLogicalExpression {
        pub dispatch_conjunctions: Vec<DispatchConjunction>,
    }
    
    /// 分发合取表达式
    /// 后面说法错误的（已经不属于“调度条件了”）：对应标准中的 dispatch_conjunction 语法,自定义了not
    #[derive(Debug, Clone)]
    pub struct DispatchConjunction {
        pub not: bool,
        pub dispatch_triggers: Vec<DispatchTrigger>,
        pub number: Option<String>,
        pub less_than: bool,
    }
    
    /// 分发触发器
    /// 对应标准中的 dispatch_trigger 语法
    #[derive(Debug, Clone)]
    pub enum DispatchTrigger {
        /// in_event_port_identifier
        InEventPort(String),
        /// in_event_data_port_identifier
        InEventDataPort(String),
    }
    
    /// 守卫条件
    /// 对应标准中的 guard 语法
    // #[derive(Debug, Clone)]
    // pub enum Guard {
    //     /// [on <expression> -->] <event> [when <expression>]
    //     EventGuard {
    //         on_expression: Option<BehaviorExpression>,
    //         event: String,
    //         when_expression: Option<BehaviorExpression>,
    //     },
    //     /// <expression>
    //     Expression(BehaviorExpression),
    // }
    
    /// 行为动作块
    /// behavior_action_block ::= { behavior_actions } [ timeout behavior_time ]
    #[derive(Debug, Clone)]
    pub struct BehaviorActionBlock {
        pub actions: BehaviorActions,
        pub timeout: Option<BehaviorTime>,
    }

    /// 行为动作
    /// behavior_actions ::= behavior_action | behavior_action_sequence | behavior_action_set
    #[derive(Debug, Clone)]
    pub enum BehaviorActions {
        Single(Box<BehaviorAction>), // 使用 Box 避免递归
        Sequence(BehaviorActionSequence),
        Set(BehaviorActionSet),
    }

    /// 行为动作序列
    /// behavior_action_sequence ::= behavior_action { ; behavior_action }+
    #[derive(Debug, Clone)]
    pub struct BehaviorActionSequence {
        pub actions: Vec<BehaviorAction>,
    }

    /// 行为动作集合
    /// behavior_action_set ::= behavior_action { & behavior_action }+
    #[derive(Debug, Clone)]
    pub struct BehaviorActionSet {
        pub actions: Vec<BehaviorAction>,
    }

    /// 行为动作
    /// behavior_action ::= basic_action | behavior_action_block | if_statement | for_statement | forall_statement | while_statement | do_until_statement
    #[derive(Debug, Clone)]
    pub enum BehaviorAction {
        Basic(BasicAction),
        Block(Box<BehaviorActionBlock>), // 使用 Box 避免递归
        If(IfStatement),
        For(ForStatement),
        Forall(ForallStatement),
        While(WhileStatement),
        DoUntil(DoUntilStatement),
    }

    /// if 语句
    /// if ( logical_value_expression ) behavior_actions { elsif ( logical_value_expression ) behavior_actions }* [ else behavior_actions ] end if
    #[derive(Debug, Clone)]
    pub struct IfStatement {
        pub condition: BehaviorExpression,
        pub then_actions: Box<BehaviorActions>, // 使用 Box 避免递归
        pub elsif_branches: Vec<ElsifBranch>,
        pub else_actions: Option<Box<BehaviorActions>>, // 使用 Box 避免递归
    }

    /// elsif 分支
    #[derive(Debug, Clone)]
    pub struct ElsifBranch {
        pub condition: BehaviorExpression,
        pub actions: Box<BehaviorActions>, // 使用 Box 避免递归
    }

    /// for 语句
    /// for ( element_identifier : data_unique_component_classifier_reference in element_values ) { behavior_actions }
    #[derive(Debug, Clone)]
    pub struct ForStatement {
        pub element_identifier: String,
        pub data_classifier: String,
        pub element_values: ElementValues,
        pub actions: Box<BehaviorActions>, // 使用 Box 避免递归
    }

    /// forall 语句
    /// forall ( element_identifier : data_unique_component_classifier_reference in element_values ) { behavior_actions }
    #[derive(Debug, Clone)]
    pub struct ForallStatement {
        pub element_identifier: String,
        pub data_classifier: String,
        pub element_values: ElementValues,
        pub actions: Box<BehaviorActions>, // 使用 Box 避免递归
    }

    /// while 语句
    /// while ( logical_value_expression ) { behavior_actions }
    #[derive(Debug, Clone)]
    pub struct WhileStatement {
        pub condition: BehaviorExpression,
        pub actions: Box<BehaviorActions>, // 使用 Box 避免递归
    }

    /// do-until 语句
    /// do behavior_actions until ( logical_value_expression )
    #[derive(Debug, Clone)]
    pub struct DoUntilStatement {
        pub actions: Box<BehaviorActions>, // 使用 Box 避免递归
        pub condition: BehaviorExpression,
    }

    /// 元素值
    /// element_values ::= integer_range | event_data_port_name | array_data_component_reference
    #[derive(Debug, Clone)]
    pub enum ElementValues {
        IntegerRange(IntegerRange),
        EventDataPort(String),
        ArrayDataComponent(String),
    }

    /// 基础动作
    /// basic_action ::= assignment_action | communication_action | timed_action
    #[derive(Debug, Clone)]
    pub enum BasicAction {
        Assignment(AssignmentAction),
        Communication(CommunicationAction),
        Timed(TimedAction),
    }

    /// 赋值动作
    /// assignment_action ::= target := ( value_expression | any )
    #[derive(Debug, Clone)]
    pub struct AssignmentAction {
        pub target: Target,
        pub value: AssignmentValue,
    }

    /// 赋值值
    #[derive(Debug, Clone)]
    pub enum AssignmentValue {
        Expression(ValueExpression),
        Any,
    }

    /// 通信动作
    /// communication_action ::= subprogram_call | port_communication | data_access_communication | broadcast
    #[derive(Debug, Clone)]
    pub enum CommunicationAction {
        //SubprogramCall(SubprogramCall),
        PortCommunication(PortCommunication),
        DataAccessCommunication(DataAccessCommunication),
        Broadcast(Broadcast),
    }

    /// 子程序调用 TODO
    /// subprogram_call ::= subprogram_prototype_name ! [ ( subprogram_parameter_list ) ] | required_subprogram_access_name ! [ ( subprogram_parameter_list ) ] | subprogram_subcomponent_name ! [ ( subprogram_parameter_list ) ] | subprogram_unique_component_classifier_reference ! [ ( subprogram_parameter_list ) ]
    // #[derive(Debug, Clone)]
    // pub struct SubprogramCall {
    //     pub name: String,
    //     pub parameters: Option<SubprogramParameterList>,
    // }

    /// 端口通信
    /// port_communication ::= output_port_name ! [ ( value_expression ) ] | input_port_name >> | input_port_name ? [ ( target ) ]
    #[derive(Debug, Clone)]
    pub enum PortCommunication {
        Output {
            port: String,
            value: Option<ValueExpression>,
        },
        InputReceive(String),
        InputCheck {
            port: String,
            target: Option<Target>,
        },
    }

    /// 数据访问通信
    /// data_access_communication ::= required_data_access_name !< | required_data_access_name !> | required_data_access_name . provided_subprogram_access_name ! [ ( subprogram_parameter_list ) ]
    #[derive(Debug, Clone)]
    pub enum DataAccessCommunication {
        RequiredDataAccess {
            name: String,
            direction: DataAccessDirection,
        },
        RequiredDataAccessSubprogram {
            data_access: String,
            subprogram: String,
            parameters: Option<SubprogramParameterList>,
        },
    }

    /// 数据访问方向
    #[derive(Debug, Clone, PartialEq)]
    pub enum DataAccessDirection {
        Input,  // !<
        Output, // !>
    }

    /// 广播
    /// broadcast ::= *!< | *!>
    #[derive(Debug, Clone)]
    pub enum Broadcast {
        Input,  // *!<
        Output, // *!>
    }

    /// 定时动作
    /// timed_action ::= computation ( behavior_time [ .. behavior_time ] )
    #[derive(Debug, Clone)]
    pub struct TimedAction {
        pub start_time: BehaviorTime,
        pub end_time: Option<BehaviorTime>,
    }

    /// 行为时间
    /// behavior_time ::= integer_value unit_identifier
    #[derive(Debug, Clone)]
    pub struct BehaviorTime {
        pub value: IntegerValue,
        pub unit: String,
    }

    /// 子程序参数列表
    /// subprogram_parameter_list ::= parameter_label { , parameter_label }*
    #[derive(Debug, Clone)]
    pub struct SubprogramParameterList {
        pub parameters: Vec<ParameterLabel>,
    }

    /// 参数标签
    /// parameter_label ::= in_parameter_value_expression | out_parameter_target
    #[derive(Debug, Clone)]
    pub enum ParameterLabel {
        In(ValueExpression),
        Out(Target),
    }

    /// 目标
    /// target ::= local_variable_name | outgoing_port_name | outgoing_subprogram_parameter_name | data_component_reference
    #[derive(Debug, Clone)]
    pub enum Target {
        LocalVariable(String),
        OutgoingPort(String),
        OutgoingSubprogramParameter(String),
        DataComponentReference(DataComponentReference),
    }

    /// 数据组件引用
    /// data_component_reference ::= data_subcomponent_name { . data_subcomponent_name }* | data_access_feature_name { . data_subcomponent_name }*
    #[derive(Debug, Clone)]
    pub struct DataComponentReference {
        pub components: Vec<String>,
    }

    /// 名称
    /// name ::= identifier { array_index }*
    #[derive(Debug, Clone)]
    pub struct Name {
        pub identifier: String,
        pub array_indices: Vec<ArrayIndex>,
    }

    /// 数组索引
    /// array_index ::= [ integer_value_variable ]
    #[derive(Debug, Clone)]
    pub struct ArrayIndex {
        pub value: IntegerValue,
    }

    /// 值
    /// value ::= value_variable | value_constant | ( value_expression )
    #[derive(Debug, Clone)]
    pub enum Value {
        Variable(ValueVariable),
        Constant(ValueConstant),
        Expression(Box<ValueExpression>),
    }

    /// 值变量
    /// value_variable ::= incoming_port_name | incoming_port_name ? | incoming_subprogram_parameter_name | local_variable_name | data_component_reference | port_name ' count | port_name ' fresh
    #[derive(Debug, Clone)]
    pub enum ValueVariable {
        IncomingPort(String),
        IncomingPortCheck(String),
        IncomingSubprogramParameter(String),
        LocalVariable(String),
        DataComponentReference(DataComponentReference),
        PortCount(String),
        PortFresh(String),
    }

    /// 值常量
    /// value_constant ::= boolean_literal | numeric_literal | string_literal | property_constant | property_value
    #[derive(Debug, Clone)]
    pub enum ValueConstant {
        Boolean(bool),
        Numeric(String),
        String(String),
        PropertyConstant(String),
        PropertyValue(String),
    }

    /// 值表达式
    /// value_expression ::= relation { logical_operator relation }*
    #[derive(Debug, Clone)]
    pub struct ValueExpression {
        pub left: Relation,
        pub operations: Vec<LogicalOperation>,
    }

    /// 逻辑操作
    #[derive(Debug, Clone)]
    pub struct LogicalOperation {
        pub operator: LogicalOperator,
        pub right: Relation,
    }

    /// 逻辑操作符
    /// logical_operator ::= and | or | xor
    #[derive(Debug, Clone, PartialEq)]
    pub enum LogicalOperator {
        And,
        Or,
        Xor,
    }

    /// 关系表达式
    /// relation ::= simple_expression [ relational_operator simple_expression ]
    #[derive(Debug, Clone)]
    pub struct Relation {
        pub left: SimpleExpression,
        pub comparison: Option<Comparison>,
    }

    /// 比较
    #[derive(Debug, Clone)]
    pub struct Comparison {
        pub operator: RelationalOperator,
        pub right: SimpleExpression,
    }

    /// 关系操作符
    /// relational_operator ::= = | != | < | <= | > | >=
    #[derive(Debug, Clone, PartialEq)]
    pub enum RelationalOperator {
        Equal,           // =
        NotEqual,        // !=
        LessThan,        // <
        LessThanOrEqual, // <=
        GreaterThan,     // >
        GreaterThanOrEqual, // >=
    }

    /// 简单表达式
    /// simple_expression ::= [ unary_adding_operator ] term { binary_adding_operator term }*
    #[derive(Debug, Clone)]
    pub struct SimpleExpression {
        pub sign: Option<UnaryAddingOperator>,
        pub left: Term,
        pub operations: Vec<AdditiveOperation>,
    }

    /// 一元加法操作符
    /// unary_adding_operator ::= + | -
    #[derive(Debug, Clone, PartialEq)]
    pub enum UnaryAddingOperator {
        Plus,   // +
        Minus,  // -
    }



    /// 动作（已废弃，使用新的BehaviorAction类型）
    /// 对应标准中的 action 语法
    #[derive(Debug, Clone)]
    pub enum Action {
        /// 基础动作 (basic_action)
        Basic(BasicAction),
    }
    
    /// 端口限定符
    #[derive(Debug, Clone, PartialEq)]
    pub enum PortQualifier {
        Count,
        Fresh,
    }
    
    /// 量词
    #[derive(Debug, Clone, PartialEq)]
    pub enum Quantifier {
        All,
        Exists,
    }
    
    /// 基础表达式
    /// 对应标准中的 basic_expression 语法
    #[derive(Debug, Clone)]
    pub enum BasicExpression {
        /// unsigned_aadlnumeric_or_constant
        NumericOrConstant(String),
        /// behavior_variable_identifier
        BehaviorVariable(String),
        /// loop_variable_identifier
        LoopVariable(String),
        /// port_identifier
        Port(String),
        /// port_identifier ' [ count | fresh ]
        PortWithQualifier {
            port: String,
            qualifier: PortQualifier,
        },
        /// data_access_identifier
        DataAccess(String),
        /// timeout behavior_expression
        Timeout(Box<BasicExpression>),
        /// data_subcomponent_identifier
        DataSubcomponent(String),
        /// data_subcomponent_identifier [ behavior_expression ]
        DataSubcomponentWithIndex {
            subcomponent: String,
            index: Box<BasicExpression>,
        },
        /// data_access_identifier . data_subcomponent_identifier
        DataAccessWithSubcomponent {
            access: String,
            subcomponent: String,
        },
        /// data_subcomponent_identifier . data_subcomponent_identifier
        DataSubcomponentWithSubcomponent {
            container: String,
            subcomponent: String,
        },
        /// data_classifier_reference . subprogram_identifier [ (behavior_expression{, behavior_expression}* ) ]
        DataClassifierSubprogram {
            classifier: String,
            subprogram: String,
            parameters: Option<Vec<BasicExpression>>,
        },
        /// data_classifier_reference . subprogram_identifier ?(behavior_expression)
        DataClassifierSubprogramWithTimeout {
            classifier: String,
            subprogram: String,
            timeout: Box<BasicExpression>,
        },
        /// data_classifier_reference . subprogram_identifier ' parameter_identifier (behavior_expression)
        DataClassifierSubprogramWithParameter {
            classifier: String,
            subprogram: String,
            parameter: String,
            expression: Box<BasicExpression>,
        },
        /// ( behavior_expression )
        Parenthesized(Box<BasicExpression>),
        /// (all | exists) (identifier in behavior_expression) : expression
        Quantified {
            quantifier: Quantifier,
            identifier: String,
            range: Box<BasicExpression>,
            expression: Box<BasicExpression>,
        },
        /// 二元操作表达式
        BinaryOp {
            left: Box<BasicExpression>,
            operator: BinaryOperator,
            right: Box<BasicExpression>,
        },
        /// 一元否定操作
        Not(Box<BasicExpression>),
    }
    
    /// 二元操作符
    #[derive(Debug, Clone, PartialEq)]
    pub enum BinaryOperator {
        // 逻辑操作符
        And,
        Or,
        
        // 比较操作符
        Equal,           // =
        NotEqual,        // !=
        LessThan,        // <
        LessThanOrEqual, // <=
        GreaterThan,     // >
        GreaterThanOrEqual, // >=
        
        // 算术操作符
        Add,      // +
        Subtract, // -
        Multiply, // *
        Divide,   // /
        Modulo,   // mod
    }
    
    /// 行为表达式
    /// 对应标准中的 behavior_expression ::= disjunction { or disjunction } *
    #[derive(Debug, Clone)]
    pub struct BehaviorExpression {
        pub disjunctions: Vec<DisjunctionExpression>,
    }
    
    /// 析取表达式
    /// 对应标准中的 disjunction ::= not_conjunction { and not_conjunction } *
    #[derive(Debug, Clone)]
    pub struct DisjunctionExpression {
        pub not_conjunctions: Vec<NotConjunctionExpression>,
    }
    
    /// 非合取表达式
    /// 对应标准中的 not_conjunction ::= not ? conjunction
    #[derive(Debug, Clone)]
    pub struct NotConjunctionExpression {
        pub has_not: bool,
        pub conjunction: ConjunctionExpression,
    }
    
    /// 合取表达式
    /// 对应标准中的 conjunction ::= arith_expression [ (<|<=|=|>|>=|!=) arith_expression ]
    #[derive(Debug, Clone)]
    pub struct ConjunctionExpression {
        pub left: ArithmeticExpression,
        pub comparison: Option<ComparisonExpression>,
    }
    
    /// 比较表达式
    #[derive(Debug, Clone)]
    pub struct ComparisonExpression {
        pub operator: ComparisonOperator,
        pub right: ArithmeticExpression,
    }
    
    /// 比较操作符
    #[derive(Debug, Clone, PartialEq)]
    pub enum ComparisonOperator {
        LessThan,        // <
        LessThanOrEqual, // <=
        Equal,           // =
        GreaterThan,     // >
        GreaterThanOrEqual, // >=
        NotEqual,        // !=
    }
    
    /// 算术表达式
    /// 对应标准中的 arith_expression ::= add_expression { ( + | - ) add_expression } *
    #[derive(Debug, Clone)]
    pub struct ArithmeticExpression {
        pub left: AddExpression,
        pub operations: Vec<AdditiveOperation>,
    }
    
    /// 加法操作
    #[derive(Debug, Clone)]
    pub struct AdditiveOperation {
        pub operator: AdditiveOperator,
        pub right: AddExpression,
    }
    
    /// 加法操作符
    /// binary_adding_operator ::= + | -
    #[derive(Debug, Clone, PartialEq)]
    pub enum AdditiveOperator {
        Add,      // +
        Subtract, // -
    }
    
    /// 加法表达式
    /// 对应标准中的 add_expression ::= basic_expression { ( * | / ) basic_expression } *
    #[derive(Debug, Clone)]
    pub struct AddExpression {
        pub left: BasicExpression,
        pub operations: Vec<MultiplicativeOperation>,
    }
    
    /// 乘法操作
    #[derive(Debug, Clone)]
    pub struct MultiplicativeOperation {
        pub operator: MultiplicativeOperator,
        pub right: BasicExpression,
    }
    
    /// 乘法操作符
    /// multiplying_operator ::= * | / | mod | rem
    #[derive(Debug, Clone, PartialEq)]
    pub enum MultiplicativeOperator {
        Multiply, // *
        Divide,   // /
        Modulo,   // mod
        Remainder, // rem
    }

    /// 项
    /// term ::= factor { multiplying_operator factor }*
    #[derive(Debug, Clone)]
    pub struct Term {
        pub left: Factor,
        pub operations: Vec<MultiplicativeOperation>,
    }

    /// 因子
    /// factor ::= value [ binary_numeric_operator value ] | unary_numeric_operator value | unary_boolean_operator value
    #[derive(Debug, Clone)]
    pub enum Factor {
        Value(Value),
        BinaryNumeric {
            left: Value,
            operator: BinaryNumericOperator,
            right: Value,
        },
        UnaryNumeric {
            operator: UnaryNumericOperator,
            value: Value,
        },
        UnaryBoolean {
            operator: UnaryBooleanOperator,
            value: Value,
        },
    }

    /// 二元数值操作符
    /// binary_numeric_operator ::= **
    #[derive(Debug, Clone, PartialEq)]
    pub enum BinaryNumericOperator {
        Power, // **
    }

    /// 一元数值操作符
    /// unary_numeric_operator ::= abs
    #[derive(Debug, Clone, PartialEq)]
    pub enum UnaryNumericOperator {
        Abs, // abs
    }

    /// 一元布尔操作符
    /// unary_boolean_operator ::= not
    #[derive(Debug, Clone, PartialEq)]
    pub enum UnaryBooleanOperator {
        Not, // not
    }

    /// 整数范围
    /// integer_range ::= integer_value .. integer_value
    #[derive(Debug, Clone)]
    pub struct IntegerRange {
        pub lower: IntegerValue,
        pub upper: IntegerValue,
    }

    /// 整数值
    /// integer_value ::= integer_value_variable | integer_value_constant
    #[derive(Debug, Clone)]
    pub enum IntegerValue {
        Variable(String),
        Constant(String),
    }


    // 组件类型的可选子句（None表示子句不存在，Empty表示显式声明none）
    //cj:非“关键字可选，使用Option”
    #[derive(Debug, Clone)]
    pub enum PrototypeClause {
        None,  // 无prototypes子句
        Empty, // prototypes none;
        Items(Vec<Prototype>),
    }

    #[derive(Debug, Clone)]
    pub enum FeatureClause {
        None,
        Empty,
        Items(Vec<Feature>),
    }

    // #[derive(Debug, Clone)]
    // pub enum FlowClause {
    //     None,
    //     Empty,
    //     Items(Vec<FlowSpec>)
    // }

    //组件类型扩展
    #[derive(Debug, Clone)]
    pub struct ComponentTypeExtension {
        pub category: ComponentCategory,
        pub identifier: String,
        pub extends: UniqueComponentReference,
        pub prototype_bindings: Option<PrototypeBindings>,
        pub prototypes: PrototypeClause,
        pub features: FeatureClause,
        //pub flows: FlowClause,
        //pub modes: Option<ModesClause>,
        pub properties: PropertyClause,
        pub annexes: Vec<AnnexSubclause>,
    }

    // #[derive(Debug, Clone)]
    // pub struct FlowSpec {
    //     pub identifier: String,
    //     pub source: Option<FlowEndpoint>,
    //     pub sink: Option<FlowEndpoint>
    // }

    // #[derive(Debug, Clone)]
    // pub enum ModesClause {
    //     Modes(Vec<Mode>),
    //     RequiresModes
    // }

    //基础类型定义
    #[derive(Debug, Clone, PartialEq)]
    pub enum ComponentCategory {
        // 抽象组件类别
        Abstract,
        // 软件类别
        Data,
        Subprogram,
        SubprogramGroup,
        Thread,
        ThreadGroup,
        Process,
        // 执行平台类别
        Memory,
        Processor,
        Bus,
        Device,
        VirtualProcessor,
        VirtualBus,
        // 复合类别
        System,
    }

    #[derive(Debug, Clone)]
    pub struct UniqueComponentReference {
        pub package_prefix: Option<PackageName>,
        pub identifier: String,
    }

    /* ========== 4.4 Component Implementations ========== */
    /* ========== 组件实现 ========== */
    #[derive(Debug, Clone)]
    pub struct ComponentImplementation {
        pub category: ComponentCategory,
        pub name: ImplementationName,
        pub prototype_bindings: Option<PrototypeBindings>,
        pub prototypes: PrototypeClause,
        pub subcomponents: SubcomponentClause,
        //pub internal_features: Vec<InternalFeature>,
        //pub processor_features: Vec<ProcessorFeature>,
        pub calls: CallSequenceClause,
        pub connections: ConnectionClause,
        //pub flows: FlowImplementationClause,
        //pub modes: Option<ModesClause>,
        pub properties: PropertyClause,
        pub annexes: Vec<AnnexSubclause>,
    }

    // 组件实现名称（type_id.impl_id）
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct ImplementationName {
        pub type_identifier: String,
        pub implementation_identifier: String,
    }

    impl ImplementationName {
        pub fn to_string(&self) -> String {
            format!(
                "{}.{}",
                self.type_identifier, self.implementation_identifier
            )
        }
    }

    // 组件实现扩展
    #[derive(Debug, Clone)]
    pub struct ComponentImplementationExtension {
        pub category: ComponentCategory,
        pub name: ImplementationName,

        // 被扩展的实现引用
        pub extends: UniqueImplementationReference,

        pub prototype_bindings: Option<PrototypeBindings>,
        pub prototypes: PrototypeClause,
        pub subcomponents: SubcomponentClause,
        //pub internal_features: Vec<InternalFeature>,
        //pub processor_features: Vec<ProcessorFeature>,
        pub calls: CallSequenceClause,
        pub connections: ConnectionClause,
        //pub flows: FlowImplementationClause,
        //pub modes: Option<ModesClause>,
        pub properties: PropertyClause,
        pub annexes: Vec<AnnexSubclause>,
    }

    // 唯一的组件实现引用（可能带包前缀）
    #[derive(Debug, Clone)]
    pub struct UniqueImplementationReference {
        pub package_prefix: Option<PackageName>,
        pub implementation_name: ImplementationName,
    }

    // 子句类型定义
    #[derive(Debug, Clone)]
    pub enum SubcomponentClause {
        None,
        Empty,
        Items(Vec<Subcomponent>),
        Refinements(Vec<SubcomponentRefinement>),
    }

    #[derive(Debug, Clone)]
    pub enum CallSequenceClause {
        None,
        Empty,
        Items(Vec<CallSequence>),
    }

    #[derive(Debug, Clone)]
    pub enum ConnectionClause {
        None,
        Empty,
        Items(Vec<Connection>),
        Refinements(Vec<ConnectionRefinement>),
    }

    // #[derive(Debug, Clone)]
    // pub enum FlowImplementationClause {
    //     None,
    //     Empty,
    //     Items(Vec<FlowImplementation>),
    //     EndToEndFlows(Vec<EndToEndFlow>),
    //     Refinements(Vec<FlowRefinement>)
    // }

    #[derive(Debug, Clone)]
    pub struct ConnectionRefinement {
        pub original_name: String,
        pub refinement: Connection,
    }

    // #[derive(Debug, Clone)]
    // pub struct FlowRefinement {
    //     pub original_name: String,
    //     pub refinement: FlowImplementation
    // }

    /* ========== 4.5 subComponent ========== */
    #[derive(Debug, Clone)]
    pub struct Subcomponent {
        pub identifier: String,
        pub category: ComponentCategory,
        pub classifier: SubcomponentClassifier,
        pub array_spec: Option<ArraySpec>,
        pub properties: Vec<Property>,
        //pub modes: Option<ComponentInModes>
    }
    #[derive(Debug, Clone)]
    pub enum SubcomponentClassifier {
        /// 组件分类器引用
        ClassifierReference(UniqueComponentClassifierReference),
        /// 原型引用
        Prototype(String),
    }
    /// 唯一的组件分类器引用
    #[derive(Debug, Clone)]
    pub enum UniqueComponentClassifierReference {
        Type(UniqueImplementationReference),
        Implementation(UniqueImplementationReference),
    }
    /* ========== 子组件精化 ========== */
    #[derive(Debug, Clone)]
    pub struct SubcomponentRefinement {
        pub identifier: String,
        pub category: ComponentCategory,
        pub classifier: Option<SubcomponentClassifier>, // refined to可能省略引用
        pub array_spec: Option<ArraySpec>,
        pub properties: Vec<Property>,
        //pub modes: Option<ComponentInModes>
    }
    /* ========== 数组维度定义 ========== */
    #[derive(Debug, Clone)]
    pub struct ArraySpec {
        pub dimensions: Vec<ArrayDimension>,
        pub element_implementations: Option<Vec<ArrayElementImplementation>>,
    }
    #[derive(Debug, Clone)]
    pub struct ArrayDimension {
        pub size: Option<ArrayDimensionSize>, // 可选尺寸表示 [ ]
    }

    #[derive(Debug, Clone)]
    pub enum ArrayDimensionSize {
        Fixed(u32),
        PropertyReference(String), // 属性常量标识符
    }

    #[derive(Debug, Clone)]
    pub struct ArrayElementImplementation {
        pub implementation: UniqueImplementationReference,
        pub prototype_bindings: Option<PrototypeBindings>,
    }

    /* ========== 4.7 Prototype ========== */
    /* ========== 基础原型定义 ========== */
    #[derive(Debug, Clone)]
    pub enum Prototype {
        Component(ComponentPrototype),
        FeatureGroup(FeatureGroupPrototype),
        Feature(FeaturePrototype),
    }

    #[derive(Debug, Clone)]
    pub struct PrototypeDeclaration {
        pub identifier: String,
        pub prototype: Prototype,
        pub properties: Vec<PrototypePropertyAssociation>,
    }

    /* ========== 组件原型 ========== */
    #[derive(Debug, Clone)]
    pub struct ComponentPrototype {
        pub category: ComponentCategory,
        pub classifier: Option<UniqueComponentClassifierReference>,
        pub is_array: bool, // 对应 [ [] ] 语法
    }
    /* ========== 特性组原型 ========== */
    #[derive(Debug, Clone)]
    pub struct FeatureGroupPrototype {
        pub classifier: Option<UniqueFeatureGroupTypeReference>,
    }
    // 对应标准中的 `unique_feature_group_type_reference`
    #[derive(Debug, Clone)]
    pub struct UniqueFeatureGroupTypeReference {
        /// 可选的包名前缀 `[ package_name :: ]`
        pub package_prefix: Option<PackageName>,

        /// 特性组类型标识符 `feature_group_type_identifier`
        pub identifier: String,
    }
    /* ========== 特性原型 ========== */
    #[derive(Debug, Clone)]
    pub struct FeaturePrototype {
        pub direction: Option<PortDirection>, // in/out
        pub classifier: Option<UniqueComponentClassifierReference>,
    }

    /* ========== 原型精化 ========== */
    #[derive(Debug, Clone)]
    pub struct PrototypeRefinement {
        pub identifier: String,
        pub prototype: Prototype, // 精化后的目标原型
        pub properties: Vec<PrototypePropertyAssociation>,
    }

    /* ========== 原型绑定 ========== */
    #[derive(Debug, Clone)]
    pub struct PrototypeBindings {
        pub bindings: Vec<PrototypeBinding>,
    }

    #[derive(Debug, Clone)]
    pub struct PrototypeBinding {
        pub identifier: String,
        pub actual: PrototypeActual,
    }

    #[derive(Debug, Clone)]
    pub enum PrototypeActual {
        Component(ComponentPrototypeActual),
        ComponentList(Vec<ComponentPrototypeActual>),
        FeatureGroup(FeatureGroupPrototypeActual),
        Feature(FeaturePrototypeActual),
    }

    /* ========== 组件原型实际值 ========== */
    #[derive(Debug, Clone)]
    pub struct ComponentPrototypeActual {
        pub category: ComponentCategory,
        pub reference: Option<ComponentPrototypeReference>,
        pub bindings: Option<PrototypeBindings>,
    }

    #[derive(Debug, Clone)]
    pub enum ComponentPrototypeReference {
        Classifier(UniqueComponentClassifierReference),
        Prototype(String), // 引用其他原型
    }

    /* ========== 特性组原型实际值 ========== */
    #[derive(Debug, Clone)]
    pub enum FeatureGroupPrototypeActual {
        Classifier {
            reference: UniqueFeatureGroupTypeReference,
            bindings: Option<PrototypeBindings>,
        },
        Prototype(String), // 引用其他特性组原型
    }

    /* ========== 特性原型实际值 ========== */
    #[derive(Debug, Clone)]
    pub enum FeaturePrototypeActual {
        Port {
            direction: PortDirection,
            port_type: PortType,
            classifier: Option<UniqueComponentClassifierReference>,
        },
        Access {
            access_type: AccessDirection,
            connection_type: AccessConnectionType,
            classifier: Option<UniqueComponentClassifierReference>,
        },
        Prototype(String), // 引用其他特性原型
    }

    /* ========== 相关枚举类型 ========== */
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum PortDirection {
        In,
        Out,
        InOut,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum AccessDirection {
        Requires,
        Provides,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum AccessConnectionType {
        Bus,
        VirtualBus,
        Data,
        SubprogramGroup,
        Subprogram,
    }

    /* ========== TODO:属性关联 ========== */
    #[derive(Debug, Clone)]
    pub struct PrototypePropertyAssociation {
        pub name: String,
        pub value: PropertyValue,
        pub applies_to: Option<Vec<String>>,
    }

    /*================software component====================*/

    /*==========5.1 Data 没有syntax========= */

    /*==========5.2 Subprograms and Subprogram Calls============= */
    #[derive(Debug, Clone)]
    pub struct CallSequence {
        /// 调用序列标识符 (defining_call_sequence_identifier)
        pub identifier: String,

        /// 子程序调用列表 (subprogram_call+)
        pub calls: Vec<SubprogramCall>,

        /// 调用序列属性 (call_sequence_property_association*)
        pub properties: Vec<Property>,

        /// 模式约束 (in_modes)
        pub in_modes: Option<Vec<String>>,
    }

    /* ========== 子程序调用 ========== */
    #[derive(Debug, Clone)]
    pub struct SubprogramCall {
        /// 调用标识符 (defining_call_identifier)
        pub identifier: String,

        /// 被调用的子程序 (called_subprogram)
        pub called: CalledSubprogram,

        /// 调用属性 (subcomponent_call_property_association*)
        pub properties: Vec<Property>,
    }

    /* ========== 被调用的子程序 ========== */
    //TODO:目前只定义了一种引用方式，其它的引用方式未见过案例
    #[derive(Debug, Clone)]
    pub enum CalledSubprogram {
        /// 通过分类器引用 (subprogram_unique_component_classifier_reference)
        Classifier(UniqueComponentClassifierReference),
    }

    /*8 features and shared access */
    //功能是组件类型定义的一部分，指定接口
    //TODO:目前只考虑port,例子在Notion中有图片
    #[derive(Debug, Clone)]
    pub enum Feature {
        // 抽象特征 (abstract_feature_spec)
        //Abstract(AbstractFeature),

        // 端口 (port_spec)
        Port(PortSpec),
        // 子组件访问 (subcomponent_access_spec)
        SubcomponentAccess(SubcomponentAccessSpec),
        // 特征组 (feature_group_spec)
        //FeatureGroup(FeatureGroupSpec),

        // 参数 (parameter_spec)
        //Parameter(ParameterSpec),

        // 精化特征 (feature_refinement)
        //Refinement(FeatureRefinement)
    }
    /* ========== 端口类型 ========== */
    /// 对应标准中的 `port_type`
    #[derive(Debug, Clone)]
    pub enum PortType {
        /// `data port [reference]`
        Data {
            classifier: Option<PortDataTypeReference>,
        },
        /// `event data port [reference]`
        EventData {
            classifier: Option<PortDataTypeReference>,
        },
        /// `event port`
        Event,
    }

    /// 端口数据类型引用（对应标准中的两种引用方式）
    #[derive(Debug, Clone)]
    pub enum PortDataTypeReference {
        /// `data_unique_component_classifier_reference`
        Classifier(UniqueComponentClassifierReference),
        /// `data_component_prototype_identifier`
        Prototype(String),
    }
    #[derive(Debug, Clone)]
    pub struct PortSpec {
        /// `defining_port_identifier`
        pub identifier: String,
        pub direction: PortDirection,
        pub port_type: PortType,
    }

    /* ========== 子组件访问总和类型 (subcomponent_access_spec) ========== */
    #[derive(Debug, Clone)]
    pub enum SubcomponentAccessSpec {
        /// 数据访问 (data_access_spec)
        Data(DataAccessSpec),
        /// 子程序访问 (subprogram_access_spec)
        Subprogram(SubprogramAccessSpec),
        // TODO: SubprogramGroup, Bus, VirtualBus
    }

    /* ========== 访问特征 ========== */
    /// 数据访问规范 (data_access_spec)
    #[derive(Debug, Clone)]
    pub struct DataAccessSpec {
        /// `defining_data_component_access_identifier`
        pub identifier: String,
        pub direction: AccessDirection, // provides | requires
        /// `data_unique_component_classifier_reference | prototype_identifier`
        pub classifier: Option<DataAccessReference>,
    }

    /// 数据访问分类器引用
    #[derive(Debug, Clone)]
    pub enum DataAccessReference {
        /// 分类器引用 (data_unique_component_classifier_reference)
        Classifier(UniqueComponentClassifierReference),
        /// 原型标识符 (prototype_identifier)
        Prototype(String),
    }

    /// 子程序访问规范 (subprogram_access_spec)
    #[derive(Debug, Clone)]
    pub struct SubprogramAccessSpec {
        /// `defining_subprogram_access_identifier`
        pub identifier: String,
        pub direction: AccessDirection, // provides | requires
        /// `subprogram_unique_component_classifier_reference | subprogram_component_prototype_identifier`
        pub classifier: Option<SubprogramAccessReference>,
    }

    /// 子程序访问分类器引用
    #[derive(Debug, Clone)]
    pub enum SubprogramAccessReference {
        /// 分类器引用 (subprogram_unique_component_classifier_reference)
        Classifier(UniqueComponentClassifierReference),
        /// 原型标识符 (subprogram_component_prototype_identifier)
        Prototype(String),
    }

    /*=================9 connection ============ */
    /* ========== 连接类型 ========== */
    #[derive(Debug, Clone)]
    pub enum Connection {
        // 端口连接 (port_connection)
        Port(PortConnection),

        // 参数连接 (parameter_connection)
        Parameter(ParameterConnection),
        // 以下为其他连接类型（暂不实现）
        // Feature(FeatureConnection),      // feature_connection
        Access(AccessConnection),       // access_connection（仅关注 data/subprogram）
        // FeatureGroup(FeatureGroupConnection), // feature_group_connection
    }

    /* ========== 端口连接符号 ========== */
    /// 对应标准中的 `connection_symbol`
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum ConnectionSymbol {
        ///  ->
        Direct,
        ///  <->
        Didirect,
    }
    /* ========== port 口连接定义 ========== */
    /// 对应标准中的 `port_connection`,但是将标识符从connection下放到了port connection
    #[derive(Debug, Clone)]
    pub struct PortConnection {
        pub identifier: String,
        pub source: PortEndpoint,
        pub destination: PortEndpoint,
        pub connection_direction: ConnectionSymbol,
    }
    // 对应标准中的 `port_connection_reference`
    #[derive(Debug, Clone)]
    pub enum PortEndpoint {
        /// 组件类型端口 (component_type_port_identifier)
        ComponentPort(String),

        /// 子组件端口 (subcomponent_identifier.port_identifier)
        SubcomponentPort { subcomponent: String, port: String },

        /// 特征组元素端口 (component_type_feature_group_identifier.element_port_identifier)
        FeatureGroupPort {
            feature_group: String,
            element: String,
        },

        /// 聚合数据端口元素 (component_type_port_identifier.data_subcomponent_identifier)
        AggregateDataElement { port: String, data_element: String },

        /// 数据访问要求 (component_type_requires_data_access_identifier)
        RequiresDataAccess(String),

        /// 数据子组件 (data_subcomponent_identifier)
        DataSubcomponent(String),

        /// 子组件提供的数据访问 (subcomponent_identifier.provides_data_access_identifier)
        SubcomponentDataAccess {
            subcomponent: String,
            access: String,
        },

        /// 特征组数据访问元素 (component_type_feature_group_identifier.element_data_access_identifier)
        FeatureGroupDataAccess {
            feature_group: String,
            element: String,
        },

        /// 数据子组件嵌套访问 (data_subcomponent_identifier.data_subcomponent_identifier)
        NestedDataAccess { container: String, element: String },

        /// 处理器端口 ([processor.]processor_port_identifier)
        ProcessorPort {
            processor: Option<String>, // None表示隐式当前处理器
            port: String,
        },

        /// 组件内部事件源 ([self.]internal_event_or_event_data_identifier)
        InternalEvent {
            self_ref: bool, // 是否显式使用"self."
            identifier: String,
        },
    }
    /* ========== 参数连接定义 ========== */
    /// 对应标准中的 `parameter_connection`
    #[derive(Debug, Clone)]
    pub struct ParameterConnection {
        pub source: ParameterEndpoint,
        pub destination: ParameterEndpoint,
        pub connection_direction: ConnectionSymbol,
    }

    /* ========== 访问连接定义（data/subprogram） ========== */
    /// 对应标准中的 `access_connection`
    #[derive(Debug, Clone)]
    pub struct AccessConnection {
        pub source: AccessEndpoint,
        pub destination: AccessEndpoint,
        pub connection_direction: ConnectionSymbol,
    }

    /// 对应标准中的 `source_access_reference` / `destination_access_reference`
    /// 仅覆盖 data/subprogram 两类的常见引用形式
    #[derive(Debug, Clone)]
    pub enum AccessEndpoint {
        /// 组件类型上的访问特征标识符
        ComponentAccess(String),
        /// 子组件上的访问特征：subcomponent_identifier.access_identifier
        SubcomponentAccess { subcomponent: String, access: String },
    }
    /* ========== 参数端点定义 ========== */
    /// 对应标准中的 `parameter_reference`
    #[derive(Debug, Clone)]
    pub enum ParameterEndpoint {
        /// 线程/子程序类型参数 (component_type_parameter_identifier[.data_subcomponent_identifier])
        ComponentParameter {
            parameter: String,
            data_subcomponent: Option<String>, // 可选数据子组件
        },

        /// 子程序调用参数 (subprogram_call_identifier.parameter_identifier)
        SubprogramCallParameter {
            call_identifier: String,
            parameter: String,
        },

        /// 线程类型的数据/事件数据端口 (component_type_port_identifier[.data_subcomponent_identifier])
        ThreadPort {
            port: String,
            data_subcomponent: Option<String>, // 可选数据元素
        },

        /// 数据子组件 (data_subcomponent_identifier)
        DataSubcomponent(String),

        /// 要求的数据访问 (requires_data_access_identifier)
        RequiresDataAccess(String),

        /// 特征组的数据访问元素 (component_type_feature_group_identifier.element_data_access_identifier)
        FeatureGroupDataAccess {
            feature_group: String,
            element: String,
        },

        /// 特征组的端口/参数元素 (component_type_feature_group_identifier.element_port_or_parameter_identifier)
        FeatureGroupElement {
            feature_group: String,
            element: String,
        },
    }

    /*==============11 属性=============== */
    #[derive(Debug, Clone)]
    pub enum Property {
        /// 基础属性关联 (basic_property_association)
        BasicProperty(BasicPropertyAssociation),
        SubcomponentProperty(BasicPropertyAssociation), //TODO:暂时使用basic代替
        CallSequenceProperty(BasicPropertyAssociation),
        //ContainedProperty(ContainedAssociation), //简化到BasicPropertyAssociation中PropertyValue中PropertyExpression里有Apply
        // 未来可扩展其他属性类型：
    }

    /* ========== 基础属性关联 ========== */
    #[derive(Debug, Clone)]
    pub struct BasicPropertyAssociation {
        /// 属性标识符 (unique_property_identifier)
        pub identifier: PropertyIdentifier,

        /// 赋值操作符 => 或 +=>
        pub operator: PropertyOperator,

        /// 是否为常量 [constant]
        pub is_constant: bool,

        /// 属性值 (property_value)
        pub value: PropertyValue,
    }
    #[derive(Debug, Clone)]
    pub struct PropertyIdentifier {
        /// 可选的属性集前缀 [property_set_identifier::]
        pub property_set: Option<String>,
        pub name: String,
    }
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum PropertyOperator {
        Assign, // =>
        Append, // +=>
    }
    // #[derive(Debug, Clone)]
    // pub struct ContainedAssociation {
    //     pub contained_element_point:String,
    //     pub contained_element:String,
    // }
    /* ========== 属性值系统 ========== */
    #[derive(Debug, Clone)]
    pub enum PropertyValue {
        Single(PropertyExpression),     // single_property_value
        List(Vec<PropertyListElement>), // property_list_value
    }

    #[derive(Debug, Clone)]
    pub enum PropertyListElement {
        Value(PropertyExpression),
        NestedList(Vec<PropertyListElement>), // 支持嵌套列表
    }

    #[derive(Debug, Clone)]
    pub enum PropertyExpression {
        // 基础类型
        Boolean(BooleanTerm),
        Real(SignedRealOrConstant),
        Integer(SignedIntergerOrConstant),
        String(StringTerm),
        //Enumeration(EnumerationTerm),
        //Unit(UnitTerm),

        // 范围类型
        IntegerRange(IntegerRangeTerm),
        //RealRange(RealRangeTerm),

        // 复杂类型
        //PropertyReference(PropertyTerm),
        ComponentClassifier(ComponentClassifierTerm),
        Reference(ReferenceTerm),
        //Record(RecordTerm),
        //Computed(ComputedTerm),
        Apply(ApplyTerm), //contained_property_association（做了简化处理）
    }
    /* ========== 属性常量项 ========== */
    #[derive(Debug, Clone)]
    pub struct PropertyConstantTerm {
        /// 可选的属性集前缀 ([property_set_identifier::])
        pub property_set: Option<String>,

        /// 常量标识符 (real_property_constant_term)
        pub name: String,
    }

    #[derive(Debug, Clone)]
    pub enum BooleanTerm {
        Literal(bool),                  // boolean_value
        Constant(PropertyConstantTerm), // boolean_property_constant_term
    }
    /* ========== 符号定义 ========== */
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Sign {
        Plus,  // +
        Minus, // -
    }
    /* ========== 带符号实数或常量 ========== */
    /// 对应标准中的 `signed_aadlreal_or_constant`
    #[derive(Debug, Clone)]
    pub enum SignedRealOrConstant {
        /// 带符号实数 (signed_aadlreal)
        Real(SignedReal),

        /// 实数属性常量 ([sign] real_property_constant_term)
        Constant {
            sign: Option<Sign>,
            constant: PropertyConstantTerm,
        },
    }

    /* ========== 带符号实数 ========== */
    /// 对应标准中的 `signed_aadlreal`
    #[derive(Debug, Clone)]
    pub struct SignedReal {
        /// 可选符号 ([sign])
        pub sign: Option<Sign>,

        /// 实数字面量 (real_literal)
        pub value: f64,

        /// 可选单位标识符 ([unit_identifier])
        pub unit: Option<String>,
    }

    /* ========== 带符号整数或常量 ========== */
    /// 对应标准中的 `signed_aadlreal_or_constant`
    #[derive(Debug, Clone)]
    pub enum SignedIntergerOrConstant {
        /// 带符号实数 (signed_aadlreal)
        Real(SignedInteger),

        /// 实数属性常量 ([sign] real_property_constant_term)
        Constant {
            sign: Option<Sign>,
            constant: PropertyConstantTerm,
        },
    }

    /* ========== 带符号实数 ========== */
    /// 对应标准中的 `signed_aadlreal`
    #[derive(Debug, Clone)]
    pub struct SignedInteger {
        /// 可选符号 ([sign])
        pub sign: Option<Sign>,

        /// 实数字面量 (real_literal)
        pub value: i64,

        /// 可选单位标识符 ([unit_identifier])
        pub unit: Option<String>,
    }
    /// 字符串项 (string_term)
    #[derive(Debug, Clone)]
    pub enum StringTerm {
        /// 字面量 (string_literal)
        Literal(String),

        /// 字符串常量 (string_property_constant_term)
        Constant(PropertyConstantTerm),
    }

    #[derive(Debug, Clone)]
    pub struct IntegerRangeTerm {
        pub lower: StringWithUnit,
        pub upper: StringWithUnit,
    }

    #[derive(Debug, Clone)]
    pub struct StringWithUnit {
        pub value: String,        // 例如 "10"
        pub unit: Option<String>, // 例如 "KByte"
    }

    /* ========== 最小引用定义，支持 reference(identifier) ========== */
    /// AADL: reference ( contained_model_element_path )
    /// 为满足 `reference (cpu) applies to node_a` 的需求，保存引用标识符和可选的 applies_to 目标。
    #[derive(Debug, Clone)]
    pub struct ReferenceTerm {
        pub identifier: String,
        /// 可选的 applies to 子句，如 `applies to node_a`
        pub applies_to: Option<String>,
    }

    //为满足Data_Model::Base_Type => classifier (Base_Types::Integer); 的需求，保存组件分类器引用
    #[derive(Debug, Clone)]
    pub struct ComponentClassifierTerm {
        pub unique_component_classifier_reference: UniqueComponentClassifierReference,
    }
    #[derive(Debug, Clone)]
    pub struct ApplyTerm {
        pub number: String,
        pub applies_to: String,
    }
} //end mod aadl_ast_cj
