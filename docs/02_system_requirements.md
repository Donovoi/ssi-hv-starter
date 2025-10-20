# System Requirements & Engineering Plan

## 0. Core Design Philosophy

**MISSION CRITICAL: Build for maximum accessibility**

This project prioritizes **consumer-grade hardware support** as a primary goal:

- **TCP first, RDMA optional**: Default transport works on ANY standard Ethernet (1G, 10G, 25G+)
- **Zero hardware barrier**: $0 cost to start (no $2000 RDMA NICs required)
- **Graceful performance scaling**: 200-500µs latency on 10G Ethernet is acceptable; <100µs on RDMA is excellent
- **Plug-and-play deployment**: Auto-detection, auto-configuration, zero manual network setup
- **Optional upgrade path**: Add RDMA NICs later without code changes for <100µs latency

**Target audience expansion:** ~100 RDMA experts → ~10,000+ developers with standard hardware

**Implementation:** Multi-transport abstraction with TCP as default, RDMA as feature flag (`--features rdma-transport`)

## 1. Terminology
- **Node**: A physical host (x86_64, Linux) participating in the SSI cluster.
- **Guest**: The single VM spanning all nodes.
- **Hyperkernel**: The distributed hypervisor process running on each node.
- **Local/Remote page**: Portion of guest‑physical memory resident on the node vs. another node.
- **Transport**: Network layer for page transfers (TCP by default, RDMA optional for high performance).

## 2. Functional Requirements (FR)
**FR‑1**: Create one VM spanning N nodes; each node contributes CPU and RAM.  
**FR‑2**: Provide guest‑physical memory using distributed page ownership with **read‑replication** and **write‑invalidate** at page granularity (4 KiB, with optional 2 MiB huge pages).  
**FR‑3**: On a **remote page fault**, fetch the page via RDMA and resume the faulting vCPU without guest-visible error.  
**FR‑4**: Implement **page migration** heuristics (LRU + recency/affinity) to reduce future remote faults.  
**FR‑5**: Expose correct UEFI/ACPI tables: FADT/DSDT, **SRAT/SLIT**, and **HMAT** (optional initially).  
**FR‑6**: NUMA‑aware vCPU scheduler: pin vCPUs to home nodes; co‑locate lock holders; minimize cross‑node IPIs/TLB shootdowns.  
**FR‑7**: Provide a **paravirtual NIC** and **vDisk** so the guest can boot and access network/storage.  
**FR‑8**: Control plane must **form the cluster**, allocate address space, and orchestrate node join/leave.  
**FR‑9**: Provide **telemetry APIs** for: remote fault rate, migration traffic, RDMA ops, tail latencies, and page heatmaps.  
**FR‑10**: Support **Linux guest** (bring‑up) and **Windows guest** (post‑ACPI milestone) without guest changes.

## 3. Non‑Functional Requirements (NFR)
- **NFR‑latency**: Median remote fault service < 100 µs; p99 < 500 µs on 100/200/400 G RDMA fabrics.
- **NFR‑stability**: Survive node failure of a **non‑owning** page provider (guest continues; owning node failure may terminate guest in MVP).
- **NFR‑observability**: All control/data‑plane components expose Prometheus metrics and structured logs.
- **NFR‑security**: Process‑level isolation; mTLS between nodes; no guest escape via RDMA verbs.
- **NFR‑portability**: Host kernels ≥ 6.2; x86_64 first; no kernel modules required for the MVP.
- **NFR‑usability**: Single `coordinator up` command to form a cluster from a YAML inventory.

## 4. Milestones
**M0 — Local VMM skeleton (Week 1–2)**  
- Rust VMM over KVM; map anonymous guest RAM; boot **OVMF** to UEFI shell.

**M1 — Userfaultfd pager (Week 2–3)**  
- Register guest RAM with `userfaultfd`; resolve faults by mapping zero‑pages or a local backing file.

**M2 — Multi-transport layer (Week 3–5)** ✅ COMPLETE  
- TCP transport (default): Works on ANY Ethernet, 200-500µs latency on 10G
- RDMA transport (optional): High-performance upgrade, <100µs latency
- Auto-detection and graceful fallback; PageTransport trait abstraction

**M3 — Two‑node bring‑up (Week 5–6)**  
- Directory service for page ownership; naive migration (first‑touch wins). Run Linux guest.

**M4 — ACPI NUMA (Week 6–7)**  
- Generate SRAT/SLIT; verify NUMA placement in guest; add HMAT examples for bandwidth/latency hints.

**M5 — Windows boot (Week 7–9)**  
- Validate ACPI/UEFI with Windows; basic networking and disk I/O.

**M6 — Telemetry & placement policies (Week 9–11)**  
- Heatmap, remote miss ratio targets; implement reactive migration and pinning.

**M7 — Hardening (Week 11–12)**  
- Backpressure, flow control, huge pages, p95/p99 tuning, failure drills.

## 5. Interfaces
### Control Plane (Python, FastAPI)
- `POST /cluster` create; `DELETE /cluster` destroy
- `POST /nodes` join/leave
- `GET /metrics` Prometheus exposition
- `GET /pages/{gpa}` ownership, heat

### Data Plane (Rust)
- **VMM**: KVM ioctls; EPT/NPT management; vCPU run loop
- **Pager**: `userfaultfd` (MISSING mode) → RDMA fetch → `UFFDIO_COPY/WAKE`
- **RDMA**: verbs for connection setup; READ/WRITE; CQE handling

## 6. Deployment Requirements

**Minimum (Consumer Hardware):**
- ANY 2+ Linux machines with standard Ethernet (1G minimum, 10G recommended)
- Linux kernel 6.2+ with `userfaultfd` enabled; KVM available
- OVMF firmware blobs present (`/usr/share/OVMF/`)
- Root privileges for KVM and userfaultfd access
- **Cost: $0** (uses existing hardware)

**Optional (High Performance):**
- RDMA NICs (RoCEv2 or InfiniBand) for <100µs latency
- PTP for TSC synchronization
- Huge pages for reduced TLB pressure
- **Cost: ~$500-2000 per node** (performance upgrade)

## 7. Observability & KPIs
- **Remote fault rate** (faults/s), **service time** histogram, **remote miss ratio** (%), **migration bytes**, **RDMA CQ errors**.
- Acceptance: after warm‑up, **remote miss ratio ≤ 5%** on a representative workload; p99 service time ≤ 500 µs.

## 8. Risks & Mitigations
- **Latency cliffs**: Tail latencies on congested fabrics → Add pacing, priority flow control, and placement policies.
- **False sharing of hot pages**: Thrashing between nodes → Introduce write fencing and pinning windows.
- **ACPI inaccuracies**: Misleading SLIT/HMAT causes bad OS placement → Measure and tune SLIT values to reflect reality; provide overrides.
- **Windows sensitivity**: Boot regressions → Validate on Linux first; add exhaustive ACPI tests and examples.

## 9. References
- ACPI NUMA (SRAT/SLIT/HMAT): https://uefi.org/specs/ACPI/6.5/17_NUMA_Architecture_Platforms.html
- Microsoft ACPI tables overview: https://learn.microsoft.com/en-us/windows-hardware/drivers/bringup/acpi-system-description-tables
- Userfaultfd kernel doc: https://docs.kernel.org/admin-guide/mm/userfaultfd.html
- QEMU post‑copy notes: https://www.qemu.org/docs/master/devel/migration/postcopy.html
- OVMF/EDK2: https://github.com/tianocore/edk2
- RDMA background: https://orhanergun.net/comparative-latency-and-speed-analysis-infiniband-vs-rocev2
- CXL 3.0 overview: https://computeexpresslink.org/wp-content/uploads/2023/12/CXL_3.0-Webinar_FINAL.pdf
