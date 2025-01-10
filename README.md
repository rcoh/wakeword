# Runtime Task Context Validator

A quick experimental utility that detects and reports when async tasks are executed in a different runtime context than where they were created.

## Purpose

This experimental tool helps debug scenarios where tasks might inadvertently cross runtime boundaries by:
- Detecting when a task is woken up in a different runtime than where it was created
- Generating and dumping a backtrace when such cross-runtime task execution occurs

## Usage

This is a development/debugging tool designed to help identify potential issues in async code where tasks may unintentionally cross runtime boundaries.

To use this tool, wrap your futures with the `instrument` function, passing the runtime handle:

```rust
use tokio::runtime::Handle;
use runtime_task_context_validator::instrument;

let handle = Handle::current();
let my_future = async {
    // Your async code here
};

// Wrap the future to detect cross-runtime wake ups
let instrumented = instrument(&handle, my_future);
tokio::spawn(instrumented);
```

When a task is woken up in a different runtime context than its creation context, a backtrace will be generated showing where the cross-runtime execution occurred.

## Output

When a cross-runtime task execution is detected, a backtrace will be printed to stderr in the following format:

```
WARNING: Task woken in different runtime than created in!
Backtrace:
   0: runtime_task_context_validator::WakeWarner::wake_by_ref
   1: <your code's stack trace here>
   ...
```

## Notes

- This is a quick experimental project intended for development debugging purposes only
- Performance impact: This adds a small overhead to task wake-ups to perform the runtime context check
- Thread-safe: The validator is safe to use in multi-threaded environments