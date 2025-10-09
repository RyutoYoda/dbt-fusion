#[macro_use]
pub mod macros;

mod adapter_errors;
mod code_location;
mod codes;
mod preprocessor_location;
mod types;
mod utils;

// Re-export all public types and utilities
pub use adapter_errors::{
    AdapterError, AdapterErrorKind, AdapterResult, AsyncAdapterResult, into_fs_error,
};
pub use code_location::{AbstractLocation, AbstractSpan, CodeLocation, Span};
pub use codes::ErrorCode;
pub use codes::Warnings;
pub use preprocessor_location::MacroSpan;
pub use types::{
    ContextableResult, ErrContext, FsError, FsResult, GenericNameError, LiftableResult,
    MAX_DISPLAY_TOKENS, NameError, WrappedError,
};

// Re-export Cancellable from dbt-cancel for convenience
pub use dbt_cancel::Cancellable;
