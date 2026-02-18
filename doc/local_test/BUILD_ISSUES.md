# Build Issues — Local Test Deployment

## Issue 1: Unnecessary inventory file for localhost deployment

**Problem:** Initial plan included creating an `inventory` file with `localhost ansible_connection=local`, but this is redundant when using `connection: local` directly in the playbook.

**Solution:** Removed inventory file from plan. Playbook uses `hosts: localhost` with `connection: local` directly, which is the standard Ansible pattern for local execution.

**Files affected:**
- `doc/local_test/PLAN.md` — removed inventory from all file manifests and verification steps
- `ansible.cfg` — kept minimal (only `host_key_checking = False`)

**Cycle/Stage/Phase:** local_test / Planning / Section 1

---

## Issue 2: Podman short-name ambiguity in Dockerfiles

**Problem:** Both `Dockerfile` and `Dockerfile.nginx` used short-name base images (`debian:bookworm-slim` and `nginx:alpine`). On systems without cached images, Podman prompts for registry selection, breaking automated builds.

**Solution:** Changed both Dockerfiles to use fully-qualified image names:
- `debian:bookworm-slim` → `docker.io/library/debian:bookworm-slim`
- `nginx:alpine` → `docker.io/library/nginx:alpine`

**Files affected:**
- `Dockerfile` — updated FROM directive
- `Dockerfile.nginx` — updated FROM directive

**Cycle/Stage/Phase:** local_test / Coding / Section 1
