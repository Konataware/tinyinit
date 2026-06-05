# TinyInit

A minimal init system written in Rust, heavily based on the design of [tini](https://github.com/krallin/tini).  
It runs as PID 1 inside a container (or on bare metal), spawns a single child process, reaps zombie processes, and forwards signals.

**Status:** ✅ Working prototype (fork + exec + SIGCHLD reaping).  
**Project deadline:** 2026-06-16

---

## What it does (current version)

- Runs as PID 1
- Forks and executes a child process (default: `/bin/sh`)
- Handles `SIGCHLD` with a proper signal handler
- Reaps all zombie children using `waitpid(-1, WNOHANG)`
- Parent process waits in `pause()` until the main child exits

---

## Building

```bash
cargo build --release
```

## Usage

1. Running inside a container:
```
docker run --rm -it --privileged -v $(pwd):/app -w /app rust:latest \
  ./target/release/tinyinit
```

Or copy the binary to a root filesystem and boot with `init=/path/to/tinyinit` (bare metal / VM).

## Next steps (roadmap to 2026‑06‑16)

- **Forward signals** to the child – `SIGTERM`, `SIGINT`, etc.
- **Proper exit code** – return the child's exit code to the parent
- **Terminal handling** – `setsid()`, `tcsetpgrp()` to fix job control
- **Command‑line arguments** – accept custom program instead of hardcoded `/bin/sh`
- **Graceful shutdown** – handle `SIGTERM` to kill the child and exit cleanly

## Attribution & License

This project is a **reimplementation** of [tini](https://github.com/krallin/tini) to Rust for educational purposes.

Original tini is copyright (c) krallin and contributors, licensed under the **MIT License**.
This Rust version follows the same MIT license terms.

The `LICENSE` file in this repository is the original MIT license from tini.

If you intend to use this code in production, please refer to the original, more complete tini project. This is purely made for educational purposes.

## Why this exists

Understanding how a minimal init works is the foundation of container runtimes (Docker, podman) and embedded Linux. This project demonstrates:

- Linux syscalls: `fork`, `execvp`, `waitpid`, `sigaction`, `pause`
- Signal handling in Rust using the `nix` crate
- The core responsibilities of PID 1 (reaping zombies, forwarding signals)

## References

- [tini – a tiny but valid init for containers](https://github.com/krallin/tini)
- [nix crate documentation](https://docs.rs/nix/)
- `man 2 fork`, `man 2 execve`, `man 2 waitpid`, `man 2 sigaction`

