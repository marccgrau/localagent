# LocalGPT Sandbox

Low-level OS-specific sandboxing implementation for secure execution of agent-initiated shell commands. This crate ensures that LocalGPT can run tasks while strictly confining filesystem and network access.

## Features

- **Multi-Platform Support**: OS-specific isolation (Landlock on Linux, Seatbelt on macOS).
- **Kernel-Enforced Security**: Uses native OS kernel features to enforce restrictions.
- **Configurable Policies**: Support for fine-grained access control to directories and resources.
- **Process Isolation**: Securely executes subprocesses with restricted privileges.
