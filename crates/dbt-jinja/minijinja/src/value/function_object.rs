//! Function object type for dbt functions in Jinja templates.
//!
//! This module provides a wrapper type for function relations that renders as
//! function calls instead of relation names. This is specifically designed for
//! dbt functions which need to be rendered as qualified function names rather
//! than generic relation objects.

use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use crate::error::{Error as MinijinjaError, ErrorKind as MinijinjaErrorKind};
use crate::listener::RenderingEventListener;
use crate::value::{Object, Value};
use crate::vm::State;

/// A wrapper type for function relations that renders as function calls instead of relation names
/// Similar to RelationObject but specifically for dbt functions
#[derive(Debug, Clone)]
pub struct FunctionObject {
    /// The qualified function name (e.g., "database.schema.function_name")
    qualified_name: String,
}

impl FunctionObject {
    /// Create a new FunctionObject with a qualified function name
    pub fn new(qualified_name: String) -> Self {
        Self { qualified_name }
    }

    /// Convert this FunctionObject into a minijinja Value
    pub fn into_value(self) -> Value {
        Value::from_object(self)
    }

    /// Get the qualified function name
    pub fn qualified_name(&self) -> &str {
        &self.qualified_name
    }

    /// Render the function as a qualified function call
    /// This is what gets returned when the function object is rendered in Jinja
    fn render_function_call(&self) -> Result<Value, MinijinjaError> {
        // For functions, we return the qualified name as-is for now
        // In the future, this could be enhanced to include argument placeholders
        // e.g., "database.schema.function_name" or "database.schema.function_name(?)"
        Ok(Value::from(self.qualified_name.clone()))
    }
}

impl Object for FunctionObject {
    fn call_method(
        self: &Arc<Self>,
        _state: &State,
        name: &str,
        _args: &[Value],
        _listeners: &[Rc<dyn RenderingEventListener>],
    ) -> Result<Value, MinijinjaError> {
        match name {
            "render" => self.render_function_call(),
            // For now, functions don't expose database/schema/identifier methods
            // as this would require parsing the qualified name
            _ => Err(MinijinjaError::new(
                MinijinjaErrorKind::UnknownMethod,
                format!("Function object has no method named '{name}'"),
            )),
        }
    }

    fn get_value(self: &Arc<Self>, _key: &Value) -> Option<Value> {
        None
    }

    // Try implementing a "call" method for function object
    fn call(
        self: &Arc<Self>,
        state: &State<'_, '_>,
        args: &[Value],
        _listeners: &[std::rc::Rc<dyn RenderingEventListener>],
    ) -> Result<Value, MinijinjaError> {
        let _ = state;

        // Format arguments as strings and join them with commas
        let arg_strings: Vec<String> = args.iter().map(|arg| format!("{}", arg)).collect();
        let args_str = arg_strings.join(", ");

        Ok(format!("{}({})", self.qualified_name, args_str).into())
    }
}

impl fmt::Display for FunctionObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // When the FunctionObject is converted to string (e.g., in Jinja rendering),
        // return the qualified function name
        write!(f, "{}", self.qualified_name)
    }
}
