#![allow(clippy::all)]
use crate::aadlight_parser;
use pest::iterators::{Pair, Pairs};

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Result, Write};
use std::path::Path;
use crate::aadlight_parser::aadl::Rule;

/* =======================
 * Component Category
 * ======================= */

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentCategory {
    Abstract,
    Data,
    Subprogram,
    SubprogramGroup,
    Thread,
    ThreadGroup,
    Processor,
    Memory,
    Process,
    Bus,
    Device,
    VirtualProcessor,
    VirtualBus,
    System,
}

impl ComponentCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ComponentCategory::Abstract => "Abstract",
            ComponentCategory::Data => "Data",
            ComponentCategory::Subprogram => "Subprogram",
            ComponentCategory::SubprogramGroup => "Subprogram Group",
            ComponentCategory::Thread => "Thread",
            ComponentCategory::ThreadGroup => "Thread Group",
            ComponentCategory::Processor => "Processor",
            ComponentCategory::Memory => "Memory",
            ComponentCategory::Process => "Process",
            ComponentCategory::Bus => "Bus",
            ComponentCategory::Device => "Device",
            ComponentCategory::VirtualProcessor => "Virtual Processor",
            ComponentCategory::VirtualBus => "Virtual Bus",
            ComponentCategory::System => "System",
        }
    }
}

/* =======================
 * Statistics Struct
 * ======================= */

#[derive(Debug, Default)]
pub struct ModelStatistics {
    component_counts: HashMap<ComponentCategory, usize>,
}

/* =======================
 * Public Entry
 * ======================= */

impl ModelStatistics {
    /// 唯一入口：只接收 Pairs<Rule>
    pub fn from_pairs(pairs: Pairs<Rule>, output_name: String) -> Result<()> {
        let mut stats = ModelStatistics::default();

        for pair in pairs {
            if pair.as_rule() == Rule::file {
                for inner in pair.into_inner() {
                    if inner.as_rule() == Rule::package_declaration {
                        let package_name =
                            extract_package_name(&inner).unwrap_or("unknown_package".to_string());

                        stats.collect_from_package(inner);
                        stats.write_to_file(
                        &output_name,
                        &package_name,
                    )?;
                        stats.component_counts.clear();
                    }
                }
            }
        }

        Ok(())
    }

    /* =======================
     * Package-level traversal
     * ======================= */

    fn collect_from_package(&mut self, pair: Pair<Rule>) {
        debug_assert_eq!(pair.as_rule(), Rule::package_declaration);

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::package_sections {
                self.collect_from_package_section(inner);
            }
        }
    }

    fn collect_from_package_section(&mut self, pair: Pair<Rule>) {
        let mut inner_iter = pair.into_inner();

        // public / private 或 declaration
        if let Some(first) = inner_iter.next() {
            match first.as_str() {
                "public" | "private" => {}
                _ => self.collect_from_declaration(first),
            }
        }

        for inner in inner_iter {
            if inner.as_rule() == Rule::declaration {
                self.collect_from_declaration(inner);
            }
        }
    }

    fn collect_from_declaration(&mut self, pair: Pair<Rule>) {
        let inner = pair.into_inner().next().unwrap();

        match inner.as_rule() {
            // Rule::component_type //不统计组件类型，只统计组件实现。因此Data的数据不可信。
            // |
            Rule::component_implementation => {
                if let Some(cat) = extract_component_category(&inner) {
                    self.increment(cat);
                }
            }
            _ => {}
        }
    }

    fn increment(&mut self, cat: ComponentCategory) {
        *self.component_counts.entry(cat).or_insert(0) += 1;
    }

    /* =======================
     * Output
     * ======================= */

    // fn write_to_file(&self, output_name: &str,package_name: &str) -> Result<()> {
    //     let dir = Path::new("generate/statistics").join(output_name);
    //     fs::create_dir_all(dir)?;

    //     let safe_name = sanitize_package_name(package_name);
    //     let path = dir.join(format!("{safe_name}.md"));
    //     println!("生成模型统计文件: {:?}", path);
    //     let mut file = File::create(path)?;
    //     println!(
    //         "Writing model statistics to generate/statistics/{}.md",
    //         package_name
    //     );

    //     writeln!(file, "# Model Statistics for package `{}`\n", package_name)?;
    //     writeln!(file, "| Component Category | Count |")?;
    //     writeln!(file, "|--------------------|-------|")?;

    //     let mut total = 0usize;

    //     let mut entries: Vec<_> = self.component_counts.iter().collect();
    //     entries.sort_by_key(|(k, _)| k.as_str());

    //     for (cat, count) in entries {
    //         writeln!(file, "| {:<18} | {:>5} |", cat.as_str(), count)?;
    //         total += count;
    //     }

    //     writeln!(file, "| **Total**          | {:>5} |", total)?;
    //     Ok(())
    // }
    fn write_to_file(
        &self,
        output_name: &str,
        package_name: &str,
    ) -> Result<()> {
        let base_dir = Path::new("generate/statistics")
            .join(output_name);

        fs::create_dir_all(&base_dir)?;

        let safe_pkg_name = sanitize_package_name(package_name).to_lowercase();
        let file_path = base_dir.join(format!("{safe_pkg_name}.md"));
        println!("生成模型统计文件: {:?}", file_path);

        let mut file = File::create(file_path)?;

        writeln!(
            file,
            "# Model Statistics for package `{}`\n",
            package_name
        )?;

        writeln!(file, "| Component Category | Count |")?;
        writeln!(file, "|--------------------|-------|")?;

        let mut total = 0usize;

        let mut entries: Vec<_> =
            self.component_counts.iter().collect();
        entries.sort_by_key(|(k, _)| k.as_str());

        for (cat, count) in entries {
            writeln!(
                file,
                "| {:<18} | {:>5} |",
                cat.as_str(),
                count
            )?;
            total += count;
        }

        writeln!(file, "| **Total**          | {:>5} |", total)?;

        Ok(())
    }

}

/* =======================
 * Helpers
 * ======================= */

fn extract_package_name(pair: &Pair<Rule>) -> Option<String> {
    let mut inner = pair.clone().into_inner();
    let name_pair = inner.next()?;
    Some(name_pair.as_str().to_string())
}

fn extract_component_category(pair: &Pair<Rule>) -> Option<ComponentCategory> {
    let mut inner = pair.clone().into_inner();
    let cat = inner.next()?.as_str();

    match cat {
        "abstract" => Some(ComponentCategory::Abstract),
        "data" => Some(ComponentCategory::Data),
        "subprogram" => Some(ComponentCategory::Subprogram),
        "subprogram group" => Some(ComponentCategory::SubprogramGroup),
        "thread" => Some(ComponentCategory::Thread),
        "thread group" => Some(ComponentCategory::ThreadGroup),
        "processor" => Some(ComponentCategory::Processor),
        "memory" => Some(ComponentCategory::Memory),
        "process" => Some(ComponentCategory::Process),
        "bus" => Some(ComponentCategory::Bus),
        "device" => Some(ComponentCategory::Device),
        "virtual processor" => Some(ComponentCategory::VirtualProcessor),
        "virtual bus" => Some(ComponentCategory::VirtualBus),
        "system" => Some(ComponentCategory::System),
        _ => None,
    }
}
fn sanitize_package_name(name: &str) -> String {
    name.replace("::", "_").replace('.', "_")
}
