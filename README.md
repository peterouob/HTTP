# HTTP http server

- This is a sample and safety http server

## Goal

- [x] TCP Foundation & Concurrency
- [x] HTTP/1.1 Parser
  - [ ] Serialize & Deserialize
- [ ] Response
- [ ] Routing
    - [ ] radix tree
      - [x] insert
      - [ ] finding
- [ ] Connection Management

---

## Contents

1. [TCP Foundation & Concurrency](#tcp-foundation--concurrency)
2. [HTTP/1.1 Parser](#http-parser-function)
3. [Radix tree Router](#radix-tree-router)
4. [Reference](#reference)

---

# TCP Foundation & Concurrency

## Project Structure

```text
src/
├── parse/
│   ├── mod.rs             # the definition of parse module and re-export
│   ├── error.rs           # the definition of parse error
│   ├── iter.rs            # the definition of parse buffer (detailed control for buf: &[u8])
│   ├── macros.rs          # parse macros to reduce duplicate code in parser.rs
│   ├── parser.rs          # the core parser function implementation
│   ├── tchar.rs           # definition of http char map
│   ├── parse_utils.rs     # support function for parse
│   └── tests/
│       ├── mod.rs
│       ├── parse_request_test.rs   # full test for parse request
│       └── parse_response_test.rs  # full test for parse response
├── router/
│   ├── mod.rs             # the definition of router module and re-export
│   └── radix_tree.rs      # the definition of radix tree for router
├── main.rs                # server entry
├── lib.rs                 # definition module and re-export
├── server.rs              # Listener / Handle / setup_tcp core tcp server logic
├── connection.rs          # Connection struct read/write frame logic
├── shutdown.rs            # Shutdown struct for graceful shutdown logic
└── error.rs               # the definition of server error

```

---

# TCP Server Function

## Component Overview

```text
┌──────────────────────────────────────────────────────────────────────┐
│ main.rs                                                              │
│  └─ run(addr, ctrl_c)                                                │
│      │                                                               │
│      ├─ setup_tcp(addr)  ──▶ socket2::Socket                         │
│      │   (SO_REUSEADDR / TCP_NODELAY / non-blocking / listen(128))   │
│      │                                                               │
│      ├─ Listener { listener, limit_connections(4096), token, ... }   │
│      │                                                               │
│      └─ tokio::select!                                               │
│           ├─ Listener::run() ───────────────────────────────────┐    │
│           └─ shutdown signal ──▶ token.cancel()                 │    │
│                                  join_set.join_all() / 30s      │    │
└─────────────────────────────────────────────────────────────────┼────┘
                                                                  │
                   ┌──────────────────────────────────────────────┘
                   ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Listener::run() (accept loop)                                        │
│                                                                      │
│  ┌────────────────────────┐                                          │
│  │ Semaphore::acquire()   │ ← blocks at MAX_CONNECTIONS              │
│  └────────────────────────┘                                          │
│         │                                                            │
│  Listener::accept()                                                  │
│    └─ exponential backoff + jitter (1.1x, up to 120s, 5 retries)     │
│         │                                                            │
│  tokio::spawn ──▶ Handle::run() (per-connection task)                │
│                        │                                             │
│                   tokio::select!                                     │
│                        ├─ Connection::read_frame()                   │
│                        │   └─ TcpStream → BytesMut (4 KB buf)        │
│                        ├─ Connection::write_frame()                  │
│                        │   └─ BufWriter flush                        │
│                        └─ Shutdown::recv() (CancellationToken)       │
│                                                                      │
│         └─ drop(permit) ← release semaphore slot on task end         │
└──────────────────────────────────────────────────────────────────────┘

```

## Call Graph

```text
main()
 └─ run(addr, ctrl_c)
     ├─ setup_tcp()
     ├─ Listener::run()
     │   └─ loop
     │       ├─ Semaphore::acquire()
     │       ├─ Listener::accept() (jittered backoff)
     │       └─ tokio::spawn
     │           └─ Handle::run()
     │               ├─ Connection::read_frame()
     │               │   └─ parse Request / Response
     │               └─ Connection::write_frame()
     └─ Shutdown signal
         ├─ token.cancel()
         └─ join_set.join_all() (30s timeout)

```

---

# HTTP Parser Function

## Parser Pipeline

```text
Raw &[u8]
    │
    ▼
┌──────────────────────────────────────────────────────────────────┐
│ ParseBuffer (iter.rs)                                            │
│                                                                  │
│ buf: [ G ][ E ][ T ][ ][ / ][ ][ H ][ T ][ T ][ P ][/][1][.][1]  │
│       ↑                                                          │
│      start                                                       │
│       ↑                                                          │
│     cursor ── advance(n) ──▶ cursor += n                         │
│                                                                  │
│  peek()       → buf[cursor] (no move)                            │
│  next_byte()  → buf[cursor], cursor += 1                         │
│  slice()      → buf[start..cursor], start = cursor               │
│  sub_slice(n) → buf[start..cursor-n], start = cursor             │
└──────────────────────────────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────────────────────────────┐
│ parse_utils.rs + tchar.rs + macros.rs                            │
│                                                                  │
│ next!(buf)         ── advance 1 byte, return it                  │
│ expect!(cond=>err) ── advance 1 byte, error if cond fails        │
│ space!(buf)        ── expect a space byte                        │
│ newline!(buf)      ── consume \r\n or \n                         │
│ complete!(expr)    ── unwrap Status::Complete, return Partial    │
│                                                                  │
│ parse_method()      → "GET" | "POST" | token (fast path 4-byte)  │
│ parse_uri()         → "/path?query"                              │
│ parse_version()     → 0 (HTTP/1.0) | 1 (HTTP/1.1) (8-byte cmp)   │
│ parse_status_code() → u16 (3-digit ASCII)                        │
│ parse_reason()      → &str reason phrase                         │
└──────────────────────────────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────────────────────────────┐
│ parser.rs                                                        │
│                                                                  │
│ Request::parse_header(&[u8])                                     │
│   skip_empty_line → parse_method → parse_uri → parse_version     │
│   → \r\n → parse_header_iter (loop until \r\n\r\n)               │
│   → Status::Complete(()) | Status::Partial | ParseError          │
│                                                                  │
│ Response::parse_header(&[u8])                                    │
│   skip_empty_line → parse_version → parse_status_code            │
│   → reason phrase → parse_header_iter                            │
│   → Status::Complete(()) | Status::Partial | ParseError          │
└──────────────────────────────────────────────────────────────────┘
    │
    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ HTTP Messages                                                          │
│                                                                        │
│  ┌─────────────────────────┐   ┌────────────────────────────────────┐  │
│  │ Request<'h, 'b>         │   │ Response<'h, 'b>                   │  │
│  │ ─────────────────────── │   │ ────────────────────────────────── │  │
│  │ method:  &str           │   │ version:     u8                    │  │
│  │ path:    &str           │   │ status_code: u16                   │  │
│  │ version: u8             │   │ reason:      &str                  │  │
│  │ headers: &HeaderMap     │   │ headers:     &HeaderMap            │  │
│  └─────────────────────────┘   └────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

## HTTP Message Format

```text
Request                               Response
───────────────────────────────       ───────────────────────────────
GET /index.html HTTP/1.1              HTTP/1.1 200 OK
Host: example.com                     Content-Type: text/html
Accept: text/html                     Content-Length: 42

│method│ │uri│  │version│             │version│ │code│ │reason│
└──────┘ └───┘  └───────┘             └───────┘ └────┘ └──────┘
start-line (\r\n)                     status-line (\r\n)
│ header-name │:│ header-value │\r\n  (same header format)
...                                   ...
\r\n  ← end of headers (CRLF)         \r\n
[body]                                [body]

```

---

# Radix Tree Router

[visualization](https://www.cs.usfca.edu/~galles/visualization/RadixTree.html)

## Data Structure

```text
RadixTree<T>
 └─ root: Node<T>
     ├─ prefix:    &[u8]
     ├─ leaf_node: Option<LeafNode<T>>  ← Some if this path has a value
     └─ edges:     Vec<Edge<T>>         (sorted by label for binary search)
                    └─ Edge
                        ├─ label: &[u8] ← first byte used for lookup
                        └─ node:  Node<T>

```

## Insert Easy Example:

1. Insert "app" with value `one`
2. Insert "apple" with value `two`
3. Insert "application" with value `three` → causes split at `l`

```text
Step 1: insert(b"app", "one")
  root
   └──[app]──● "one"

Step 2: insert(b"apple", "two")
  lcp("app", "apple") = 3 = len("app") → recurse with b"le"
  root
   └──[app]──● "one"
         └──[le]──● "two"

Step 3: insert(b"application", "three")
  lcp("app", "application") = 3 → recurse with b"lication"
  lcp("le", "lication") = 1     → SPLIT at b"l"
  root
   └──[app]──● "one"
         └──[l]── (no value)
             ├──[e]───────● "two"     ← old "le" node, prefix trimmed
             └──[ication]──● "three"

```

## Longest Common Prefix

```text
k1: [ a ][ p ][ p ][ l ][ e ]
k2: [ a ][ p ][ p ][ l ][ i ][ c ][ a ][ t ][ i ][ o ][ n ]
      ✓    ✓    ✓    ✓    ✗
                          └─ stop → lcp = 4

```

# Reference

- [httparse](https://docs.rs/httparse/latest/httparse/)
- [graceful-shutdown](https://hyper.rs/guides/1/server/graceful-shutdown/)
- [radix-tree](https://xixiliguo.github.io/algorithm/radix-tree)
- [go-radix-tree](https://github.com/armon/go-radix)
