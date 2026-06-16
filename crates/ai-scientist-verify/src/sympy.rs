//! SymPy Math Verification Tool
//!
//! Wraps Python SymPy via subprocess for mathematical verification.
//! All operations are JSON-in/JSON-out for reliable parsing.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymPyResult {
    pub success: bool,
    pub operation: String,
    pub result: String,
    pub latex: Option<String>,
    pub error: Option<String>,
    pub steps: Vec<String>,
}

pub struct SymPyVerifier {
    python_path: String,
}

impl SymPyVerifier {
    pub fn new(python_path: impl Into<String>) -> Self {
        Self { python_path: python_path.into() }
    }

    /// Build a Python script that imports sympy and runs the operation
    fn build_script(operation: &str, expr: &str, extra: &str) -> String {
        format!(
            r#"
import json, sys
try:
    from sympy import *
    from sympy.parsing.sympy_parser import parse_expr, standard_transformations, implicit_multiplication_application
    x, y, z, t, n = symbols('x y z t n')
    transformations = standard_transformations + (implicit_multiplication_application,)
    expr = parse_expr(r'{}', transformations=transformations)
    {}
    result = str({})
    latex_out = latex(result) if 'latex' in dir() else latex({})
    print(json.dumps({{"success": True, "result": str(result), "latex": str(latex_out)}}))
except Exception as e:
    print(json.dumps({{"success": False, "error": str(e)}}))
"#,
            expr, extra, operation, operation
        )
    }

    fn run_sympy(&self, script: &str) -> SymPyResult {
        let output = std::process::Command::new(&self.python_path)
            .args(["-c", script])
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                match serde_json::from_str::<serde_json::Value>(&stdout) {
                    Ok(v) => SymPyResult {
                        success: v["success"].as_bool().unwrap_or(false),
                        operation: String::new(),
                        result: v["result"].as_str().unwrap_or("").to_string(),
                        latex: v["latex"].as_str().map(|s| s.to_string()),
                        error: v["error"].as_str().map(|s| s.to_string()),
                        steps: vec![],
                    },
                    Err(_) => SymPyResult {
                        success: false,
                        operation: String::new(),
                        result: String::new(),
                        latex: None,
                        error: Some(format!("Parse error: {}", stdout)),
                        steps: vec![],
                    },
                }
            }
            Err(e) => SymPyResult {
                success: false,
                operation: String::new(),
                result: String::new(),
                latex: None,
                error: Some(format!("Python execution failed: {}", e)),
                steps: vec![],
            },
        }
    }

    pub fn simplify(&self, expression: &str) -> SymPyResult {
        let script = Self::build_script("simplify(expr)", expression, "");
        let mut result = self.run_sympy(&script);
        result.operation = "simplify".to_string();
        result
    }

    pub fn solve(&self, equation: &str, variable: &str) -> SymPyResult {
        let script = format!(
            r#"
import json, sys
try:
    from sympy import *
    x, y, z, t, n = symbols('x y z t n')
    eq = {}
    sol = solve(eq, {})
    print(json.dumps({{"success": True, "result": str(sol), "latex": latex(sol)}}))
except Exception as e:
    print(json.dumps({{"success": False, "error": str(e)}}))
"#,
            equation, variable
        );
        let mut result = self.run_sympy(&script);
        result.operation = "solve".to_string();
        result
    }

    pub fn integrate(&self, expression: &str, variable: &str) -> SymPyResult {
        let script = format!(
            r#"
import json, sys
try:
    from sympy import *
    from sympy.parsing.sympy_parser import parse_expr, standard_transformations, implicit_multiplication_application
    x, y, z, t, n = symbols('x y z t n')
    transformations = standard_transformations + (implicit_multiplication_application,)
    expr = parse_expr(r'{}', transformations=transformations)
    result = integrate(expr, {})
    print(json.dumps({{"success": True, "result": str(result), "latex": latex(result)}}))
except Exception as e:
    print(json.dumps({{"success": False, "error": str(e)}}))
"#,
            expression, variable
        );
        let mut result = self.run_sympy(&script);
        result.operation = "integrate".to_string();
        result
    }

    pub fn differentiate(&self, expression: &str, variable: &str) -> SymPyResult {
        let script = format!(
            r#"
import json, sys
try:
    from sympy import *
    from sympy.parsing.sympy_parser import parse_expr, standard_transformations, implicit_multiplication_application
    x, y, z, t, n = symbols('x y z t n')
    transformations = standard_transformations + (implicit_multiplication_application,)
    expr = parse_expr(r'{}', transformations=transformations)
    result = diff(expr, {})
    print(json.dumps({{"success": True, "result": str(result), "latex": latex(result)}}))
except Exception as e:
    print(json.dumps({{"success": False, "error": str(e)}}))
"#,
            expression, variable
        );
        let mut result = self.run_sympy(&script);
        result.operation = "diff".to_string();
        result
    }

    pub fn matrix_ops(&self, operation: &str, data: &serde_json::Value) -> SymPyResult {
        let matrix_json = data.to_string();
        let script = format!(
            r#"
import json, sys
try:
    from sympy import Matrix
    data = json.loads(r'{}')
    m = Matrix(data.get('matrix', [[]]))
    op = data.get('op', 'det')
    if op == 'det':
        result = m.det()
    elif op == 'inv':
        result = m.inv()
    elif op == 'eigenvals':
        result = m.eigenvals()
    elif op == 'eigenvects':
        result = m.eigenvects()
    elif op == 'rref':
        result = m.rref()[0]
    elif op == 'rank':
        result = m.rank()
    elif op == 'transpose':
        result = m.T
    else:
        result = str(m)
    print(json.dumps({{"success": True, "result": str(result), "latex": latex(result) if 'latex' in dir() else str(result)}}))
except Exception as e:
    print(json.dumps({{"success": False, "error": str(e)}}))
"#,
            matrix_json
        );
        let mut result = self.run_sympy(&script);
        result.operation = format!("matrix_{}", operation);
        result
    }
}

impl Default for SymPyVerifier {
    fn default() -> Self {
        let python_path = if std::process::Command::new("python3")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            "python3"
        } else {
            "python"
        };

        Self::new(python_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sympy_verifier_creation() {
        let verifier = SymPyVerifier::default();
        assert!(!verifier.python_path.is_empty());
    }

    #[test]
    fn test_sympy_result_format() {
        let result = SymPyResult {
            success: true,
            operation: "simplify".into(),
            result: "2*x".into(),
            latex: Some("2x".into()),
            error: None,
            steps: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: SymPyResult = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
        assert_eq!(parsed.result, "2*x");
    }
}
