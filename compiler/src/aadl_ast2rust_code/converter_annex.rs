// src/aadlAst2rustCode/converter_annex.rs
// Behavior Annex 代码生成器
#![allow(
    clippy::empty_line_after_doc_comments,
    clippy::only_used_in_recursion,
)]
use super::intermediate_ast::*;
use crate::ast::aadl_ast_cj::*;
use std::collections::HashMap;

/// Behavior Annex 代码生成器
#[derive(Default)]
pub struct AnnexConverter {
    /// 存储状态信息，用于判断状态是否需要continue
    state_info: HashMap<String, bool>, // state_name -> needs_continue
    /// 存储需要默认分支的状态（有条件判断的状态）
    states_with_conditions: std::collections::HashSet<String>,
}


impl AnnexConverter {
    /// 为线程实现生成Behavior Annex代码
    // pub fn generate_annex_code(&mut self, impl_: &ComponentImplementation) -> Option<Vec<Statement>> {
    //     // 查找Behavior Annex
    //     if let Some(behavior_annex) = self.find_behavior_annex(impl_) {
    //         // 生成状态机代码
    //         self.generate_state_machine_code(impl_, behavior_annex)
    //     } else {
    //         None
    //     }
    // }

    /// 查找Behavior Annex
    pub fn find_behavior_annex<'a>(&self, impl_: &'a ComponentImplementation) -> Option<&'a BehaviorAnnexContent> {
        for annex in &impl_.annexes {
            if let AnnexContent::BehaviorAnnex(content) = &annex.content {
                return Some(content);
            }
        }
        None
    }

    // 封装调用generate_state_variables、generate_state_enum、generate_initial_state
    pub fn generate_ba_variables_states(&mut self, _: &ComponentImplementation, behavior_annex: &BehaviorAnnexContent) -> Vec<Statement> {
        let mut stmts = Vec::new();
        
        // 1. 定义局部变量
        if let Some(state_vars) = &behavior_annex.state_variables {
            stmts.extend(self.generate_state_variables(state_vars));
        }

        // 2. 生成状态枚举并存储状态信息
        if let Some(states) = &behavior_annex.states {
            self.store_state_info(states);
            stmts.extend(self.generate_state_enum(states));
        }

        // 3. 设置初始状态
        if let Some(states) = &behavior_annex.states {
            stmts.extend(self.generate_initial_state(states));
        }
        stmts
    }

    /// 生成状态机代码
    // fn generate_state_machine_code(&mut self, _: &ComponentImplementation, behavior_annex: &BehaviorAnnexContent) -> Option<Vec<Statement>> {
    //     let mut stmts = Vec::new();

    //     // 1. 定义局部变量
    //     if let Some(state_vars) = &behavior_annex.state_variables {
    //         stmts.extend(self.generate_state_variables(state_vars));
    //     }

    //     // 2. 生成状态枚举并存储状态信息
    //     if let Some(states) = &behavior_annex.states {
    //         self.store_state_info(states);
    //         stmts.extend(self.generate_state_enum(states));
    //     }

    //     // 3. 设置初始状态
    //     if let Some(states) = &behavior_annex.states {
    //         stmts.extend(self.generate_initial_state(states));
    //     }

    //     // 4. 生成状态机循环
    //     if let Some(transitions) = &behavior_annex.transitions {
    //         stmts.extend(self.generate_state_machine_loop(transitions));
    //     }

    //     Some(stmts)
    // }

    /// 生成状态变量声明
    fn generate_state_variables(&self, state_vars: &[StateVariable]) -> Vec<Statement> {
        let mut stmts = Vec::new();

        for var in state_vars {
            let var_name = var.identifier.clone();
            let rust_type = self.convert_aadl_type_to_rust(&var.data_type);

            // 生成变量声明
            let init_value = if let Some(init) = &var.initial_value {
                // 如果有初始值，使用它
                self.parse_initial_value(init, &rust_type)
            } else {
                // 否则使用默认值
                self.generate_default_value_for_type(&rust_type)
            };

            stmts.push(Statement::Let(LetStmt {
                ifmut: true,
                name: var_name,
                ty: Some(rust_type),
                init: Some(init_value),
            }));
        }

        stmts
    }

    /// 生成状态枚举
    fn generate_state_enum(&self, states: &[State]) -> Vec<Statement> {
        let mut variants = Vec::new();
        
        for state in states {
            for state_id in &state.identifiers {
                variants.push(Variant {
                    name: state_id.clone(),
                    data: None, // 状态枚举通常没有数据
                    docs: vec![format!("// State: {}", state_id)],
                });
            }
        }

        // 生成枚举定义
        let enum_def = EnumDef {
            name: "State".to_string(), // 使用通用名称
            variants,
            generics: Vec::new(),
            derives: vec![], // vec!["Debug".to_string(), "Clone".to_string()],
            docs: vec!["// Behavior Annex state machine states".to_string()],
            vis: Visibility::Private, // 在函数内部定义
        };

        vec![Statement::Item(Box::new(Item::Enum(enum_def)))]
    }

    /// 存储状态信息到内部数据结构
    fn store_state_info(&mut self, states: &[State]) {
        for state in states {
            for state_id in &state.identifiers {
                // 如果状态有 Complete 或 Final 修饰符，则不需要 continue
                let needs_continue = !state.modifiers.contains(&StateModifier::Complete) 
                    && !state.modifiers.contains(&StateModifier::Final);
                self.state_info.insert(state_id.clone(), needs_continue);
            }
        }
    }

    /// 生成初始状态设置
    fn generate_initial_state(&self, states: &[State]) -> Vec<Statement> {
        let mut stmts = Vec::new();

        // 查找初始状态
        let initial_state = states.iter()
            .find(|state| state.modifiers.contains(&StateModifier::Initial))
            .and_then(|state| state.identifiers.first());

        if let Some(init_state) = initial_state {
            stmts.push(Statement::Let(LetStmt {
                ifmut: true,
                name: "state".to_string(),
                ty: Some(Type::Named("State".to_string())),
                init: Some(Expr::Path(
                    vec!["State".to_string(), init_state.clone()],
                    PathType::Namespace,
                )),
            }));
        } else {
            eprintln!("generate_initial_state: no initial state found");
        }

        stmts
    }

    /// 生成状态机循环
    pub fn generate_state_machine_loop(&mut self,transitions: &[Transition]) -> Vec<Statement> {
        // let mut stmts = Vec::new();

        // 生成端口数据接收代码
        let port_receive_stmts = self.generate_port_receive_code(transitions);

        // 生成状态转换逻辑
        let state_transition_stmts = self.generate_state_transition_logic(transitions);

        // 构建完整的循环
        let mut loop_body = vec![];

        // 直接将端口接收语句添加到循环体中，不包装在Block中
        loop_body.extend(port_receive_stmts);
        
        // 只有当状态转换语句不为空时才添加Block
        if !state_transition_stmts.is_empty() {
            // loop_body.push(Statement::Expr(Expr::Block(Block {
            //     stmts: state_transition_stmts,
            //     expr: None,
            // })));
            loop_body.extend(state_transition_stmts);
        }

        // 添加计算执行时间并睡眠的语句
        // loop_body.push(Statement::Let(LetStmt {
        //     ifmut: false,
        //     name: "elapsed".to_string(),
        //     ty: None,
        //     init: Some(Expr::MethodCall(
        //         Box::new(Expr::Ident("start".to_string())),
        //         "elapsed".to_string(),
        //         Vec::new(),
        //     )),
        // }));
        
        // loop_body.push(Statement::Expr(Expr::MethodCall(
        //     Box::new(Expr::Path(
        //         vec!["std".to_string(), "thread".to_string(), "sleep".to_string()],
        //         PathType::Namespace,
        //     )),
        //     "".to_string(),
        //     vec![Expr::MethodCall(
        //         Box::new(Expr::Ident("period".to_string())),
        //         "saturating_sub".to_string(),
        //         vec![Expr::Ident("elapsed".to_string())],
        //     )],
        // )));

        // 不用放在循环里
        // stmts.push(Statement::Expr(Expr::Loop(Box::new(Block {
        //     stmts: loop_body,
        //     expr: None,
        // }))));

        loop_body
    }

    /// 生成端口接收代码
    /// 为转换中的端口生成接收代码
    pub fn generate_port_receive_code(&self, transitions: &[Transition]) -> Vec<Statement> {
        let mut stmts = Vec::new();
        
        // 从转换中提取端口
        let mut ports_to_receive = std::collections::HashSet::new();
        for transition in transitions {
            if let Some(condition) = &transition.behavior_condition {
                self.extract_ports_from_condition(condition, &mut ports_to_receive);
            }
        }
        
        // 为每个需要接收的端口生成接收代码
        for port_name in ports_to_receive {
            let receive_stmt = Statement::Let(LetStmt {
                ifmut: false,
                name: port_name.to_string(),
                ty: None,
                init: Some(self.build_port_receive_expr(&port_name)),
            });

            stmts.push(receive_stmt);
        }

        stmts
    }

    /// 从行为条件中提取端口名称
    pub fn extract_ports_from_condition(&self, condition: &BehaviorCondition, ports: &mut std::collections::HashSet<String>) {
        match condition {
            BehaviorCondition::Dispatch(dispatch_cond) => {
                if let Some(trigger_condition) = &dispatch_cond.trigger_condition {
                    match trigger_condition {
                        DispatchTriggerCondition::LogicalExpression(logical_expr) => {
                            for conjunction in &logical_expr.dispatch_conjunctions {
                                for trigger in &conjunction.dispatch_triggers {
                                    self.extract_port_from_trigger(trigger, ports);
                                }
                            }
                        }
                        _ => { // 其他类型不涉及端口接收 
                            eprintln!("extract_ports_from_condition: other type not involve port receive");
                        }
                        // DispatchTriggerCondition::SubprogramAccess(_) => {
                        //     // 子程序访问不涉及端口接收
                        // }
                        // DispatchTriggerCondition::Stop => {
                        //     // stop 不涉及端口接收
                        // }
                        // DispatchTriggerCondition::CompletionTimeout => {
                        //     // 完成超时不涉及端口接收
                        // }
                        // DispatchTriggerCondition::DispatchTimeout => {
                        //     // 分发超时不涉及端口接收
                        // }
                    }
                }
            }
            BehaviorCondition::Execute(execute_cond) => {
                // for trigger in &execute_cond.dispatch_triggers {
                //     self.extract_port_from_trigger(trigger, ports);
                // }
                
                // 提取端口/变量
                let ast_string = format!("{:?}", execute_cond);

                // 按所有非字母数字的符号切开
                for word in ast_string.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
                    if !word.is_empty() && word.chars().next().unwrap().is_ascii_lowercase() {
                        // 过滤掉 Rust 生成的无关词汇和 AADL 关键字
                        let ignore_list = [
                            "some", "none", "true", "false", "otherwise", "timeout", 
                            "execute", "logicalexpression", "behaviorexpression", "string", "vec",
                            "left", "right", "operator", "operations", "sign", "comparison",
                            "value", "constant", "variable", "identifier",
                            "and", "or", "xor", "not", "mod", "rem", "abs"
                        ];
                        
                        if !ignore_list.contains(&word.to_lowercase().as_str()) {
                            ports.insert(word.to_string());
                        }
                    }
                }
            }
        }
    }

    /// 从分发触发器中提取端口名称
    fn extract_port_from_trigger(&self, trigger: &DispatchTrigger, ports: &mut std::collections::HashSet<String>) {
        match trigger {
            DispatchTrigger::InEventPort(port_name) => {
                ports.insert(port_name.clone());
            }
            DispatchTrigger::InEventDataPort(port_name) => {
                ports.insert(port_name.clone());
            }
        }
    }

    /// 构造端口接收表达式：self.port.as_mut().and_then(|rx| rx.try_recv().ok()).unwrap_or_default()
    fn build_port_receive_expr(&self, port_name: &str) -> Expr {
        let base_expr = Expr::Path(
            vec!["self".to_string(), port_name.to_string()],
            PathType::Member,
        );
        let as_mut_expr = Expr::MethodCall(
            Box::new(base_expr),
            "as_mut".to_string(),
            Vec::new(),
        );
        let try_recv_expr = Expr::MethodCall(
            Box::new(Expr::Ident("rx".to_string())),
            "try_recv".to_string(),
            Vec::new(),
        );
        let ok_expr = Expr::MethodCall(
            Box::new(try_recv_expr),
            "ok".to_string(),
            Vec::new(),
        );
        let closure_expr = Expr::Closure(vec!["rx".to_string()], Box::new(ok_expr));
        let and_then_expr = Expr::MethodCall(
            Box::new(as_mut_expr),
            "and_then".to_string(),
            vec![closure_expr],
        );

        Expr::MethodCall(
            Box::new(and_then_expr),
            "unwrap_or_else".to_string(),
            vec![Expr::Closure(vec!["".to_string()], Box::new(Expr::Ident("Default::default()".to_string())))],
        )
    }

    /// 生成状态转换逻辑
    fn generate_state_transition_logic(&mut self, transitions: &[Transition]) -> Vec<Statement> {
        let mut stmts = Vec::new();

        // 添加注释
        stmts.push(Statement::Comment("--- BA 宏步执行 ---".to_string()));

        // 清空之前的状态条件记录
        self.states_with_conditions.clear();

        // 生成状态转换循环
        let mut match_arms = Vec::new();

        // 为每个状态生成匹配分支
        for transition in transitions {
            for source_state in &transition.source_states {
                let arm = self.generate_state_match_arm(transition, source_state);
                match_arms.push(arm);
            }
        }

        // 为有条件判断的状态state添加默认分支
        // eg.State::s1 => break,
        // 被 _ => break, 替代，这样就不需要针对某个状态单独构造了
        // for state_name in &self.states_with_conditions {
        //     let default_arm = MatchArm {
        //         pattern: format!("State::{}", state_name),
        //         guard: None,
        //         body: Block {
        //             stmts: vec![
        //                 Statement::Comment(format!("理论上不会执行到这里，但编译器需要这个分支")),
        //                 Statement::Expr(Expr::Ident("break".to_string())),
        //             ],
        //             expr: None,
        //         },
        //     };
        //     match_arms.push(default_arm);
        // }

        // 添加分支 _ => break,
        if !&self.states_with_conditions.is_empty() {
            let default_arm = MatchArm {
                pattern: "_".to_string(),
                guard: None,
                body: Block {
                    stmts: vec![Statement::Expr(Expr::Ident("break".to_string()))],
                    expr: None,
                },
            };
            match_arms.push(default_arm);
        }
        

        // 构建match表达式
        let match_expr = Expr::Match {
            expr: Box::new(Expr::Ident("state".to_string())),
            arms: match_arms,
        };

        // 包装在循环中
        stmts.push(Statement::Expr(Expr::Loop(Box::new(Block {
            stmts: vec![
                Statement::Expr(match_expr),
                Statement::Break,
            ],
            expr: None,
        }))));

        stmts
    }

    /// 生成状态匹配分支
    fn generate_state_match_arm(&mut self, transition: &Transition, source_state: &str) -> MatchArm {
        let mut stmts = Vec::new();
        let mut guard = None;

        // 处理动作
        if let Some(actions) = &transition.actions {
            stmts.extend(self.generate_action_code(actions));
        }

        // 处理转换条件
        if let Some(condition) = &transition.behavior_condition {
            match condition {
                BehaviorCondition::Dispatch(dispatch_cond) => {
                    if dispatch_cond.trigger_condition.is_some() || dispatch_cond.frozen_ports.is_some() {
                        // 生成一个带有注释的 guard，让 Rust 编译器认为这是一个独立的条件分支
                        guard = Some(Expr::Ident("true /* 占位: 遇到了 stop 或 frozen 等特殊调度 */".to_string()));
                        self.states_with_conditions.insert(source_state.to_string());
                    }
                    // 处理 "on dispatch" 条件
                    stmts.push(Statement::Comment(format!("on dispatch → {}", transition.destination_state)));
                    stmts.push(Statement::Expr(Expr::Assign(
                        Box::new(Expr::Ident("state".to_string())),
                        Box::new(Expr::Path(
                            vec!["State".to_string(), transition.destination_state.clone()],
                            PathType::Namespace,
                        )),
                    )));
                    
                    // 检查目标状态是否需要continue
                    if self.should_continue_state(&transition.destination_state) {
                        stmts.push(Statement::Continue);
                    } else {
                        stmts.push(Statement::Comment("complete,需要停".to_string()));
                    }
                }
                BehaviorCondition::Execute(execute_cond) => {
                    // 处理执行条件，将端口条件作为guard
                    // if !execute_cond.dispatch_triggers.is_empty() {
                    //     guard = Some(self.generate_guard_condition(execute_cond));
                    //     // 记录这个状态有条件判断，需要添加默认分支
                    //     self.states_with_conditions.insert(source_state.to_string());
                    // }
                    if let BehaviorCondition::Execute(execute_cond) = &condition {
                        // 检查是否不是 Otherwise
                        if !matches!(execute_cond, ExecuteCondition::Otherwise) {
                            guard = Some(self.generate_guard_condition(execute_cond));
                        }
                    }
                    
                    // 状态转换
                    stmts.push(Statement::Expr(Expr::Assign(
                        Box::new(Expr::Ident("state".to_string())),
                        Box::new(Expr::Path(
                            vec!["State".to_string(), transition.destination_state.clone()],
                            PathType::Namespace,
                        )),
                    )));
                    
                    // 检查目标状态是否需要continue
                    if self.should_continue_state(&transition.destination_state) {
                        stmts.push(Statement::Continue);
                    } else {
                        stmts.push(Statement::Comment("complete,需要停".to_string()));
                    }
                }
            }
        } else {
            // 无条件转换
            stmts.push(Statement::Expr(Expr::Assign(
                Box::new(Expr::Ident("state".to_string())),
                Box::new(Expr::Path(
                    vec!["State".to_string(), transition.destination_state.clone()],
                    PathType::Namespace,
                )),
            )));
            
            if self.should_continue_state(&transition.destination_state) {
                stmts.push(Statement::Continue);
            } else {
                stmts.push(Statement::Comment("complete，停".to_string()));
            }
        }

        MatchArm {
            pattern: format!("State::{}", source_state),
            guard,
            body: Block {
                stmts,
                expr: None,
            },
        }
    }

    /// 生成守卫条件表达式
    // fn generate_guard_condition(&self, execute_cond: &DispatchConjunction) -> Expr {
    //     if execute_cond.dispatch_triggers.is_empty() {
    //         // 无条件，根据not字段返回true或false
    //         Expr::Literal(Literal::Bool(!execute_cond.not))
    //     } else {
    //         // 有端口条件，生成检查表达式
    //         let mut conditions = Vec::new();
    //         let not_flag = execute_cond.not;
    //         let parsed_number = execute_cond.number.as_ref().and_then(|num_str| {
    //             if let Ok(int_val) = num_str.parse::<i64>() {
    //                 Some(Literal::Int(int_val))
    //             } else if let Ok(float_val) = num_str.parse::<f64>() {
    //                 Some(Literal::Float(float_val))
    //             } else {
    //                 None
    //             }
    //         });
    //         let use_less_than = execute_cond.less_than && parsed_number.is_some();
            
    //         for trigger in &execute_cond.dispatch_triggers {
    //             match trigger {
    //                 DispatchTrigger::InEventPort(port_name) => {
    //                     let port_var = port_name.to_lowercase().to_string();
    //                     let mut condition = if use_less_than {
    //                         let number_literal = parsed_number.as_ref().unwrap().clone();
    //                         Expr::BinaryOp(
    //                             Box::new(Expr::Literal(number_literal)),
    //                             "<".to_string(),
    //                             Box::new(Expr::Ident(port_var.clone())),
    //                         )
    //                     } else {
    //                         let expected_value = !not_flag;
    //                         Expr::BinaryOp(
    //                             Box::new(Expr::Ident(port_var.clone())),
    //                             "==".to_string(),
    //                             Box::new(Expr::Literal(Literal::Bool(expected_value))),
    //                         )
    //                     };
                        
    //                     if use_less_than && not_flag {
    //                         condition = Expr::UnaryOp("!".to_string(), Box::new(condition));
    //                     }
                        
    //                     conditions.push(condition);
    //                 }
    //                 DispatchTrigger::InEventDataPort(port_name) => {
    //                     let port_var = format!("{}_val", port_name.to_lowercase());
    //                     let mut condition = Expr::MethodCall(
    //                         Box::new(Expr::Ident(port_var)),
    //                         "is_some".to_string(),
    //                         Vec::new(),
    //                     );
    //                     if not_flag {
    //                         condition = Expr::UnaryOp("!".to_string(), Box::new(condition));
    //                     }
    //                     conditions.push(condition);
    //                 }
    //             }
    //         }
    //         // 生成条件
    //         if conditions.len() == 1 {
    //             conditions[0].clone()
    //         } else {
    //             let mut result = conditions.remove(0);
    //             for condition in conditions {
    //                 result = Expr::BinaryOp(
    //                     Box::new(result),
    //                     "&&".to_string(),
    //                     Box::new(condition),
    //                 );
    //             }
    //             result
    //         }
    //     }
    // }
    fn generate_guard_condition(&self, execute_cond: &ExecuteCondition) -> Expr {
        match execute_cond {
            // 情况 1: 逻辑表达式 (即 ValueExpression)
            ExecuteCondition::LogicalExpression(val_expr) => {
                self.convert_value_expression(val_expr)
            }
            
            // 情况 2: Otherwise (通常对应 true)
            ExecuteCondition::Otherwise => {
                Expr::Literal(Literal::Bool(true))
            }
            
            // 情况 3: Timeout (暂时不支持或返回 false)
            ExecuteCondition::Timeout => {
                Expr::Literal(Literal::Bool(false)) 
            }
        }
    }

    /// 生成执行条件代码

    /// 生成动作代码
    fn generate_action_code(&self, actions: &BehaviorActionBlock) -> Vec<Statement> {
        let mut stmts = Vec::new();

        match &actions.actions {
            BehaviorActions::Sequence(seq) => {
                for action in &seq.actions {
                    stmts.extend(self.generate_single_action(action));
                }
            }
            BehaviorActions::Set(set) => {
                for action in &set.actions {
                    stmts.extend(self.generate_single_action(action));
                }
            }
            BehaviorActions::Single(action) => {
                stmts.extend(self.generate_single_action(action));
            }
        }

        stmts
    }

    /// 生成单个动作代码
    fn generate_single_action(&self, action: &BehaviorAction) -> Vec<Statement> {
        let mut stmts = Vec::new();

        match action {
            BehaviorAction::Basic(basic_action) => {
                stmts.extend(self.generate_basic_action(basic_action));
            }
            BehaviorAction::Block(block) => {
                stmts.extend(self.generate_action_code(block));
            }
            BehaviorAction::If(if_stmt) => {
                stmts.extend(self.generate_if_statement(if_stmt));
            }
            _ => {
                // 其他类型的动作暂时跳过
                stmts.push(Statement::Comment("TODO: Unsupported action type".to_string()));
            }
        }

        stmts
    }

    /// 生成基本动作代码
    fn generate_basic_action(&self, action: &BasicAction) -> Vec<Statement> {
        let mut stmts = Vec::new();

        match action {
            BasicAction::Assignment(assignment) => {
                stmts.extend(self.generate_assignment_action(assignment));
            }
            BasicAction::Communication(comm) => {
                stmts.extend(self.generate_communication_action(comm));
            }
            BasicAction::Timed(timed) => {
                stmts.extend(self.generate_timed_action(timed));
            }
        }

        stmts
    }

    /// 生成赋值动作,暂时只支持前两种
    fn generate_assignment_action(&self, assignment: &AssignmentAction) -> Vec<Statement> {
        let mut stmts: Vec<Statement> = Vec::new();

        match &assignment.target {
            Target::LocalVariable(name) => {
                // 本地变量赋值
                let target_expr = Expr::Ident(name.clone());
                let value_expr = match &assignment.value {
                    AssignmentValue::Expression(expr) => self.convert_value_expression(expr),
                    AssignmentValue::Any => Expr::Literal(Literal::Int(0)), // 默认值
                };

                stmts.push(Statement::Expr(Expr::Assign(
                    Box::new(target_expr),
                    Box::new(value_expr),
                )));
            }
            //TODO：待修改，输出端口赋值这个动作，不该有发送操作，只是赋值，发送是!(value)
            Target::OutgoingPort(port_name) => {
                // 输出端口赋值 - 生成发送代码
                let value_expr = match &assignment.value {
                    AssignmentValue::Expression(expr) => self.convert_value_expression(expr),
                    AssignmentValue::Any => Expr::Literal(Literal::Bool(true)), // 默认发送true
                };

                // 生成发送代码
                stmts.push(Statement::Expr(Expr::IfLet {
                    pattern: "Some(sender)".to_string(),
                    value: Box::new(Expr::Reference(
                        Box::new(Expr::Path(
                            vec!["self".to_string(), port_name.clone()],
                            PathType::Member,
                        )),
                        true,
                        false,
                    )),
                    then_branch: Block {
                        stmts: vec![
                            Statement::Let(LetStmt {
                                ifmut: false,
                                name: "_".to_string(),
                                ty: None,
                                init: Some(Expr::MethodCall(
                                    Box::new(Expr::Ident("sender".to_string())),
                                    "send".to_string(),
                                    vec![value_expr],
                                )),
                            }),
                        ],
                        expr: None,
                    },
                    else_branch: None,
                }));
            }
            Target::OutgoingSubprogramParameter(param_name) => {
                // 输出子程序参数赋值
                let target_expr = Expr::Ident(param_name.clone());
                let value_expr = match &assignment.value {
                    AssignmentValue::Expression(expr) => self.convert_value_expression(expr),
                    AssignmentValue::Any => Expr::Literal(Literal::Int(0)), // 默认值
                };

                stmts.push(Statement::Expr(Expr::Assign(
                    Box::new(target_expr),
                    Box::new(value_expr),
                )));
            }
            Target::DataComponentReference(ref_data) => {
                // 数据组件引用赋值
                let target_expr = Expr::Path(
                    vec!["self".to_string(), ref_data.components.join("_")],
                    PathType::Member,
                );
                let value_expr = match &assignment.value {
                    AssignmentValue::Expression(expr) => self.convert_value_expression(expr),
                    AssignmentValue::Any => Expr::Literal(Literal::Int(0)), // 默认值
                };

                stmts.push(Statement::Expr(Expr::Assign(
                    Box::new(target_expr),
                    Box::new(value_expr),
                )));
            }
        }

        stmts
    }

    /// 生成通信动作
    fn generate_communication_action(&self, comm: &CommunicationAction) -> Vec<Statement> {
        let mut stmts = Vec::new();

        match comm {
            CommunicationAction::PortCommunication(port_comm) => {
                match port_comm {
                    PortCommunication::Output { port, value } => {
                        let port_name = port.to_lowercase();
                        let value_expr = if let Some(val) = value {
                            self.convert_value_expression(val)
                        } else {
                            Expr::Literal(Literal::Bool(true)) // 默认发送true
                        };

                        stmts.push(Statement::Expr(Expr::IfLet {
                            pattern: "Some(sender)".to_string(),
                            value: Box::new(Expr::Reference(
                                Box::new(Expr::Path(
                                    vec!["self".to_string(), port_name],
                                    PathType::Member,
                                )),
                                true,
                                false,
                            )),
                            then_branch: Block {
                                stmts: vec![
                                    Statement::Let(LetStmt {
                                        ifmut: false,
                                        name: "_".to_string(),
                                        ty: None,
                                        init: Some(Expr::MethodCall(
                                            Box::new(Expr::Ident("sender".to_string())),
                                            "send".to_string(),
                                            vec![value_expr],
                                        )),
                                    }),
                                ],
                                expr: None,
                            },
                            else_branch: None,
                        }));
                    }
                    _ => {
                        stmts.push(Statement::Comment("TODO: Unsupported port communication".to_string()));
                    }
                }
            }
            _ => {
                stmts.push(Statement::Comment("TODO: Unsupported communication action".to_string()));
            }
        }

        stmts
    }

    /// 生成定时动作
    fn generate_timed_action(&self, _timed: &TimedAction) -> Vec<Statement> {
        // 定时动作暂时跳过
        vec![Statement::Comment("TODO: Timed action not implemented".to_string())]
    }

    /// 生成if语句
    fn generate_if_statement(&self, _if_stmt: &IfStatement) -> Vec<Statement> {
        // if语句暂时跳过
        vec![Statement::Comment("TODO: If statement not implemented".to_string())]
    }

    /// 判断状态是否需要continue
    fn should_continue_state(&self, state_name: &str) -> bool {
        // 从存储的状态信息中查找
        self.state_info.get(state_name).copied().unwrap_or(true)
    }

    /// 转换AADL类型到Rust类型
    fn convert_aadl_type_to_rust(&self, aadl_type: &str) -> Type { //TODO：待修改，这里只处理了部分类型，需要处理所有类型
        match aadl_type.to_lowercase().as_str() {
            "integer" | "i32" | "base_types::integer" => Type::Named("i32".to_string()),
            "boolean" | "bool" => Type::Named("bool".to_string()),
            "string" => Type::Named("String".to_string()),
            "real" | "f64" => Type::Named("f64".to_string()),
            "base_types::unsigned_16" => Type::Named("u16".to_string()),
            "base_types::boolean" => Type::Named("bool".to_string()),
            _ => Type::Named(aadl_type.to_string()),
        }
    }

    /// 解析初始值
    fn parse_initial_value(&self, init_value: &str, rust_type: &Type) -> Expr {
        match rust_type {
            Type::Named(type_name) => {
                match type_name.as_str() {
                    "i32" => {
                        if let Ok(val) = init_value.parse::<i64>() {
                            Expr::Literal(Literal::Int(val))
                        } else {
                            Expr::Literal(Literal::Int(0))
                        }
                    }
                    "bool" => {
                        match init_value.to_lowercase().as_str() {
                            "true" => Expr::Literal(Literal::Bool(true)),
                            "false" => Expr::Literal(Literal::Bool(false)),
                            _ => Expr::Literal(Literal::Bool(false)),
                        }
                    }
                    "String" => Expr::Literal(Literal::Str(init_value.to_string())),
                    "f64" => {
                        if let Ok(val) = init_value.parse::<f64>() {
                            Expr::Literal(Literal::Float(val))
                        } else {
                            Expr::Literal(Literal::Float(0.0))
                        }
                    }
                    _ => Expr::Literal(Literal::Int(0)),
                }
            }
            _ => Expr::Literal(Literal::Int(0)),
        }
    }

    /// 为类型生成默认值
    fn generate_default_value_for_type(&self, rust_type: &Type) -> Expr {
        match rust_type {
            Type::Named(type_name) => {
                match type_name.as_str() {
                    "i32" => Expr::Literal(Literal::Int(0)),
                    "bool" => Expr::Literal(Literal::Bool(false)),
                    "String" => Expr::Literal(Literal::Str("".to_string())),
                    "f64" => Expr::Literal(Literal::Float(0.0)),
                    _ => Expr::Literal(Literal::Int(0)),
                }
            }
            _ => Expr::Literal(Literal::Int(0)),
        }
    }

    /// 转换值表达式 And Or Xor
    fn convert_value_expression(&self, expr: &ValueExpression) -> Expr {
        // 转换左侧关系表达式
        let left = self.convert_relation(&expr.left);
        
        if expr.operations.is_empty() {
            left
        } else {
            // 处理逻辑操作
            let mut result = left;
            for op in &expr.operations {
                let right = self.convert_relation(&op.right);
                result = Expr::BinaryOp(
                    Box::new(result),
                    match op.operator {
                        LogicalOperator::And => "&&".to_string(),
                        LogicalOperator::Or => "||".to_string(),
                        LogicalOperator::Xor => "^".to_string(),
                    },
                    Box::new(right),
                );
            }
            result
        }
    }

    /// 转换关系表达式 = != < <= > >=
    fn convert_relation(&self, relation: &Relation) -> Expr {
        let left = self.convert_simple_expression(&relation.left);
        
        if let Some(comparison) = &relation.comparison {
            // 有比较操作
            let right = self.convert_simple_expression(&comparison.right);
            Expr::BinaryOp(
                Box::new(left),
                match comparison.operator {
                    RelationalOperator::Equal => "==".to_string(),
                    RelationalOperator::NotEqual => "!=".to_string(),
                    RelationalOperator::LessThan => "<".to_string(),
                    RelationalOperator::LessThanOrEqual => "<=".to_string(),
                    RelationalOperator::GreaterThan => ">".to_string(),
                    RelationalOperator::GreaterThanOrEqual => ">=".to_string(),
                },
                Box::new(right),
            )
        } else {
            // 没有比较操作，直接返回左侧表达式
            left
        }
    }

    /// 转换简单表达式
    fn convert_simple_expression(&self, expr: &SimpleExpression) -> Expr {
        let mut result = self.convert_term(&expr.left);
        
        // 处理一元符号
        if let Some(sign) = &expr.sign {
            result = match sign {
                UnaryAddingOperator::Plus => result, // +x = x
                UnaryAddingOperator::Minus => Expr::UnaryOp(
                    "-".to_string(),
                    Box::new(result),
                ),
            };
        }
        
        // 处理二元加法操作
        for op in &expr.operations {
            let right = self.convert_add_expression(&op.right);
            result = Expr::BinaryOp(
                Box::new(result),
                match op.operator {
                    AdditiveOperator::Add => "+".to_string(),
                    AdditiveOperator::Subtract => "-".to_string(),
                },
                Box::new(right),
            );
        }
        
        result
    }

    /// 转换加法表达式
    fn convert_add_expression(&self, expr: &AddExpression) -> Expr {
        let mut result = self.convert_basic_expression(&expr.left);
        
        // 处理乘法操作
        for op in &expr.operations {
            let right = self.convert_basic_expression(&op.right);
            result = Expr::BinaryOp(
                Box::new(result),
                match op.operator {
                    MultiplicativeOperator::Multiply => "*".to_string(),
                    MultiplicativeOperator::Divide => "/".to_string(),
                    MultiplicativeOperator::Modulo => "%".to_string(),
                    MultiplicativeOperator::Remainder => "%".to_string(), // rem 和 mod 在 Rust 中都是 %
                },
                Box::new(right),
            );
        }
        
        result
    }

    /// 转换项
    fn convert_term(&self, term: &Term) -> Expr {
        let mut result = self.convert_factor(&term.left);
        
        // 处理乘法操作
        for op in &term.operations {
            let right = self.convert_basic_expression(&op.right);
            result = Expr::BinaryOp(
                Box::new(result),
                match op.operator {
                    MultiplicativeOperator::Multiply => "*".to_string(),
                    MultiplicativeOperator::Divide => "/".to_string(),
                    MultiplicativeOperator::Modulo => "%".to_string(),
                    MultiplicativeOperator::Remainder => "%".to_string(),
                },
                Box::new(right),
            );
        }
        
        result
    }

    /// 转换因子
    fn convert_factor(&self, factor: &Factor) -> Expr {
        match factor {
            Factor::Value(value) => self.convert_value(value),
            Factor::BinaryNumeric { left, operator, right } => {
                let left_expr = self.convert_value(left);
                let right_expr = self.convert_value(right);
                match operator {
                    BinaryNumericOperator::Power => {
                        // 使用 pow 方法
                        Expr::MethodCall(
                            Box::new(left_expr),
                            "pow".to_string(),
                            vec![right_expr],
                        )
                    }
                }
            }
            Factor::UnaryNumeric { operator, value } => {
                let value_expr = self.convert_value(value);
                match operator {
                    UnaryNumericOperator::Abs => {
                        Expr::MethodCall(
                            Box::new(value_expr),
                            "abs".to_string(),
                            Vec::new(),
                        )
                    }
                }
            }
            Factor::UnaryBoolean { operator, value } => {
                let value_expr = self.convert_value(value);
                match operator {
                    UnaryBooleanOperator::Not => {
                        Expr::UnaryOp("!".to_string(), Box::new(value_expr))
                    }
                }
            }
        }
    }

    /// 转换值
    fn convert_value(&self, value: &Value) -> Expr {
        match value {
            Value::Variable(var) => self.convert_value_variable(var),
            Value::Constant(constant) => self.convert_value_constant(constant),
            Value::Expression(expr) => self.convert_value_expression(expr),
        }
    }

    /// 转换值变量
    fn convert_value_variable(&self, var: &ValueVariable) -> Expr {
        match var {
            ValueVariable::IncomingPort(port_name) => {
                // 端口变量，转换为port_name_val
                Expr::Path(
                    vec![port_name.clone(),"_val".to_string()],
                    PathType::Member,
                )
            }
            ValueVariable::IncomingPortCheck(port_name) => {
                // 端口检查，转换为 self.port_name.is_some()
                Expr::MethodCall(
                    Box::new(Expr::Path(
                        vec!["self".to_string(), port_name.clone()],
                        PathType::Member,
                    )),
                    "is_some".to_string(),
                    Vec::new(),
                )
            }
            ValueVariable::LocalVariable(var_name) => {
                // 局部变量
                Expr::Ident(var_name.clone())
            }
            ValueVariable::DataComponentReference(ref_data) => {
                // 数据组件引用，转换为 self.data_component
                Expr::Path(
                    vec!["self".to_string(), ref_data.components.join("_")],
                    PathType::Member,
                )
            }
            ValueVariable::PortCount(port_name) => {
                // 端口计数，转换为 self.port_name.len()
                Expr::MethodCall(
                    Box::new(Expr::Path(
                        vec!["self".to_string(), port_name.clone()],
                        PathType::Member,
                    )),
                    "len".to_string(),
                    Vec::new(),
                )
            }
            ValueVariable::PortFresh(_) => {
                // 端口新鲜度，暂时返回 true
                Expr::Literal(Literal::Bool(true))
            }
            ValueVariable::IncomingSubprogramParameter(param_name) => {
                // 子程序参数，转换为参数名
                Expr::Ident(param_name.clone())
            }
        }
    }

    /// 转换值常量
    fn convert_value_constant(&self, constant: &ValueConstant) -> Expr {
        match constant {
            ValueConstant::Boolean(b) => Expr::Literal(Literal::Bool(*b)),
            ValueConstant::Numeric(num_str) => {
                // 尝试解析为整数或浮点数
                if let Ok(int_val) = num_str.parse::<i64>() {
                    Expr::Literal(Literal::Int(int_val))
                } else if let Ok(float_val) = num_str.parse::<f64>() {
                    Expr::Literal(Literal::Float(float_val))
                } else {
                    // 无法解析，返回 0
                    println!("!!!!!!!!!!!!!!!!!!!!!!!该数值无法解析，返回0");
                    Expr::Literal(Literal::Int(0))
                }
            }
            ValueConstant::String(s) => Expr::Literal(Literal::Str(s.clone())),
            ValueConstant::PropertyConstant(_) => {
                // 属性常量，暂时返回默认值
                Expr::Literal(Literal::Int(0))
            }
            ValueConstant::PropertyValue(_) => {
                // 属性值，暂时返回默认值
                Expr::Literal(Literal::Int(0))
            }
        }
    }

    /// 转换基础表达式
    fn convert_basic_expression(&self, expr: &BasicExpression) -> Expr {
        match expr {
            BasicExpression::NumericOrConstant(num_str) => {
                if let Ok(int_val) = num_str.parse::<i64>() {
                    Expr::Literal(Literal::Int(int_val))
                } else if let Ok(float_val) = num_str.parse::<f64>() {
                    Expr::Literal(Literal::Float(float_val))
                } else {
                    println!("!!!!!!!!!!!!!!!!!!!!!!!该数值无法解析，返回0");
                    Expr::Literal(Literal::Int(0))
                }
            }
            BasicExpression::BehaviorVariable(var_name) => {
                Expr::Ident(var_name.clone())
            }
            BasicExpression::LoopVariable(var_name) => {
                Expr::Ident(var_name.clone())
            }
            BasicExpression::Port(port_name) => {
                Expr::Path(
                    vec!["self".to_string(), port_name.clone()],
                    PathType::Member,
                )
            }
            BasicExpression::PortWithQualifier { port, qualifier } => {
                let port_expr = Expr::Path(
                    vec!["self".to_string(), port.clone()],
                    PathType::Member,
                );
                match qualifier {
                    PortQualifier::Count => {
                        Expr::MethodCall(
                            Box::new(port_expr),
                            "len".to_string(),
                            Vec::new(),
                        )
                    }
                    PortQualifier::Fresh => {
                        Expr::Literal(Literal::Bool(true))
                    }
                }
            }
            BasicExpression::DataAccess(access_name) => {
                Expr::Path(
                    vec!["self".to_string(), access_name.clone()],
                    PathType::Member,
                )
            }
            BasicExpression::Timeout(_) => {
                // 超时表达式，暂时返回默认值
                Expr::Literal(Literal::Int(0))
            }
            BasicExpression::DataSubcomponent(sub_name) => {
                Expr::Path(
                    vec!["self".to_string(), sub_name.clone()],
                    PathType::Member,
                )
            }
            BasicExpression::DataSubcomponentWithIndex { subcomponent, index } => {
                let sub_expr = Expr::Path(
                    vec!["self".to_string(), subcomponent.clone()],
                    PathType::Member,
                );
                let index_expr = self.convert_basic_expression(index);
                Expr::Index(Box::new(sub_expr), Box::new(index_expr))
            }
            BasicExpression::DataAccessWithSubcomponent { access, subcomponent } => {
                Expr::Path(
                    vec!["self".to_string(), access.clone(), subcomponent.clone()],
                    PathType::Member,
                )
            }
            BasicExpression::DataSubcomponentWithSubcomponent { container, subcomponent } => {
                Expr::Path(
                    vec!["self".to_string(), container.clone(), subcomponent.clone()],
                    PathType::Member,
                )
            }
            BasicExpression::DataClassifierSubprogram { classifier:_, subprogram:_, parameters:_ } => {
                // 子程序调用，暂时返回默认值
                Expr::Literal(Literal::Int(0))
            }
            BasicExpression::DataClassifierSubprogramWithTimeout { classifier:_, subprogram:_, timeout:_ } => {
                // 带超时的子程序调用，暂时返回默认值
                Expr::Literal(Literal::Int(0))
            }
            BasicExpression::DataClassifierSubprogramWithParameter { classifier:_, subprogram:_, parameter:_, expression:_ } => {
                // 带参数的子程序调用，暂时返回默认值
                Expr::Literal(Literal::Int(0))
            }
            BasicExpression::Parenthesized(expr) => {
                Expr::Parenthesized(Box::new(self.convert_basic_expression(expr)))
            }
            BasicExpression::Quantified { quantifier:_, identifier:_, range:_, expression:_ } => {
                // 量词表达式，暂时返回默认值
                Expr::Literal(Literal::Bool(false))
            }
            BasicExpression::BinaryOp { left, operator, right } => {
                let left_expr = self.convert_basic_expression(left);
                let right_expr = self.convert_basic_expression(right);
                Expr::BinaryOp(
                    Box::new(left_expr),
                    match operator {
                        BinaryOperator::And => "&&".to_string(),
                        BinaryOperator::Or => "||".to_string(),
                        BinaryOperator::Equal => "==".to_string(),
                        BinaryOperator::NotEqual => "!=".to_string(),
                        BinaryOperator::LessThan => "<".to_string(),
                        BinaryOperator::LessThanOrEqual => "<=".to_string(),
                        BinaryOperator::GreaterThan => ">".to_string(),
                        BinaryOperator::GreaterThanOrEqual => ">=".to_string(),
                        BinaryOperator::Add => "+".to_string(),
                        BinaryOperator::Subtract => "-".to_string(),
                        BinaryOperator::Multiply => "*".to_string(),
                        BinaryOperator::Divide => "/".to_string(),
                        BinaryOperator::Modulo => "%".to_string(),
                    },
                    Box::new(right_expr),
                )
            }
            BasicExpression::Not(expr) => {
                Expr::UnaryOp("!".to_string(), Box::new(self.convert_basic_expression(expr)))
            }
        }
    }
} 