#![allow(clippy::all)]
#[allow(dead_code)]
pub mod aadl_ast_cj {

    /* ========== 4.2 Package ========== */
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct PackageName(pub Vec<String>);

    impl PackageName {
        pub fn to_string(&self) -> String {
            self.0.join("::")
        }
    }

    // （with/renames）
    #[derive(Debug, Clone)]
    pub enum VisibilityDeclaration {
        // with package1, package2, property_set; `with`
        Import {
            packages: Vec<PackageName>,
            property_sets: Vec<String>,
        },
        // renames package::component; `renames`
        Alias {
            new_name: String,
            original: QualifiedName,
            is_package: bool, //
        },
        // renames package::all;
        ImportAll(PackageName),
    }

    //
    #[derive(Debug, Clone)]
    pub struct QualifiedName {
        pub package_prefix: Option<PackageName>,
        pub identifier: String,
    }

    // （/）
    #[derive(Debug, Clone)]
    pub struct PackageSection {
        pub is_public: bool,
        pub declarations: Vec<AadlDeclaration>,
    }

    //
    #[derive(Debug, Clone)]
    pub enum PropertyClause {
        ExplicitNone, // none;
        Properties(Vec<Property>),
    }

    //
    #[derive(Debug, Clone)]
    pub struct Package {
        pub name: PackageName,
        pub visibility_decls: Vec<VisibilityDeclaration>,
        pub public_section: Option<PackageSection>,
        pub private_section: Option<PackageSection>,
        pub properties: PropertyClause,
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

    pub enum ComponentRef<'a> {
        Type(&'a ComponentType),
        Impl(&'a ComponentImplementation),
    }

    /* ========== 4.3 Component Types ========== */
    #[derive(Debug, Clone)]
    pub struct ComponentType {
        pub category: ComponentCategory,
        pub identifier: String,
        pub prototypes: PrototypeClause,
        pub features: FeatureClause,
        //pub flows: FlowClause,
        //pub modes: Option<ModesClause>,
        pub properties: PropertyClause,
        pub annexes: Vec<AnnexSubclause>,
    }

    //sTODO 4.8 Annex Subclauses and Annex Libraries
    #[derive(Debug, Clone)]
    pub struct AnnexLibrary {}

    #[derive(Debug, Clone)]
    pub struct AnnexSubclause {
        pub identifier: AnnexIdentifier,
        pub content: AnnexContent,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum AnnexIdentifier {
        BehaviorSpecification,
        EMV2,
    }

    #[derive(Debug, Clone)]
    pub enum AnnexContent {
        None,

        BehaviorAnnex(BehaviorAnnexContent),
    }

    #[derive(Debug, Clone)]
    pub struct BehaviorAnnexContent {
        pub state_variables: Option<Vec<StateVariable>>,

        pub states: Option<Vec<State>>,

        pub transitions: Option<Vec<Transition>>,
    }

    #[derive(Debug, Clone)]
    pub struct StateVariable {
        pub identifier: String,
        pub data_type: String,
        pub initial_value: Option<String>,
    }

    #[derive(Debug, Clone)]
    pub struct State {
        pub identifiers: Vec<String>,
        pub modifiers: Vec<StateModifier>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum StateModifier {
        Initial,  // initial
        Complete, // complete
        //Return,     // return
        //Urgent,     // urgent
        //Composite,  // composite
        //Exit,       // exit
        Final, // final
    }

    #[derive(Debug, Clone)]
    pub struct Transition {
        pub transition_identifier: Option<String>,

        pub priority: Option<String>,

        pub source_states: Vec<String>,

        pub destination_state: String,

        pub behavior_condition: Option<BehaviorCondition>,

        pub actions: Option<BehaviorActionBlock>,
    }

    #[derive(Debug, Clone)]
    pub enum BehaviorCondition {
        Dispatch(DispatchCondition),

        Execute(DispatchConjunction),
    }

    #[derive(Debug, Clone)]
    pub enum ExecuteCondition {
        /// logical_value_expression
        LogicalExpression(BehaviorExpression),

        ActionBlockTimeoutCatch,
        /// otherwise
        Otherwise,
    }

    #[derive(Debug, Clone)]
    pub struct DispatchCondition {
        /// dispatch_trigger_condition
        pub trigger_condition: Option<DispatchTriggerCondition>,

        pub frozen_ports: Option<Vec<String>>,
    }

    #[derive(Debug, Clone)]
    pub enum DispatchTriggerCondition {
        /// dispatch_trigger_logical_expression
        LogicalExpression(DispatchTriggerLogicalExpression),
        /// provides_subprogram_access_identifier
        SubprogramAccess(String),
        /// stop
        Stop,

        CompletionTimeout,

        DispatchTimeout,
    }

    #[derive(Debug, Clone)]
    pub struct DispatchTriggerLogicalExpression {
        pub dispatch_conjunctions: Vec<DispatchConjunction>,
    }

    #[derive(Debug, Clone)]
    pub struct DispatchConjunction {
        pub not: bool,
        pub dispatch_triggers: Vec<DispatchTrigger>,
        pub number: Option<String>,
        pub less_than: bool,
    }

    #[derive(Debug, Clone)]
    pub enum DispatchTrigger {
        /// in_event_port_identifier
        InEventPort(String),
        /// in_event_data_port_identifier
        InEventDataPort(String),
    }

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

    /// behavior_action_block ::= { behavior_actions } [ timeout behavior_time ]
    #[derive(Debug, Clone)]
    pub struct BehaviorActionBlock {
        pub actions: BehaviorActions,
        pub timeout: Option<BehaviorTime>,
    }

    /// behavior_actions ::= behavior_action | behavior_action_sequence | behavior_action_set
    #[derive(Debug, Clone)]
    pub enum BehaviorActions {
        Single(Box<BehaviorAction>),
        Sequence(BehaviorActionSequence),
        Set(BehaviorActionSet),
    }

    /// behavior_action_sequence ::= behavior_action { ; behavior_action }+
    #[derive(Debug, Clone)]
    pub struct BehaviorActionSequence {
        pub actions: Vec<BehaviorAction>,
    }

    /// behavior_action_set ::= behavior_action { & behavior_action }+
    #[derive(Debug, Clone)]
    pub struct BehaviorActionSet {
        pub actions: Vec<BehaviorAction>,
    }

    /// behavior_action ::= basic_action | behavior_action_block | if_statement | for_statement | forall_statement | while_statement | do_until_statement
    #[derive(Debug, Clone)]
    pub enum BehaviorAction {
        Basic(BasicAction),
        Block(Box<BehaviorActionBlock>),
        If(IfStatement),
        For(ForStatement),
        Forall(ForallStatement),
        While(WhileStatement),
        DoUntil(DoUntilStatement),
    }

    /// if ( logical_value_expression ) behavior_actions { elsif ( logical_value_expression ) behavior_actions }* [ else behavior_actions ] end if
    #[derive(Debug, Clone)]
    pub struct IfStatement {
        pub condition: BehaviorExpression,
        pub then_actions: Box<BehaviorActions>,
        pub elsif_branches: Vec<ElsifBranch>,
        pub else_actions: Option<Box<BehaviorActions>>,
    }

    #[derive(Debug, Clone)]
    pub struct ElsifBranch {
        pub condition: BehaviorExpression,
        pub actions: Box<BehaviorActions>,
    }

    /// for ( element_identifier : data_unique_component_classifier_reference in element_values ) { behavior_actions }
    #[derive(Debug, Clone)]
    pub struct ForStatement {
        pub element_identifier: String,
        pub data_classifier: String,
        pub element_values: ElementValues,
        pub actions: Box<BehaviorActions>,
    }

    /// forall ( element_identifier : data_unique_component_classifier_reference in element_values ) { behavior_actions }
    #[derive(Debug, Clone)]
    pub struct ForallStatement {
        pub element_identifier: String,
        pub data_classifier: String,
        pub element_values: ElementValues,
        pub actions: Box<BehaviorActions>,
    }

    /// while ( logical_value_expression ) { behavior_actions }
    #[derive(Debug, Clone)]
    pub struct WhileStatement {
        pub condition: BehaviorExpression,
        pub actions: Box<BehaviorActions>,
    }

    /// do behavior_actions until ( logical_value_expression )
    #[derive(Debug, Clone)]
    pub struct DoUntilStatement {
        pub actions: Box<BehaviorActions>,
        pub condition: BehaviorExpression,
    }

    /// element_values ::= integer_range | event_data_port_name | array_data_component_reference
    #[derive(Debug, Clone)]
    pub enum ElementValues {
        IntegerRange(IntegerRange),
        EventDataPort(String),
        ArrayDataComponent(String),
    }

    /// basic_action ::= assignment_action | communication_action | timed_action
    #[derive(Debug, Clone)]
    pub enum BasicAction {
        Assignment(AssignmentAction),
        Communication(CommunicationAction),
        Timed(TimedAction),
    }

    /// assignment_action ::= target := ( value_expression | any )
    #[derive(Debug, Clone)]
    pub struct AssignmentAction {
        pub target: Target,
        pub value: AssignmentValue,
    }

    #[derive(Debug, Clone)]
    pub enum AssignmentValue {
        Expression(ValueExpression),
        Any,
    }

    /// communication_action ::= subprogram_call | port_communication | data_access_communication | broadcast
    #[derive(Debug, Clone)]
    pub enum CommunicationAction {
        //SubprogramCall(SubprogramCall),
        PortCommunication(PortCommunication),
        DataAccessCommunication(DataAccessCommunication),
        Broadcast(Broadcast),
    }

    /// subprogram_call ::= subprogram_prototype_name ! [ ( subprogram_parameter_list ) ] | required_subprogram_access_name ! [ ( subprogram_parameter_list ) ] | subprogram_subcomponent_name ! [ ( subprogram_parameter_list ) ] | subprogram_unique_component_classifier_reference ! [ ( subprogram_parameter_list ) ]
    // #[derive(Debug, Clone)]
    // pub struct SubprogramCall {
    //     pub name: String,
    //     pub parameters: Option<SubprogramParameterList>,
    // }

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

    #[derive(Debug, Clone, PartialEq)]
    pub enum DataAccessDirection {
        Input,  // !<
        Output, // !>
    }

    /// broadcast ::= *!< | *!>
    #[derive(Debug, Clone)]
    pub enum Broadcast {
        Input,  // *!<
        Output, // *!>
    }

    /// timed_action ::= computation ( behavior_time [ .. behavior_time ] )
    #[derive(Debug, Clone)]
    pub struct TimedAction {
        pub start_time: BehaviorTime,
        pub end_time: Option<BehaviorTime>,
    }

    /// behavior_time ::= integer_value unit_identifier
    #[derive(Debug, Clone)]
    pub struct BehaviorTime {
        pub value: IntegerValue,
        pub unit: String,
    }

    /// subprogram_parameter_list ::= parameter_label { , parameter_label }*
    #[derive(Debug, Clone)]
    pub struct SubprogramParameterList {
        pub parameters: Vec<ParameterLabel>,
    }

    /// parameter_label ::= in_parameter_value_expression | out_parameter_target
    #[derive(Debug, Clone)]
    pub enum ParameterLabel {
        In(ValueExpression),
        Out(Target),
    }

    /// target ::= local_variable_name | outgoing_port_name | outgoing_subprogram_parameter_name | data_component_reference
    #[derive(Debug, Clone)]
    pub enum Target {
        LocalVariable(String),
        OutgoingPort(String),
        OutgoingSubprogramParameter(String),
        DataComponentReference(DataComponentReference),
    }

    /// data_component_reference ::= data_subcomponent_name { . data_subcomponent_name }* | data_access_feature_name { . data_subcomponent_name }*
    #[derive(Debug, Clone)]
    pub struct DataComponentReference {
        pub components: Vec<String>,
    }

    /// name ::= identifier { array_index }*
    #[derive(Debug, Clone)]
    pub struct Name {
        pub identifier: String,
        pub array_indices: Vec<ArrayIndex>,
    }

    /// array_index ::= [ integer_value_variable ]
    #[derive(Debug, Clone)]
    pub struct ArrayIndex {
        pub value: IntegerValue,
    }

    /// value ::= value_variable | value_constant | ( value_expression )
    #[derive(Debug, Clone)]
    pub enum Value {
        Variable(ValueVariable),
        Constant(ValueConstant),
        Expression(Box<ValueExpression>),
    }

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

    /// value_constant ::= boolean_literal | numeric_literal | string_literal | property_constant | property_value
    #[derive(Debug, Clone)]
    pub enum ValueConstant {
        Boolean(bool),
        Numeric(String),
        String(String),
        PropertyConstant(String),
        PropertyValue(String),
    }

    /// value_expression ::= relation { logical_operator relation }*
    #[derive(Debug, Clone)]
    pub struct ValueExpression {
        pub left: Relation,
        pub operations: Vec<LogicalOperation>,
    }

    #[derive(Debug, Clone)]
    pub struct LogicalOperation {
        pub operator: LogicalOperator,
        pub right: Relation,
    }

    /// logical_operator ::= and | or | xor
    #[derive(Debug, Clone, PartialEq)]
    pub enum LogicalOperator {
        And,
        Or,
        Xor,
    }

    /// relation ::= simple_expression [ relational_operator simple_expression ]
    #[derive(Debug, Clone)]
    pub struct Relation {
        pub left: SimpleExpression,
        pub comparison: Option<Comparison>,
    }

    #[derive(Debug, Clone)]
    pub struct Comparison {
        pub operator: RelationalOperator,
        pub right: SimpleExpression,
    }

    /// relational_operator ::= = | != | < | <= | > | >=
    #[derive(Debug, Clone, PartialEq)]
    pub enum RelationalOperator {
        Equal,              // =
        NotEqual,           // !=
        LessThan,           // <
        LessThanOrEqual,    // <=
        GreaterThan,        // >
        GreaterThanOrEqual, // >=
    }

    /// simple_expression ::= [ unary_adding_operator ] term { binary_adding_operator term }*
    #[derive(Debug, Clone)]
    pub struct SimpleExpression {
        pub sign: Option<UnaryAddingOperator>,
        pub left: Term,
        pub operations: Vec<AdditiveOperation>,
    }

    /// unary_adding_operator ::= + | -
    #[derive(Debug, Clone, PartialEq)]
    pub enum UnaryAddingOperator {
        Plus,  // +
        Minus, // -
    }

    #[derive(Debug, Clone)]
    pub enum Action {
        Basic(BasicAction),
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum PortQualifier {
        Count,
        Fresh,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum Quantifier {
        All,
        Exists,
    }

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

        BinaryOp {
            left: Box<BasicExpression>,
            operator: BinaryOperator,
            right: Box<BasicExpression>,
        },

        Not(Box<BasicExpression>),
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum BinaryOperator {
        And,
        Or,

        Equal,              // =
        NotEqual,           // !=
        LessThan,           // <
        LessThanOrEqual,    // <=
        GreaterThan,        // >
        GreaterThanOrEqual, // >=

        Add,      // +
        Subtract, // -
        Multiply, // *
        Divide,   // /
        Modulo,   // mod
    }

    #[derive(Debug, Clone)]
    pub struct BehaviorExpression {
        pub disjunctions: Vec<DisjunctionExpression>,
    }

    #[derive(Debug, Clone)]
    pub struct DisjunctionExpression {
        pub not_conjunctions: Vec<NotConjunctionExpression>,
    }

    #[derive(Debug, Clone)]
    pub struct NotConjunctionExpression {
        pub has_not: bool,
        pub conjunction: ConjunctionExpression,
    }

    #[derive(Debug, Clone)]
    pub struct ConjunctionExpression {
        pub left: ArithmeticExpression,
        pub comparison: Option<ComparisonExpression>,
    }

    #[derive(Debug, Clone)]
    pub struct ComparisonExpression {
        pub operator: ComparisonOperator,
        pub right: ArithmeticExpression,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum ComparisonOperator {
        LessThan,           // <
        LessThanOrEqual,    // <=
        Equal,              // =
        GreaterThan,        // >
        GreaterThanOrEqual, // >=
        NotEqual,           // !=
    }

    #[derive(Debug, Clone)]
    pub struct ArithmeticExpression {
        pub left: AddExpression,
        pub operations: Vec<AdditiveOperation>,
    }

    #[derive(Debug, Clone)]
    pub struct AdditiveOperation {
        pub operator: AdditiveOperator,
        pub right: AddExpression,
    }

    /// binary_adding_operator ::= + | -
    #[derive(Debug, Clone, PartialEq)]
    pub enum AdditiveOperator {
        Add,      // +
        Subtract, // -
    }

    #[derive(Debug, Clone)]
    pub struct AddExpression {
        pub left: BasicExpression,
        pub operations: Vec<MultiplicativeOperation>,
    }

    #[derive(Debug, Clone)]
    pub struct MultiplicativeOperation {
        pub operator: MultiplicativeOperator,
        pub right: BasicExpression,
    }

    /// multiplying_operator ::= * | / | mod | rem
    #[derive(Debug, Clone, PartialEq)]
    pub enum MultiplicativeOperator {
        Multiply,  // *
        Divide,    // /
        Modulo,    // mod
        Remainder, // rem
    }

    /// term ::= factor { multiplying_operator factor }*
    #[derive(Debug, Clone)]
    pub struct Term {
        pub left: Factor,
        pub operations: Vec<MultiplicativeOperation>,
    }

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

    /// binary_numeric_operator ::= **
    #[derive(Debug, Clone, PartialEq)]
    pub enum BinaryNumericOperator {
        Power, // **
    }

    /// unary_numeric_operator ::= abs
    #[derive(Debug, Clone, PartialEq)]
    pub enum UnaryNumericOperator {
        Abs, // abs
    }

    /// unary_boolean_operator ::= not
    #[derive(Debug, Clone, PartialEq)]
    pub enum UnaryBooleanOperator {
        Not, // not
    }

    /// integer_range ::= integer_value .. integer_value
    #[derive(Debug, Clone)]
    pub struct IntegerRange {
        pub lower: IntegerValue,
        pub upper: IntegerValue,
    }

    /// integer_value ::= integer_value_variable | integer_value_constant
    #[derive(Debug, Clone)]
    pub enum IntegerValue {
        Variable(String),
        Constant(String),
    }

    #[derive(Debug, Clone)]
    pub enum PrototypeClause {
        None,
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

    #[derive(Debug, Clone, PartialEq)]
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

    #[derive(Debug, Clone)]
    pub struct UniqueComponentReference {
        pub package_prefix: Option<PackageName>,
        pub identifier: String,
    }

    /* ========== 4.4 Component Implementations ========== */
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

    #[derive(Debug, Clone)]
    pub struct ComponentImplementationExtension {
        pub category: ComponentCategory,
        pub name: ImplementationName,

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

    #[derive(Debug, Clone)]
    pub struct UniqueImplementationReference {
        pub package_prefix: Option<PackageName>,
        pub implementation_name: ImplementationName,
    }

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
        ClassifierReference(UniqueComponentClassifierReference),

        Prototype(String),
    }

    #[derive(Debug, Clone)]
    pub enum UniqueComponentClassifierReference {
        Type(UniqueImplementationReference),
        Implementation(UniqueImplementationReference),
    }
    #[derive(Debug, Clone)]
    pub struct SubcomponentRefinement {
        pub identifier: String,
        pub category: ComponentCategory,
        pub classifier: Option<SubcomponentClassifier>,
        pub array_spec: Option<ArraySpec>,
        pub properties: Vec<Property>,
        //pub modes: Option<ComponentInModes>
    }

    #[derive(Debug, Clone)]
    pub struct ArraySpec {
        pub dimensions: Vec<ArrayDimension>,
        pub element_implementations: Option<Vec<ArrayElementImplementation>>,
    }
    #[derive(Debug, Clone)]
    pub struct ArrayDimension {
        pub size: Option<ArrayDimensionSize>,
    }

    #[derive(Debug, Clone)]
    pub enum ArrayDimensionSize {
        Fixed(u32),
        PropertyReference(String),
    }

    #[derive(Debug, Clone)]
    pub struct ArrayElementImplementation {
        pub implementation: UniqueImplementationReference,
        pub prototype_bindings: Option<PrototypeBindings>,
    }

    /* ========== 4.7 Prototype ========== */
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

    #[derive(Debug, Clone)]
    pub struct ComponentPrototype {
        pub category: ComponentCategory,
        pub classifier: Option<UniqueComponentClassifierReference>,
        pub is_array: bool,
    }

    #[derive(Debug, Clone)]
    pub struct FeatureGroupPrototype {
        pub classifier: Option<UniqueFeatureGroupTypeReference>,
    }

    #[derive(Debug, Clone)]
    pub struct UniqueFeatureGroupTypeReference {
        pub package_prefix: Option<PackageName>,

        pub identifier: String,
    }

    #[derive(Debug, Clone)]
    pub struct FeaturePrototype {
        pub direction: Option<PortDirection>, // in/out
        pub classifier: Option<UniqueComponentClassifierReference>,
    }

    #[derive(Debug, Clone)]
    pub struct PrototypeRefinement {
        pub identifier: String,
        pub prototype: Prototype,
        pub properties: Vec<PrototypePropertyAssociation>,
    }

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

    #[derive(Debug, Clone)]
    pub struct ComponentPrototypeActual {
        pub category: ComponentCategory,
        pub reference: Option<ComponentPrototypeReference>,
        pub bindings: Option<PrototypeBindings>,
    }

    #[derive(Debug, Clone)]
    pub enum ComponentPrototypeReference {
        Classifier(UniqueComponentClassifierReference),
        Prototype(String),
    }

    #[derive(Debug, Clone)]
    pub enum FeatureGroupPrototypeActual {
        Classifier {
            reference: UniqueFeatureGroupTypeReference,
            bindings: Option<PrototypeBindings>,
        },
        Prototype(String),
    }

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
        Prototype(String),
    }

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

    #[derive(Debug, Clone)]
    pub struct PrototypePropertyAssociation {
        pub name: String,
        pub value: PropertyValue,
        pub applies_to: Option<Vec<String>>,
    }

    /*================software component====================*/

    /*==========5.2 Subprograms and Subprogram Calls============= */
    #[derive(Debug, Clone)]
    pub struct CallSequence {
        pub identifier: String,

        pub calls: Vec<SubprogramCall>,

        pub properties: Vec<Property>,

        pub in_modes: Option<Vec<String>>,
    }

    #[derive(Debug, Clone)]
    pub struct SubprogramCall {
        pub identifier: String,

        pub called: CalledSubprogram,

        pub properties: Vec<Property>,
    }

    #[derive(Debug, Clone)]
    pub enum CalledSubprogram {
        Classifier(UniqueComponentClassifierReference),
    }

    /*8 features and shared access */

    #[derive(Debug, Clone)]
    pub enum Feature {
        //Abstract(AbstractFeature),
        Port(PortSpec),

        SubcomponentAccess(SubcomponentAccessSpec),
        //FeatureGroup(FeatureGroupSpec),

        //Parameter(ParameterSpec),

        //Refinement(FeatureRefinement)
    }

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

    #[derive(Debug, Clone)]
    pub enum SubcomponentAccessSpec {
        Data(DataAccessSpec),
        Subprogram(SubprogramAccessSpec),
        // TODO: SubprogramGroup, Bus, VirtualBus
    }

    #[derive(Debug, Clone)]
    pub struct DataAccessSpec {
        /// `defining_data_component_access_identifier`
        pub identifier: String,
        pub direction: AccessDirection, // provides | requires
        /// `data_unique_component_classifier_reference | prototype_identifier`
        pub classifier: Option<DataAccessReference>,
    }

    #[derive(Debug, Clone)]
    pub enum DataAccessReference {
        Classifier(UniqueComponentClassifierReference),

        Prototype(String),
    }

    #[derive(Debug, Clone)]
    pub struct SubprogramAccessSpec {
        /// `defining_subprogram_access_identifier`
        pub identifier: String,
        pub direction: AccessDirection, // provides | requires
        /// `subprogram_unique_component_classifier_reference | subprogram_component_prototype_identifier`
        pub classifier: Option<SubprogramAccessReference>,
    }

    #[derive(Debug, Clone)]
    pub enum SubprogramAccessReference {
        Classifier(UniqueComponentClassifierReference),
        Prototype(String),
    }

    /*=================9 connection ============ */
    #[derive(Debug, Clone)]
    pub enum Connection {
        Port(PortConnection),

        Parameter(ParameterConnection),
        // Feature(FeatureConnection),      // feature_connection
        Access(AccessConnection), // access_connection( data/subprogram)
                                  // FeatureGroup(FeatureGroupConnection), // feature_group_connection
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum ConnectionSymbol {
        ///  ->
        Direct,
        ///  <->
        Didirect,
    }
    #[derive(Debug, Clone)]
    pub struct PortConnection {
        pub identifier: String,
        pub source: PortEndpoint,
        pub destination: PortEndpoint,
        pub connection_direction: ConnectionSymbol,
    }

    #[derive(Debug, Clone)]
    pub enum PortEndpoint {
        ComponentPort(String),

        SubcomponentPort {
            subcomponent: String,
            port: String,
        },

        FeatureGroupPort {
            feature_group: String,
            element: String,
        },

        AggregateDataElement {
            port: String,
            data_element: String,
        },

        RequiresDataAccess(String),

        DataSubcomponent(String),

        SubcomponentDataAccess {
            subcomponent: String,
            access: String,
        },

        FeatureGroupDataAccess {
            feature_group: String,
            element: String,
        },

        NestedDataAccess {
            container: String,
            element: String,
        },

        ProcessorPort {
            processor: Option<String>,
            port: String,
        },

        InternalEvent {
            self_ref: bool,
            identifier: String,
        },
    }

    #[derive(Debug, Clone)]
    pub struct ParameterConnection {
        pub source: ParameterEndpoint,
        pub destination: ParameterEndpoint,
        pub connection_direction: ConnectionSymbol,
    }

    #[derive(Debug, Clone)]
    pub struct AccessConnection {
        pub source: AccessEndpoint,
        pub destination: AccessEndpoint,
        pub connection_direction: ConnectionSymbol,
    }

    #[derive(Debug, Clone)]
    pub enum AccessEndpoint {
        ComponentAccess(String),

        SubcomponentAccess {
            subcomponent: String,
            access: String,
        },
    }

    #[derive(Debug, Clone)]
    pub enum ParameterEndpoint {
        ComponentParameter {
            parameter: String,
            data_subcomponent: Option<String>,
        },

        SubprogramCallParameter {
            call_identifier: String,
            parameter: String,
        },

        ThreadPort {
            port: String,
            data_subcomponent: Option<String>,
        },

        DataSubcomponent(String),

        RequiresDataAccess(String),

        FeatureGroupDataAccess {
            feature_group: String,
            element: String,
        },

        FeatureGroupElement {
            feature_group: String,
            element: String,
        },
    }

    #[derive(Debug, Clone)]
    pub enum Property {
        BasicProperty(BasicPropertyAssociation),
        SubcomponentProperty(BasicPropertyAssociation),
        CallSequenceProperty(BasicPropertyAssociation),
    }

    #[derive(Debug, Clone)]
    pub struct BasicPropertyAssociation {
        pub identifier: PropertyIdentifier,

        pub operator: PropertyOperator,

        pub is_constant: bool,

        pub value: PropertyValue,

        pub applies_to: Option<Vec<String>>,
    }
    #[derive(Debug, Clone)]
    pub struct PropertyIdentifier {
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

    #[derive(Debug, Clone)]
    pub enum PropertyValue {
        Single(PropertyExpression),     // single_property_value
        List(Vec<PropertyListElement>), // property_list_value
    }

    #[derive(Debug, Clone)]
    pub enum PropertyListElement {
        Value(PropertyExpression),
        NestedList(Vec<PropertyListElement>),
    }

    #[derive(Debug, Clone)]
    pub enum PropertyExpression {
        Boolean(BooleanTerm),
        Real(SignedRealOrConstant),
        Integer(SignedIntergerOrConstant),
        String(StringTerm),
        //Enumeration(EnumerationTerm),
        //Unit(UnitTerm),
        IntegerRange(IntegerRangeTerm),
        //RealRange(RealRangeTerm),

        //PropertyReference(PropertyTerm),
        ComponentClassifier(ComponentClassifierTerm),
        Reference(ReferenceTerm),
        //Record(RecordTerm),
        //Computed(ComputedTerm),
    }

    #[derive(Debug, Clone)]
    pub struct PropertyConstantTerm {
        pub property_set: Option<String>,
        pub name: String,
    }

    #[derive(Debug, Clone)]
    pub enum BooleanTerm {
        Literal(bool),                  // boolean_value
        Constant(PropertyConstantTerm), // boolean_property_constant_term
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Sign {
        Plus,  // +
        Minus, // -
    }

    #[derive(Debug, Clone)]
    pub enum SignedRealOrConstant {
        Real(SignedReal),
        Constant {
            sign: Option<Sign>,
            constant: PropertyConstantTerm,
        },
    }

    #[derive(Debug, Clone)]
    pub struct SignedReal {
        pub sign: Option<Sign>,
        pub value: f64,
        pub unit: Option<String>,
    }

    #[derive(Debug, Clone)]
    pub enum SignedIntergerOrConstant {
        Real(SignedInteger),

        Constant {
            sign: Option<Sign>,
            constant: PropertyConstantTerm,
        },
    }

    #[derive(Debug, Clone)]
    pub struct SignedInteger {
        pub sign: Option<Sign>,
        pub value: i64,
        pub unit: Option<String>,
    }
    #[derive(Debug, Clone)]
    pub enum StringTerm {
        Literal(String),
        Constant(PropertyConstantTerm),
    }

    #[derive(Debug, Clone)]
    pub struct IntegerRangeTerm {
        pub lower: StringWithUnit,
        pub upper: StringWithUnit,
    }

    #[derive(Debug, Clone)]
    pub struct StringWithUnit {
        pub value: String,
        pub unit: Option<String>,
    }

    #[derive(Debug, Clone)]
    pub struct ReferenceTerm {
        pub identifier: String,
    }

    #[derive(Debug, Clone)]
    pub struct ComponentClassifierTerm {
        pub unique_component_classifier_reference: UniqueComponentClassifierReference,
    }
} //end mod aadl_ast_cj
