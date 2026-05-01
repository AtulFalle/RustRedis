# Rust Development Skill

This document defines the core competencies and standards for developing the Redis implementation in Rust.

## Core Capabilities
- **High-Performance Networking**: Utilizing Tokio for asynchronous I/O.
- **RESP Protocol Implementation**: Efficient parsing and serialization of the Redis Serialization Protocol.
- **Concurrent Data Structures**: Thread-safe storage engines using atomics, Mutexes, and RwLocks.
- **Memory Management**: Leveraging Rust's ownership model for zero-copy operations where possible.

## Project Vision
To build a highly efficient, thread-safe, and modular Redis-compatible server that demonstrates the power of Rust's systems programming capabilities.

## Technical Stack
- **Language**: Rust (Stable)
- **Runtime**: Tokio
- **Serialization**: Custom RESP Parser
- **Testing**: `cargo test` and integration tests for Redis compatibility.
