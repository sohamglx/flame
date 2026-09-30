pub mod definition;
pub mod keywords;
pub mod modules;
pub mod scanner;
pub mod semantic_tokens;

use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct JsonDiagnostic {
    pub severity: String,
    pub message: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Serialize, Clone, Debug)]
pub struct JsonCompletion {
    pub label: String,
    pub kind: String,
    pub detail: String,
    pub documentation: Option<String>,
    #[serde(rename = "sortText", skip_serializing_if = "Option::is_none")]
    pub sort_text: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct JsonHover {
    pub label: String,
    pub documentation: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct JsonSignatureHelp {
    pub label: String,
    pub parameters: Vec<String>,
    pub active_parameter: u32,
}

#[allow(unused_imports)]
pub use definition::*;
#[allow(unused_imports)]
pub use keywords::*;
#[allow(unused_imports)]
pub use modules::*;
#[allow(unused_imports)]
pub use scanner::*;
#[allow(unused_imports)]
pub use semantic_tokens::*;
