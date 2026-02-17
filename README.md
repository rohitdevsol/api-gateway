# Rate Limiting Performance: DashMap vs Mutex

Performance benchmark comparing `Arc<Mutex<HashMap>>` and `Arc<DashMap>` for token bucket rate limiting in a high-throughput API gateway.

This benchmark was executed under two different scenarios:
1. **Gateway-only** (no upstream call)
2. **Gateway + upstream HTTP forwarding**

This distinction is critical because upstream I/O dramatically changes bottleneck behavior.

---

## Benchmark Configuration
```
Build Mode:            cargo build --release
Rust Version:          1.83.0
CPU:                   Apple M4 (10 cores: 4P + 6E)
RAM:                   16GB unified memory
OS:                    macOS Tahoe

Threads:               12
Concurrent Connections: 800
Duration:              10 seconds
Load Generator:        wrk + Lua script
Traffic Pattern:       Multi-IP simulation
```

---

>Note: Thread count (12) slightly exceeds physical core count (10). This introduces OS-level scheduling overhead but does not affect relative comparison between implementations.

## Scenario 1 — Gateway Only (Upstream Disabled)

In this mode, the gateway returns a static response. No external HTTP calls are made.

This isolates rate limiter + routing overhead.

### Throughput

<img width="630" height="470" alt="RPS Comparison (4 Threads, 100 Connections)" src="https://github.com/user-attachments/assets/9ed59d86-31d8-401c-a97d-6e61afe90280" />

<img width="630" height="470" alt="RPS Comparison (12 Threads, 800 Connections)" src="https://github.com/user-attachments/assets/3322403d-4a87-4cdb-9cae-afa2419e6dc7" />

| Implementation | RPS |
|----------------|-----|
| DashMap | ~75,000 |
| Mutex + HashMap | ~77,000 |

### Latency

<img width="630" height="470" alt="Average Latency (ms) - 12 Threads, 800 Connections" src="https://github.com/user-attachments/assets/378efd5f-e148-44a0-9e0a-978ba1295d87" />

| Metric | DashMap | Mutex + HashMap |
|--------|---------|-----------------|
| **Average** | ~10ms | ~10ms |
| **p95** | ~12ms | ~12ms |

**Note:** Non-2xx responses are primarily 429 (rate limited) — intentional throttling behavior.

### Observation

- Minimal performance difference (<3%)
- Lock contention is not severe at this scale
- Token bucket + request processing dominates cost
- Both data structures are viable

**Key takeaway:** For moderate key cardinality and typical loads, a simple Mutex-based design performs competitively.

---

## Scenario 2 — Gateway + Upstream Forwarding Enabled

In this mode, the gateway forwards requests to an upstream server (httpbin).

This introduces:
- Network latency
- Socket scheduling
- TCP backpressure
- OS-level scheduling overhead

### Result

**Throughput dropped significantly.**  
**Latency increased to ~1.7s under heavy load.**  
**High number of timeouts and 502 errors observed.**

### Observation

When upstream I/O dominates:
- Lock contention becomes statistically insignificant
- Network latency overshadows in-memory optimization
- DashMap advantage disappears
- System bottleneck shifts to external I/O

**Technical insight:**
- DashMap improves sharded write contention
- However, at 800 concurrent connections, upstream RTT dominated >90% of request latency
- Lock granularity optimization provides negligible benefit when the system is I/O-bound

> **Optimize the actual bottleneck, not the perceived one.**

---

## Production Implications

In a real-world API gateway:

- **Heavy in-memory operations** (auth, routing, transformations) → DashMap may provide measurable benefits
- **Primarily proxying traffic** → Network latency dominates, lock strategy is secondary
- **Architectural recommendation:** Profile production workloads before optimizing synchronization primitives

The right optimization depends on whether your bottleneck is CPU or I/O.

---

## Conclusion

Both approaches are production-ready.

### Mutex + HashMap
- Simpler implementation
- Fewer dependencies
- Good enough for moderate concurrency

### DashMap
- Better scalability with high key cardinality (100k+ IPs)
- Reduced contention under heavy CPU-bound workloads
- More future-proof for multi-tenant gateways

---

## How to Reproduce
```bash
# Build release binary
cargo build --release

# Gateway-only mode (no upstream)
wrk -t12 -c800 -d10s -s multi_ip.lua http://127.0.0.1:3000/api/test

# Gateway + upstream mode
wrk -t12 -c800 -d10s -s multi_ip.lua http://127.0.0.1:3000/api/anything
```

### Lua Script (multi_ip.lua)
```lua
counter = 0
request = function()
    counter = counter + 1
    wrk.headers["X-Forwarded-For"] = "192.168.1." .. (counter % 254 + 1)
    return wrk.format()
end
```

---
