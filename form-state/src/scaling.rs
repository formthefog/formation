use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::thread;
use std::collections::HashMap;

/// Types of scaling operations that can be performed
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScalingOperation {
    /// Increase the number of instances in the cluster
    ScaleOut {
        /// Target number of instances after scaling
        target_instances: u32,
    },
    /// Decrease the number of instances in the cluster
    ScaleIn {
        /// Target number of instances after scaling
        target_instances: u32,
        /// IDs of instances to remove (if specified)
        instance_ids: Option<Vec<String>>,
    },
    /// Replace specific instances in the cluster
    ReplaceInstances {
        /// IDs of instances to replace
        instance_ids: Vec<String>,
    },
}

/// Represents an error that occurred during a scaling operation
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScalingError {
    /// The type of error that occurred
    pub error_type: String,
    /// A human-readable error message
    pub message: String,
    /// The phase in which the error occurred
    pub phase: String,
}

/// Represents the current phase of a scaling operation
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScalingPhase {
    /// The operation has been requested but not yet validated
    Requested {
        /// The type of scaling operation
        operation: ScalingOperation,
        /// When the operation was requested (Unix timestamp in seconds)
        requested_at: i64,
    },
    /// Validating that the requested operation is permissible
    Validating {
        /// The type of scaling operation
        operation: ScalingOperation,
        /// When validation started (Unix timestamp in seconds)
        started_at: i64,
    },
    /// Planning the execution of the scaling operation
    Planning {
        /// The type of scaling operation
        operation: ScalingOperation,
        /// When planning started (Unix timestamp in seconds)
        started_at: i64,
        /// Pre-operation metrics (may be used for comparison later)
        pre_metrics: Option<ScalingMetrics>,
    },
    /// Allocating resources needed for the scaling operation
    ResourceAllocating {
        /// The type of scaling operation
        operation: ScalingOperation,
        /// When resource allocation started (Unix timestamp in seconds)
        started_at: i64,
        /// Details about resources being allocated
        resources: Option<ScalingResources>,
    },
    /// Preparing instances for addition or removal
    InstancePreparing {
        /// The type of scaling operation
        operation: ScalingOperation,
        /// When instance preparation started (Unix timestamp in seconds)
        started_at: i64,
        /// IDs of instances being affected
        instance_ids: Vec<String>,
    },
    /// Applying configuration changes to the cluster
    Configuring {
        /// The type of scaling operation
        operation: ScalingOperation,
        /// When configuration started (Unix timestamp in seconds)
        started_at: i64,
        /// Previous configuration (for rollback if needed)
        previous_config: Option<String>, // Simplified; would be more structured in reality
    },
    /// Verifying that changes were applied correctly
    Verifying {
        /// The type of scaling operation
        operation: ScalingOperation,
        /// When verification started (Unix timestamp in seconds)
        started_at: i64,
        /// Results of verification tests so far
        test_results: Vec<VerificationResult>,
    },
    /// Performing cleanup and final adjustments
    Finalizing {
        /// The type of scaling operation
        operation: ScalingOperation,
        /// When finalization started (Unix timestamp in seconds)
        started_at: i64,
        /// Tasks that need to be completed
        cleanup_tasks: Vec<String>,
    },
    /// The operation completed successfully
    Completed {
        /// The type of scaling operation
        operation: ScalingOperation,
        /// When the operation completed (Unix timestamp in seconds)
        completed_at: i64,
        /// Total duration of the operation in seconds
        duration_seconds: u64,
        /// Result metrics (for comparison with pre-operation metrics)
        result_metrics: Option<ScalingMetrics>,
    },
    /// The operation failed
    Failed {
        /// The type of scaling operation
        operation: ScalingOperation,
        /// When the operation failed (Unix timestamp in seconds)
        failed_at: i64,
        /// Reason for the failure
        failure_reason: String,
        /// The phase in which the failure occurred
        failure_phase: String,
        /// Partial results (if any)
        partial_results: Option<String>, // Simplified; would be more structured in reality
    },
    /// The operation was manually canceled
    Canceled {
        /// The type of scaling operation
        operation: ScalingOperation,
        /// When the operation was canceled (Unix timestamp in seconds)
        canceled_at: i64,
        /// Reason for the cancellation
        cancellation_reason: String,
        /// The phase at which cancellation occurred
        phase_at_cancellation: String,
    },
}

/// Metrics collected before and after scaling operations
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScalingMetrics {
    /// CPU utilization percentage (0-100)
    pub cpu_utilization: u32,
    /// Memory utilization percentage (0-100)
    pub memory_utilization: u32,
    /// Network throughput in Mbps
    pub network_throughput_mbps: u32,
    /// Storage utilization percentage (0-100)
    pub storage_utilization: u32,
    /// Number of active instances
    pub instance_count: u32,
}

/// Resources being allocated during scaling operations
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScalingResources {
    /// Number of CPU cores being allocated
    pub cpu_cores: u32,
    /// Amount of memory in MB being allocated
    pub memory_mb: u32,
    /// Amount of storage in GB being allocated
    pub storage_gb: u32,
    /// Network bandwidth in Mbps being allocated
    pub network_bandwidth_mbps: u32,
}

/// Result of a verification test during the Verifying phase
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Name of the verification test
    pub test_name: String,
    /// Whether the test passed
    pub passed: bool,
    /// Details about the test result
    pub details: String,
    /// When the test was run (Unix timestamp in seconds)
    pub timestamp: i64,
}

/// Records the history of scaling operations and their outcomes
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScalingOperationRecord {
    /// Unique identifier for this operation
    pub operation_id: String,
    /// The type of scaling operation
    pub operation: ScalingOperation,
    /// When the operation was started (Unix timestamp in seconds)
    pub started_at: i64,
    /// When the operation ended (Unix timestamp in seconds), if it has ended
    pub ended_at: Option<i64>,
    /// The final phase of the operation
    pub final_phase: String,
    /// Whether the operation was successful
    pub successful: bool,
    /// Error details, if the operation failed
    pub error: Option<ScalingError>,
}

/// Manager for the scaling state machine
///
/// The ScalingManager tracks the current phase of a scaling operation, manages
/// transitions between phases, and maintains a history of past operations.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq)]
pub struct ScalingManager {
    /// The current phase of the scaling operation
    current_phase: Option<ScalingPhase>,
    /// History of past scaling operations
    operation_history: Vec<ScalingOperationRecord>,
    /// Maximum allowed duration for each phase in seconds
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default)]
    phase_timeouts: std::collections::HashMap<String, u64>,
    /// Last check timestamp (used for timeout detection)
    last_check_time: i64,
}

impl std::hash::Hash for ScalingManager {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.current_phase.hash(state);
        self.operation_history.hash(state);
        self.last_check_time.hash(state);
        // We intentionally skip hashing phase_timeouts as HashMap is not Hash
    }
}

// Implement PartialOrd and Ord manually, since there's no natural ordering for ScalingManager
impl PartialOrd for ScalingManager {
    fn partial_cmp(&self, _other: &Self) -> Option<std::cmp::Ordering> {
        // We don't have a natural ordering, so we'll just say they're equal
        // This is a minimal implementation to satisfy the trait requirement
        Some(std::cmp::Ordering::Equal)
    }
}

impl Ord for ScalingManager {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // We don't have a natural ordering, so we'll just say they're equal
        // This is a minimal implementation to satisfy the trait requirement
        self.partial_cmp(other).unwrap()
    }
}

impl ScalingManager {
    /// Creates a new ScalingManager with default settings
    pub fn new() -> Self {
        let mut phase_timeouts = std::collections::HashMap::new();
        
        // Set default timeouts for each phase
        phase_timeouts.insert("Requested".to_string(), 60);         // 1 minute
        phase_timeouts.insert("Validating".to_string(), 60);        // 1 minute
        phase_timeouts.insert("Planning".to_string(), 120);         // 2 minutes
        phase_timeouts.insert("ResourceAllocating".to_string(), 300); // 5 minutes
        phase_timeouts.insert("InstancePreparing".to_string(), 600); // 10 minutes
        phase_timeouts.insert("Configuring".to_string(), 300);      // 5 minutes
        phase_timeouts.insert("Verifying".to_string(), 180);        // 3 minutes
        phase_timeouts.insert("Finalizing".to_string(), 120);       // 2 minutes
        
        Self {
            current_phase: None,
            operation_history: Vec::new(),
            phase_timeouts,
            last_check_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs() as i64,
        }
    }
    
    /// Returns a reference to the current phase, if any
    pub fn current_phase(&self) -> Option<&ScalingPhase> {
        self.current_phase.as_ref()
    }
    
    /// Returns the history of scaling operations
    pub fn operation_history(&self) -> &[ScalingOperationRecord] {
        &self.operation_history
    }
    
    /// Sets the timeout for a specific phase
    pub fn set_phase_timeout(&mut self, phase: &str, timeout_seconds: u64) {
        self.phase_timeouts.insert(phase.to_string(), timeout_seconds);
    }
    
    /// Starts a new scaling operation
    ///
    /// This method creates a new Requested phase and sets it as the current phase.
    /// It returns an error if there is already an operation in progress.
    ///
    /// # Arguments
    ///
    /// * `operation` - The scaling operation to start
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the operation was started successfully
    /// * `Err(ScalingError)` if the operation could not be started
    pub fn start_operation(&mut self, operation: ScalingOperation) -> Result<(), ScalingError> {
        // Check if there's already an operation in progress
        if let Some(phase) = &self.current_phase {
            if !phase.is_terminal() {
                return Err(ScalingError {
                    error_type: "ConcurrentOperation".to_string(),
                    message: format!("Cannot start a new operation while in {} phase", phase.phase_name()),
                    phase: phase.phase_name().to_string(),
                });
            }
        }
        
        // Start a new operation
        let requested_phase = ScalingPhase::new_requested(operation.clone());
        
        // Create operation ID as a timestamp-based string
        let operation_id = format!("op-{}", SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs());
        
        // Record the start of the operation
        self.operation_history.push(ScalingOperationRecord {
            operation_id,
            operation: operation.clone(),
            started_at: match &requested_phase {
                ScalingPhase::Requested { requested_at, .. } => *requested_at,
                _ => unreachable!(),
            },
            ended_at: None,
            final_phase: "Requested".to_string(),
            successful: false,
            error: None,
        });
        
        // Set the current phase
        self.current_phase = Some(requested_phase);
        
        Ok(())
    }
    
    /// Transitions to the Validating phase
    ///
    /// This method can only be called when in the Requested phase.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the transition was successful
    /// * `Err(ScalingError)` if the transition failed
    pub fn transition_to_validating(&mut self) -> Result<(), ScalingError> {
        let current_phase = match &self.current_phase {
            Some(phase) => phase,
            None => return Err(ScalingError {
                error_type: "NoActiveOperation".to_string(),
                message: "No active operation to transition".to_string(),
                phase: "None".to_string(),
            }),
        };
        
        // Ensure we're in the Requested phase
        if !matches!(current_phase, ScalingPhase::Requested { .. }) {
            return Err(ScalingError {
                error_type: "InvalidTransition".to_string(),
                message: format!("Cannot transition from {} to Validating", current_phase.phase_name()),
                phase: current_phase.phase_name().to_string(),
            });
        }
        
        // Get the operation from the current phase
        let operation = current_phase.operation().clone();
        
        // Create the new phase
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;
        
        let validating_phase = ScalingPhase::Validating {
            operation,
            started_at: now,
        };
        
        // Update the current phase
        self.current_phase = Some(validating_phase);
        
        // Update the operation history
        if let Some(record) = self.operation_history.last_mut() {
            record.final_phase = "Validating".to_string();
        }
        
        Ok(())
    }
    
    /// Transitions to the Planning phase
    ///
    /// This method can only be called when in the Validating phase.
    ///
    /// # Arguments
    ///
    /// * `pre_metrics` - Optional metrics collected before the operation
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the transition was successful
    /// * `Err(ScalingError)` if the transition failed
    pub fn transition_to_planning(&mut self, pre_metrics: Option<ScalingMetrics>) -> Result<(), ScalingError> {
        let current_phase = match &self.current_phase {
            Some(phase) => phase,
            None => return Err(ScalingError {
                error_type: "NoActiveOperation".to_string(),
                message: "No active operation to transition".to_string(),
                phase: "None".to_string(),
            }),
        };
        
        // Ensure we're in the Validating phase
        if !matches!(current_phase, ScalingPhase::Validating { .. }) {
            return Err(ScalingError {
                error_type: "InvalidTransition".to_string(),
                message: format!("Cannot transition from {} to Planning", current_phase.phase_name()),
                phase: current_phase.phase_name().to_string(),
            });
        }
        
        // Get the operation from the current phase
        let operation = current_phase.operation().clone();
        
        // Create the new phase
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;
        
        let planning_phase = ScalingPhase::Planning {
            operation,
            started_at: now,
            pre_metrics,
        };
        
        // Update the current phase
        self.current_phase = Some(planning_phase);
        
        // Update the operation history
        if let Some(record) = self.operation_history.last_mut() {
            record.final_phase = "Planning".to_string();
        }
        
        Ok(())
    }
    
    /// Transitions to the ResourceAllocating phase
    ///
    /// This method can only be called when in the Planning phase.
    ///
    /// # Arguments
    ///
    /// * `resources` - Optional details about the resources being allocated
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the transition was successful
    /// * `Err(ScalingError)` if the transition failed
    pub fn transition_to_resource_allocating(&mut self, resources: Option<ScalingResources>) -> Result<(), ScalingError> {
        let current_phase = match &self.current_phase {
            Some(phase) => phase,
            None => return Err(ScalingError {
                error_type: "NoActiveOperation".to_string(),
                message: "No active operation to transition".to_string(),
                phase: "None".to_string(),
            }),
        };
        
        // Ensure we're in the Planning phase
        if !matches!(current_phase, ScalingPhase::Planning { .. }) {
            return Err(ScalingError {
                error_type: "InvalidTransition".to_string(),
                message: format!("Cannot transition from {} to ResourceAllocating", current_phase.phase_name()),
                phase: current_phase.phase_name().to_string(),
            });
        }
        
        // Get the operation from the current phase
        let operation = current_phase.operation().clone();
        
        // Create the new phase
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;
        
        let resource_allocating_phase = ScalingPhase::ResourceAllocating {
            operation,
            started_at: now,
            resources,
        };
        
        // Update the current phase
        self.current_phase = Some(resource_allocating_phase);
        
        // Update the operation history
        if let Some(record) = self.operation_history.last_mut() {
            record.final_phase = "ResourceAllocating".to_string();
        }
        
        Ok(())
    }
    
    /// Transitions to the InstancePreparing phase
    ///
    /// This method can only be called when in the ResourceAllocating phase.
    ///
    /// # Arguments
    ///
    /// * `instance_ids` - IDs of instances being affected
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the transition was successful
    /// * `Err(ScalingError)` if the transition failed
    pub fn transition_to_instance_preparing(&mut self, instance_ids: Vec<String>) -> Result<(), ScalingError> {
        let current_phase = match &self.current_phase {
            Some(phase) => phase,
            None => return Err(ScalingError {
                error_type: "NoActiveOperation".to_string(),
                message: "No active operation to transition".to_string(),
                phase: "None".to_string(),
            }),
        };
        
        // Ensure we're in the ResourceAllocating phase
        if !matches!(current_phase, ScalingPhase::ResourceAllocating { .. }) {
            return Err(ScalingError {
                error_type: "InvalidTransition".to_string(),
                message: format!("Cannot transition from {} to InstancePreparing", current_phase.phase_name()),
                phase: current_phase.phase_name().to_string(),
            });
        }
        
        // Get the operation from the current phase
        let operation = current_phase.operation().clone();
        
        // Create the new phase
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;
        
        let instance_preparing_phase = ScalingPhase::InstancePreparing {
            operation,
            started_at: now,
            instance_ids,
        };
        
        // Update the current phase
        self.current_phase = Some(instance_preparing_phase);
        
        // Update the operation history
        if let Some(record) = self.operation_history.last_mut() {
            record.final_phase = "InstancePreparing".to_string();
        }
        
        Ok(())
    }
    
    /// Transitions to the Configuring phase
    ///
    /// This method can only be called when in the InstancePreparing phase.
    ///
    /// # Arguments
    ///
    /// * `previous_config` - Optional previous configuration (for rollback if needed)
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the transition was successful
    /// * `Err(ScalingError)` if the transition failed
    pub fn transition_to_configuring(&mut self, previous_config: Option<String>) -> Result<(), ScalingError> {
        let current_phase = match &self.current_phase {
            Some(phase) => phase,
            None => return Err(ScalingError {
                error_type: "NoActiveOperation".to_string(),
                message: "No active operation to transition".to_string(),
                phase: "None".to_string(),
            }),
        };
        
        // Ensure we're in the InstancePreparing phase
        if !matches!(current_phase, ScalingPhase::InstancePreparing { .. }) {
            return Err(ScalingError {
                error_type: "InvalidTransition".to_string(),
                message: format!("Cannot transition from {} to Configuring", current_phase.phase_name()),
                phase: current_phase.phase_name().to_string(),
            });
        }
        
        // Get the operation from the current phase
        let operation = current_phase.operation().clone();
        
        // Create the new phase
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;
        
        let configuring_phase = ScalingPhase::Configuring {
            operation,
            started_at: now,
            previous_config,
        };
        
        // Update the current phase
        self.current_phase = Some(configuring_phase);
        
        // Update the operation history
        if let Some(record) = self.operation_history.last_mut() {
            record.final_phase = "Configuring".to_string();
        }
        
        Ok(())
    }
    
    /// Transitions to the Verifying phase
    ///
    /// This method can only be called when in the Configuring phase.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the transition was successful
    /// * `Err(ScalingError)` if the transition failed
    pub fn transition_to_verifying(&mut self) -> Result<(), ScalingError> {
        let current_phase = match &self.current_phase {
            Some(phase) => phase,
            None => return Err(ScalingError {
                error_type: "NoActiveOperation".to_string(),
                message: "No active operation to transition".to_string(),
                phase: "None".to_string(),
            }),
        };
        
        // Ensure we're in the Configuring phase
        if !matches!(current_phase, ScalingPhase::Configuring { .. }) {
            return Err(ScalingError {
                error_type: "InvalidTransition".to_string(),
                message: format!("Cannot transition from {} to Verifying", current_phase.phase_name()),
                phase: current_phase.phase_name().to_string(),
            });
        }
        
        // Get the operation from the current phase
        let operation = current_phase.operation().clone();
        
        // Create the new phase
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;
        
        let verifying_phase = ScalingPhase::Verifying {
            operation,
            started_at: now,
            test_results: Vec::new(),
        };
        
        // Update the current phase
        self.current_phase = Some(verifying_phase);
        
        // Update the operation history
        if let Some(record) = self.operation_history.last_mut() {
            record.final_phase = "Verifying".to_string();
        }
        
        Ok(())
    }
    
    /// Adds a verification test result to the current Verifying phase
    ///
    /// # Arguments
    ///
    /// * `test_result` - The verification test result to add
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the test result was added successfully
    /// * `Err(ScalingError)` if the operation is not in the Verifying phase
    pub fn add_verification_result(&mut self, test_result: VerificationResult) -> Result<(), ScalingError> {
        let current_phase = match &mut self.current_phase {
            Some(ScalingPhase::Verifying { test_results, .. }) => {
                test_results.push(test_result);
                return Ok(());
            },
            Some(phase) => phase,
            None => return Err(ScalingError {
                error_type: "NoActiveOperation".to_string(),
                message: "No active operation to add verification result to".to_string(),
                phase: "None".to_string(),
            }),
        };
        
        Err(ScalingError {
            error_type: "InvalidPhase".to_string(),
            message: format!("Cannot add verification result in {} phase", current_phase.phase_name()),
            phase: current_phase.phase_name().to_string(),
        })
    }
    
    /// Transitions to the Finalizing phase
    ///
    /// This method can only be called when in the Verifying phase.
    ///
    /// # Arguments
    ///
    /// * `cleanup_tasks` - Tasks that need to be completed
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the transition was successful
    /// * `Err(ScalingError)` if the transition failed
    pub fn transition_to_finalizing(&mut self, cleanup_tasks: Vec<String>) -> Result<(), ScalingError> {
        let current_phase = match &self.current_phase {
            Some(phase) => phase,
            None => return Err(ScalingError {
                error_type: "NoActiveOperation".to_string(),
                message: "No active operation to transition".to_string(),
                phase: "None".to_string(),
            }),
        };
        
        // Ensure we're in the Verifying phase
        if !matches!(current_phase, ScalingPhase::Verifying { .. }) {
            return Err(ScalingError {
                error_type: "InvalidTransition".to_string(),
                message: format!("Cannot transition from {} to Finalizing", current_phase.phase_name()),
                phase: current_phase.phase_name().to_string(),
            });
        }
        
        // Get the operation from the current phase
        let operation = current_phase.operation().clone();
        
        // Create the new phase
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;
        
        let finalizing_phase = ScalingPhase::Finalizing {
            operation,
            started_at: now,
            cleanup_tasks,
        };
        
        // Update the current phase
        self.current_phase = Some(finalizing_phase);
        
        // Update the operation history
        if let Some(record) = self.operation_history.last_mut() {
            record.final_phase = "Finalizing".to_string();
        }
        
        Ok(())
    }
    
    /// Checks for phase timeouts and transitions to Failed if a timeout is detected
    ///
    /// # Returns
    ///
    /// * `true` if a timeout was detected and handled
    /// * `false` otherwise
    pub fn check_timeouts(&mut self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;
        
        // Update the last check time
        self.last_check_time = now;
        
        // Check if there's an active operation
        let current_phase = match &self.current_phase {
            Some(phase) if !phase.is_terminal() => phase,
            _ => return false, // No active operation or already in a terminal state
        };
        
        // Get the start time of the current phase
        let start_time = match current_phase.start_time() {
            Some(time) => time,
            None => return false, // No start time (shouldn't happen for non-terminal states)
        };
        
        // Get the timeout for the current phase
        let timeout_seconds = match self.phase_timeouts.get(current_phase.phase_name()) {
            Some(timeout) => *timeout,
            None => return false, // No timeout defined for this phase
        };
        
        // Check if the phase has timed out
        if (now - start_time) > timeout_seconds as i64 {
            // Phase has timed out, transition to Failed
            let operation = current_phase.operation().clone();
            let phase_name = current_phase.phase_name().to_string();
            
            let failed_phase = ScalingPhase::Failed {
                operation,
                failed_at: now,
                failure_reason: format!("Timeout after {} seconds", timeout_seconds),
                failure_phase: phase_name.clone(),
                partial_results: None,
            };
            
            // Update the current phase
            self.current_phase = Some(failed_phase);
            
            // Update the operation history
            if let Some(record) = self.operation_history.last_mut() {
                record.final_phase = "Failed".to_string();
                record.ended_at = Some(now);
                record.successful = false;
                record.error = Some(ScalingError {
                    error_type: "Timeout".to_string(),
                    message: format!("Operation timed out in {} phase", phase_name),
                    phase: phase_name,
                });
            }
            
            return true;
        }
        
        false
    }
    
    /// Cancels the current scaling operation
    ///
    /// # Arguments
    ///
    /// * `reason` - The reason for cancellation
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the operation was canceled successfully
    /// * `Err(ScalingError)` if there was no active operation to cancel
    pub fn cancel_operation(&mut self, reason: &str) -> Result<(), ScalingError> {
        let current_phase = match &self.current_phase {
            Some(phase) if !phase.is_terminal() => phase,
            _ => return Err(ScalingError {
                error_type: "NoActiveOperation".to_string(),
                message: "No active operation to cancel".to_string(),
                phase: "None".to_string(),
            }),
        };
        
        let operation = current_phase.operation().clone();
        let phase_name = current_phase.phase_name().to_string();
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;
        
        let canceled_phase = ScalingPhase::Canceled {
            operation,
            canceled_at: now,
            cancellation_reason: reason.to_string(),
            phase_at_cancellation: phase_name,
        };
        
        // Update the current phase
        self.current_phase = Some(canceled_phase);
        
        // Update the operation history
        if let Some(record) = self.operation_history.last_mut() {
            record.final_phase = "Canceled".to_string();
            record.ended_at = Some(now);
            record.successful = false;
            record.error = None; // Cancellation is not an error
        }
        
        Ok(())
    }
    
    /// Marks the current operation as failed
    ///
    /// # Arguments
    ///
    /// * `error_type` - The type of error that occurred
    /// * `message` - A human-readable error message
    /// * `partial_results` - Optional partial results from the operation
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the operation was marked as failed successfully
    /// * `Err(ScalingError)` if there was no active operation to fail
    pub fn fail_operation(&mut self, error_type: &str, message: &str, partial_results: Option<String>) -> Result<(), ScalingError> {
        let current_phase = match &self.current_phase {
            Some(phase) if !phase.is_terminal() => phase,
            _ => return Err(ScalingError {
                error_type: "NoActiveOperation".to_string(),
                message: "No active operation to fail".to_string(),
                phase: "None".to_string(),
            }),
        };
        
        let operation = current_phase.operation().clone();
        let phase_name = current_phase.phase_name().to_string();
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;
        
        let failed_phase = ScalingPhase::Failed {
            operation,
            failed_at: now,
            failure_reason: message.to_string(),
            failure_phase: phase_name.clone(),
            partial_results,
        };
        
        // Update the current phase
        self.current_phase = Some(failed_phase);
        
        // Update the operation history
        if let Some(record) = self.operation_history.last_mut() {
            record.final_phase = "Failed".to_string();
            record.ended_at = Some(now);
            record.successful = false;
            record.error = Some(ScalingError {
                error_type: error_type.to_string(),
                message: message.to_string(),
                phase: phase_name,
            });
        }
        
        Ok(())
    }
    
    /// Marks the current operation as completed successfully
    ///
    /// # Arguments
    ///
    /// * `result_metrics` - Optional metrics collected after the operation
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the operation was marked as completed successfully
    /// * `Err(ScalingError)` if there was no active operation to complete
    pub fn complete_operation(&mut self, result_metrics: Option<ScalingMetrics>) -> Result<(), ScalingError> {
        let current_phase = match &self.current_phase {
            Some(phase) if !phase.is_terminal() => phase,
            _ => return Err(ScalingError {
                error_type: "NoActiveOperation".to_string(),
                message: "No active operation to complete".to_string(),
                phase: "None".to_string(),
            }),
        };
        
        let operation = current_phase.operation().clone();
        
        // Compute the duration of the operation
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;
        
        let started_at = match &self.operation_history.last() {
            Some(record) => record.started_at,
            None => now, // Shouldn't happen, but handle it gracefully
        };
        
        let duration_seconds = if now > started_at {
            (now - started_at) as u64
        } else {
            0
        };
        
        let completed_phase = ScalingPhase::Completed {
            operation,
            completed_at: now,
            duration_seconds,
            result_metrics,
        };
        
        // Update the current phase
        self.current_phase = Some(completed_phase);
        
        // Update the operation history
        if let Some(record) = self.operation_history.last_mut() {
            record.final_phase = "Completed".to_string();
            record.ended_at = Some(now);
            record.successful = true;
            record.error = None;
        }
        
        Ok(())
    }
}

impl ScalingPhase {
    /// Returns the type of scaling operation in this phase
    pub fn operation(&self) -> &ScalingOperation {
        match self {
            Self::Requested { operation, .. } => operation,
            Self::Validating { operation, .. } => operation,
            Self::Planning { operation, .. } => operation,
            Self::ResourceAllocating { operation, .. } => operation,
            Self::InstancePreparing { operation, .. } => operation,
            Self::Configuring { operation, .. } => operation,
            Self::Verifying { operation, .. } => operation,
            Self::Finalizing { operation, .. } => operation,
            Self::Completed { operation, .. } => operation,
            Self::Failed { operation, .. } => operation,
            Self::Canceled { operation, .. } => operation,
        }
    }

    /// Returns the start time of the current phase
    pub fn start_time(&self) -> Option<i64> {
        match self {
            Self::Requested { requested_at, .. } => Some(*requested_at),
            Self::Validating { started_at, .. } => Some(*started_at),
            Self::Planning { started_at, .. } => Some(*started_at),
            Self::ResourceAllocating { started_at, .. } => Some(*started_at),
            Self::InstancePreparing { started_at, .. } => Some(*started_at),
            Self::Configuring { started_at, .. } => Some(*started_at),
            Self::Verifying { started_at, .. } => Some(*started_at),
            Self::Finalizing { started_at, .. } => Some(*started_at),
            Self::Completed { .. } => None, // Terminal states don't have start times
            Self::Failed { .. } => None,
            Self::Canceled { .. } => None,
        }
    }

    /// Returns the name of the current phase as a string
    pub fn phase_name(&self) -> &'static str {
        match self {
            Self::Requested { .. } => "Requested",
            Self::Validating { .. } => "Validating",
            Self::Planning { .. } => "Planning",
            Self::ResourceAllocating { .. } => "ResourceAllocating",
            Self::InstancePreparing { .. } => "InstancePreparing",
            Self::Configuring { .. } => "Configuring",
            Self::Verifying { .. } => "Verifying",
            Self::Finalizing { .. } => "Finalizing",
            Self::Completed { .. } => "Completed",
            Self::Failed { .. } => "Failed",
            Self::Canceled { .. } => "Canceled",
        }
    }

    /// Checks if the phase is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed { .. } | Self::Failed { .. } | Self::Canceled { .. })
    }

    /// Creates a new Requested phase with the current timestamp
    pub fn new_requested(operation: ScalingOperation) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;
        
        Self::Requested {
            operation,
            requested_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_scaling_manager_basic_operations() {
        // Create a new ScalingManager
        let mut manager = ScalingManager::new();
        
        // Verify it starts with no active operations
        assert!(manager.current_phase().is_none());
        assert!(manager.operation_history().is_empty());
        
        // Start a new scale-out operation
        let operation = ScalingOperation::ScaleOut { target_instances: 5 };
        assert!(manager.start_operation(operation.clone()).is_ok());
        
        // Verify the operation was started correctly
        let phase = manager.current_phase().unwrap();
        match phase {
            ScalingPhase::Requested { operation: op, .. } => {
                match op {
                    ScalingOperation::ScaleOut { target_instances } => {
                        assert_eq!(*target_instances, 5);
                    },
                    _ => panic!("Wrong operation type"),
                }
            },
            _ => panic!("Wrong phase type"),
        }
        
        // Verify operation history was updated
        assert_eq!(manager.operation_history().len(), 1);
        assert_eq!(manager.operation_history()[0].final_phase, "Requested");
        assert!(!manager.operation_history()[0].successful);
        
        // Transition to Validating phase
        assert!(manager.transition_to_validating().is_ok());
        
        // Verify the transition was successful
        match manager.current_phase().unwrap() {
            ScalingPhase::Validating { .. } => {},
            _ => panic!("Failed to transition to Validating"),
        }
        
        // Verify operation history was updated
        assert_eq!(manager.operation_history()[0].final_phase, "Validating");
        
        // Test cancellation
        assert!(manager.cancel_operation("Testing cancellation").is_ok());
        
        // Verify the operation was canceled
        match manager.current_phase().unwrap() {
            ScalingPhase::Canceled { 
                cancellation_reason, 
                phase_at_cancellation, 
                .. 
            } => {
                assert_eq!(cancellation_reason, "Testing cancellation");
                assert_eq!(phase_at_cancellation, "Validating");
            },
            _ => panic!("Failed to cancel operation"),
        }
        
        // Verify operation history was updated
        assert_eq!(manager.operation_history()[0].final_phase, "Canceled");
        assert!(manager.operation_history()[0].ended_at.is_some());
        assert!(!manager.operation_history()[0].successful);
        assert!(manager.operation_history()[0].error.is_none());
    }
    
    #[test]
    fn test_timeout_detection() {
        // Create a manager with very short timeouts for testing
        let mut manager = ScalingManager::new();
        manager.set_phase_timeout("Requested", 1); // 1 second timeout
        
        // Start an operation
        let operation = ScalingOperation::ScaleOut { target_instances: 3 };
        assert!(manager.start_operation(operation).is_ok());
        
        // Sleep to trigger the timeout
        thread::sleep(std::time::Duration::from_secs(2));
        
        // Check timeouts
        assert!(manager.check_timeouts());
        
        // Verify the operation timed out
        match manager.current_phase().unwrap() {
            ScalingPhase::Failed { 
                failure_reason, 
                failure_phase, 
                .. 
            } => {
                assert!(failure_reason.contains("Timeout"));
                assert_eq!(failure_phase, "Requested");
            },
            _ => panic!("Failed to detect timeout"),
        }
        
        // Verify operation history was updated
        assert_eq!(manager.operation_history()[0].final_phase, "Failed");
        assert!(manager.operation_history()[0].error.is_some());
        assert_eq!(manager.operation_history()[0].error.as_ref().unwrap().error_type, "Timeout");
    }
} 