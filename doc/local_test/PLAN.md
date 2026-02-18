# Local Test Deployment — Plan

## Section 1: Playbook Skeleton and Container Image Build

### Target State

- `ansible.cfg` and `local-test.yml` exist.
- Playbook builds two container images with Podman:
  - `waddle-ws:local` — Rust backend (from existing `Dockerfile`)
  - `nginx-frontend:local` — nginx serving static files with reverse proxy

### Architecture

- **Ansible playbook** targeting `localhost` with `local` connection
- `containers.podman.podman_image` module builds both images
- Images stored in local Podman storage (no registry push)

### File Manifest

| File | Action |
|------|--------|
| `ansible.cfg` | Create — default settings |
| `local-test.yml` | Create — playbook skeleton + build tasks |
| `Dockerfile` | Existing — builds Rust backend |
| `Dockerfile.nginx` | Create — nginx image with static files and pod config |
| `nginx/nginx-pod.conf` | Create — nginx config with `proxy_pass http://127.0.0.1:3000/;` |
| `nginx/html/` | Existing — static frontend files |

### Implementation Plan

1. Create `ansible.cfg` with `host_key_checking = False`.
2. Create `local-test.yml` with `hosts: localhost`, `connection: local`, `gather_facts: false`.
3. Create `nginx/nginx-pod.conf` — copy `nginx/nginx.conf`, change `proxy_pass` to `http://127.0.0.1:3000/;`.
4. Create `Dockerfile.nginx` based on `nginx:alpine`. Copy `nginx/html` and `nginx/nginx-pod.conf` (as `/etc/nginx/conf.d/default.conf`).
5. Add tagged task `build` to build `waddle-ws:local` from `Dockerfile`.
6. Add tagged task `build` to build `nginx-frontend:local` from `Dockerfile.nginx`.

### Verification Steps

```bash
ansible-playbook local-test.yml --tags build
podman images | grep waddle-ws
podman images | grep nginx-frontend
```

Both images should appear with `local` tag.

---

## Section 2: Local Deployment

### Target State

Playbook gains deploy tasks. A Podman pod named `waddle-test-pod` runs:

- `waddle-ws` container (backend on port 3000)
- `nginx-frontend` container (frontend on port 80, proxying `/api/` to backend)

Pod exposes port 8080 to host.

### Architecture

- **Podman pod** — containers share network namespace (localhost)
- Backend listens on `127.0.0.1:3000` inside pod
- Nginx proxies `/api/` to `http://127.0.0.1:3000/`

### File Manifest

| File | Action |
|------|--------|
| `local-test.yml` | Modify — add deploy tasks |

### Implementation Plan

1. Add tagged task `deploy` (also tagged `build`) to create pod `waddle-test-pod` with port mapping `8080:80`.
2. Add tagged task `deploy` (also tagged `build`) to start `waddle-ws` container in pod with `WADDLE_LISTEN_PORT=127.0.0.1:3000` and `RUST_LOG=info`.
3. Add tagged task `deploy` (also tagged `build`) to start `nginx-frontend` container in pod.
4. Add tagged task `deploy` to wait for backend health (poll `http://localhost:8080/api/transactions` with retries).

### Verification Steps

```bash
ansible-playbook local-test.yml --tags deploy
podman pod ps | grep waddle-test-pod
curl -sf http://localhost:8080/api/transactions
```

Pod and both containers should be running.

---

## Section 3: Integration Tests

### Target State

Playbook gains test tasks. All hurl test files under `tests/integration/` execute successfully against `http://localhost:8080`.

### Architecture

- **hurl** binary installed on host
- Tests run via `ansible.builtin.command`
- Test files use `--variable host=http://localhost:8080`

### File Manifest

| File | Action |
|------|--------|
| `local-test.yml` | Modify — add test tasks |
| `tests/integration/*.hurl` | Existing — HTTP test scenarios |

### Implementation Plan

1. Add tagged task `test` (also tagged `deploy`) to install hurl (download `.deb` from GitHub releases, install with `apt`).
2. Add tagged task `test` (also tagged `deploy`) to run `hurl --test tests/integration/*.hurl --variable host=http://localhost:8080`.
3. Configure task to fail playbook if hurl exits non-zero.

### Verification Steps

```bash
ansible-playbook local-test.yml --tags test
echo $?
```

Exit code should be 0.

---

## Section 4: Teardown

### Target State

Playbook gains teardown tasks wrapped in `block/always`. All containers stopped and removed. Pod `waddle-test-pod` removed.

### Architecture

- Deploy and test tasks wrapped in `block`
- Teardown runs in `always` (executes even if tests fail)
- `containers.podman` collection with `state: absent`

### File Manifest

| File | Action |
|------|--------|
| `local-test.yml` | Modify — wrap deploy/test in `block`, add `always` teardown |

### Implementation Plan

1. Wrap deploy and test tasks (from Sections 2–3) in a `block`.
2. Add `always` section with tasks to remove `nginx-frontend` container (`state: absent`).
3. Add `always` task to remove `waddle-ws` container (`state: absent`).
4. Add `always` task to remove pod `waddle-test-pod` (`state: absent`).

### Verification Steps

```bash
ansible-playbook local-test.yml
podman pod ps | grep waddle-test-pod
podman ps -a | grep waddle
```

Full run succeeds. No resources remain after teardown.

---

## Section 5: Idempotence Check

### Target State

Playbook runs twice without errors.

### File Manifest

No new files.

### Implementation Plan

1. Run `ansible-playbook local-test.yml` twice consecutively.
2. Run `ansible-lint local-test.yml`.

### Verification Steps

```bash
ansible-playbook local-test.yml
ansible-playbook local-test.yml
ansible-lint local-test.yml
```

Both runs succeed. No lint errors.
