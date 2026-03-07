#![allow(clippy::all)]
use crate::aadlight_parser;
use pest::{iterators::Pair};
use super::ast::aadl_ast_cj::*;

pub fn print_pair(pair: Pair<aadlight_parser::Rule>, indent: usize) {
    match pair.as_rule() {
        aadlight_parser::Rule::WHITESPACE | aadlight_parser::Rule::COMMENT => return,
        _ => {
            let span = pair.as_span();
            let (start_line, _) = span.start_pos().line_col();
            let (end_line, _) = span.end_pos().line_col();
            
            let content = pair.as_str().trim();
            let truncated_content = if content.len() > 30 {
                format!("{}...", &content[..30])
            } else {
                content.to_string()
            };
            
            println!(
                "{}{:<25} {:<30} (lines {}-{})",
                "  ".repeat(indent),
                format!("{:?}:", pair.as_rule()),
                truncated_content,
                start_line,
                end_line
            );

            for inner in pair.into_inner() {
                print_pair(inner, indent + 1);
            }
        }
    }
}

pub fn print_ast(ast: &Vec<Package>) {
    for package in ast {
        println!("Package: {}", package.name.to_string());
        println!("  Visibility Declarations: {:?}", package.visibility_decls);

        if let Some(public_section) = &package.public_section {
            for decl in &public_section.declarations {
                match decl {
                    AadlDeclaration::ComponentType(comp) => {
                        println!("  Component Type: {} ({:?})", comp.identifier, comp.category);
                        if let FeatureClause::Items(features) = &comp.features {
                            for feature in features {
                                match feature {
                                    Feature::Port(port) => {
                                        println!(
                                            "    Port: {} {:?} {:?}",
                                            port.identifier, port.direction, port.port_type
                                        );
                                    }
                                    Feature::SubcomponentAccess(access) => {
                                        match access {
                                            SubcomponentAccessSpec::Data(d) => {
                                                println!(
                                                    "    Data Access: {} {:?} {:?}",
                                                    d.identifier, d.direction, d.classifier
                                                );
                                            }
                                            SubcomponentAccessSpec::Subprogram(s) => {
                                                println!(
                                                    "    Subprogram Access: {} {:?} {:?}",
                                                    s.identifier, s.direction, s.classifier
                                                );
                                            }
                                        }
                                    }
                                    // _ => {}
                                }
                            }
                        }
                        if let PropertyClause::Properties(props) = &comp.properties {
                            for prop in props {
                                if let Property::BasicProperty(bp) = prop {
                                    println!("    Property: {} => {:?}", bp.identifier.name, bp.value);
                                }
                            }
                        }
                        // 打印 annex 信息
                        if !comp.annexes.is_empty() {
                            println!("    Annexes:");
                            for annex in &comp.annexes {
                                println!("      {:?}: {:?}", annex.identifier, annex.content);
                            }
                        }
                    }
                    AadlDeclaration::ComponentImplementation(impl_) => {
                        println!(
                            "  Component Implementation: {} ({:?})",
                            impl_.name.to_string(),
                            impl_.category
                        );
                        if let SubcomponentClause::Items(subcomps) = &impl_.subcomponents {
                            for subcomp in subcomps {
                                println!("    Subcomponent: {} {:?}", subcomp.identifier, subcomp.category);
                            }
                        }
                        if let CallSequenceClause::Items(callitems) = &impl_.calls {
                            for callitem in callitems {
                                println!("    Subcomponent: {} {:?}", callitem.identifier, callitem.calls);
                            }
                        }
                        if let ConnectionClause::Items(conns) = &impl_.connections {
                            for conn in conns {
                                match conn {
                                    Connection::Port(port_conn) =>{
                                        println!(
                                            "    Connection: {:?}: {:?} -> {:?}",
                                            port_conn.identifier,port_conn.source, port_conn.destination
                                        );
                                    }
                                    Connection::Parameter(parameter_conn) => {
                                        println!(
                                            "    Connection: {:?} -> {:?}",
                                            parameter_conn.source, parameter_conn.destination
                                        );
                                    }
                                    Connection::Access(access_conn) => {
                                        println!(
                                            "    Access Connection: {:?} -> {:?}",
                                            access_conn.source, access_conn.destination
                                        );
                                    }
                                }
                                
                            }
                        }
                        if let PropertyClause::Properties(props) = &impl_.properties {
                            for prop in props {
                                if let Property::BasicProperty(bp) = prop {
                                    println!("    Property: {} => {:?}", bp.identifier.name, bp.value);
                                }
                            }
                        }
                        // 打印 annex 信息
                        if !impl_.annexes.is_empty() {
                            println!("    Annexes:");
                            for annex in &impl_.annexes {
                                println!("      {:?}: {:?}", annex.identifier, annex.content);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}