#[derive(Debug, Clone)]
pub struct StruProperty {
    pub name: String,
    pub value: StruPropertyValue,
    pub docs: Vec<String>,
}

#[derive(Debug, Clone)]
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
