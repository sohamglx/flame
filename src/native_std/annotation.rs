use crate::vm::{NativeClosureType, NativeFunctionDef, NativeModuleDef, NativeTypeDef, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn def() -> NativeModuleDef {
    NativeModuleDef {
        name: "std.annotation".to_string(),
        description: "Flame Annotation Metaprogramming and Compiler Context Subsystem".to_string(),
        functions: vec![
            NativeFunctionDef {
                name: "context".to_string(),
                description: "Retrieves the active AnnotationContext for the annotated target (target metadata, callable reference via ref(), compiler, module, and build info). Can only be called inside custom annotations.".to_string(),
                params: vec![],
                return_type: "AnnotationContext".to_string(),
            },
        ],
        types: vec![
            NativeTypeDef {
                name: "AnnotationContext".to_string(),
                description: "Structured context provided to custom annotations.".to_string(),
                fields: vec![
                    ("target".to_string(), "TargetMetadata".to_string()),
                    ("compiler".to_string(), "CompilerInfo".to_string()),
                    ("module".to_string(), "ModuleInfo".to_string()),
                    ("build".to_string(), "BuildInfo".to_string()),
                ],
                methods: vec![],
            },
            NativeTypeDef {
                name: "TargetMetadata".to_string(),
                description: "Metadata and callable reference for the annotated declaration.".to_string(),
                fields: vec![
                    ("name".to_string(), "String".to_string()),
                    ("kind".to_string(), "String".to_string()),
                    ("parameters".to_string(), "Array".to_string()),
                    ("return_type".to_string(), "String".to_string()),
                    ("annotations".to_string(), "Array".to_string()),
                ],
                methods: vec![
                    NativeFunctionDef {
                        name: "ref".to_string(),
                        description: "Returns a callable reference to the annotated function or declaration.".to_string(),
                        params: vec![],
                        return_type: "Function".to_string(),
                    },
                    NativeFunctionDef {
                        name: "transform".to_string(),
                        description: "Replaces the target function with a wrapper produced by the transformer closure.".to_string(),
                        params: vec![("transformer".to_string(), "Function".to_string())],
                        return_type: "Function".to_string(),
                    },
                ],
            },
        ],
        features: vec!["annotation".to_string()],
    }
}

pub fn init() -> HashMap<String, Value> {
    let mut map = HashMap::new();
    map.insert(
        "context".to_string(),
        Value::NativeClosure(NativeClosureType(Arc::new(|_args| {
            if let Some(ctx) = crate::runner::core::get_current_annotation_context() {
                Ok(ctx)
            } else {
                Err("annotation.context() can only be called inside of a custom annotation".to_string())
            }
        }))),
    );
    map
}
