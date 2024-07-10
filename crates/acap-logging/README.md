_Logging utilities for ACAP applications_

## Example

The logger is initialized as early as possible:

```no_run
use log::{error, warn};

fn main() {
    # std::env::set_var("RUST_LOG_STYLE", "always");
    error!("This will never be shown");
    acap_logging::init_logger();
    error!("This will usually be shown");
}
```

## Pitfalls

- Messages logged at the `trace` level will not be shown in the system logs on target.
- Messages logged at the `warn` level or less severe will not be shown in terminals by default.
- When the `tracing` crate is used in place of the `log` crate, its `log` feature must be enabled.
