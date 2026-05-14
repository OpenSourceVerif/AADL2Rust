// src/aadlAst2rustCode/converter_annex.rs
// Behavior Annex code generator
#![allow(clippy::empty_line_after_doc_comments, clippy::only_used_in_recursion)]
use aadl_intermediate::*;
use crate::ast::aadl_ast_cj::*;
use std::collections::HashMap;

/// Behavior Annex code generator
#[derive(Default)]
pub struct AnnexConverter {
    /// Stores state info for deciding whether a state should `continue`
    state_info: HashMap<String, bool>, // state_name -> needs_continue
    /// Stores states that require a default branch (states with conditional guards)
    states_with_conditions: std::collections::HashSet<String>,
}

impl AnnexConverter {
    /// Generate Behavior Annex code for a thread implementation
    // pub fn generate_annex_code(&mut self, impl_: &ComponentImplementation) -> Option<Vec<Statement>> {
    //     // Find Behavior Annex
    //     if let Some(behavior_annex) = self.find_behavior_annex(impl_) {
    //         // Generate state machine code
    //         self.generate_state_machine_code(impl_, behavior_annex)
    //     } else {
    //         None
    //     }
    // }

    /// Find Behavior Annex
    pub fn find_behavior_annex<'a>(
        &self,
        impl_: &'a ComponentImplementation,
    ) -> Option<&'a BehaviorAnnexContent> {
        for annex in &impl_.annexes {
            if let AnnexContent::BehaviorAnnex(content) = &annex.content {
                return Some(content);
            }
        }
        None
    }

    // Wrapper for calling generate_state_variables, generate_state_enum, generate_initial_state
    pub fn generate_ba_variables_states(
        &mut self,
        _: &ComponentImplementation,
        behavior_annex: &BehaviorAnnexContent,
    ) -> Vec<Statement> {
        let mut stmts = Vec::new();

        // 1. Define local variables
        if let Some(state_vars) = &behavior_annex.state_variables {
            stmts.extend(self.generate_state_variables(state_vars));
        }

        // 2. Generate the state enum and store state info
        if let Some(states) = &behavior_annex.states {
            self.store_state_info(states);
            stmts.extend(self.generate_state_enum(states));
        }

        // 3. Set the initial state
        if let Some(states) = &behavior_annex.states {
            stmts.extend(self.generate_initial_state(states));
        }
        stmts
    }

    /// Generate state machine code
    // fn generate_state_machine_code(&mut self, _: &ComponentImplementation, behavior_annex: &BehaviorAnnexContent) -> Option<Vec<Statement>> {
    //     let mut stmts = Vec::new();

    //     // 1. Define local variables
    //     if let Some(state_vars) = &behavior_annex.state_variables {
    //         stmts.extend(self.generate_state_variables(state_vars));
    //     }

    //     // 2. Generate the state enum and store state info
    //     if let Some(states) = &behavior_annex.states {
    //         self.store_state_info(states);
    //         stmts.extend(self.generate_state_enum(states));
    //     }

    //     // 3. Set the initial state
    //     if let Some(states) = &behavior_annex.states {
    //         stmts.extend(self.generate_initial_state(states));
    //     }

    //     // 4. Generate the state machine loop
    //     if let Some(transitions) = &behavior_annex.transitions {
    //         stmts.extend(self.generate_state_machine_loop(transitions));
    //     }

    //     Some(stmts)
    // }

    /// Generate state variable declarations
    fn generate_state_variables(&self, state_vars: &[StateVariable]) -> Vec<Statement> {
        let mut stmts = Vec::new();

        for var in state_vars {
            let var_name = var.identifier.clone();
            let rust_type = self.convert_aadl_type_to_rust(&var.data_type);

            // Generate the variable declaration
            let init_value = if let Some(init) = &var.initial_value {
                // If an initial value is provided, use it
                self.parse_initial_value(init, &rust_type)
            } else {
                // Otherwise use a default value
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

    /// Generate the state enum
    fn generate_state_enum(&self, states: &[State]) -> Vec<Statement> {
        let mut variants = Vec::new();

        for state in states {
            for state_id in &state.identifiers {
                variants.push(Variant {
                    name: state_id.clone(),
                    data: None, // State enums usually carry no data
                    docs: vec![format!("// State: {}", state_id)],
                });
            }
        }

        // Generate the enum definition
        let enum_def = EnumDef {
            name: "State".to_string(), // Use a generic name
            variants,
            generics: Vec::new(),
            derives: vec![], // vec!["Debug".to_string(), "Clone".to_string()],
            docs: vec!["// Behavior Annex state machine states".to_string()],
            vis: Visibility::Private, // Defined inside the function scope
        };

        vec![Statement::Item(Box::new(Item::Enum(enum_def)))]
    }

    /// Store state info in internal data structures
    fn store_state_info(&mut self, states: &[State]) {
        for state in states {
            for state_id in &state.identifiers {
                // If a state has the Complete or Final modifier, it does not need `continue`
                let needs_continue = !state.modifiers.contains(&StateModifier::Complete)
                    && !state.modifiers.contains(&StateModifier::Final);
                self.state_info.insert(state_id.clone(), needs_continue);
            }
        }
    }

    /// Generate initial state initialization
    fn generate_initial_state(&self, states: &[State]) -> Vec<Statement> {
        let mut stmts = Vec::new();

        // Find the initial state
        let initial_state = states
            .iter()
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

    /// Generate the state machine loop
    pub fn generate_state_machine_loop(&mut self, transitions: &[Transition]) -> Vec<Statement> {
        // let mut stmts = Vec::new();

        // Generate port receive code
        let port_receive_stmts = self.generate_port_receive_code(transitions);

        // Generate state transition logic
        let state_transition_stmts = self.generate_state_transition_logic(transitions);

        // Build the full loop body
        let mut loop_body = vec![];

        // Directly append port receive statements into the loop body (no Block wrapper)
        loop_body.extend(port_receive_stmts);

        // Only append transition statements when non-empty
        if !state_transition_stmts.is_empty() {
            // loop_body.push(Statement::Expr(Expr::Block(Block {
            //     stmts: state_transition_stmts,
            //     expr: None,
            // })));
            loop_body.extend(state_transition_stmts);
        }

        // Append elapsed-time computation and sleep statements
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

        // Not placed inside the loop
        // stmts.push(Statement::Expr(Expr::Loop(Box::new(Block {
        //     stmts: loop_body,
        //     expr: None,
        // }))));

        loop_body
    }

    /// Generate port receive code
    /// Generate receive code for ports involved in transitions
    pub fn generate_port_receive_code(&self, transitions: &[Transition]) -> Vec<Statement> {
        let mut stmts = Vec::new();

        // Extract ports from transitions
        let mut ports_to_receive = std::collections::HashSet::new();
        for transition in transitions {
            if let Some(condition) = &transition.behavior_condition {
                self.extract_ports_from_condition(condition, &mut ports_to_receive);
            }
        }

        // Generate receive code for each port
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

    /// Extract port names from a behavior condition
    pub fn extract_ports_from_condition(
        &self,
        condition: &BehaviorCondition,
        ports: &mut std::collections::HashSet<String>,
    ) {
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
                        _ => {
                            // Other types do not involve port receiving
                            eprintln!(
                                "extract_ports_from_condition: other type not involve port receive"
                            );
                        } // DispatchTriggerCondition::SubprogramAccess(_) => {
                          //     // Subprogram access does not involve port receiving
                          // }
                          // DispatchTriggerCondition::Stop => {
                          //     // stop does not involve port receiving
                          // }
                          // DispatchTriggerCondition::CompletionTimeout => {
                          //     // completion timeout does not involve port receiving
                          // }
                          // DispatchTriggerCondition::DispatchTimeout => {
                          //     // dispatch timeout does not involve port receiving
                          // }
                    }
                }
            }
            BehaviorCondition::Execute(execute_cond) => {
                for trigger in &execute_cond.dispatch_triggers {
                    self.extract_port_from_trigger(trigger, ports);
                }
            }
        }
    }

    /// Extract port names from a dispatch trigger
    fn extract_port_from_trigger(
        &self,
        trigger: &DispatchTrigger,
        ports: &mut std::collections::HashSet<String>,
    ) {
        match trigger {
            DispatchTrigger::InEventPort(port_name) => {
                ports.insert(port_name.clone());
            }
            DispatchTrigger::InEventDataPort(port_name) => {
                ports.insert(port_name.clone());
            }
        }
    }

    /// Build a port receive expression: self.port.as_mut().and_then(|rx| rx.try_recv().ok()).unwrap_or_default()
    fn build_port_receive_expr(&self, port_name: &str) -> Expr {
        let base_expr = Expr::Path(
            vec!["self".to_string(), port_name.to_string()],
            PathType::Member,
        );
        let as_mut_expr = Expr::MethodCall(Box::new(base_expr), "as_mut".to_string(), Vec::new());
        let try_recv_expr = Expr::MethodCall(
            Box::new(Expr::Ident("rx".to_string())),
            "try_recv".to_string(),
            Vec::new(),
        );
        let ok_expr = Expr::MethodCall(Box::new(try_recv_expr), "ok".to_string(), Vec::new());
        let closure_expr = Expr::Closure(vec!["rx".to_string()], Box::new(ok_expr));
        let and_then_expr = Expr::MethodCall(
            Box::new(as_mut_expr),
            "and_then".to_string(),
            vec![closure_expr],
        );

        Expr::MethodCall(
            Box::new(and_then_expr),
            "unwrap_or_else".to_string(),
            vec![Expr::Closure(
                vec!["".to_string()],
                Box::new(Expr::Ident("Default::default()".to_string())),
            )],
        )
    }

    /// Generate state transition logic
    fn generate_state_transition_logic(&mut self, transitions: &[Transition]) -> Vec<Statement> {
        let mut stmts = Vec::new();

        // Add a comment
        stmts.push(Statement::Comment(
            "--- BA macro-step execution ---".to_string(),
        ));

        // Clear the previous state-condition records
        self.states_with_conditions.clear();

        // Generate the state transition loop
        let mut match_arms = Vec::new();

        // Generate a match arm for each state
        for transition in transitions {
            for source_state in &transition.source_states {
                let arm = self.generate_state_match_arm(transition, source_state);
                match_arms.push(arm);
            }
        }

        // Add a default branch for `state` when there are conditional states
        // e.g. State::s1 => break,
        // replaced by `_ => break`, so no need to build per-state defaults
        // for state_name in &self.states_with_conditions {
        //     let default_arm = MatchArm {
        //         pattern: format!("State::{}", state_name),
        //         guard: None,
        //         body: Block {
        //             stmts: vec![
        //                 Statement::Comment(format!("This should never execute, but the compiler needs this arm")),
        //                 Statement::Expr(Expr::Ident("break".to_string())),
        //             ],
        //             expr: None,
        //         },
        //     };
        //     match_arms.push(default_arm);
        // }

        // Add the `_ => break` arm
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

        // Build the match expression
        let match_expr = Expr::Match {
            expr: Box::new(Expr::Ident("state".to_string())),
            arms: match_arms,
        };

        // Wrap in a loop
        stmts.push(Statement::Expr(Expr::Loop(Box::new(Block {
            stmts: vec![Statement::Expr(match_expr), Statement::Break],
            expr: None,
        }))));

        stmts
    }

    /// Generate a match arm for a state
    fn generate_state_match_arm(
        &mut self,
        transition: &Transition,
        source_state: &str,
    ) -> MatchArm {
        let mut stmts = Vec::new();
        let mut guard = None;

        // Handle actions
        if let Some(actions) = &transition.actions {
            stmts.extend(self.generate_action_code(actions));
        }

        // Handle transition conditions
        if let Some(condition) = &transition.behavior_condition {
            match condition {
                BehaviorCondition::Dispatch(_) => {
                    // Handle the "on dispatch" condition
                    stmts.push(Statement::Comment(format!(
                        "on dispatch → {}",
                        transition.destination_state
                    )));
                    stmts.push(Statement::Expr(Expr::Assign(
                        Box::new(Expr::Ident("state".to_string())),
                        Box::new(Expr::Path(
                            vec!["State".to_string(), transition.destination_state.clone()],
                            PathType::Namespace,
                        )),
                    )));

                    // Check whether the destination state needs `continue`
                    if self.should_continue_state(&transition.destination_state) {
                        stmts.push(Statement::Continue);
                    } else {
                        stmts.push(Statement::Comment("complete, need to stop".to_string()));
                    }
                }
                BehaviorCondition::Execute(execute_cond) => {
                    // Handle execute condition: use port conditions as a guard
                    if !execute_cond.dispatch_triggers.is_empty() {
                        guard = Some(self.generate_guard_condition(execute_cond));
                        // Record that this state has a conditional guard and needs a default branch
                        self.states_with_conditions.insert(source_state.to_string());
                    }

                    // State transition
                    stmts.push(Statement::Expr(Expr::Assign(
                        Box::new(Expr::Ident("state".to_string())),
                        Box::new(Expr::Path(
                            vec!["State".to_string(), transition.destination_state.clone()],
                            PathType::Namespace,
                        )),
                    )));

                    // Check whether the destination state needs `continue`
                    if self.should_continue_state(&transition.destination_state) {
                        stmts.push(Statement::Continue);
                    } else {
                        stmts.push(Statement::Comment("complete, need to stop".to_string()));
                    }
                }
            }
        } else {
            // Unconditional transition
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
                stmts.push(Statement::Comment("complete, stop".to_string()));
            }
        }

        MatchArm {
            pattern: format!("State::{}", source_state),
            guard,
            body: Block { stmts, expr: None },
        }
    }

    /// Generate a guard condition expression
    fn generate_guard_condition(&self, execute_cond: &DispatchConjunction) -> Expr {
        if execute_cond.dispatch_triggers.is_empty() {
            // Unconditional: return true/false based on the `not` field
            Expr::Literal(Literal::Bool(!execute_cond.not))
        } else {
            // Has port triggers: generate check expressions
            let mut conditions = Vec::new();
            let not_flag = execute_cond.not;
            let parsed_number = execute_cond.number.as_ref().and_then(|num_str| {
                if let Ok(int_val) = num_str.parse::<i64>() {
                    Some(Literal::Int(int_val))
                } else if let Ok(float_val) = num_str.parse::<f64>() {
                    Some(Literal::Float(float_val))
                } else {
                    None
                }
            });
            let use_less_than = execute_cond.less_than && parsed_number.is_some();

            for trigger in &execute_cond.dispatch_triggers {
                match trigger {
                    DispatchTrigger::InEventPort(port_name) => {
                        let port_var = port_name.to_lowercase().to_string();
                        let mut condition = if use_less_than {
                            let number_literal = parsed_number.as_ref().unwrap().clone();
                            Expr::BinaryOp(
                                Box::new(Expr::Literal(number_literal)),
                                "<".to_string(),
                                Box::new(Expr::Ident(port_var.clone())),
                            )
                        } else {
                            let expected_value = !not_flag;
                            Expr::BinaryOp(
                                Box::new(Expr::Ident(port_var.clone())),
                                "==".to_string(),
                                Box::new(Expr::Literal(Literal::Bool(expected_value))),
                            )
                        };

                        if use_less_than && not_flag {
                            condition = Expr::UnaryOp("!".to_string(), Box::new(condition));
                        }

                        conditions.push(condition);
                    }
                    DispatchTrigger::InEventDataPort(port_name) => {
                        let port_var = format!("{}_val", port_name.to_lowercase());
                        let mut condition = Expr::MethodCall(
                            Box::new(Expr::Ident(port_var)),
                            "is_some".to_string(),
                            Vec::new(),
                        );
                        if not_flag {
                            condition = Expr::UnaryOp("!".to_string(), Box::new(condition));
                        }
                        conditions.push(condition);
                    }
                }
            }
            // Combine conditions
            if conditions.len() == 1 {
                conditions[0].clone()
            } else {
                let mut result = conditions.remove(0);
                for condition in conditions {
                    result =
                        Expr::BinaryOp(Box::new(result), "&&".to_string(), Box::new(condition));
                }
                result
            }
        }
    }

    /// Generate execute condition code

    /// Generate action code
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

    /// Generate code for a single action
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
                // Other action types are skipped for now
                stmts.push(Statement::Comment(
                    "TODO: Unsupported action type".to_string(),
                ));
            }
        }

        stmts
    }

    /// Generate code for a basic action
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

    /// Generate assignment action code (currently only supports the first two kinds)
    fn generate_assignment_action(&self, assignment: &AssignmentAction) -> Vec<Statement> {
        let mut stmts: Vec<Statement> = Vec::new();

        match &assignment.target {
            Target::LocalVariable(name) => {
                // Local variable assignment
                let target_expr = Expr::Ident(name.clone());
                let value_expr = match &assignment.value {
                    AssignmentValue::Expression(expr) => self.convert_value_expression(expr),
                    AssignmentValue::Any => Expr::Literal(Literal::Int(0)), // Default value
                };

                stmts.push(Statement::Expr(Expr::Assign(
                    Box::new(target_expr),
                    Box::new(value_expr),
                )));
            }
            // TODO: To be revised. Outgoing port assignment should not send; it should only assign.
            // Sending should be done by !(value)
            Target::OutgoingPort(port_name) => {
                // Outgoing port assignment - generate send code
                let value_expr = match &assignment.value {
                    AssignmentValue::Expression(expr) => self.convert_value_expression(expr),
                    AssignmentValue::Any => Expr::Literal(Literal::Bool(true)), // Default: send true
                };

                // Generate send code
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
                        stmts: vec![Statement::Let(LetStmt {
                            ifmut: false,
                            name: "_".to_string(),
                            ty: None,
                            init: Some(Expr::MethodCall(
                                Box::new(Expr::Ident("sender".to_string())),
                                "send".to_string(),
                                vec![value_expr],
                            )),
                        })],
                        expr: None,
                    },
                    else_branch: None,
                }));
            }
            Target::OutgoingSubprogramParameter(param_name) => {
                // Outgoing subprogram parameter assignment
                let target_expr = Expr::Ident(param_name.clone());
                let value_expr = match &assignment.value {
                    AssignmentValue::Expression(expr) => self.convert_value_expression(expr),
                    AssignmentValue::Any => Expr::Literal(Literal::Int(0)), // Default value
                };

                stmts.push(Statement::Expr(Expr::Assign(
                    Box::new(target_expr),
                    Box::new(value_expr),
                )));
            }
            Target::DataComponentReference(ref_data) => {
                // Data component reference assignment
                let target_expr = Expr::Path(
                    vec!["self".to_string(), ref_data.components.join("_")],
                    PathType::Member,
                );
                let value_expr = match &assignment.value {
                    AssignmentValue::Expression(expr) => self.convert_value_expression(expr),
                    AssignmentValue::Any => Expr::Literal(Literal::Int(0)), // Default value
                };

                stmts.push(Statement::Expr(Expr::Assign(
                    Box::new(target_expr),
                    Box::new(value_expr),
                )));
            }
        }

        stmts
    }

    /// Generate communication action code
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
                            Expr::Literal(Literal::Bool(true)) // Default: send true
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
                                stmts: vec![Statement::Let(LetStmt {
                                    ifmut: false,
                                    name: "_".to_string(),
                                    ty: None,
                                    init: Some(Expr::MethodCall(
                                        Box::new(Expr::Ident("sender".to_string())),
                                        "send".to_string(),
                                        vec![value_expr],
                                    )),
                                })],
                                expr: None,
                            },
                            else_branch: None,
                        }));
                    }
                    _ => {
                        stmts.push(Statement::Comment(
                            "TODO: Unsupported port communication".to_string(),
                        ));
                    }
                }
            }
            _ => {
                stmts.push(Statement::Comment(
                    "TODO: Unsupported communication action".to_string(),
                ));
            }
        }

        stmts
    }

    /// Generate timed action code
    fn generate_timed_action(&self, _timed: &TimedAction) -> Vec<Statement> {
        // Timed actions are skipped for now
        vec![Statement::Comment(
            "TODO: Timed action not implemented".to_string(),
        )]
    }

    /// Generate if-statement code
    fn generate_if_statement(&self, _if_stmt: &IfStatement) -> Vec<Statement> {
        // If statements are skipped for now
        vec![Statement::Comment(
            "TODO: If statement not implemented".to_string(),
        )]
    }

    /// Decide whether a state needs `continue`
    fn should_continue_state(&self, state_name: &str) -> bool {
        // Look up in stored state info
        self.state_info.get(state_name).copied().unwrap_or(true)
    }

    /// Convert an AADL type to a Rust type
    fn convert_aadl_type_to_rust(&self, aadl_type: &str) -> Type {
        // TODO: To be revised. Only partial types are handled here; all types should be covered.
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

    /// Parse an initial value
    fn parse_initial_value(&self, init_value: &str, rust_type: &Type) -> Expr {
        match rust_type {
            Type::Named(type_name) => match type_name.as_str() {
                "i32" => {
                    if let Ok(val) = init_value.parse::<i64>() {
                        Expr::Literal(Literal::Int(val))
                    } else {
                        Expr::Literal(Literal::Int(0))
                    }
                }
                "bool" => match init_value.to_lowercase().as_str() {
                    "true" => Expr::Literal(Literal::Bool(true)),
                    "false" => Expr::Literal(Literal::Bool(false)),
                    _ => Expr::Literal(Literal::Bool(false)),
                },
                "String" => Expr::Literal(Literal::Str(init_value.to_string())),
                "f64" => {
                    if let Ok(val) = init_value.parse::<f64>() {
                        Expr::Literal(Literal::Float(val))
                    } else {
                        Expr::Literal(Literal::Float(0.0))
                    }
                }
                _ => Expr::Literal(Literal::Int(0)),
            },
            _ => Expr::Literal(Literal::Int(0)),
        }
    }

    /// Generate a default value for a type
    fn generate_default_value_for_type(&self, rust_type: &Type) -> Expr {
        match rust_type {
            Type::Named(type_name) => match type_name.as_str() {
                "i32" => Expr::Literal(Literal::Int(0)),
                "bool" => Expr::Literal(Literal::Bool(false)),
                "String" => Expr::Literal(Literal::Str("".to_string())),
                "f64" => Expr::Literal(Literal::Float(0.0)),
                _ => Expr::Literal(Literal::Int(0)),
            },
            _ => Expr::Literal(Literal::Int(0)),
        }
    }

    /// Convert a value expression: And / Or / Xor
    fn convert_value_expression(&self, expr: &ValueExpression) -> Expr {
        // Convert the left relation expression
        let left = self.convert_relation(&expr.left);

        if expr.operations.is_empty() {
            left
        } else {
            // Handle logical operations
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

    /// Convert a relational expression: = != < <= > >=
    fn convert_relation(&self, relation: &Relation) -> Expr {
        let left = self.convert_simple_expression(&relation.left);

        if let Some(comparison) = &relation.comparison {
            // Has a comparison operator
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
            // No comparison operator; return the left expression directly
            left
        }
    }

    /// Convert a simple expression
    fn convert_simple_expression(&self, expr: &SimpleExpression) -> Expr {
        let mut result = self.convert_term(&expr.left);

        // Handle unary sign
        if let Some(sign) = &expr.sign {
            result = match sign {
                UnaryAddingOperator::Plus => result, // +x = x
                UnaryAddingOperator::Minus => Expr::UnaryOp("-".to_string(), Box::new(result)),
            };
        }

        // Handle binary additive operations
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

    /// Convert an additive expression
    fn convert_add_expression(&self, expr: &AddExpression) -> Expr {
        let mut result = self.convert_basic_expression(&expr.left);

        // Handle multiplicative operations
        for op in &expr.operations {
            let right = self.convert_basic_expression(&op.right);
            result = Expr::BinaryOp(
                Box::new(result),
                match op.operator {
                    MultiplicativeOperator::Multiply => "*".to_string(),
                    MultiplicativeOperator::Divide => "/".to_string(),
                    MultiplicativeOperator::Modulo => "%".to_string(),
                    MultiplicativeOperator::Remainder => "%".to_string(), // rem and mod are both `%` in Rust
                },
                Box::new(right),
            );
        }

        result
    }

    /// Convert a term
    fn convert_term(&self, term: &Term) -> Expr {
        let mut result = self.convert_factor(&term.left);

        // Handle multiplicative operations
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

    /// Convert a factor
    fn convert_factor(&self, factor: &Factor) -> Expr {
        match factor {
            Factor::Value(value) => self.convert_value(value),
            Factor::BinaryNumeric {
                left,
                operator,
                right,
            } => {
                let left_expr = self.convert_value(left);
                let right_expr = self.convert_value(right);
                match operator {
                    BinaryNumericOperator::Power => {
                        // Use the `pow` method
                        Expr::MethodCall(Box::new(left_expr), "pow".to_string(), vec![right_expr])
                    }
                }
            }
            Factor::UnaryNumeric { operator, value } => {
                let value_expr = self.convert_value(value);
                match operator {
                    UnaryNumericOperator::Abs => {
                        Expr::MethodCall(Box::new(value_expr), "abs".to_string(), Vec::new())
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

    /// Convert a value
    fn convert_value(&self, value: &Value) -> Expr {
        match value {
            Value::Variable(var) => self.convert_value_variable(var),
            Value::Constant(constant) => self.convert_value_constant(constant),
            Value::Expression(expr) => self.convert_value_expression(expr),
        }
    }

    /// Convert a value variable
    fn convert_value_variable(&self, var: &ValueVariable) -> Expr {
        match var {
            ValueVariable::IncomingPort(port_name) => {
                // Port variable: convert to `port_name_val`
                Expr::Path(
                    vec![port_name.clone(), "_val".to_string()],
                    PathType::Member,
                )
            }
            ValueVariable::IncomingPortCheck(port_name) => {
                // Port presence check: convert to `self.port_name.is_some()`
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
                // Local variable
                Expr::Ident(var_name.clone())
            }
            ValueVariable::DataComponentReference(ref_data) => {
                // Data component reference: convert to `self.data_component`
                Expr::Path(
                    vec!["self".to_string(), ref_data.components.join("_")],
                    PathType::Member,
                )
            }
            ValueVariable::PortCount(port_name) => {
                // Port count: convert to `self.port_name.len()`
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
                // Port freshness: return true for now
                Expr::Literal(Literal::Bool(true))
            }
            ValueVariable::IncomingSubprogramParameter(param_name) => {
                // Subprogram parameter: use the parameter identifier
                Expr::Ident(param_name.clone())
            }
        }
    }

    /// Convert a value constant
    fn convert_value_constant(&self, constant: &ValueConstant) -> Expr {
        match constant {
            ValueConstant::Boolean(b) => Expr::Literal(Literal::Bool(*b)),
            ValueConstant::Numeric(num_str) => {
                // Try parsing as integer or float
                if let Ok(int_val) = num_str.parse::<i64>() {
                    Expr::Literal(Literal::Int(int_val))
                } else if let Ok(float_val) = num_str.parse::<f64>() {
                    Expr::Literal(Literal::Float(float_val))
                } else {
                    // Parsing failed; return 0
                    println!("!!!!!!!!!!!!!!!!!!!!!!!failed to parse numeric literal, return 0");
                    Expr::Literal(Literal::Int(0))
                }
            }
            ValueConstant::String(s) => Expr::Literal(Literal::Str(s.clone())),
            ValueConstant::PropertyConstant(_) => {
                // Property constant: return a default value for now
                Expr::Literal(Literal::Int(0))
            }
            ValueConstant::PropertyValue(_) => {
                // Property value: return a default value for now
                Expr::Literal(Literal::Int(0))
            }
        }
    }

    /// Convert a basic expression
    fn convert_basic_expression(&self, expr: &BasicExpression) -> Expr {
        match expr {
            BasicExpression::NumericOrConstant(num_str) => {
                if let Ok(int_val) = num_str.parse::<i64>() {
                    Expr::Literal(Literal::Int(int_val))
                } else if let Ok(float_val) = num_str.parse::<f64>() {
                    Expr::Literal(Literal::Float(float_val))
                } else {
                    println!("!!!!!!!!!!!!!!!!!!!!!!!failed to parse numeric literal, return 0");
                    Expr::Literal(Literal::Int(0))
                }
            }
            BasicExpression::BehaviorVariable(var_name) => Expr::Ident(var_name.clone()),
            BasicExpression::LoopVariable(var_name) => Expr::Ident(var_name.clone()),
            BasicExpression::Port(port_name) => Expr::Path(
                vec!["self".to_string(), port_name.clone()],
                PathType::Member,
            ),
            BasicExpression::PortWithQualifier { port, qualifier } => {
                let port_expr =
                    Expr::Path(vec!["self".to_string(), port.clone()], PathType::Member);
                match qualifier {
                    PortQualifier::Count => {
                        Expr::MethodCall(Box::new(port_expr), "len".to_string(), Vec::new())
                    }
                    PortQualifier::Fresh => Expr::Literal(Literal::Bool(true)),
                }
            }
            BasicExpression::DataAccess(access_name) => Expr::Path(
                vec!["self".to_string(), access_name.clone()],
                PathType::Member,
            ),
            BasicExpression::Timeout(_) => {
                // Timeout expression: return a default value for now
                Expr::Literal(Literal::Int(0))
            }
            BasicExpression::DataSubcomponent(sub_name) => {
                Expr::Path(vec!["self".to_string(), sub_name.clone()], PathType::Member)
            }
            BasicExpression::DataSubcomponentWithIndex {
                subcomponent,
                index,
            } => {
                let sub_expr = Expr::Path(
                    vec!["self".to_string(), subcomponent.clone()],
                    PathType::Member,
                );
                let index_expr = self.convert_basic_expression(index);
                Expr::Index(Box::new(sub_expr), Box::new(index_expr))
            }
            BasicExpression::DataAccessWithSubcomponent {
                access,
                subcomponent,
            } => Expr::Path(
                vec!["self".to_string(), access.clone(), subcomponent.clone()],
                PathType::Member,
            ),
            BasicExpression::DataSubcomponentWithSubcomponent {
                container,
                subcomponent,
            } => Expr::Path(
                vec!["self".to_string(), container.clone(), subcomponent.clone()],
                PathType::Member,
            ),
            BasicExpression::DataClassifierSubprogram {
                classifier: _,
                subprogram: _,
                parameters: _,
            } => {
                // Subprogram call: return a default value for now
                Expr::Literal(Literal::Int(0))
            }
            BasicExpression::DataClassifierSubprogramWithTimeout {
                classifier: _,
                subprogram: _,
                timeout: _,
            } => {
                // Subprogram call with timeout: return a default value for now
                Expr::Literal(Literal::Int(0))
            }
            BasicExpression::DataClassifierSubprogramWithParameter {
                classifier: _,
                subprogram: _,
                parameter: _,
                expression: _,
            } => {
                // Subprogram call with parameter: return a default value for now
                Expr::Literal(Literal::Int(0))
            }
            BasicExpression::Parenthesized(expr) => {
                Expr::Parenthesized(Box::new(self.convert_basic_expression(expr)))
            }
            BasicExpression::Quantified {
                quantifier: _,
                identifier: _,
                range: _,
                expression: _,
            } => {
                // Quantified expression: return a default value for now
                Expr::Literal(Literal::Bool(false))
            }
            BasicExpression::BinaryOp {
                left,
                operator,
                right,
            } => {
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
            BasicExpression::Not(expr) => Expr::UnaryOp(
                "!".to_string(),
                Box::new(self.convert_basic_expression(expr)),
            ),
        }
    }
}
