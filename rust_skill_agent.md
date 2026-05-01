# Rust System Development Agent Rules

## 1. Core Philosophy

- Prefer correctness over cleverness
- Prefer explicit over implicit
- Avoid unnecessary abstraction
- Optimize only after correctness

Rust code MUST:
- Be memory safe
- Avoid unnecessary cloning
- Follow ownership principles strictly
- Compile without warnings

---

## 2. Project Context

This project builds a high-performance, Redis-like in-memory server.

Key characteristics:
- Async networking (Tokio)
- High concurrency
- Low latency
- Binary-safe data handling

---

## 3. Code Style Rules

### Naming
- snake_case → variables, functions
- PascalCase → structs, enums
- SCREAMING_SNAKE_CASE → constants

### File Organization
- One responsibility per module
- Avoid large files (>300 lines)
- Use mod.rs or flat modules consistently

---

## 4. Error Handling

- NEVER use unwrap() or expect() in production code
- Always return Result<T, E>
- Use `thiserror` for custom errors

Example: fn process() -> Result<(), AppError>

- Use `?` operator for propagation
- Fail fast on invalid input

---

## 5. Async Rules (Tokio)

Use:
- `#[tokio::main]` for entry
- `tokio::spawn` for concurrency
- `async fn` for I/O operations

Avoid:
- Blocking operations inside async
- Long CPU tasks inside async threads

Important:
- Async is for I/O, NOT CPU work
- Use `spawn_blocking` for heavy tasks

Tokio enables efficient concurrency using async I/O and scheduling, allowing many tasks without blocking threads. :contentReference[oaicite:0]{index=0}

---

## 6. Concurrency & Shared State

Use:
- Arc<T> for shared ownership
- Mutex / RwLock for shared mutable state

Rules:
- Keep lock scope minimal
- Avoid nested locks
- Avoid deadlocks

Prefer:
- Read-heavy → RwLock
- Write-heavy → Mutex

---

## 7. Data Handling

- Use Vec<u8> for binary-safe storage
- Avoid String unless necessary
- Minimize allocations

Avoid:
- Repeated cloning
- Large copies

---

## 8. Networking Rules

- Always handle partial reads/writes
- Use buffered reads where possible
- Never assume full message arrives at once

Buffering improves performance by reducing syscalls and overhead. :contentReference[oaicite:1]{index=1}

---

## 9. Performance Rules

- Avoid unnecessary heap allocations
- Avoid blocking threads
- Use zero-copy where possible
- Keep hot paths minimal

Critical paths:
- Parsing
- HashMap access
- Lock contention

---

## 10. Command Handling Pattern

Always follow:

Connection → Parse → Command → Execute → Response

DO NOT mix layers.

---

## 11. Module Responsibilities

server/ → TCP listener  
connection/ → socket read/write  
protocol/ → parsing  
command/ → command mapping  
engine/ → execution logic  
storage/ → data store  

Each module MUST be independent.

---

## 12. Testing Rules

- Unit test parsing and logic
- Integration test full flow

Use: #[tokio::test]


---

## 13. Anti-Patterns (STRICTLY FORBIDDEN)

- unwrap() in production
- blocking inside async
- mixing parsing + execution
- global mutable state
- large monolithic functions

---

## 14. Code Generation Guidelines (IMPORTANT FOR AI)

When generating code:
- Always include full working snippets
- Avoid pseudo-code
- Prefer minimal but complete implementation
- Include error handling
- Use idiomatic Rust patterns

---

## 15. Incremental Development Strategy

Follow strict phases:

1. TCP server
2. Connection handling
3. Protocol parsing
4. Command execution
5. Storage engine
6. TTL
7. Persistence

Never skip layers.

---

## 16. Documentation Rules

- Document public functions
- Explain WHY, not WHAT
- Keep comments concise

---

## 17. Safety & Reliability

Rust provides memory safety without garbage collection and ensures predictable performance. :contentReference[oaicite:2]{index=2}

Always:
- Handle edge cases
- Validate inputs
- Avoid panics

---

## 18. AI Behavior Constraints

AI MUST:
- Ask for clarification if requirements are unclear
- Not invent APIs
- Not hallucinate crates
- Stick to stable Rust ecosystem

Preferred crates:
- tokio
- bytes
- thiserror

---

## 19. Future Optimization Awareness

Code should be written so it can later support:
- Lock-free structures
- Sharding
- Event-loop model (Redis-style)

---

## 20. Golden Rule

Write code like it will handle:
- 10,000 concurrent connections
- Production load
- Real-world failures
