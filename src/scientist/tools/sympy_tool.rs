//! SymPy Tool — Mathematical verification via Python SymPy
//!
//! Uses `ai-scientist-verify::SymPyVerifier` under the hood.
//! Exposed as tokitai `#[tool]` methods for LLM function calling.

use ai_scientist_verify::SymPyVerifier;
use serde_json::{json, Value};
use tokitai::tool;

pub struct SymPyTool {
    verifier: SymPyVerifier,
}

impl SymPyTool {
    pub fn new() -> Self {
        Self {
            verifier: SymPyVerifier::default(),
        }
    }
}

impl Default for SymPyTool {
    fn default() -> Self {
        Self::new()
    }
}

#[tool]
impl SymPyTool {
    /// Simplify a mathematical expression using SymPy.
    ///
    /// ## Parameters
    /// - `expression`: Mathematical expression (e.g., "x+x", "sin(x)**2 + cos(x)**2")
    ///
    /// ## Returns
    /// JSON with simplified result and LaTeX representation.
    ///
    /// ## Example
    /// sympy_simplify("x+x") => "2*x"
    pub fn sympy_simplify(&self, expression: String) -> Result<Value, String> {
        let result = self.verifier.simplify(&expression);
        if result.success {
            Ok(json!({
                "status": "success",
                "operation": "simplify",
                "expression": expression,
                "result": result.result,
                "latex": result.latex
            }))
        } else {
            Err(format!(
                "SymPy simplify failed: {}",
                result.error.unwrap_or_else(|| "unknown error".into())
            ))
        }
    }

    /// Solve an equation or expression using SymPy.
    ///
    /// ## Parameters
    /// - `equation`: Equation as a string (e.g., "x**2 - 4", "Eq(x**2, 4)")
    /// - `variable`: Variable to solve for (e.g., "x")
    ///
    /// ## Returns
    /// JSON with solution(s).
    pub fn sympy_solve(&self, equation: String, variable: String) -> Result<Value, String> {
        // Convert equation string to SymPy-compatible form
        let eq = if equation.contains('=') {
            let parts: Vec<&str> = equation.splitn(2, '=').collect();
            format!("Eq({},{})", parts[0].trim(), parts[1].trim())
        } else {
            format!("Eq({}, 0)", equation)
        };

        let result = self.verifier.solve(&eq, &variable);
        if result.success {
            Ok(json!({
                "status": "success",
                "operation": "solve",
                "equation": equation,
                "variable": variable,
                "solutions": result.result,
                "latex": result.latex
            }))
        } else {
            Err(format!(
                "SymPy solve failed: {}",
                result.error.unwrap_or_else(|| "unknown error".into())
            ))
        }
    }

    /// Integrate an expression using SymPy.
    ///
    /// ## Example
    /// sympy_integrate("x", "x") => "x**2/2"
    pub fn sympy_integrate(&self, expression: String, variable: String) -> Result<Value, String> {
        let result = self.verifier.integrate(&expression, &variable);
        if result.success {
            Ok(json!({
                "status": "success",
                "operation": "integrate",
                "expression": expression,
                "variable": variable,
                "result": result.result,
                "latex": result.latex
            }))
        } else {
            Err(format!(
                "SymPy integrate failed: {}",
                result.error.unwrap_or_else(|| "unknown error".into())
            ))
        }
    }

    /// Differentiate an expression using SymPy.
    ///
    /// ## Example
    /// sympy_diff("x**2", "x") => "2*x"
    pub fn sympy_diff(&self, expression: String, variable: String) -> Result<Value, String> {
        let result = self.verifier.differentiate(&expression, &variable);
        if result.success {
            Ok(json!({
                "status": "success",
                "operation": "diff",
                "expression": expression,
                "variable": variable,
                "result": result.result,
                "latex": result.latex
            }))
        } else {
            Err(format!(
                "SymPy diff failed: {}",
                result.error.unwrap_or_else(|| "unknown error".into())
            ))
        }
    }

    /// Perform matrix operations using SymPy.
    ///
    /// ## Parameters
    /// - `matrix`: 2D array of numbers
    /// - `operation`: One of "det", "inv", "eigenvals", "eigenvects", "rref", "rank", "transpose"
    pub fn sympy_matrix(
        &self,
        matrix: Vec<Vec<f64>>,
        operation: String,
    ) -> Result<Value, String> {
        let data = json!({
            "matrix": matrix,
            "op": operation
        });

        let result = self.verifier.matrix_ops(&operation, &data);
        if result.success {
            Ok(json!({
                "status": "success",
                "operation": format!("matrix_{}", operation),
                "matrix_size": format!("{}x{}", matrix.len(), matrix.first().map(|r| r.len()).unwrap_or(0)),
                "result": result.result,
                "latex": result.latex
            }))
        } else {
            Err(format!(
                "SymPy matrix {} failed: {}",
                operation,
                result.error.unwrap_or_else(|| "unknown error".into())
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skip_if_no_sympy() -> bool {
        let verifier = SymPyVerifier::default();
        let result = verifier.simplify("1+1");
        !result.success
    }

    #[test]
    fn test_sympy_simplify() {
        if skip_if_no_sympy() {
            eprintln!("Skipping: SymPy not available");
            return;
        }
        let tool = SymPyTool::new();
        let result = tool.sympy_simplify("x+x".into()).unwrap();
        let data: Value = result;
        assert_eq!(data["status"], "success");
        assert!(data["result"].as_str().unwrap().contains("2*x"));
    }

    #[test]
    fn test_sympy_integrate() {
        if skip_if_no_sympy() {
            eprintln!("Skipping: SymPy not available");
            return;
        }
        let tool = SymPyTool::new();
        let result = tool.sympy_integrate("x".into(), "x".into()).unwrap();
        let data: Value = result;
        assert_eq!(data["status"], "success");
        assert!(data["result"].as_str().unwrap().contains("x**2/2"));
    }

    #[test]
    fn test_sympy_diff() {
        if skip_if_no_sympy() {
            eprintln!("Skipping: SymPy not available");
            return;
        }
        let tool = SymPyTool::new();
        let result = tool.sympy_diff("x**2".into(), "x".into()).unwrap();
        let data: Value = result;
        assert_eq!(data["status"], "success");
        assert!(data["result"].as_str().unwrap().contains("2*x"));
    }

    #[test]
    fn test_sympy_not_available_returns_error() {
        // Find a path that definitely doesn't have python
        let verifier = SymPyVerifier::new("__nonexistent_python_binary__");
        let result = verifier.simplify("1+1");
        assert!(!result.success);
    }
}
