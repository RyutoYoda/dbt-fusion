//! Query abstraction used to carry the query sources and associated
//! metadata around adapter code.

use std::fmt::Display;
use std::str::FromStr;

use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionPhase {
    Unspecified,
    Render,
    Analyze,
    Run,
}

impl FromStr for ExecutionPhase {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "render" => Ok(ExecutionPhase::Render),
            "analyze" => Ok(ExecutionPhase::Analyze),
            "run" => Ok(ExecutionPhase::Run),
            _ => Err("Invalid execution phase".to_string()),
        }
    }
}

impl Display for ExecutionPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ExecutionPhase::Unspecified => "unspecified",
                ExecutionPhase::Render => "render",
                ExecutionPhase::Analyze => "analyze",
                ExecutionPhase::Run => "run",
            }
        )
    }
}

/// Query source plus metadata.
#[derive(Clone, Debug)]
pub struct QueryCtx {
    // Adapter type executing this query
    adapter_type: String,
    // Model executing this query
    node_unique_id: Option<String>,
    // Execution Phase
    phase: Option<ExecutionPhase>,
    // Actual query content
    sql: Option<String>,
    // Time this instance was created
    created_at: DateTime<Utc>,
    // Description (abribrary string) associated with the query
    desc: Option<String>,
}

impl QueryCtx {
    fn create(
        adapter_type: impl Into<String>,
        node_unique_id: Option<String>,
        phase: Option<ExecutionPhase>,
        sql: Option<String>,
        desc: Option<String>,
    ) -> Self {
        Self {
            adapter_type: adapter_type.into(),
            node_unique_id,
            phase,
            sql,
            created_at: Utc::now(),
            desc,
        }
    }

    /// Create a new query with the given adapter type.
    pub fn new(adapter_type: impl Into<String>) -> Self {
        Self::create(adapter_type, None, None, None, None)
    }

    /// Creates a new context by keeping other fields same but
    /// updating unique node id.
    pub fn with_node_id(&self, node_unique_id: impl Into<String>) -> Self {
        // We never allow unique id to be reassigned
        assert!(self.node_unique_id.is_none());
        Self::create(
            self.adapter_type.clone(),
            Some(node_unique_id.into()),
            self.phase.clone(),
            self.sql.clone(),
            self.desc.clone(),
        )
    }

    /// Creates a new context by keeping other fields same and setting
    /// the given sql query.
    pub fn with_sql(&self, sql: impl Into<String>) -> Self {
        // We allow creating new queries by replacing sql
        Self::create(
            self.adapter_type.clone(),
            self.node_unique_id.clone(),
            self.phase.clone(),
            Some(sql.into()),
            self.desc.clone(),
        )
    }

    /// Create a new context by keeping other fields same and using
    /// the given description.
    pub fn with_desc(&self, desc: impl Into<String>) -> Self {
        // We never allow one to reassign description
        assert!(self.desc.is_none());
        Self::create(
            self.adapter_type.clone(),
            self.node_unique_id.clone(),
            self.phase.clone(),
            self.sql.clone(),
            Some(desc.into()),
        )
    }

    /// Creates a new context by keeping other fields same and setting
    /// the given execution phase.
    pub fn with_phase(&self, phase: ExecutionPhase) -> Self {
        Self::create(
            self.adapter_type.clone(),
            self.node_unique_id.clone(),
            Some(phase),
            self.sql.clone(),
            self.desc.clone(),
        )
    }

    /// Return unique node id associated with this context
    pub fn node_id(&self) -> Option<String> {
        self.node_unique_id.clone()
    }

    /// Returns adapter type in this context.
    pub fn adapter_type(&self) -> String {
        self.adapter_type.clone()
    }

    /// Returns time this instance was created.
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Returns time this instance was created as a string.
    pub fn created_at_as_str(&self) -> String {
        self.created_at.to_rfc3339()
    }

    /// Returns a clone of the actual sql code carried by this
    /// instance.
    pub fn sql(&self) -> Option<String> {
        self.sql.clone()
    }

    /// Returns a clone of the description associated with the
    /// context.
    pub fn desc(&self) -> Option<String> {
        self.desc.clone()
    }

    /// Returns the Execution Phase
    pub fn phase(&self) -> Option<ExecutionPhase> {
        self.phase.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desc() {
        let query_ctx = QueryCtx::new("fake").with_desc("this is a really good query");
        assert_eq!(query_ctx.desc().unwrap(), "this is a really good query");
    }

    #[test]
    #[should_panic]
    fn test_desc_twice() {
        QueryCtx::new("fake").with_desc("abc").with_desc("123");
    }

    #[test]
    fn test_unique_id() {
        let query_ctx = QueryCtx::new("fake").with_node_id("123");
        assert_eq!(query_ctx.node_id().unwrap(), "123");
    }

    #[test]
    #[should_panic]
    fn test_unique_id_twice() {
        QueryCtx::new("fake")
            .with_node_id("123")
            .with_node_id("abc");
    }

    #[test]
    fn test_sql() {
        let query_ctx = QueryCtx::new("fake").with_sql("select 1");
        assert_eq!(query_ctx.sql().unwrap(), "select 1");
    }
}
