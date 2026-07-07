// Functional tests for the generated cross-cutting long-operation commands
// (frontend::commands::long_operation_commands).
//
// These commands are feature-agnostic: they only take the opaque operation id.
// Against a fresh AppContext (no operations started) the generic surface must
// behave gracefully — unknown ids return None/false, listings are empty, and
// cleanup is a no-op. This smoke-tests that the generated module is importable
// and that every wrapper correctly delegates to the shared LongOperationManager.

use frontend::AppContext;
use frontend::commands::long_operation_commands as lo;

#[test]
fn test_generic_surface_on_empty_context() {
    let ctx = AppContext::new();

    // Unknown operation id -> graceful "not found" everywhere.
    assert!(!lo::cancel_operation(&ctx, "op_does_not_exist"));
    assert!(lo::get_operation_status(&ctx, "op_does_not_exist").is_none());
    assert!(lo::get_operation_progress(&ctx, "op_does_not_exist").is_none());
    assert!(lo::is_operation_finished(&ctx, "op_does_not_exist").is_none());
    assert!(lo::get_operation_result(&ctx, "op_does_not_exist").is_none());

    // No operations tracked yet.
    assert!(lo::list_operations(&ctx).is_empty());
    assert!(lo::get_operations_summary(&ctx).is_empty());

    // Cleanup on an empty manager must not panic and must stay empty.
    lo::cleanup_finished_operations(&ctx);
    assert!(lo::list_operations(&ctx).is_empty());

    ctx.shutdown();
}
