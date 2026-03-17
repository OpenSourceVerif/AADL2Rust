#![allow(
    unused_mut,
    clippy::manual_strip,
)]
// use crate::aadlight_parser;
use crate::aadlight_parser::aadl::{AADLParser, Rule as AADLRule};
use crate::aadlight_parser::ba::{BAParser, Rule as BARule};
use pest::Parser;
use super::ast::aadl_ast_cj::*;
use pest::{iterators::Pair};

// 辅助函数：从 Pair 中提取标识符
pub fn extract_ba_identifier(pair: Pair<BARule>) -> String {
    pair.as_str().trim().to_string()
}

/// 转换 annex 子句
/// 处理 annex_subclause 规则
pub fn transform_annex_subclause(pair: Pair<AADLRule>) -> Option<AnnexSubclause> {
    let mut inner_iter = pair.into_inner();
    
    // 第一个元素应该是 annex_identifier
    let identifier_pair = inner_iter.next().unwrap();
    let identifier = transform_annex_identifier(identifier_pair);
    
    // 第二个元素应该是 annex_content (在 {** **} 中)
    let content_pair = inner_iter.next().unwrap();
    let content = transform_annex_content(content_pair);
    
    Some(AnnexSubclause {
        identifier,
        content,
    })
}

/// 转换 annex 标识符
/// 处理 annex_identifier 规则
pub fn transform_annex_identifier(pair: Pair<AADLRule>) -> AnnexIdentifier {
    match pair.as_str().trim() {
        "Behavior_specification" => AnnexIdentifier::BehaviorSpecification,
        "EMV2" => AnnexIdentifier::EMV2,
        _ => {
            // 对于其他标识符，暂时默认为 BehaviorSpecification
            // 可以根据需要扩展
            AnnexIdentifier::BehaviorSpecification
        }
    }
}

/// 转换 annex 内容
/// 处理 annex_content 规则
pub fn transform_annex_content(pair: Pair<AADLRule>) -> AnnexContent {
    // 1. 获取纯文本字符串
    let content_str = pair.as_str().trim();
    
    if content_str.is_empty() {
        return AnnexContent::None;
    }

    // 2. 调用 BA 解析器解析字符串
    // parse 得到的是 BARule 类型的 Pairs
    match BAParser::parse(BARule::behavior_annex, content_str) {
        Ok(mut pairs) => {
            // 获取根节点 behavior_annex
            let root = pairs.next().unwrap(); 
            
            // 3. 在新解析出的树中查找 behavior_annex_content
            for inner in root.into_inner() {
                // 现在 inner 是 BARule，类型匹配正确
                if inner.as_rule() == BARule::behavior_annex_content {
                    if let Some(ba_content) = transform_behavior_annex_content(inner) {
                        return AnnexContent::BehaviorAnnex(ba_content);
                    }
                }
            }
            AnnexContent::None
        }
        Err(e) => {
            // 打印详细的语法错误信息，方便调试
            eprintln!("Behavior Annex 解析错误:\n{}", e);
            AnnexContent::None
        }
    }
}

/// 转换 Behavior Annex 内容
/// 处理 behavior_annex_content 规则
pub fn transform_behavior_annex_content(pair: Pair<BARule>) -> Option<BehaviorAnnexContent> {
    let mut state_variables = None;
    let mut states = None;
    let mut transitions = None;
    
    for inner in pair.into_inner() {
        match inner.as_rule() {
            BARule::state_variables => {
                state_variables = Some(transform_state_variables(inner));
            }
            BARule::states => {
                states = Some(transform_states(inner));
            }
            BARule::transitions => {
                transitions = Some(transform_transitions(inner));
            }
            _ => {}
        }
    }
    
    Some(BehaviorAnnexContent {
        state_variables,
        states,
        transitions,
    })
}

/// 转换状态变量声明
/// 处理 state_variables 规则
pub fn transform_state_variables(pair: Pair<BARule>) -> Vec<StateVariable> {
    let mut variables = Vec::new();
    
    for inner in pair.into_inner() {
        if inner.as_rule() == BARule::state_variable_declaration {
            variables.push(transform_state_variable_declaration(inner));
        }
    }
    
    variables
}

/// 转换单个状态变量声明
/// 处理 state_variable_declaration 规则
pub fn transform_state_variable_declaration(pair: Pair<BARule>) -> StateVariable {
    let mut inner_iter = pair.into_inner();
    
    let identifier = extract_ba_identifier(inner_iter.next().unwrap());
    //let _colon = inner_iter.next(); // 跳过 ":"
    let data_type = extract_ba_identifier(inner_iter.next().unwrap());
    
    let mut initial_value = None;
    if let Some(assignment_pair) = inner_iter.next() {
        if assignment_pair.as_rule() == BARule::assignment_operator {
            // 跳过 ":="
            if let Some(value_pair) = inner_iter.next() {
                if value_pair.as_rule() == BARule::behavior_expression {
                    initial_value = Some(extract_ba_identifier(value_pair));
                }
            }
        }
    }
    
    StateVariable {
        identifier,
        data_type,
        initial_value,
    }
}

/// 转换状态定义
/// 处理 states 规则
pub fn transform_states(pair: Pair<BARule>) -> Vec<State> {
    let mut state_list = Vec::new();
    
    for inner in pair.into_inner() {
        if inner.as_rule() == BARule::state_declaration {
            state_list.push(transform_state_declaration(inner));
        }
    }
    
    state_list
}

/// 转换单个状态声明
/// 处理 state_declaration 规则
pub fn transform_state_declaration(pair: Pair<BARule>) -> State {
    let mut inner_iter = pair.into_inner();
    
    let mut identifiers = Vec::new();
    let mut modifiers = Vec::new();
    
    // 处理标识符列表
    for inner in inner_iter {
        match inner.as_rule() {
            BARule::identifier => {
                identifiers.push(extract_ba_identifier(inner));
            }
            BARule::state_modifier => {
                modifiers.push(transform_state_modifier(inner));
            }
            _ => {}
        }
    }
    
    State {
        identifiers,
        modifiers,
    }
}

/// 转换状态修饰符
/// 处理 state_modifier 规则
pub fn transform_state_modifier(pair: Pair<BARule>) -> StateModifier {
    match pair.as_str().trim() {
        "initial" => StateModifier::Initial,
        "complete" => StateModifier::Complete,
        "final" => StateModifier::Final,
        // 其他修饰符可以根据需要添加
        _ => StateModifier::Complete, // 默认值
    }
}

/// 转换转换定义
/// 处理 transitions 规则
pub fn transform_transitions(pair: Pair<BARule>) -> Vec<Transition> {
    let mut transition_list = Vec::new();
    
    for inner in pair.into_inner() {
        if inner.as_rule() == BARule::transition_declaration {
            transition_list.push(transform_transition_declaration(inner));
        }
    }
    
    transition_list
}

/// 转换单个转换声明
/// 处理 transition_declaration 规则
/// 语法: identifier ~ "-[" ~ guard? ~ "]->" ~ identifier ~ behavior_action_block? ~ ";"
pub fn transform_transition_declaration(pair: Pair<BARule>) -> Transition {
    let mut inner_iter = pair.into_inner();
    
    // 第一个 identifier 是源状态
    let source_state = extract_ba_identifier(inner_iter.next().unwrap());
    let source_states = vec![source_state];
    
    // 跳过 "-["
    //let _dash_bracket = inner_iter.next();
    
    let mut behavior_condition = None;
    let mut destination_state = String::new();
    let mut actions = None;
    
    // 处理守卫条件（可选）
    if let Some(guard_pair) = inner_iter.next() {
        if guard_pair.as_rule() == BARule::guard {
            behavior_condition = Some(transform_guard(guard_pair));
        } else {
            // 如果不是guard，那么这是目标状态
            destination_state = extract_ba_identifier(guard_pair);
        }
    }
    
    // 跳过 "]->"
    //let _arrow_bracket = inner_iter.next();
    
    // 如果还没有设置目标状态，那么下一个identifier就是目标状态
    if destination_state.is_empty() {
        if let Some(dest_pair) = inner_iter.next() {
            if dest_pair.as_rule() == BARule::identifier {
                destination_state = extract_ba_identifier(dest_pair);
            }
        }
    }
    
    // 处理动作块（可选）
    if let Some(action_pair) = inner_iter.next() {
        if action_pair.as_rule() == BARule::behavior_action_block {
            actions = Some(transform_behavior_action_block(action_pair));
        }
    }
    
    Transition {
        transition_identifier: None, // 转换暂不设置独立的标识符
        priority: None,
        source_states,
        destination_state,
        behavior_condition,
        actions,
    }
}

/// 转换守卫条件
/// 处理 guard 规则
pub fn transform_guard(pair: Pair<BARule>) -> BehaviorCondition {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        BARule::dispatch_condition => {
            // 处理 "on dispatch"
            transform_dispatch_condition(inner)
        }
        BARule::execute_condition => {
            // 处理 execute_condition: unary_boolean_operator? ~ identifier
            transform_execute_condition(inner)
        }
        _ => {
            // 默认处理
            BehaviorCondition::Execute(DispatchConjunction {
                not: false,
                dispatch_triggers: vec![],
                number: None,
                less_than: false,
            })
        }
    }
}

/// 转换分发条件
/// 处理 dispatch_condition 规则
fn transform_dispatch_condition(pair: Pair<BARule>) -> BehaviorCondition {
    let mut inner_iter = pair.into_inner();
    
    // 跳过 "on dispatch"
    // 处理可选的 dispatch_trigger_condition
    let mut trigger_condition = None;
    let mut frozen_ports = None;
    
    for inner in inner_iter {
        match inner.as_rule() {
            BARule::dispatch_trigger_condition => {
                trigger_condition = Some(transform_dispatch_trigger_condition(inner));
            }
            BARule::frozen_ports => {
                frozen_ports = Some(transform_frozen_ports(inner));
            }
            _ => {}
        }
    }
    
    BehaviorCondition::Dispatch(DispatchCondition {
        trigger_condition,
        frozen_ports,
    })
}

/// 转换执行条件
/// 处理 execute_condition 规则
fn transform_execute_condition(pair: Pair<BARule>) -> BehaviorCondition {
    let inner_iter = pair.into_inner();
    let mut has_not = false;
    let mut identifier = String::new();
    let mut number = String::new();
    let mut less_than = false;
    // 遍历所有内部元素
    for inner in inner_iter {
        match inner.as_rule() {
            BARule::unary_boolean_operator => {
                has_not = true;
            }
            BARule::identifier => {
                identifier = inner.as_str().to_string();
            }
            BARule::number => {
                number = inner.as_str().trim().to_string();
            }
            BARule::less_than_operator => {
                less_than = true;
            }
            _ => {
                // 忽略其他规则
            }
        }
    }
    
    // 创建 DispatchConjunction
    let conjunction = DispatchConjunction {
        not: has_not,
        dispatch_triggers: vec![DispatchTrigger::InEventPort(identifier)],
        number: Some(number),
        less_than,
    };
    
    BehaviorCondition::Execute(conjunction)
}

/// 转换分发触发条件
/// 处理 dispatch_trigger_condition 规则
fn transform_dispatch_trigger_condition(pair: Pair<BARule>) -> DispatchTriggerCondition {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        BARule::dispatch_trigger_logical_expression => {
            DispatchTriggerCondition::LogicalExpression(transform_dispatch_trigger_logical_expression(inner))
        }
        BARule::provides_subprogram_access_identifier => {
            DispatchTriggerCondition::SubprogramAccess(inner.as_str().to_string())
        }
        _ if inner.as_str() == "stop" => {
            DispatchTriggerCondition::Stop
        }
        _ => {
            // 默认处理
            DispatchTriggerCondition::Stop
        }
    }
}

/// 转换分发触发逻辑表达式
/// 处理 dispatch_trigger_logical_expression 规则
fn transform_dispatch_trigger_logical_expression(pair: Pair<BARule>) -> DispatchTriggerLogicalExpression {
    let mut conjunctions = Vec::new();
    
    for inner in pair.into_inner() {
        if inner.as_rule() == BARule::dispatch_conjunction {
            conjunctions.push(transform_dispatch_conjunction(inner));
        }
    }
    
    DispatchTriggerLogicalExpression {
        dispatch_conjunctions: conjunctions,
    }
}

/// 转换分发合取表达式
/// 处理 dispatch_conjunction 规则
fn transform_dispatch_conjunction(pair: Pair<BARule>) -> DispatchConjunction {
    let mut triggers = Vec::new();
    
    for inner in pair.into_inner() {
        if inner.as_rule() == BARule::dispatch_trigger {
            triggers.push(transform_dispatch_trigger(inner));
        }
    }
    
    DispatchConjunction {
        not: false, // 简化处理，暂时不支持 not
        dispatch_triggers: triggers,
        number: None,
        less_than: false,
    }
}

/// 转换分发触发器
/// 处理 dispatch_trigger 规则
fn transform_dispatch_trigger(pair: Pair<BARule>) -> DispatchTrigger {
    let identifier = pair.as_str().to_string();
    
    // 简化处理，假设都是事件端口
    DispatchTrigger::InEventPort(identifier)
}

/// 转换冻结端口
/// 处理 frozen_ports 规则
fn transform_frozen_ports(pair: Pair<BARule>) -> Vec<String> {
    let mut ports = Vec::new();
    
    for inner in pair.into_inner() {
        if inner.as_rule() == BARule::identifier {
            ports.push(inner.as_str().to_string());
        }
    }
    
    ports
}

/// 转换行为动作块
/// 处理 behavior_action_block 规则
/// 语法: "{" ~ behavior_actions ~ "}" ~ ("timeout" ~ behavior_time)?
pub fn transform_behavior_action_block(pair: Pair<BARule>) -> BehaviorActionBlock {
    let mut inner_iter = pair.into_inner();
    
    // 跳过开头的 "{"
    //let _open_brace = inner_iter.next();
    
    // 处理 behavior_actions
    let behavior_actions_pair = inner_iter.next().unwrap();
    let actions = transform_behavior_actions(behavior_actions_pair);
    
    // 跳过结尾的 "}"
    //let _close_brace = inner_iter.next();
    
    // 处理可选的 timeout
    let mut timeout = None;
    if let Some(timeout_pair) = inner_iter.next() {
        if timeout_pair.as_rule() == BARule::behavior_time {
            timeout = Some(transform_behavior_time(timeout_pair));
        }
    }
    
    BehaviorActionBlock {
        actions,
        timeout,
    }
}

/// 转换行为动作
/// 处理 behavior_actions 规则
/// 语法: behavior_action_sequence | behavior_action_set | behavior_action
pub fn transform_behavior_actions(pair: Pair<BARule>) -> BehaviorActions {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        BARule::behavior_action_sequence => {
            BehaviorActions::Sequence(transform_behavior_action_sequence(inner))
        }
        BARule::behavior_action_set => {
            BehaviorActions::Set(transform_behavior_action_set(inner))
        }
        BARule::behavior_action => {
            // 单个动作包装成序列
            let action = transform_behavior_action(inner);
            BehaviorActions::Sequence(BehaviorActionSequence {
                actions: vec![action],
            })
        }
        _ => {
            // 默认处理
            BehaviorActions::Sequence(BehaviorActionSequence {
                actions: vec![BehaviorAction::Basic(BasicAction::Assignment(AssignmentAction {
                    target: Target::LocalVariable("default".to_string()),
                    value: AssignmentValue::Any,
                }))],
            })
        }
    }
}

/// 转换行为动作序列
/// 处理 behavior_action_sequence 规则
/// 语法: behavior_action ~ (";" ~ behavior_action)+
pub fn transform_behavior_action_sequence(pair: Pair<BARule>) -> BehaviorActionSequence {
    let mut actions = Vec::new();
    
    for inner in pair.into_inner() {
        if inner.as_rule() == BARule::behavior_action {
            actions.push(transform_behavior_action(inner));
        }
    }
    
    BehaviorActionSequence { actions }
}

/// 转换行为动作集合
/// 处理 behavior_action_set 规则
/// 语法: behavior_action ~ ("&" ~ behavior_action)+
pub fn transform_behavior_action_set(pair: Pair<BARule>) -> BehaviorActionSet {
    let mut actions = Vec::new();
    
    for inner in pair.into_inner() {
        if inner.as_rule() == BARule::behavior_action {
            actions.push(transform_behavior_action(inner));
        }
    }
    
    BehaviorActionSet { actions }
}

/// 转换单个行为动作
/// 处理 behavior_action 规则
pub fn transform_behavior_action(pair: Pair<BARule>) -> BehaviorAction {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        BARule::basic_action => {
            BehaviorAction::Basic(transform_basic_action(inner))
        }
        BARule::behavior_action_block => {
            BehaviorAction::Block(Box::new(transform_behavior_action_block(inner)))
        }
        BARule::if_statement => {
            BehaviorAction::If(transform_if_statement(inner))
        }
        BARule::for_statement => {
            BehaviorAction::For(transform_for_statement(inner))
        }
        BARule::forall_statement => {
            BehaviorAction::Forall(transform_forall_statement(inner))
        }
        BARule::while_statement => {
            BehaviorAction::While(transform_while_statement(inner))
        }
        BARule::do_until_statement => {
            BehaviorAction::DoUntil(transform_do_until_statement(inner))
        }
        _ => {
            // 默认处理
            BehaviorAction::Basic(BasicAction::Assignment(AssignmentAction {
                target: Target::LocalVariable("default".to_string()),
                value: AssignmentValue::Any,
            }))
        }
    }
}

/// 转换基本动作
/// 处理 basic_action 规则
pub fn transform_basic_action(pair: Pair<BARule>) -> BasicAction {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        BARule::assignment_action => {
            transform_assignment_action(inner)
        }
        BARule::communication_action => {
            transform_communication_action(inner)
        }
        BARule::computation_action => {
            transform_computation_action(inner)
        }
        _ => {
            // 默认赋值动作
            BasicAction::Assignment(AssignmentAction {
                target: Target::LocalVariable("default".to_string()),
                value: AssignmentValue::Any,
            })
        }
    }
}

/// 转换赋值动作
/// 处理 assignment_action 规则
pub fn transform_assignment_action(pair: Pair<BARule>) -> BasicAction {
    let mut inner_iter = pair.into_inner();
    
    let target_str = extract_ba_identifier(inner_iter.next().unwrap());
    //let _assign = inner_iter.next(); // 跳过 ":="
    
    // 处理赋值表达式或 "any"
    let value = if let Some(value_pair) = inner_iter.next() {
        match value_pair.as_rule() {
            BARule::behavior_expression => {
                // 处理行为表达式
                AssignmentValue::Expression(transform_behavior_expression(value_pair))
            }
            _ => {
                // 检查是否是 "any"
                if value_pair.as_str() == "any" {
                    AssignmentValue::Any
                } else {
                    // 如果不是 "any"，尝试作为表达式处理
                    AssignmentValue::Expression(transform_behavior_expression(value_pair))
                }
            }
        }
    } else {
        AssignmentValue::Any
    };
    
    // 根据 target_str 确定目标类型
    let target = {
        use crate::transform::get_global_port_manager;
        if let Ok(manager) = get_global_port_manager().lock() {
            if manager.is_outgoing_port(&target_str) {
                Target::OutgoingPort(target_str)
            } else {
                Target::LocalVariable(target_str)
            }
        } else {
            // 如果无法获取端口管理器，默认为本地变量
            Target::LocalVariable(target_str)
        }
    };
    
    BasicAction::Assignment(AssignmentAction {
        target,
        value,
    })
}

/// 转换通信动作
/// 处理 communication_action 规则
pub fn transform_communication_action(pair: Pair<BARule>) -> BasicAction {
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        BARule::port_communication => {
            let mut inner_iter = inner.into_inner();
            BasicAction::Communication(CommunicationAction::PortCommunication(
                PortCommunication::Output {
                    port: extract_ba_identifier(inner_iter.next().unwrap()),
                    value: Some(transform_behavior_expression(inner_iter.next().unwrap())),
                }
            ))
        }
        BARule::data_access_communication => {
            BasicAction::Communication(CommunicationAction::DataAccessCommunication(
                DataAccessCommunication::RequiredDataAccess {
                    name: extract_ba_identifier(inner),
                    direction: DataAccessDirection::Input,
                }
            ))
        }
        BARule::broadcast_action => {
            BasicAction::Communication(CommunicationAction::Broadcast(
                Broadcast::Input
            ))
        }
        _ => {
            // 默认端口通信
            BasicAction::Communication(CommunicationAction::PortCommunication(
                PortCommunication::Output {
                    port: "".to_string(),
                    value: None,
                }
            ))
        }
    }
}

/// 转换计算动作
/// 处理 computation_action 规则
pub fn transform_computation_action(pair: Pair<BARule>) -> BasicAction {
    let inner = pair.into_inner().next().unwrap();
    
    BasicAction::Timed(TimedAction {
        start_time: BehaviorTime {
            value: IntegerValue::Constant(extract_ba_identifier(inner)),
            unit: "ms".to_string(),
        },
        end_time: None,
    })
}

/// 转换 annexes 子句（用于组件类型和实现）
/// 这个函数被 transform.rs 调用
pub fn transform_annexes_clause(pair: Pair<AADLRule>) -> Vec<AnnexSubclause> {
    let mut annexes = Vec::new();
    
    for inner in pair.into_inner() {
        if inner.as_rule() == AADLRule::annex_subclause {
            if let Some(annex) = transform_annex_subclause(inner) {
                annexes.push(annex);
            }
        }
    }
    
    annexes
}

/// 转换行为时间
/// 处理 behavior_time 规则
pub fn transform_behavior_time(_pair: Pair<BARule>) -> BehaviorTime {
    // 暂时返回默认值，后续可以根据需要完善
    BehaviorTime {
        value: IntegerValue::Constant("0".to_string()),
        unit: "ms".to_string(),
    }
}

/// 转换 if 语句
/// 处理 if_statement 规则
pub fn transform_if_statement(_pair: Pair<BARule>) -> IfStatement {
    // 暂时返回默认值，后续可以根据需要完善
    IfStatement {
        condition: BehaviorExpression { disjunctions: vec![] },
        then_actions: Box::new(BehaviorActions::Sequence(BehaviorActionSequence { actions: vec![] })),
        elsif_branches: vec![],
        else_actions: None,
    }
}

/// 转换 for 语句
/// 处理 for_statement 规则
pub fn transform_for_statement(_pair: Pair<BARule>) -> ForStatement {
    // 暂时返回默认值，后续可以根据需要完善
    ForStatement {
        element_identifier: "".to_string(),
        data_classifier: "".to_string(),
        element_values: ElementValues::IntegerRange(IntegerRange { 
            lower: IntegerValue::Constant("0".to_string()),
            upper: IntegerValue::Constant("0".to_string())
        }),
        actions: Box::new(BehaviorActions::Sequence(BehaviorActionSequence { actions: vec![] })),
    }
}

/// 转换 forall 语句
/// 处理 forall_statement 规则
pub fn transform_forall_statement(_pair: Pair<BARule>) -> ForallStatement {
    // 暂时返回默认值，后续可以根据需要完善
    ForallStatement {
        element_identifier: "".to_string(),
        data_classifier: "".to_string(),
        element_values: ElementValues::IntegerRange(IntegerRange { 
            lower: IntegerValue::Constant("0".to_string()),
            upper: IntegerValue::Constant("0".to_string())
        }),
        actions: Box::new(BehaviorActions::Sequence(BehaviorActionSequence { actions: vec![] })),
    }
}

/// 转换 while 语句
/// 处理 while_statement 规则
pub fn transform_while_statement(_pair: Pair<BARule>) -> WhileStatement {
    // 暂时返回默认值，后续可以根据需要完善
    WhileStatement {
        condition: BehaviorExpression { disjunctions: vec![] },
        actions: Box::new(BehaviorActions::Sequence(BehaviorActionSequence { actions: vec![] })),
    }
}

/// 转换 do-until 语句
/// 处理 do_until_statement 规则
pub fn transform_do_until_statement(_pair: Pair<BARule>) -> DoUntilStatement {
    // 暂时返回默认值，后续可以根据需要完善
    DoUntilStatement {
        actions: Box::new(BehaviorActions::Sequence(BehaviorActionSequence { actions: vec![] })),
        condition: BehaviorExpression { disjunctions: vec![] },
    }
}

/// 转换行为表达式
/// 处理 behavior_expression 规则
pub fn transform_behavior_expression(pair: Pair<BARule>) -> ValueExpression {
    // behavior_expression = value_expression
    // value_expression = relation ~ (logical_operator ~ relation)*
    let mut inner_iter = pair.into_inner().next().unwrap().into_inner();//这里需要深入一层
    let first_relation = inner_iter.next().unwrap();
    
    let left = transform_relation(first_relation);
    let mut operations = Vec::new();
    
    // 处理后续的逻辑操作
    while let Some(logical_op) = inner_iter.next() {
        let operator = match logical_op.as_str() {
            "and" => LogicalOperator::And,
            "or" => LogicalOperator::Or,
            "xor" => LogicalOperator::Xor,
            _ => panic!("Unknown logical operator: {}", logical_op.as_str()),
        };
        
        let right_relation = inner_iter.next().unwrap();
        let right = transform_relation(right_relation);
        
        operations.push(LogicalOperation { operator, right });
    }
    
    ValueExpression { left, operations }
}

fn transform_relation(pair: Pair<BARule>) -> Relation {
    // relation = simple_expression ~ (relational_operator ~ simple_expression)?
    let mut inner_iter = pair.into_inner();
    let left = transform_simple_expression(inner_iter.next().unwrap());
    
    let comparison = if let Some(relational_op) = inner_iter.next() {
        let operator = match relational_op.as_str() {
            "=" => RelationalOperator::Equal,
            "!=" => RelationalOperator::NotEqual,
            "<" => RelationalOperator::LessThan,
            "<=" => RelationalOperator::LessThanOrEqual,
            ">" => RelationalOperator::GreaterThan,
            ">=" => RelationalOperator::GreaterThanOrEqual,
            _ => panic!("Unknown relational operator: {}", relational_op.as_str()),
        };
        
        let right = transform_simple_expression(inner_iter.next().unwrap());
        Some(Comparison { operator, right })
    } else {
        None
    };
    
    Relation { left, comparison }
}

fn transform_simple_expression(pair: Pair<BARule>) -> SimpleExpression {
    // simple_expression = unary_adding_operator? ~ term ~ (binary_adding_operator ~ term)*
    let mut inner_iter = pair.into_inner();
    
    // 处理一元加法操作符
    let sign = if let Some(unary_op) = inner_iter.peek() {
        if unary_op.as_rule() == BARule::unary_adding_operator {
            let op = inner_iter.next().unwrap();
            match op.as_str() {
                "+" => Some(UnaryAddingOperator::Plus),
                "-" => Some(UnaryAddingOperator::Minus),
                _ => panic!("Unknown unary adding operator: {}", op.as_str()),
            }
        } else {
            None
        }
    } else {
        None
    };
    
    let left = transform_term(inner_iter.next().unwrap());
    let mut operations = Vec::new();
    
    // 处理二元加法操作
    while let Some(binary_op) = inner_iter.next() {
        // 检查是否是二元加法操作符
        if binary_op.as_rule() == BARule::binary_adding_operator {
            let operator = match binary_op.as_str() {
                "+" => AdditiveOperator::Add,
                "-" => AdditiveOperator::Subtract,
                _ => panic!("Unknown binary adding operator: {}", binary_op.as_str()),
            };
            
            let right_term = transform_term(inner_iter.next().unwrap());
            // 将 Factor 转换为 BasicExpression
            let right_basic = match &right_term.left {
                Factor::Value(Value::Variable(ValueVariable::LocalVariable(name))) => {
                    BasicExpression::BehaviorVariable(name.clone())
                }
                Factor::Value(Value::Constant(ValueConstant::Numeric(num))) => {
                    BasicExpression::NumericOrConstant(num.clone())
                }
                Factor::Value(Value::Constant(ValueConstant::Boolean(b))) => {
                    BasicExpression::NumericOrConstant(if *b { "true".to_string() } else { "false".to_string() })
                }
                _ => BasicExpression::BehaviorVariable("temp".to_string()), // 默认值
            };
            let right = AddExpression {
                left: right_basic,
                operations: Vec::new(), // 暂时为空，后续可以完善
            };
            operations.push(AdditiveOperation { operator, right });
        } else {
            // 如果不是二元加法操作符，说明这个操作符属于更高层次的表达式
            // 我们需要停止处理，让上层函数处理
            break;
        }
    }
    SimpleExpression { sign, left, operations }
}

fn transform_term(pair: Pair<BARule>) -> Term {
    // term = factor ~ (multiplying_operator ~ factor)*
    let mut inner_iter = pair.into_inner();
    let left = transform_factor(inner_iter.next().unwrap());
    let mut operations = Vec::new();
    
    // 处理乘法操作
    while let Some(mult_op) = inner_iter.next() {
        // 检查是否是乘法操作符
        if mult_op.as_rule() == BARule::multiplying_operator {
            let operator = match mult_op.as_str() {
                "*" => MultiplicativeOperator::Multiply,
                "/" => MultiplicativeOperator::Divide,
                "mod" => MultiplicativeOperator::Modulo,
                "rem" => MultiplicativeOperator::Remainder,
                _ => panic!("Unknown multiplying operator: {}", mult_op.as_str()),
            };
            
            let right_factor = transform_factor(inner_iter.next().unwrap());
            // 将 Factor 转换为 BasicExpression
            let right_basic = match &right_factor {
                Factor::Value(Value::Variable(ValueVariable::LocalVariable(name))) => {
                    BasicExpression::BehaviorVariable(name.clone())
                }
                Factor::Value(Value::Constant(ValueConstant::Numeric(num))) => {
                    BasicExpression::NumericOrConstant(num.clone())
                }
                Factor::Value(Value::Constant(ValueConstant::Boolean(b))) => {
                    BasicExpression::NumericOrConstant(if *b { "true".to_string() } else { "false".to_string() })
                }
                _ => BasicExpression::BehaviorVariable("temp".to_string()), // 默认值
            };
            operations.push(MultiplicativeOperation { operator, right: right_basic });
        } else {
            // 如果不是乘法操作符，说明这个操作符属于更高层次的表达式
            // 我们需要停止处理，让上层函数处理
            break;
        }
    }
    
    Term { left, operations }
}

fn transform_factor(pair: Pair<BARule>) -> Factor {
    // factor = value | (value ~ binary_numeric_operator ~ value) | (unary_numeric_operator ~ value) | (unary_boolean_operator ~ value)
    let mut inner_iter = pair.into_inner();
    let first = inner_iter.next().unwrap();
    
    match first.as_rule() {
        BARule::value => {
            let value = transform_value(first);
            // 检查是否有二元数值操作符
            if let Some(binary_op) = inner_iter.next() {
                if binary_op.as_rule() == BARule::binary_numeric_operator {
                    let operator = match binary_op.as_str() {
                        "**" => BinaryNumericOperator::Power,
                        _ => panic!("Unknown binary numeric operator: {}", binary_op.as_str()),
                    };
                    
                    let right = transform_value(inner_iter.next().unwrap());
                    Factor::BinaryNumeric { left: value, operator, right }
                } else {
                    // 如果不是二元操作符，回退到第一个元素
                    Factor::Value(value)
                }
            } else {
                Factor::Value(value)
            }
        }
        BARule::unary_numeric_operator => {
            let operator = match first.as_str() {
                "abs" => UnaryNumericOperator::Abs,
                _ => panic!("Unknown unary numeric operator: {}", first.as_str()),
            };
            
            let value = transform_value(inner_iter.next().unwrap());
            Factor::UnaryNumeric { operator, value }
        }
        BARule::unary_boolean_operator => {
            let operator = match first.as_str() {
                "not" => UnaryBooleanOperator::Not,
                _ => panic!("Unknown unary boolean operator: {}", first.as_str()),
            };
            
            let value = transform_value(inner_iter.next().unwrap());
            Factor::UnaryBoolean { operator, value }
        }
        BARule::factor => {
            // 处理嵌套的 factor 规则
            transform_factor(first)
        }
        _ => panic!("Unexpected factor rule: {:?}", first.as_rule()),
    }
}

fn transform_value(pair: Pair<BARule>) -> Value {
    // value = value_constant | value_variable | ("(" ~ simple_expression ~ ")")
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        BARule::value_constant => {
            Value::Constant(transform_value_constant(inner))
        }
        BARule::value_variable => {
            Value::Variable(transform_value_variable(inner))
        }
        BARule::simple_expression => {
            // 处理括号表达式: (simple_expression)
            let expr = transform_simple_expression(inner);
            // 注意：这里需要将 SimpleExpression 转换为 ValueExpression
            // 由于 ValueExpression 需要 Relation，我们需要创建一个简单的转换
            let relation = Relation {
                left: expr,
                comparison: None,
            };
            let value_expr = ValueExpression {
                left: relation,
                operations: Vec::new(),
            };
            Value::Expression(Box::new(value_expr))
        }
        _ => panic!("Unknown value rule: {:?}", inner.as_rule()),
    }
}

fn transform_value_constant(pair: Pair<BARule>) -> ValueConstant {
    // value_constant = number | boolean | string_literal
    let inner = pair.into_inner().next().unwrap();
    
    match inner.as_rule() {
        BARule::number => {
            ValueConstant::Numeric(inner.as_str().trim().to_string())
        }
        BARule::boolean => {
            let val = match inner.as_str() {
                "true" => true,
                "false" => false,
                _ => panic!("Invalid boolean: {}", inner.as_str()),
            };
            ValueConstant::Boolean(val)
        }
        BARule::string_literal => {
            let raw = inner.as_str();
            let value = if raw.starts_with('"') && raw.ends_with('"') {
                raw[1..raw.len() - 1].to_string()
            } else {
                raw.to_string()
            };
            ValueConstant::String(value)
        }
        _ => panic!("Unknown value constant rule: {:?}", inner.as_rule()),
    }
}

fn transform_value_variable(pair: Pair<BARule>) -> ValueVariable {
    // value_variable = (identifier ~ "'count") | (identifier ~ "'fresh") | (identifier ~ "?") | identifier
    let text = pair.as_str();
    
    if text.ends_with("'count") {
        let port = text[..text.len() - 6].to_string();
        ValueVariable::PortCount(port)
    } else if text.ends_with("'fresh") {
        let port = text[..text.len() - 7].to_string();
        ValueVariable::PortFresh(port)
    } else if text.ends_with("?") {
        let port = text[..text.len() - 1].to_string();
        ValueVariable::IncomingPortCheck(port)
    } else {
        // 默认为本地变量
        ValueVariable::LocalVariable(text.to_string())
    }
}

