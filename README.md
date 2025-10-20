# SSI-HV (Single‑System‑Image Hypervisor) — Starter Repo

This repository seeds a research prototype that **aggregates multiple x86_64 machines into one large NUMA system** presented to a guest OS via UEFI/ACPI. It is **not production‑ready**; it provides documents, interfaces, and stubs that an engineering agent can extend.

## Contents
- `docs/01_problem_statement.md` — crisp problem statement & scope
- `docs/02_system_requirements.md` — functional & non‑functional requirements, milestones
- Minimal code scaffolding for a Rust/KVM VMM, userfaultfd pager, RDMA transport, ACPI generator, and a Python control plane

## Quick start
```bash
# (optional) create a new git repo and push
git init
git add .
git commit -m "bootstrap SSI-HV starter"
git branch -M main
git remote add origin <YOUR_ORIGIN>
git push -u origin main
```

## License
Apache-2.0 (see LICENSE).
