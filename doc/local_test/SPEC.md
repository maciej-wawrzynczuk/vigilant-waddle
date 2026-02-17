# Local Test Deployment — Specification

## Objective

Provide a fully local, repeatable deployment and integration test pipeline
that replaces the GitHub Actions workflow, using Ansible as the orchestrator.

## Tech Stack

| Component | Tool | Version |
|-----------|------|---------|
| Container engine | Podman | 4.x+ |
| Orchestration | Ansible | 2.17+ |
| Ansible collection | containers.podman | 1.x+ |
| Integration testing | hurl | 6.0 |

## Core Features

1. **Container image build** — Build the Rust backend image and the nginx
   frontend image locally with Podman.
2. **Local deployment** — Create a Podman pod and start both containers
   (backend + nginx) using Ansible and the `containers.podman` collection.
3. **Integration tests** — Run declarative HTTP tests (hurl files) against
   the locally deployed services to verify upload and query endpoints.
4. **Teardown** — Stop and remove all containers and the pod after tests
   complete, regardless of test outcome.
5. **Single entry point** — A single Ansible playbook invoked with
   `ansible-playbook` that executes the full pipeline with one command.

## Acceptance Criteria

1. `ansible-playbook local-test.yml` builds images, starts containers, runs
   tests, and tears down — all locally, with no external dependencies beyond
   the tech stack.
2. The Rust backend `/transactions` PUT and GET endpoints pass hurl tests.
3. The nginx frontend serves static files through the pod.
4. Teardown leaves no running containers or dangling pods from the pipeline.
5. The pipeline is idempotent — running it twice in a row succeeds.

## Constraints

- No Docker daemon; Podman only (rootless where possible).
- No remote registry; images stay local.
- All container base images use `debian-slim` variants.
- The pipeline must run on a single Linux host.
- Ansible playbook must be lintable with `ansible-lint`.
- Hurl test files must live under `tests/integration/`.

## Data Structures

No new data structures are introduced. The pipeline operates on the existing
`Transactions` CSV format (semicolon-delimited) already defined in the
codebase.
