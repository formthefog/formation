# Rust Compilation Errors Task List

## Category 1: Missing Trait Implementations

### 1.1 `PartialEq` Trait Missing

These structs need to have `#[derive(PartialEq)]` added:

- [x] **Task 1.1.1:** Add `PartialEq` to `StripeSubscription` in `form-state/src/billing/stripe.rs`
- [x] **Task 1.1.2:** Add `PartialEq` to `EarningTransaction` in `form-state/src/billing/stripe.rs`
- [x] **Task 1.1.3:** Add `PartialEq` to `BillingEvent` in `form-state/src/billing/db.rs`
- [x] **Task 1.1.4:** Add `PartialEq` to `TokenUsage` in `form-state/src/billing/usage.rs` 
- [x] **Task 1.1.5:** Add `PartialEq` to `AgentHire` in `form-state/src/billing/usage.rs`

### 1.2 `AsRef<[u8]>` and `Sha3Hash` Missing

These structs need implementations of `AsRef<[u8]>` or the `Sha3Hash` trait:

- [x] **Task 1.2.1:** Implement `AsRef<[u8]>` for `StripeSubscription`
- [x] **Task 1.2.2:** Implement `AsRef<[u8]>` for `EarningTransaction`
- [x] **Task 1.2.3:** Fix implementations for `TokenUsage` and `AgentHire` to avoid returning references to temporary values

## Category 2: `Debug` Trait Missing

- [x] **Task 2.1:** Add `#[derive(Debug)]` to `PlanRegistry` in `form-state/src/billing/plans.rs`

## Category 3: Enum Variant Issues in `StripeError`

The `StripeError` enum in `stripe.rs` is missing some variants that are used in `stripe_client.rs`:

- [x] **Task 3.1:** Add the `Configuration` variant to `StripeError`
- [x] **Task 3.2:** Add the `ClientCreation` variant to `StripeError`
- [x] **Task 3.3:** Add the `Parsing` variant to `StripeError`

## Category 4: Moved Value Issues

- [x] **Task 4.1:** Fix moved value issue for `node_id` and `pk` in `datastore.rs` by using `clone()`
- [x] **Task 4.2:** Fix moved value issue for `status` in `update_status` method in `stripe.rs`

## Category 5: Field Access Errors

- [x] **Task 5.1:** Fix field access in `webhook.rs` where it tries to access a non-existent `accounts` field on `DataStore`

## Category 6: Temporary Value References

There are issues where functions return references to temporary values:

- [x] **Task 6.1:** Fix `as_ref` implementations in `billing/db.rs` for `TokenUsage` and `AgentHire` to avoid returning references to temporary formatted strings.

## Category 7: Missing Trait Implementations for Enums

Several enums need additional trait implementations:

- [x] **Task 7.1:** Add `PartialOrd`, `Ord`, and `Hash` traits to `SubscriptionStatus` enum in `billing/plans.rs`
- [x] **Task 7.2:** Add `PartialOrd`, `Ord`, and `Hash` traits to `AccountType` enum in `billing/plans.rs`

## Category 8: Mutex Lock `.await` Issues

There are errors with awaiting mutex locks that aren't async:

- [x] **Task 8.1:** Remove `.await` from calls to `lock()` on regular `Mutex` (not Tokio mutex) in `billing/token_tracking.rs` and other files

## Category 9: Struct Field Errors in `QuotaCheck` 

The `QuotaCheck` struct is being initialized with fields that don't exist:

- [x] **Task 9.1:** Fix field initialization in `QuotaCheck` in `billing/quota.rs` (resource_type, account_id, available, etc.)
- [x] **Task 9.2:** Fix field access to use correct field names in `QuotaCheck` (is_allowed -> allowed, etc.)

## Category 10: Missing Clone and Debug Implementations 

- [x] **Task 10.1:** Add `#[derive(Clone)]` to `AuthService` 
- [x] **Task 10.2:** Add `#[derive(Debug)]` to `AuthService`
- [x] **Task 10.3:** Add `Clone` trait implementation for `BillingService`

## Category 11: Type Mismatch Errors

- [x] **Task 11.1:** Fix mismatched types in `EarningTransaction::new()` (f64 vs u32)
- [x] **Task 11.2:** Fix Router return type in `app()` function
- [x] **Task 11.3:** Fix `bool` dereference error in `helpers/instances.rs`

## Category 12: Scope Enum Variant Errors

- [x] **Task 12.1:** Add missing variants (`Create`, `Update`, `Delete`) to `permissions::Scope` enum or fix usage in `api.rs`

## Category 13: Claims Struct Field Errors

- [x] **Task 13.1:** Fix `Claims` struct field errors in `auth/jwt.rs` (`jti` and `nbf` fields)
- [x] **Task 13.2:** Fix `validate_iss` field usage on `jsonwebtoken::Validation`

## Category 14: Move Out of Borrowed Value

- [x] **Task 14.1:** Fix move out of borrowed `inner` in `auth/token_cache.rs`

## Category 15: MutexGuard Type Mismatches

There are issues with using std::sync::Mutex instead of tokio::sync::Mutex in an async context:

- [ ] **Task 15.1:** Fix instances where `std::sync::MutexGuard` is used with `.await` - which is invalid as it's not a Future
- [x] **Task 15.2:** Ensure `Arc<Mutex<DataStore>>` is consistently using `tokio::sync::Mutex` throughout the codebase
- [ ] **Task 15.3:** Update method signatures to use the correct MutexGuard type (tokio::sync::MutexGuard)
- [ ] **Task 15.4:** Fix pattern matching in functions that expect tuple variants but are receiving struct variants

The key issues are:
1. DataStore should be wrapped in tokio::sync::Mutex, not std::sync::Mutex
2. Many functions are calling `.await` on the result of `.lock()` which is only valid for tokio Mutex

## Category 16: Missing DataStore Field Access Methods

- [x] **Task 16.1:** Add missing methods for accessing DataStore components (get_account, billing_service, etc.)
- [x] **Task 16.2:** Fix field access in various modules to use appropriate accessor methods

## Category 17: UsageReport Struct Field Mismatches

- [x] **Task 17.1:** Fix field mismatches in `UsageReport` struct (missing total_cost, token_records, etc.)
- [x] **Task 17.2:** Update code to use the correct fields that exist in `UsageReport`

## Category 18: Missing SubscriptionStatus Enum Variants

- [x] **Task 18.1:** Add missing `Unpaid` variant to `SubscriptionStatus` enum or fix usage

## Category 19: Account Struct Issues with f64 Fields

- [x] **Task 19.1:** Fix Eq/Ord/Hash trait issues with f64 fields in Account struct - FIXED
- [x] **Task 19.2:** Add missing field `developer_earnings_balance` and `lifetime_earnings` to Account struct - FIXED
- [x] **Task 19.3:** Fix all references to these missing fields in the codebase - FIXED

## Category 20: JWT and Auth Service Issues

- [x] **Task 20.1:** Add missing fields to `Claims` struct initialization - FIXED
- [x] **Task 20.2:** Implement `Debug` for `ThreadSafeTokenCache` 
- [x] **Task 20.3:** Fix JwtConfig parameter in AuthService::new()
- [x] **Task 20.4:** Fix validation field name in JWT validation - FIXED

## Category 21: InstanceMetadata Issues

- [ ] **Task 21.1:** Fix `get` method usage on `InstanceMetadata`

## Category 22: Await Missing for Async Functions

- [ ] **Task 22.1:** Add `.await` to async DataStore methods in `subscription_service.rs`
- [ ] **Task 22.2:** Add `.await` to async DataStore methods in `agent_tracking.rs`
- [ ] **Task 22.3:** Add `.await` to async DataStore methods in `token_tracking.rs`
- [ ] **Task 22.4:** Add `.await` to `DataStore::write_to_queue` calls throughout the codebase

## Category 23: Missing Methods in BillingDb

- [ ] **Task 23.1:** Implement or fix calls to `record_subscription_created` in BillingDb
- [ ] **Task 23.2:** Implement or fix calls to `record_subscription_canceled` in BillingDb  
- [ ] **Task 23.3:** Implement or fix calls to `record_subscription_updated` in BillingDb
- [ ] **Task 23.4:** Implement or fix calls to `record_agent_hired` in BillingDb
- [ ] **Task 23.5:** Implement or fix calls to `record_agent_released` in BillingDb
- [ ] **Task 23.6:** Implement or fix calls to `record_token_usage` in BillingDb
- [ ] **Task 23.7:** Implement or fix calls to `record_token_credits_added` in BillingDb
- [ ] **Task 23.8:** Fix `get_token_usage_in_range` method name (should be `get_token_usage_for_account_in_range`)
- [ ] **Task 23.9:** Fix `get_agent_hires_in_range` method name (should be `get_agent_hires_for_account_in_range`)

## Category 24: StripeClient Missing Methods

- [ ] **Task 24.1:** Fix or implement `cancel_subscription_immediately` in StripeClient
- [ ] **Task 24.2:** Fix or implement `cancel_subscription_at_period_end` in StripeClient
- [ ] **Task 24.3:** Fix or implement `customer_has_payment_method` in StripeClient
- [ ] **Task 24.4:** Fix or implement `update_subscription_plan` in StripeClient

## Category 25: Method Parameter Type Issues

- [ ] **Task 25.1:** Fix `get_agent` in DataStore to use `&str` instead of `&String`
- [ ] **Task 25.2:** Fix field access issue in agent_tracking.rs where it tries to access `.id` on a tuple

## Category 26: Subscription Type Conversion Issues

- [ ] **Task 26.1:** Implement `From<StripeSubscription>` for `accounts::SubscriptionInfo`
- [ ] **Task 26.2:** Fix type mismatches between `accounts::SubscriptionInfo` and `billing::subscription_service::SubscriptionInfo`
- [ ] **Task 26.3:** Fix `billing_status` type mismatch (String vs BillingStatus)

## Category 27: Miscellaneous Field Access Errors

- [ ] **Task 27.1:** Fix field access in `earnings.rs` where it tries to access `agent_map` on DataStore
- [ ] **Task 27.2:** Fix `unwrap_or` on u8 in `earnings.rs` (should use a different approach)
- [ ] **Task 27.3:** Fix `agent_tracking.rs` where it tries to access `.id` on a tuple
- [ ] **Task 27.4:** Fix field access in `token_tracking.rs` where it tries to access non-existent field `input_tokens`

## Approach

Let's continue addressing these issues in a methodical way:

1. First, let's focus on fixing the Account struct issues (Category 19) since many other errors depend on this.
2. Then tackle the `.await` missing issues (Category 22) to fix async function calls.
3. Address the method parameter type issues (Category 25) which are simple fixes.
4. Implement the missing methods in BillingDb (Category 23).
5. Fix the StripeClient missing methods (Category 24).
6. Finally, address the remaining miscellaneous field access errors (Category 27).

This approach should resolve the most critical errors first and make it easier to fix the remaining ones. 