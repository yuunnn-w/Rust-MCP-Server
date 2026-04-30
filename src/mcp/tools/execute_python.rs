use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExecutePythonParams {
    /// Python code to execute. Assign the desired return value to __result.
    #[schemars(description = "Python code to execute. Assign the desired return value to __result. Available standard library modules: math, random, statistics, datetime, itertools, functools, collections, re, string, json, fractions, decimal, typing, hashlib, base64, bisect, heapq, copy, pprint, enum, types, dataclasses, inspect, sys. Filesystem access is controlled by a separate toggle.")]
    pub code: String,
    /// Execution timeout in milliseconds (default: 5000, max: 30000)
    #[schemars(description = "Execution timeout in milliseconds (default: 5000, max: 30000)")]
    pub timeout_ms: Option<u64>,
}

pub async fn execute_python(
    params: Parameters<ExecutePythonParams>,
    working_dir: &std::path::Path,
    allow_fs_access: bool,
) -> Result<CallToolResult, String> {
    let params = params.0;
    if params.code.len() > 10000 {
        return Err("Code exceeds maximum length of 10000 characters".to_string());
    }
    let timeout = Duration::from_millis(params.timeout_ms.unwrap_or(5000).clamp(1000, 30000));

    let code = params.code;
    let working_dir_str = working_dir.to_string_lossy().to_string();

    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let result = run_python_code(&code, &working_dir_str, allow_fs_access);
            let _ = tx.send(result);
        })
        .map_err(|e| format!("Failed to spawn Python execution thread: {}", e))?;

    let start = Instant::now();
    let result = match rx.recv_timeout(timeout) {
        Ok(r) => r,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            return Ok(CallToolResult::error(vec![rmcp::model::Content::text(format!(
                "Python execution timed out after {}ms. Consider simplifying the code or increasing timeout_ms.",
                params.timeout_ms.unwrap_or(5000)
            ))]));
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            return Ok(CallToolResult::error(vec![rmcp::model::Content::text(
                "Python execution thread panicked".to_string(),
            )]));
        }
    };

    let elapsed_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok((result_value, stdout, stderr)) => {
            let json = serde_json::json!({
                "result": result_value,
                "stdout": stdout,
                "stderr": stderr,
                "execution_time_ms": elapsed_ms,
            });
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                json.to_string(),
            )]))
        }
        Err(e) => Ok(CallToolResult::error(vec![rmcp::model::Content::text(
            format!("Python execution error: {}", e),
        )])),
    }
}

fn run_python_code(code: &str, working_dir: &str, allow_fs_access: bool) -> Result<(Value, String, String), String> {
    use rustpython_vm as vm;
    use rustpython_vm::convert::ToPyObject;

    let interpreter = vm::Interpreter::builder(Default::default())
        .add_native_modules(&rustpython_stdlib::stdlib_module_defs(vm::Context::genesis()))
        .add_frozen_modules(rustpython_pylib::FROZEN_STDLIB)
        .build();

    interpreter.enter(|vm| {
        let scope = vm.new_scope_with_builtins();

        if allow_fs_access {
            // Inject working_dir into Python globals so code can reference it
            let wd_py = working_dir.to_pyobject(vm);
            if let Err(e) = scope.globals.set_item("__working_dir", wd_py, vm) {
                let mut buf = String::new();
                let _ = vm.write_exception(&mut buf, &e);
                return Err(buf);
            }
        } else {
            // Sandbox: block filesystem access by replacing _io.FileIO and builtins.open,
            // pre-loading random so it can capture os.urandom, then replacing random's os
            // reference with a safe stub and blocking future os/nt/posix imports.
            let sandbox_code = r#"
import sys
import builtins
import _io

class _BlockedFileIO:
    def __init__(self, *args, **kwargs):
        raise OSError("Filesystem access is disabled in sandbox mode")

_io_module = sys.modules.get('_io')
if _io_module is not None:
    _io_module.FileIO = _BlockedFileIO

def _blocked_open(*args, **kwargs):
    raise OSError("Filesystem access is disabled in sandbox mode")
builtins.open = _blocked_open

# Pre-load random so it can import the real os module and capture urandom
import random

# Replace random's internal os reference with a safe stub containing only urandom
import types
_safe_os = types.ModuleType('os')
_safe_os.urandom = random._os.urandom
_safe_os.name = random._os.name
random._os = _safe_os

# Remove os/nt/posix from sys.modules to prevent direct access
for _mod in ('os', 'nt', 'posix', 'os.path'):
    if _mod in sys.modules:
        del sys.modules[_mod]

# Block future imports of os/nt/posix
class _SandboxFinder:
    def find_spec(self, fullname, path, target=None):
        if fullname in ('os', 'nt', 'posix', 'os.path'):
            raise ModuleNotFoundError("Module is disabled in sandbox mode")
        return None

sys.meta_path.insert(0, _SandboxFinder())
"#;
            match vm.compile(sandbox_code, vm::compiler::Mode::Exec, "<sandbox>".to_owned()) {
                Ok(sandbox_obj) => {
                    if let Err(e) = vm.run_code_obj(sandbox_obj, scope.clone()) {
                        let mut buf = String::new();
                        let _ = vm.write_exception(&mut buf, &e);
                        return Err(format!("Sandbox initialization error: {}", buf));
                    }
                }
                Err(e) => return Err(format!("Sandbox compilation error: {:?}", e)),
            }
        }

        // 1. Redirect stdout/stderr to io.StringIO for capture
        let setup_code = "import sys, io\n__stdout_capture = io.StringIO()\n__stderr_capture = io.StringIO()\nsys.stdout = __stdout_capture\nsys.stderr = __stderr_capture";
        let setup_obj = match vm.compile(setup_code, vm::compiler::Mode::Exec, "<setup>".to_owned())
        {
            Ok(c) => c,
            Err(e) => return Err(format!("{:?}", e)),
        };
        if let Err(e) = vm.run_code_obj(setup_obj, scope.clone()) {
            let mut buf = String::new();
            let _ = vm.write_exception(&mut buf, &e);
            return Err(buf);
        }

        // 2. Compile and execute user code
        let code_obj = match vm.compile(code, vm::compiler::Mode::Exec, "<embedded>".to_owned()) {
            Ok(c) => c,
            Err(e) => return Err(format!("{:?}", e)),
        };
        if let Err(e) = vm.run_code_obj(code_obj, scope.clone()) {
            let mut buf = String::new();
            let _ = vm.write_exception(&mut buf, &e);
            return Err(buf);
        }

        // 3. Read captured stdout/stderr
        let capture_code = "__stdout_text = __stdout_capture.getvalue()\n__stderr_text = __stderr_capture.getvalue()";
        let capture_obj = match vm.compile(capture_code, vm::compiler::Mode::Exec, "<capture>".to_owned()) {
            Ok(c) => c,
            Err(e) => return Err(format!("{:?}", e)),
        };
        if let Err(e) = vm.run_code_obj(capture_obj, scope.clone()) {
            let mut buf = String::new();
            let _ = vm.write_exception(&mut buf, &e);
            return Err(buf);
        }

        let stdout = match scope.globals.get_item("__stdout_text", vm) {
            Ok(obj) => obj
                .downcast_ref::<vm::builtins::PyStr>()
                .and_then(|s| s.to_str().map(|v| v.to_string()))
                .unwrap_or_default(),
            Err(_) => String::new(),
        };
        let mut stderr = match scope.globals.get_item("__stderr_text", vm) {
            Ok(obj) => obj
                .downcast_ref::<vm::builtins::PyStr>()
                .and_then(|s| s.to_str().map(|v| v.to_string()))
                .unwrap_or_default(),
            Err(_) => String::new(),
        };

        // 4. Try to get __result; fallback to auto-evaluating last line as expression
        let result_value = match scope.globals.get_item("__result", vm) {
            Ok(obj) => python_obj_to_json(vm, obj),
            Err(_) => {
                let last_line = code
                    .lines()
                    .rev()
                    .map(|s| s.trim())
                    .find(|s| !s.is_empty())
                    .unwrap_or("");
                if !last_line.is_empty() {
                    let expr_code = format!("__result = ({})", last_line);
                    match vm.compile(&expr_code, vm::compiler::Mode::Exec, "<embedded>".to_owned())
                    {
                        Ok(expr_obj) => {
                            if vm.run_code_obj(expr_obj, scope.clone()).is_ok() {
                                if let Ok(obj) = scope.globals.get_item("__result", vm) {
                                    python_obj_to_json(vm, obj)
                                } else {
                                    stderr.push_str("\nNote: __result was not set. Assign the desired return value to __result.");
                                    Value::Null
                                }
                            } else {
                                stderr.push_str("\nNote: __result was not set and the last line is not a valid expression. Assign the desired return value to __result.");
                                Value::Null
                            }
                        }
                        Err(_) => {
                            stderr.push_str("\nNote: __result was not set and the last line is not a valid expression. Assign the desired return value to __result.");
                            Value::Null
                        }
                    }
                } else {
                    stderr.push_str("\nNote: __result was not set. Assign the desired return value to __result.");
                    Value::Null
                }
            }
        };

        Ok((result_value, stdout, stderr))
    })
}

fn python_obj_to_json(vm: &rustpython_vm::VirtualMachine, obj: rustpython_vm::PyObjectRef) -> Value {
    use rustpython_vm::AsObject;
    use rustpython_vm::builtins::{PyBool, PyDict, PyFloat, PyInt, PyList, PyNone, PyStr};

    // None
    if obj.downcast_ref::<PyNone>().is_some() {
        return Value::Null;
    }

    // bool (must check before int since PyBool wraps PyInt)
    if obj.downcast_ref::<PyBool>().is_some() {
        let val = obj.is(&vm.ctx.true_value);
        return Value::Bool(val);
    }

    // int
    if let Some(i) = obj.downcast_ref::<PyInt>() {
        let bigint = i.as_bigint();
        let s = bigint.to_string();
        if let Ok(v) = s.parse::<i64>() {
            return Value::Number(serde_json::Number::from(v));
        }
        if let Ok(f) = s.parse::<f64>() {
            if let Some(n) = serde_json::Number::from_f64(f) {
                return Value::Number(n);
            }
        }
        return Value::String(s);
    }

    // float
    if let Some(f) = obj.downcast_ref::<PyFloat>() {
        if let Some(n) = serde_json::Number::from_f64(f.to_f64()) {
            return Value::Number(n);
        }
    }

    // string
    if let Some(s) = obj.downcast_ref::<PyStr>() {
        if let Some(str_val) = s.to_str() {
            return Value::String(str_val.to_string());
        }
    }

    // list
    if let Some(list) = obj.downcast_ref::<PyList>() {
        let arr: Vec<Value> = list
            .borrow_vec()
            .iter()
            .map(|item| python_obj_to_json(vm, item.clone()))
            .collect();
        return Value::Array(arr);
    }

    // dict
    if let Some(dict) = obj.downcast_ref::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (key_obj, value_obj) in dict.items_vec() {
            let key = if let Some(key_str) = key_obj.downcast_ref::<PyStr>() {
                key_str.to_str().unwrap_or("").to_string()
            } else {
                continue;
            };
            map.insert(key, python_obj_to_json(vm, value_obj));
        }
        return Value::Object(map);
    }

    // Fallback: try string representation
    if let Ok(s) = obj.str(vm) {
        if let Some(str_val) = s.to_str() {
            return Value::String(str_val.to_string());
        }
    }

    Value::Null
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_params(code: &str, timeout_ms: Option<u64>) -> Parameters<ExecutePythonParams> {
        Parameters(ExecutePythonParams {
            code: code.to_string(),
            timeout_ms,
        })
    }

    #[tokio::test]
    async fn test_simple_math() {
        let result = execute_python(
            make_params("import math\n__result = math.pi * 10 ** 2", None),
            std::path::Path::new("."),
            false,
        )
        .await
        .unwrap();
        let text = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .unwrap()
            .text
            .clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        let val = parsed["result"].as_f64().unwrap();
        assert!((val - 314.159265).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_boolean_result() {
        let result = execute_python(
            make_params(
                "radius = 10.2\ntolerance = 0.5\n__result = (10 - tolerance) <= radius <= (10 + tolerance)",
                None,
            ),
            std::path::Path::new("."),
            false,
        )
        .await
        .unwrap();
        let text = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .unwrap()
            .text
            .clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["result"], true);
    }

    #[tokio::test]
    async fn test_execution_error() {
        let result = execute_python(
            make_params("1 / 0", None),
            std::path::Path::new("."),
            false,
        )
        .await;
        assert!(result.is_err() || result.unwrap().is_error.unwrap_or(false));
    }

    #[tokio::test]
    async fn test_timeout() {
        let result = execute_python(
            make_params("while True: pass", Some(1000)),
            std::path::Path::new("."),
            false,
        )
        .await;
        assert!(result.is_err() || result.unwrap().is_error.unwrap_or(false));
    }

    #[tokio::test]
    async fn test_auto_expression() {
        let result = execute_python(make_params("1 + 1", None), std::path::Path::new("."), false)
            .await
            .unwrap();
        let text = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .unwrap()
            .text
            .clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["result"], 2);
    }

    #[tokio::test]
    async fn test_auto_expression_multiline() {
        let result = execute_python(
            make_params("a = 10\nb = 20\na + b", None),
            std::path::Path::new("."),
            false,
        )
        .await
        .unwrap();
        let text = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .unwrap()
            .text
            .clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["result"], 30);
    }

    #[tokio::test]
    async fn test_import_random() {
        let result = execute_python(
            make_params("import random\n__result = random.randint(1, 100)", None),
            std::path::Path::new("."),
            false,
        )
        .await
        .unwrap();
        let text = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .unwrap()
            .text
            .clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        let val = parsed["result"].as_i64().unwrap();
        assert!(val >= 1 && val <= 100);
    }

    #[tokio::test]
    async fn test_print_capture() {
        let result = execute_python(
            make_params(
                "print('hello stdout')\nprint('line2')\nimport sys\nprint('error msg', file=sys.stderr)\n__result = 42",
                None,
            ),
            std::path::Path::new("."),
            false,
        )
        .await
        .unwrap();
        let text = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .unwrap()
            .text
            .clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["result"], 42);
        let stdout = parsed["stdout"].as_str().unwrap_or("");
        assert!(stdout.contains("hello stdout"));
        assert!(stdout.contains("line2"));
        let stderr = parsed["stderr"].as_str().unwrap_or("");
        assert!(stderr.contains("error msg"));
    }

    #[tokio::test]
    async fn test_import_no_result() {
        let result = execute_python(
            make_params("import math", None),
            std::path::Path::new("."),
            false,
        )
        .await
        .unwrap();
        let text = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .unwrap()
            .text
            .clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed["result"].is_null());
        let stderr = parsed["stderr"].as_str().unwrap_or("");
        assert!(stderr.contains("__result"));
    }

    #[tokio::test]
    async fn test_working_dir_injected() {
        let result = execute_python(
            make_params("__result = __working_dir", None),
            std::path::Path::new("/tmp"),
            true,
        )
        .await
        .unwrap();
        let text = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .unwrap()
            .text
            .clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["result"], "/tmp");
    }

    #[tokio::test]
    async fn test_sandbox_blocks_open() {
        let result = execute_python(
            make_params("f = open('test.txt', 'w')", None),
            std::path::Path::new("."),
            false,
        )
        .await;
        assert!(result.is_err() || result.unwrap().is_error.unwrap_or(false));
    }

    #[tokio::test]
    async fn test_sandbox_blocks_os_import() {
        let result = execute_python(
            make_params("import os\n__result = os.name", None),
            std::path::Path::new("."),
            false,
        )
        .await;
        assert!(result.is_err() || result.unwrap().is_error.unwrap_or(false));
    }

    #[tokio::test]
    async fn test_fs_access_allows_open() {
        let result = execute_python(
            make_params("__result = __working_dir", None),
            std::path::Path::new("."),
            true,
        )
        .await;
        assert!(result.is_ok());
        let text = result
            .unwrap()
            .content
            .first()
            .and_then(|c| c.as_text())
            .unwrap()
            .text
            .clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["result"], ".");
    }
}
