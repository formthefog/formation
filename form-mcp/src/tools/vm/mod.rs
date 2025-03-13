// VM tools module
//
// This module implements specific VM management tools for the MCP server.

mod status;
mod control;
mod create;
mod list;
mod delete;

pub use status::VMStatusTool;
pub use control::VMControlTool;
pub use create::VMCreateTool;
pub use list::VMListTool;
pub use delete::VMDeleteTool;

use std::sync::Arc;
use crate::tools::registry::ToolRegistry;

/// Register VM management tools with the registry
pub fn register_tools(registry: &ToolRegistry) {
    // Register VM status tool
    if let Err(err) = VMStatusTool::register(registry) {
        eprintln!("Failed to register VM status tool: {}", err);
    }
    
    // Register VM control tool
    if let Err(err) = VMControlTool::register(registry) {
        eprintln!("Failed to register VM control tool: {}", err);
    }
    
    // Register VM create tool
    if let Err(err) = VMCreateTool::register(registry) {
        eprintln!("Failed to register VM create tool: {}", err);
    }
    
    // Register VM list tool
    if let Err(err) = VMListTool::register(registry) {
        eprintln!("Failed to register VM list tool: {}", err);
    }
    
    // Register VM delete tool
    if let Err(err) = VMDeleteTool::register(registry) {
        eprintln!("Failed to register VM delete tool: {}", err);
    }
} 