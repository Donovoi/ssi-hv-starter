# Problem Statement: Single‑System‑Image Virtualization Framework

## Summary
We aim to build a **software Single‑System‑Image (SSI)** virtualization layer that makes a cluster of commodity x86_64 hosts appear to a guest OS (Windows or Linux) as **one large NUMA machine**. The guest should run **unmodified**, believing it has CPUs, RAM, and I/O resources organized into NUMA nodes with realistic latencies and bandwidths.

## Motivation
Certain workloads benefit from large shared memory and high aggregate CPU counts, yet do not scale out easily as distributed services. Commercial SSI products (e.g., TidalScale HyperKernel; historical ScaleMP vSMP) demonstrate feasibility but are proprietary and platform‑restricted. This project explores an **open, research‑oriented** path that prioritizes clarity, observability, and reproducibility.

## Scope
**In scope**
- A distributed hypervisor (“hyperkernel”) running on each host that **presents a single virtual machine** spanning all hosts.
- **Guest‑physical memory** implemented as **distributed, coherent, page‑granularity memory** with migration and replication.
- **UEFI/ACPI interface** exposing NUMA topology (SRAT/SLIT/HMAT) to the guest.
- **vCPU scheduling** pinned to home nodes with NUMA‑aware placement.
- **I/O virtualization** sufficient to boot and operate the guest (paravirtual NIC, vDisk).

**Out of scope (initially)**
- Pooling discrete GPUs across hosts for a single frame pipeline.
- Transparent acceleration of tightly coupled real‑time graphics workloads (e.g., PC games).
- Security hardening beyond basic isolation; HA/failover.
- Live migration of the aggregated VM.

## Constraints & Assumptions
- **Interconnect**: Low‑latency RDMA fabric (InfiniBand or RoCEv2). Regular TCP/Ethernet is insufficient for acceptable tail latency.
- **Host OS**: Linux (recent kernel with `userfaultfd` and KVM).
- **Guest OS**: Start with Linux for bring‑up, then boot Windows once ACPI/UEFI are correct.
- **Latency reality**: Remote memory will be orders of magnitude slower than local DRAM. Success depends on **locality** and **page placement** policies.

## Success Criteria (MVP)
- Boot an unmodified Linux guest on a 2‑node cluster and **touch >90% of guest RAM** with correct data under a page‑migration regime.
- Demonstrate **remote page fault service time** median < 100 µs and 99th‑percentile < 500 µs on a modern RDMA fabric (measured in‑guest via a provided microbenchmark).
- Expose ACPI SRAT/SLIT that the guest recognizes; verify NUMA placement decisions in the guest scheduler.
- Run a standard workload (e.g., `memcached`, `postgres`, or `stress-ng` mix) and show **remote miss ratio** below 1–5% in steady state after warm‑up.

## References (non‑exhaustive)
- TidalScale HyperKernel overview (AWS partnership): https://aws.amazon.com/blogs/hpc/hyper-metal-scaling-aws-instances-up-with-tidalscale/
- ScaleMP vSMP (Single‑system‑image aggregation): https://www.infoworld.com/article/2283406/scalemp-expands-server-virtualization-for-aggregation-software-platform.html
- Linux `userfaultfd` (post‑copy migration mechanism): https://docs.kernel.org/admin-guide/mm/userfaultfd.html
- QEMU post‑copy migration notes: https://www.qemu.org/docs/master/devel/migration/postcopy.html
- ACPI SRAT/SLIT/HMAT (NUMA & latency/bw description): https://uefi.org/specs/ACPI/6.5/17_NUMA_Architecture_Platforms.html
- OVMF/EDK2 for UEFI guests: https://github.com/tianocore/edk2
- RoCE vs InfiniBand latency characteristics (general background): https://nednex.com/en/roce-vs-infiniband-vs-iwarp-vs-alternatives/
- CXL 3.0 memory pooling (future direction): https://computeexpresslink.org/wp-content/uploads/2023/12/CXL_3.0-Webinar_FINAL.pdf
