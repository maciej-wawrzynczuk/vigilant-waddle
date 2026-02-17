# Local Test Deployment — Plan

## Section 1: Container Image Build

### Target State

Two container images built locally with Podman:

- `waddle-ws:local` — Rust backend (from existing `Dockerfile`)
- `nginx-frontend:local` — nginx serving static files with reverse proxy to backend

### Architecture

- **Podman CLI** invoked via Ansible `containers.podman.podman_image` module
- Images stored in local Podman storage (no registry push)
- Build context: repository root

### File Manifest

| File | Action |
|------|--------|
| `Dockerfile` | Existing — builds Rust backend |
| `Dockerfile.nginx` | Create — nginx image with static files and pod config |
| `nginx/nginx-pod.conf` | Create — copy of `nginx.conf` with `proxy_pass http://127.0.0.1:3000/;` |
| `nginx/html/` | Existing — static frontend files |

### Implementation Plan

1. Create `Dockerfile.nginx` based on `nginx:alpine`. Copy `nginx/html` and `nginx/nginx-pod.conf` (as `/etc/nginx/conf.d/default.conf`).
2. Create `nginx/nginx-pod.conf` — copy `nginx/nginx.conf` and change `proxy_pass` to `http://127.0.0.1:3000/;`.
3. Write Ansible task to build `waddle-ws:local` from `Dockerfile` using `containers.podman.podman_image`.
4. Write Ansible task to build `nginx-frontend:local` from `Dockerfile.nginx`.

### Verification Steps

```bash
podman images | grep waddle-ws
podman images | grep nginx-frontend
```

Both images should appear with `local` tag.

---

## Section 2: Local Deployment

### Target State

A Podman pod named `waddle-test-pod` running:

- `waddle-ws` container (backend on port 3000)
- `nginx-frontend` container (frontend on port 8080, proxying `/api/` to backend)

Pod exposes port 8080 to host.

### Architecture

- **Podman pod** — containers share network namespace (localhost)
- **Ansible `containers.podman` collection** — manages pod and container lifecycle
- Backend listens on `127.0.0.1:3000` inside pod
- Nginx proxies `/api/` to `http://127.0.0.1:3000/`

### File Manifest

| File | Action |
|------|--------|
| `nginx/nginx-pod.conf` | Created in Section 1; uses `127.0.0.1:3000` for pod networking |

### Implementation Plan

1. Write Ansible task to create pod `waddle-test-pod` with port mapping `8080:80`.
2. Write Ansible task to start `waddle-ws` container in pod with environment:
   - `WADDLE_LISTEN_PORT=127.0.0.1:3000`
   - `RUST_LOG=info`
3. Write Ansible task to start `nginx-frontend` container in pod.
4. Write Ansible task to wait for backend health (poll `http://localhost:8080/api/transactions` with retries).

### Verification Steps

```bash
podman pod ps | grep waddle-test-pod
podman ps --pod | grep waddle-test-pod
curl -sf http://localhost:8080/api/transactions
```

Pod and both containers should be running. Health check should return 200.

---

## Section 3: Integration Tests

### Target State

All hurl test files under `tests/integration/` execute successfully against `http://localhost:8080`.

### Architecture

- **hurl** binary installed on host
- Tests run via Ansible `ansible.builtin.command` module
- Test files use `--variable host=http://localhost:8080`

### File Manifest

| File | Action |
|------|--------|
| `tests/integration/*.hurl` | Existing — HTTP test scenarios |

### Implementation Plan

1. Write Ansible task to install hurl (download latest `.deb` from GitHub releases, install with `apt`).
2. Write Ansible task to run `hurl --test tests/integration/*.hurl --variable host=http://localhost:8080`.
3. Configure task to fail playbook if hurl exits non-zero.

### Verification Steps

```bash
hurl --test tests/integration/*.hurl --variable host=http://localhost:8080
echo $?
```

Exit code should be 0.

---

## Section 4: Teardown

### Target State

All containers stopped and removed. Pod `waddle-test-pod` removed. No dangling resources.

### Architecture

- **Ansible `containers.podman` collection** — `state: absent` for containers and pod
- Teardown runs in `always` block (executes even if tests fail)

### File Manifest

No new files.

### Implementation Plan

1. Write Ansible `block` with `always` section.
2. In `always`: stop and remove `nginx-frontend` container.
3. In `always`: stop and remove `waddle-ws` container.
4. In `always`: remove pod `waddle-test-pod`.

### Verification Steps

```bash
podman pod ps | grep waddle-test-pod
podman ps -a | grep waddle
```

No output — all resources cleaned up.

---

## Section 5: Orchestration Playbook

### Target State

Single playbook `local-test.yml` that executes Sections 1–4 in order.

### Architecture

- **Ansible playbook** with `localhost` connection
- Tasks grouped by section (build, deploy, test, teardown)
- Idempotent — can run multiple times safely

### File Manifest

| File | Action |
|------|--------|
| `local-test.yml` | Create — main playbook |
| `ansible.cfg` | Create — disable host key checking, set inventory to `localhost,` |

### Implementation Plan

1. Create `ansible.cfg` with:
   - `[defaults]`
   - `inventory = localhost,`
   - `host_key_checking = False`
2. Create `local-test.yml` with:
   - `hosts: localhost`
   - `connection: local`
   - `gather_facts: false`
3. Add tasks from Sections 1–4 in order.
4. Wrap deploy/test tasks in `block`, teardown in `always`.

### Verification Steps

```bash
ansible-playbook local-test.yml
ansible-playbook local-test.yml  # Run twice to verify idempotence
ansible-lint local-test.yml
```

Both runs should succeed. No lint errors.
