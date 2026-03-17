// use pest_derive::Parser;

// #[derive(Parser)]
// #[grammar = "aadl.pest"]
// pub struct AADLParser;

pub mod aadl {
    use pest_derive::Parser;
    #[derive(Parser)]
    #[grammar = "aadl.pest"]
    pub struct AADLParser;
}

pub mod ba {
    use pest_derive::Parser;
    #[derive(Parser)]
    #[grammar = "aadl_ba.pest"]
    pub struct BAParser;
}