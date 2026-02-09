use rust_decimal::Decimal;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CorpFinanceError {
    #[error("Invalid input: {field} — {reason}")]
    InvalidInput { field: String, reason: String },

    #[error("Financial impossibility: {0}")]
    FinancialImpossibility(String),

    #[error("Convergence failure: {function} did not converge after {iterations} iterations (delta: {last_delta})")]
    ConvergenceFailure {
        function: String,
        iterations: u32,
        last_delta: Decimal,
    },

    #[error("Insufficient data: {0}")]
    InsufficientData(String),

    #[error("Division by zero in {context}")]
    DivisionByZero { context: String },

    #[error("Date error: {0}")]
    DateError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<serde_json::Error> for CorpFinanceError {
    fn from(e: serde_json::Error) -> Self {
        CorpFinanceError::SerializationError(e.to_string())
    }
}
