// Pack management tools
//
// This module implements tools for package management in the Formation network,
// including building and shipping workloads based on Formfile specifications.

pub mod build;
pub mod ship;

pub use build::PackBuildTool;
pub use ship::PackShipTool;

use std::error::Error;
use crate::tools::registry::ToolRegistry;

/// Register pack management tools with the registry
pub fn register_tools(registry: &ToolRegistry) {
    // Register pack build tool
    if let Err(e) = PackBuildTool::register(registry) {
        log::error!("Failed to register PackBuildTool: {}", e);
    }
    
    // Register pack ship tool
    if let Err(e) = PackShipTool::register(registry) {
        log::error!("Failed to register PackShipTool: {}", e);
    }
} 