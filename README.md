# http

## Goal


- [x] TCP Foundation & Concurrency
- [ ] HTTP/1.1 Parser
- [ ] Response & Routing
- [ ] Connection Management
- [ ] Observability & Hardening
- [ ] Load Balancing & HA
- [ ] Production Deployment & Postmortem

# TCP Foundation & Concurrency

## TCP Foundation & Concurrency 專案結構

```
src/
├── main.rs        # 程式進入點，建立 runtime 並啟動 server
├── lib.rs         # 模組宣告與 re-export
├── server.rs      # Listener / Handle / setup_tcp，核心接受連線邏輯
├── connection.rs  # Connection struct，讀寫 TCP frame
├── shutdown.rs    # Shutdown struct，封裝 CancellationToken
└── error.rs       # 自訂錯誤型別（TCPSocketError）
```

## 架構說明

```
main()
 └─ run(addr, ctrl_c)
     ├─ setup_tcp()          — 用 socket2 建立並設定 TCP socket
     ├─ Listener::run()      — accept loop，每個連線取得 Semaphore permit
     │    └─ tokio::spawn    — 每個連線獨立 task
     │         └─ Handle::run()
     │              ├─ Connection::read_frame()   — 讀取請求
     │              └─ Connection::write_frame()  — 寫入回應
     └─ Shutdown signal
          ├─ token.cancel()               — 廣播關機訊號給所有 task
          └─ join_set.join_all() / 30s timeout
```
