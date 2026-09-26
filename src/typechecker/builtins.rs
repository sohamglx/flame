use std::collections::{HashMap, HashSet};
use crate::diagnostics::Diagnostic;
use crate::lexer::Span;
use crate::parser::*;
use super::checker::*;
use super::types::*;

impl TypeChecker {
    pub(crate) fn register_builtins(&mut self) {
        self.functions.insert(
            "print".to_string(),
            FunctionSig {
                is_static: false,
                params: vec![ParamInfo {
                    name: "value".to_string(),
                    ty: Type::Unknown,
                    is_ref: false,
                    is_mut: false,
                    has_default: false,
                }],
                hover_doc: Some("Prints a value to standard output without a newline.".to_string()),
                return_type: Type::Nil,
            },
        );
        self.functions.insert(
            "eprint".to_string(),
            FunctionSig {
                is_static: false,
                params: vec![ParamInfo {
                    name: "value".to_string(),
                    ty: Type::Unknown,
                    is_ref: false,
                    is_mut: false,
                    has_default: false,
                }],
                hover_doc: Some("Prints a value to standard error without a newline.".to_string()),
                return_type: Type::Nil,
            },
        );
        self.functions.insert(
            "println".to_string(),
            FunctionSig {
                is_static: false,
                params: vec![ParamInfo {
                    name: "value".to_string(),
                    ty: Type::Unknown,
                    is_ref: false,
                    is_mut: false,
                    has_default: false,
                }],
                hover_doc: Some(
                    "Prints a value to standard output, followed by a newline.".to_string(),
                ),
                return_type: Type::Nil,
            },
        );
        self.functions.insert(
            "panic".to_string(),
            FunctionSig {
                is_static: false,
                params: vec![ParamInfo {
                    name: "message".to_string(),
                    ty: Type::Unknown,
                    is_ref: false,
                    is_mut: false,
                    has_default: false,
                }],
                hover_doc: Some(
                    "Terminates the program immediately with an error message.".to_string(),
                ),
                return_type: Type::Unknown,
            },
        );
        self.functions.insert(
            "assert".to_string(),
            FunctionSig {
                is_static: false,
                params: vec![ParamInfo {
                    name: "condition".to_string(),
                    ty: Type::Bool,
                    is_ref: false,
                    is_mut: false,
                    has_default: false,
                }],
                hover_doc: Some("Asserts that a condition is true. Panics if false.".to_string()),
                return_type: Type::Nil,
            },
        );
        self.functions.insert(
            "RustServer".to_string(),
            FunctionSig {
                is_static: false,
                params: vec![ParamInfo {
                    name: "value".to_string(),
                    ty: Type::Unknown,
                    is_ref: false,
                    is_mut: false,
                    has_default: false,
                }],
                hover_doc: Some("Creates a new native Rust server handle.".to_string()),
                return_type: Type::Named("ServerHandle".to_string()),
            },
        );
        self.functions.insert(
            "input".to_string(),
            FunctionSig {
                is_static: false,
                params: vec![ParamInfo {
                    name: "prompt".to_string(),
                    ty: Type::String,
                    is_ref: false,
                    is_mut: false,
                    has_default: false,
                }],
                hover_doc: Some(
                    "Prompts the user for input from standard input and returns the read string."
                        .to_string(),
                ),
                return_type: Type::String,
            },
        );
        self.functions.insert(
            "assertEq".to_string(),
            FunctionSig {
                is_static: false,
                params: vec![
                    ParamInfo {
                        name: "actual".to_string(),
                        ty: Type::Unknown,
                        is_ref: false,
                        is_mut: false,
                    has_default: false,
                    },
                    ParamInfo {
                        name: "expected".to_string(),
                        ty: Type::Unknown,
                        is_ref: false,
                        is_mut: false,
                    has_default: false,
                    },
                    ParamInfo {
                        name: "msg".to_string(),
                        ty: Type::String,
                        is_ref: false,
                        is_mut: false,
                    has_default: false,
                    },
                ],
                hover_doc: Some("Asserts that two values are equal. Panics with the provided message if they are not.".to_string()),
                return_type: Type::Nil,
            },
        );
        self.functions.insert(
            "assertNe".to_string(),
            FunctionSig {
                is_static: false,
                params: vec![
                    ParamInfo {
                        name: "actual".to_string(),
                        ty: Type::Unknown,
                        is_ref: false,
                        is_mut: false,
                    has_default: false,
                    },
                    ParamInfo {
                        name: "expected".to_string(),
                        ty: Type::Unknown,
                        is_ref: false,
                        is_mut: false,
                    has_default: false,
                    },
                    ParamInfo {
                        name: "msg".to_string(),
                        ty: Type::String,
                        is_ref: false,
                        is_mut: false,
                    has_default: false,
                    },
                ],
                hover_doc: Some("Asserts that two values are not equal. Panics with the provided message if they are.".to_string()),
                return_type: Type::Nil,
            },
        );
        self.functions.insert(
            "assert_true".to_string(),
            FunctionSig {
                is_static: false,
                params: vec![
                    ParamInfo {
                        name: "cond".to_string(),
                        ty: Type::Bool,
                        is_ref: false,
                        is_mut: false,
                    has_default: false,
                    },
                    ParamInfo {
                        name: "msg".to_string(),
                        ty: Type::String,
                        is_ref: false,
                        is_mut: false,
                    has_default: false,
                    },
                ],
                hover_doc: Some("Asserts that a boolean condition is true. Panics with the provided message if false.".to_string()),
                return_type: Type::Nil,
            },
        );
        self.functions.insert(
            "assert_false".to_string(),
            FunctionSig {
                is_static: false,
                params: vec![
                    ParamInfo {
                        name: "cond".to_string(),
                        ty: Type::Bool,
                        is_ref: false,
                        is_mut: false,
                    has_default: false,
                    },
                    ParamInfo {
                        name: "msg".to_string(),
                        ty: Type::String,
                        is_ref: false,
                        is_mut: false,
                    has_default: false,
                    },
                ],
                hover_doc: Some("Asserts that a boolean condition is false. Panics with the provided message if true.".to_string()),
                return_type: Type::Nil,
            },
        );
        self.functions.insert(
            "mock_api".to_string(),
            FunctionSig {
                is_static: false,
                params: vec![],
                hover_doc: Some("Mocks an API endpoint for testing purposes.".to_string()),
                return_type: Type::Unknown,
            },
        );
        self.functions.insert(
            "mock_data".to_string(),
            FunctionSig {
                is_static: false,
                params: vec![],
                hover_doc: Some("Returns mock data for testing purposes.".to_string()),
                return_type: Type::Unknown,
            },
        );
        self.functions.insert(
            "mock_function".to_string(),
            FunctionSig {
                is_static: false,
                params: vec![],
                hover_doc: Some("Returns a mock function for testing purposes.".to_string()),
                return_type: Type::Nil,
            },
        );

        // Built-in Enums
        let mut result_variants = HashMap::new();
        result_variants.insert("Ok".to_string(), VariantInfo { tuple_items: vec![Type::Unknown], struct_fields: vec![], hover_doc: Some("The successful variant of `Result`, containing the value.\n\n### Example\n```flame\nlet res = Ok(42)\n```".to_string()) });
        result_variants.insert("Err".to_string(), VariantInfo { tuple_items: vec![Type::Unknown], struct_fields: vec![], hover_doc: Some("The error variant of `Result`, containing the error data.\n\n### Example\n```flame\nlet err = Err(\"Something went wrong\")\n```".to_string()) });
        self.enums.insert("Result".to_string(), EnumInfo {
            variants: result_variants,
            hover_doc: Some("`Result` is a generic type that represents either success (`Ok`) or failure (`Err`).\nIt is commonly used for error handling instead of exceptions.\n\n### Example\n```flame\nfn divide(a: Int, b: Int) -> Result<Int, Error> {\n    if b == 0 {\n        return Err(Error { code: 1, message: \"Divide by zero\" })\n    }\n    return Ok(a / b)\n}\n```".to_string()),
        });

        let mut option_variants = HashMap::new();
        option_variants.insert("Some".to_string(), VariantInfo { tuple_items: vec![Type::Unknown], struct_fields: vec![], hover_doc: Some("Contains a value in an `Option`.\n\n### Example\n```flame\nlet val = Some(\"data\")\n```".to_string()) });
        option_variants.insert("None".to_string(), VariantInfo { tuple_items: vec![], struct_fields: vec![], hover_doc: Some("Indicates no value in an `Option`.\n\n### Example\n```flame\nlet empty = None\n```".to_string()) });
        self.enums.insert("Option".to_string(), EnumInfo {
            variants: option_variants,
            hover_doc: Some("`Option` is a generic type that represents an optional value: every `Option` is either `Some` and contains a value, or `None`, and does not.\n\n### Example\n```flame\nlet val = Some(42)\nlet empty = None\n```".to_string()),
        });

        // Built-in Structs
        self.structs.insert("Error".to_string(), StructInfo {
            fields: vec![
                ("message".to_string(), Type::String),
                ("code".to_string(), Type::Named("Int".to_string())),
            ],
            hover_doc: Some("`Error` is a built-in type that represents a standard runtime error.\n\n### Example\n```flame\nlet err = Error { message: \"Not found\", code: 404 }\n```".to_string()),
        });

        self.structs.insert("ParameterInfo".to_string(), StructInfo {
            fields: vec![
                ("name".to_string(), Type::String),
                ("type".to_string(), Type::String),
                ("hasDefault".to_string(), Type::Bool),
                ("isRef".to_string(), Type::Bool),
                ("isMut".to_string(), Type::Bool),
            ],
            hover_doc: Some("Parameter reflection info for the annotated target.".to_string()),
        });

        self.structs.insert("AnnotationInfo".to_string(), StructInfo {
            fields: vec![
                ("name".to_string(), Type::String),
                ("args".to_string(), Type::Vector(Box::new(Type::String))),
            ],
            hover_doc: Some("Annotation reflection info applied to a declaration.".to_string()),
        });

        self.structs.insert("CompilerInfo".to_string(), StructInfo {
            fields: vec![
                ("name".to_string(), Type::String),
                ("version".to_string(), Type::String),
            ],
            hover_doc: Some("Compiler reflection info available to custom annotations.".to_string()),
        });

        self.structs.insert("ModuleInfo".to_string(), StructInfo {
            fields: vec![
                ("name".to_string(), Type::String),
                ("filepath".to_string(), Type::String),
            ],
            hover_doc: Some("Current module reflection info available to custom annotations.".to_string()),
        });

        self.structs.insert("BuildInfo".to_string(), StructInfo {
            fields: vec![
                ("target".to_string(), Type::String),
                ("mode".to_string(), Type::String),
                ("platform".to_string(), Type::String),
                ("arch".to_string(), Type::String),
                ("features".to_string(), Type::Vector(Box::new(Type::String))),
            ],
            hover_doc: Some("Target platform and compilation build info available to custom annotations.".to_string()),
        });

        self.structs.insert("TargetMetadata".to_string(), StructInfo {
            fields: vec![
                ("name".to_string(), Type::String),
                ("kind".to_string(), Type::String),
                ("parameters".to_string(), Type::Vector(Box::new(Type::Named("ParameterInfo".to_string())))),
                ("returnType".to_string(), Type::String),
                ("annotations".to_string(), Type::Vector(Box::new(Type::Named("AnnotationInfo".to_string())))),
            ],
            hover_doc: Some("Metadata and callable reference for the annotated declaration.\n\n### Methods\n- `ref()`: Callable reference to the target function\n- `transform(transformer)`: Wraps target function".to_string()),
        });

        let mut target_methods = HashMap::new();
        target_methods.insert(
            "ref".to_string(),
            FunctionSig {
                is_static: true,
                params: vec![],
                hover_doc: Some("Returns a callable reference to the annotated target declaration.\n\n### Example\n```flame\nlet handler = ctx.target.ref()\n```".to_string()),
                return_type: Type::Unknown,
            },
        );
        target_methods.insert(
            "transform".to_string(),
            FunctionSig {
                is_static: true,
                params: vec![ParamInfo {
                    name: "transformer".to_string(),
                    ty: Type::Unknown,
                    is_ref: false,
                    is_mut: false,
                    has_default: false,
                }],
                hover_doc: Some("Wraps the target function with the provided transformer callback.\n\n### Examples\n```flame\n// Direct wrapper receiving target ref and parameters:\nctx.target.transform((ref, text: String) {\n    let res = ref(text)\n    return prefix + \": \" + res\n})\n\n// No-arg wrapper:\nctx.target.transform((ref) {\n    let res = ref()\n    return \"[LOGGED: \" + res + \"]\"\n})\n```".to_string()),
                return_type: Type::Unknown,
            },
        );
        self.methods.insert("TargetMetadata".to_string(), target_methods);

        self.structs.insert("AnnotationContext".to_string(), StructInfo {
            fields: vec![
                ("target".to_string(), Type::Named("TargetMetadata".to_string())),
                ("compiler".to_string(), Type::Named("CompilerInfo".to_string())),
                ("module".to_string(), Type::Named("ModuleInfo".to_string())),
                ("build".to_string(), Type::Named("BuildInfo".to_string())),
            ],
            hover_doc: Some("Context provided to custom annotations exposing target metadata, compiler info, module info, and build configuration.".to_string()),
        });

        let anno_ctx_sig = FunctionSig {
            is_static: false,
            params: vec![],
            hover_doc: Some("Retrieves the active AnnotationContext for the annotated target.\n\nOnly callable inside a custom annotation declaration.\n\n### Example\n```flame\nannotation Route(path: String) {\n    let ctx = annotation.context()\n    let handler = ctx.target.ref()\n}\n```".to_string()),
            return_type: Type::Named("AnnotationContext".to_string()),
        };

        self.functions.insert("context".to_string(), anno_ctx_sig.clone());
        self.functions.insert("annotation.context".to_string(), anno_ctx_sig.clone());
        self.functions.insert("annoation.context".to_string(), anno_ctx_sig.clone());
        self.functions.insert("annotations.context".to_string(), anno_ctx_sig.clone());
        self.functions.insert("std.annotation.context".to_string(), anno_ctx_sig.clone());
        self.functions.insert("std.annotations.context".to_string(), anno_ctx_sig);
    }

    pub(crate) fn get_std_module_type(&self, mod_name: &str) -> Type {
        match mod_name {
            "time" => {
                let mut map = HashMap::new();
                let mut docs = HashMap::new();

                let mut ts_map = HashMap::new();
                ts_map.insert("millis".to_string(), Type::Int);
                ts_map.insert(
                    "toMillis".to_string(),
                    Type::Function(vec![], Box::new(Type::Int)),
                );
                ts_map.insert(
                    "toSeconds".to_string(),
                    Type::Function(vec![], Box::new(Type::Int)),
                );
                ts_map.insert(
                    "toString".to_string(),
                    Type::Function(vec![], Box::new(Type::String)),
                );
                let ts_ty = Type::Formula(ts_map, HashMap::new());

                map.insert("now".to_string(), Type::Function(vec![], Box::new(ts_ty)));
                if let Some(doc) = crate::blaze::get_std_function_doc("std.time", "now") {
                    docs.insert("now".to_string(), doc.to_string());
                }

                Type::Formula(map, docs)
            }
            "unit" => {
                let mut map = HashMap::new();
                let mut docs = HashMap::new();

                let eq_fn = Type::Function(
                    vec![Type::Int, Type::Int, Type::Int],
                    Box::new(Type::Named("Unit".to_string())),
                );

                map.insert("Equation".to_string(), eq_fn);
                map.insert(
                    "meter".to_string(),
                    Type::Quantity(HashMap::from([("m".to_string(), 1)])),
                );
                map.insert(
                    "second".to_string(),
                    Type::Quantity(HashMap::from([("s".to_string(), 1)])),
                );
                map.insert(
                    "kilogram".to_string(),
                    Type::Quantity(HashMap::from([("kg".to_string(), 1)])),
                );

                if let Some(doc) = crate::blaze::get_std_function_doc("std.unit", "Equation") {
                    docs.insert("Equation".to_string(), doc.to_string());
                }
                docs.insert(
                    "meter".to_string(),
                    "```flame\nunit.meter: Quantity\n```\nThe SI base unit for length.".to_string(),
                );
                docs.insert(
                    "second".to_string(),
                    "```flame\nunit.second: Quantity\n```\nThe SI base unit for time.".to_string(),
                );
                docs.insert(
                    "kilogram".to_string(),
                    "```flame\nunit.kilogram: Quantity\n```\nThe SI base unit for mass."
                        .to_string(),
                );

                Type::Formula(map, docs)
            }
            "math" => {
                let mut map = HashMap::new();
                let mut docs = HashMap::new();

                let float_fn = Type::Function(vec![Type::Unknown], Box::new(Type::Float));
                let float_fn2 =
                    Type::Function(vec![Type::Unknown, Type::Unknown], Box::new(Type::Float));

                map.insert(
                    "pi".to_string(),
                    Type::Function(vec![], Box::new(Type::Float)),
                );
                map.insert(
                    "e".to_string(),
                    Type::Function(vec![], Box::new(Type::Float)),
                );
                map.insert(
                    "inf".to_string(),
                    Type::Function(vec![], Box::new(Type::Float)),
                );
                map.insert("abs".to_string(), float_fn.clone());
                map.insert("sin".to_string(), float_fn.clone());
                map.insert("cos".to_string(), float_fn.clone());
                map.insert("sqrt".to_string(), float_fn.clone());
                map.insert("pow".to_string(), float_fn2.clone());
                map.insert("min".to_string(), float_fn2.clone());
                map.insert("max".to_string(), float_fn2.clone());
                map.insert("round".to_string(), float_fn.clone());
                map.insert("floor".to_string(), float_fn.clone());
                map.insert("ceil".to_string(), float_fn.clone());

                for name in [
                    "pi", "e", "inf", "abs", "sin", "cos", "sqrt", "pow", "min", "max",
                    "round", "floor", "ceil",
                ] {
                    if let Some(doc) = crate::blaze::get_std_function_doc("std.math", name) {
                        docs.insert(name.to_string(), doc.to_string());
                    }
                }

                Type::Formula(map, docs)
            }
            "http" => {
                let mut map = HashMap::new();
                let mut docs = HashMap::new();

                let mut resp_map = HashMap::new();
                resp_map.insert("status".to_string(), Type::Int);
                resp_map.insert("ok".to_string(), Type::Bool);
                resp_map.insert(
                    "text".to_string(),
                    Type::Function(vec![], Box::new(Type::String)),
                );
                resp_map.insert(
                    "json".to_string(),
                    Type::Function(vec![], Box::new(Type::Unknown)),
                );
                let resp_ty = Type::Formula(resp_map, HashMap::new());

                map.insert(
                    "get".to_string(),
                    Type::Function(vec![Type::String], Box::new(resp_ty.clone())),
                );
                if let Some(doc) = crate::blaze::get_std_function_doc("std.http", "get") {
                    docs.insert("get".to_string(), doc.to_string());
                }
                map.insert(
                    "post".to_string(),
                    Type::Function(vec![Type::String, Type::Unknown], Box::new(resp_ty.clone())),
                );
                if let Some(doc) = crate::blaze::get_std_function_doc("std.http", "post") {
                    docs.insert("post".to_string(), doc.to_string());
                }
                Type::Formula(map, docs)
            }
            "json" => {
                let mut map = HashMap::new();
                let mut docs = HashMap::new();
                map.insert(
                    "parse".to_string(),
                    Type::Function(
                        vec![Type::Union(vec![
                            Type::String,
                            Type::Named("Formula".to_string()),
                            Type::Formula(HashMap::new(), HashMap::new()),
                        ])],
                        Box::new(Type::Unknown),
                    ),
                );
                map.insert(
                    "stringify".to_string(),
                    Type::Function(vec![Type::Unknown], Box::new(Type::String)),
                );
                map.insert(
                    "fromJson".to_string(),
                    Type::Function(vec![Type::Unknown], Box::new(Type::Unknown)),
                );
                map.insert(
                    "fromByte".to_string(),
                    Type::Function(vec![Type::Unknown], Box::new(Type::Unknown)),
                );
                map.insert(
                    "fromBytes".to_string(),
                    Type::Function(vec![Type::Unknown], Box::new(Type::Unknown)),
                );
                for name in ["parse", "stringify", "fromJson", "fromByte", "fromBytes"] {
                    if let Some(doc) = crate::blaze::get_std_function_doc("std.json", name) {
                        docs.insert(name.to_string(), doc.to_string());
                    }
                }
                Type::Formula(map, docs)
            }
            "thread" => {
                let mut map = HashMap::new();
                let mut docs = HashMap::new();

                map.insert("sleep".to_string(), Type::Function(vec![Type::Int], Box::new(Type::Nil)));
                map.insert("yield".to_string(), Type::Function(vec![], Box::new(Type::Nil)));
                map.insert("yieldNow".to_string(), Type::Function(vec![], Box::new(Type::Nil)));
                map.insert("yield_now".to_string(), Type::Function(vec![], Box::new(Type::Nil)));
                map.insert("id".to_string(), Type::Function(vec![], Box::new(Type::String)));
                map.insert("spawn".to_string(), Type::Function(vec![Type::Unknown], Box::new(Type::Named("ThreadHandler".to_string()))));
                map.insert("channel".to_string(), Type::Function(vec![], Box::new(Type::Tuple(vec![Type::Named("Sender".to_string()), Type::Named("Receiver".to_string())]))));

                for name in ["sleep", "yield", "yieldNow", "yield_now", "id", "spawn", "channel"] {
                    if let Some(doc) = crate::blaze::get_std_function_doc("std.thread", name) {
                        docs.insert(name.to_string(), doc.to_string());
                    }
                }

                Type::Formula(map, docs)
            }
            "fs" => {
                let mut map = HashMap::new();
                let mut docs = HashMap::new();

                map.insert("read".to_string(), Type::Function(vec![Type::String], Box::new(Type::String)));
                map.insert("write".to_string(), Type::Function(vec![Type::String, Type::String], Box::new(Type::Nil)));
                map.insert("append".to_string(), Type::Function(vec![Type::String, Type::String], Box::new(Type::Nil)));
                map.insert("readBytes".to_string(), Type::Function(vec![Type::String], Box::new(Type::Named("Bytes".to_string()))));
                map.insert("writeBytes".to_string(), Type::Function(vec![Type::String, Type::Unknown], Box::new(Type::Nil)));
                map.insert("appendBytes".to_string(), Type::Function(vec![Type::String, Type::Unknown], Box::new(Type::Nil)));
                map.insert("exists".to_string(), Type::Function(vec![Type::String], Box::new(Type::Bool)));
                map.insert("remove".to_string(), Type::Function(vec![Type::String], Box::new(Type::Nil)));
                map.insert("readDir".to_string(), Type::Function(vec![Type::String], Box::new(Type::Vector(Box::new(Type::String)))));

                for name in ["read", "write", "append", "readBytes", "writeBytes", "appendBytes", "exists", "remove", "readDir"] {
                    if let Some(doc) = crate::blaze::get_std_function_doc("std.fs", name) {
                        docs.insert(name.to_string(), doc.to_string());
                    }
                }

                Type::Formula(map, docs)
            }
            "byte" => {
                let mut map = HashMap::new();
                let mut docs = HashMap::new();

                map.insert("fromByte".to_string(), Type::Function(vec![Type::Unknown], Box::new(Type::Byte)));
                map.insert("fromBytes".to_string(), Type::Function(vec![Type::Unknown], Box::new(Type::Named("Bytes".to_string()))));
                map.insert("toString".to_string(), Type::Function(vec![Type::Unknown], Box::new(Type::String)));
                map.insert("toHex".to_string(), Type::Function(vec![Type::Unknown], Box::new(Type::String)));
                map.insert("toInt".to_string(), Type::Function(vec![Type::Unknown], Box::new(Type::Int)));
                map.insert("readBytes".to_string(), Type::Function(vec![Type::String], Box::new(Type::Named("Bytes".to_string()))));
                map.insert("writeBytes".to_string(), Type::Function(vec![Type::String, Type::Unknown], Box::new(Type::Nil)));
                map.insert("appendBytes".to_string(), Type::Function(vec![Type::String, Type::Unknown], Box::new(Type::Nil)));
                map.insert("readByte".to_string(), Type::Function(vec![Type::String], Box::new(Type::Byte)));
                map.insert("writeByte".to_string(), Type::Function(vec![Type::String, Type::Unknown], Box::new(Type::Nil)));
                map.insert("appendByte".to_string(), Type::Function(vec![Type::String, Type::Unknown], Box::new(Type::Nil)));
                map.insert("readByteAt".to_string(), Type::Function(vec![Type::String, Type::Int], Box::new(Type::Byte)));
                map.insert("writeByteAt".to_string(), Type::Function(vec![Type::String, Type::Int, Type::Unknown], Box::new(Type::Nil)));

                for name in ["fromByte", "fromBytes", "toString", "toHex", "toInt", "readBytes", "writeBytes", "appendBytes", "readByte", "writeByte", "appendByte", "readByteAt", "writeByteAt"] {
                    if let Some(doc) = crate::blaze::get_std_function_doc("std.byte", name) {
                        docs.insert(name.to_string(), doc.to_string());
                    }
                }

                Type::Formula(map, docs)
            }
            "ws" | "net.ws" | "std.net.ws" => {
                let mut map = HashMap::new();
                let mut docs = HashMap::new();

                let mut socket_map = HashMap::new();
                let mut socket_docs = HashMap::new();

                socket_map.insert("connect".to_string(), Type::Function(vec![Type::String], Box::new(Type::Named("ClientSocket".to_string()))));
                socket_map.insert("listen".to_string(), Type::Function(vec![Type::String], Box::new(Type::Named("Server".to_string()))));
                if let Some(doc) = crate::blaze::get_std_function_doc("std.net.ws", "connect") {
                    socket_docs.insert("connect".to_string(), doc.to_string());
                }
                if let Some(doc) = crate::blaze::get_std_function_doc("std.net.ws", "listen") {
                    socket_docs.insert("listen".to_string(), doc.to_string());
                }

                let socket_formula = Type::Formula(socket_map, socket_docs);

                map.insert("connect".to_string(), Type::Function(vec![Type::String], Box::new(Type::Named("ClientSocket".to_string()))));
                map.insert("listen".to_string(), Type::Function(vec![Type::String], Box::new(Type::Named("Server".to_string()))));
                map.insert("Socket".to_string(), socket_formula.clone());
                map.insert("WebSocket".to_string(), socket_formula);
                map.insert("ClientSocket".to_string(), Type::Named("ClientSocket".to_string()));
                map.insert("Server".to_string(), Type::Named("Server".to_string()));
                map.insert("ServerClient".to_string(), Type::Named("ServerClient".to_string()));
                map.insert("Stream".to_string(), Type::Named("Stream".to_string()));

                docs.insert(
                    "Socket".to_string(),
                    "```flame\nstruct Socket\n```\nWebSocket subsystem factory interface exposing client connection and server binding.\n\n**Methods**:\n- `connect(url: String) -> ClientSocket`\n- `listen(addr: String) -> Server`\n\n**Example**:\n```flame\nimport std.net.ws\n\nlet s = ws.Socket.listen(\"127.0.0.1:8080\")\nlet c = ws.Socket.connect(\"ws://127.0.0.1:8080\")\n```".to_string(),
                );
                docs.insert(
                    "WebSocket".to_string(),
                    "```flame\nstruct WebSocket\n```\nWebSocket subsystem factory interface exposing client connection and server binding.\n\n**Methods**:\n- `connect(url: String) -> ClientSocket`\n- `listen(addr: String) -> Server`".to_string(),
                );
                docs.insert(
                    "Server".to_string(),
                    "```flame\nstruct Server\n```\nWebSocket server listener instance returned by `ws.Socket.listen()` or `ws.listen()`.\n\nAll event handlers (`onConnect`, `onMessage`, etc.) and controls (`broadcast`, `close`) are accessible directly on this variable `s`.".to_string(),
                );
                docs.insert(
                    "ClientSocket".to_string(),
                    "```flame\nstruct ClientSocket\n```\nWebSocket client connection handle returned by `ws.connect()` or `ws.Socket.connect()`.".to_string(),
                );
                docs.insert(
                    "ServerClient".to_string(),
                    "```flame\nstruct ServerClient\n```\nConnected client connection handle passed to server lifecycle callbacks.".to_string(),
                );
                docs.insert(
                    "Stream".to_string(),
                    "```flame\nstruct Stream\n```\nAsynchronous message stream returned by `socket.messages()`.".to_string(),
                );

                for name in ["connect", "listen"] {
                    if let Some(doc) = crate::blaze::get_std_function_doc("std.net.ws", name) {
                        docs.insert(name.to_string(), doc.to_string());
                    }
                }

                Type::Formula(map, docs)
            }
            "annotation" | "annotations" => {
                let mut map = HashMap::new();
                let mut docs = HashMap::new();

                let ctx_fn = Type::Function(
                    vec![],
                    Box::new(Type::Named("AnnotationContext".to_string())),
                );
                map.insert("context".to_string(), ctx_fn);
                docs.insert(
                    "context".to_string(),
                    "```flame\nfn context() -> AnnotationContext\n```\nRetrieves the active AnnotationContext for the annotated target.\n\nOnly callable inside a custom annotation declaration.\n\n### Example\n```flame\nannotation Route(path: String) {\n    let ctx = annotation.context()\n    let handler = ctx.target.ref()\n}\n```".to_string(),
                );

                map.insert("AnnotationContext".to_string(), Type::Named("AnnotationContext".to_string()));
                map.insert("TargetMetadata".to_string(), Type::Named("TargetMetadata".to_string()));
                map.insert("CompilerInfo".to_string(), Type::Named("CompilerInfo".to_string()));
                map.insert("ModuleInfo".to_string(), Type::Named("ModuleInfo".to_string()));
                map.insert("BuildInfo".to_string(), Type::Named("BuildInfo".to_string()));
                map.insert("ParameterInfo".to_string(), Type::Named("ParameterInfo".to_string()));
                map.insert("AnnotationInfo".to_string(), Type::Named("AnnotationInfo".to_string()));

                Type::Formula(map, docs)
            }
            _ => Type::Named(format!("module:{}", mod_name)),
        }
    }


}
