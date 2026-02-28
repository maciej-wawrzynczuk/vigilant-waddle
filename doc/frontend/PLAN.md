<!-- markdownlint-disable MD024 -->
# PLAN: Frontend

Reference spec: `doc/frontend/SPEC.md`

---

## Feature 1: Static frontend page

### Target state

`frontend/index.html` — single HTML file with a file-upload form, a status
message area, and a portfolio table. `frontend/pico.min.css` provides minimal
styling with no CDN references at runtime.

### Architecture

Three logical sections in the page:

1. **Upload form** — `<input type="file" accept=".csv">` + `<button>Upload`
2. **Status area** — `<p id="status">` updated by JS with success/error text
3. **Portfolio table** — `<table>` with fixed `<thead>` (Symbol | Quantity)
   and a JS-populated `<tbody id="portfolio-body">`

The inline `<script>` contains four single-responsibility functions:

| Function | Responsibility |
| -------- | -------------- |
| `parseYaml(text)` | Parse flat YAML mapping into a plain JS object |
| `renderPortfolio(data)` | Build table rows via `createElement` |
| `loadPortfolio()` | `GET /portfolio`; render or show "No data" on 404 |
| `uploadCsv(file)` | PUT multipart CSV, then call `loadPortfolio()` |

`DOMContentLoaded` wires the button click and calls `loadPortfolio()` once.

`parseYaml` splits on newlines then on `": "` — sufficient for the flat-mapping
format the backend always returns.

All DOM mutations use `textContent` or `appendChild` — never `innerHTML`
with untrusted data.

### File manifest

| File | Change |
| ---- | ------ |
| `frontend/pico.min.css` | Created — vendored from picocss.com |
| `frontend/index.html` | Created — HTML + inline JS |

### WBS

1. Download Pico.css minified to `frontend/pico.min.css`.
2. Create `frontend/index.html`:
   - `<head>`: charset, viewport, title, `<link>` to `pico.min.css`.
   - `<body>`: `<main>` with upload `<article>`, status `<p>`,
     portfolio `<table>`.
   - `<script>`: four functions; `DOMContentLoaded` handler.

### Verification

- Open `frontend/index.html` in a browser; page renders without console
  errors and makes no external network requests (check DevTools Network tab).

---

## Feature 2: nginx configuration

### Target state

`nginx/nginx.conf` serves static files from `/usr/share/nginx/html` and
proxies `/transactions` and `/portfolio` to the waddle-ws service.

### Architecture

Single `server` block on port 80:

- `location /` — `try_files $uri $uri/ /index.html`
- `location /transactions` — `proxy_pass http://waddle-ws:3000`
- `location /portfolio` — `proxy_pass http://waddle-ws:3000`

`proxy_set_header Host $host` and `proxy_http_version 1.1` are set on both
proxy locations to ensure correct behaviour with Axum.

### File manifest

| File | Change |
| ---- | ------ |
| `nginx/nginx.conf` | Created |

### WBS

1. Create `nginx/nginx.conf` with the `server` block above.

### Verification

- `nginx -t -c nginx/nginx.conf` (inside a container if needed) exits 0.

---

## Feature 3: Frontend Docker image

### Target state

`Dockerfile.frontend` builds an `nginx:alpine` image that bundles the static
frontend and nginx config.

### Architecture

```dockerfile
FROM nginx:alpine
COPY frontend/ /usr/share/nginx/html/
COPY nginx/nginx.conf /etc/nginx/conf.d/default.conf
```

No custom entrypoint — `nginx:alpine` already provides one.

### File manifest

| File | Change |
| ---- | ------ |
| `Dockerfile.frontend` | Created |

### WBS

1. Create `Dockerfile.frontend` (three lines as above).

### Verification

- `docker build -f Dockerfile.frontend -t localhost/waddle-frontend:latest .`
  exits 0.
- `docker run --rm -p 8080:80 localhost/waddle-frontend:latest` starts and
  serves the page.

---

## Feature 4: Kubernetes manifests

### Target state

Three manifest files deploy both services into a `waddle` namespace and
expose the frontend on NodePort 30080.

### Architecture

**`k8s/namespace.yaml`** — `Namespace: waddle`.

**`k8s/waddle-ws.yaml`**:

- Deployment: 1 replica, `image: localhost/waddle-ws:latest`,
  `imagePullPolicy: Never`, port 3000, env vars `WADDLE_LISTEN_ADDR`
  and `RUST_LOG`.
- Service: ClusterIP, port 3000, name `waddle-ws`.

**`k8s/frontend.yaml`**:

- Deployment: 1 replica, `image: localhost/waddle-frontend:latest`,
  `imagePullPolicy: Never`, port 80.
- Service: NodePort, port 80 → `nodePort: 30080`, name `waddle-frontend`.

`imagePullPolicy: Never` is required because images are preloaded into the
node's image store by the Ansible playbook (Feature 5) rather than pulled
from a registry.

### File manifest

| File | Change |
| ---- | ------ |
| `k8s/namespace.yaml` | Created |
| `k8s/waddle-ws.yaml` | Created — Deployment + ClusterIP Service |
| `k8s/frontend.yaml` | Created — Deployment + NodePort Service |

### WBS

1. Create `k8s/namespace.yaml`.
2. Create `k8s/waddle-ws.yaml`.
3. Create `k8s/frontend.yaml`.

### Verification

- `kubectl apply -f k8s/` returns no errors.
- `kubectl get pods -n waddle` shows both pods Running.
- `curl http://<node-ip>:30080` returns the frontend HTML.

---

## Feature 5: Ansible orchestration

### Target state

`main-playbook.yml` gains a `k8s`-tagged block that builds both images,
imports them into the cluster, applies manifests, and waits for the rollout.

### Architecture

All existing tasks receive an additional `e2e` tag so `--tags e2e` continues
to run the full Docker-compose E2E suite unchanged.

New tasks appended, all tagged `k8s`:

1. **Build waddle-ws image** — reuse
   `community.docker.docker_image_build` (already present, add `k8s` tag).
2. **Build frontend image** — `community.docker.docker_image_build` with
   `dockerfile: Dockerfile.frontend`.
3. **Save images to tarballs** — `community.docker.docker_image` with
   `source: save` for each image.
4. **Import into cluster** — `ansible.builtin.command: k3s ctr images import`
   for each tarball.
   *(Substitute `kind load docker-image` or `minikube image load` for other
   distros.)*
5. **Apply manifests** —
   `ansible.builtin.command: kubectl apply -f k8s/`.
6. **Wait for rollout** —
   `ansible.builtin.command: kubectl rollout status deployment
   -n waddle --timeout=60s` for each deployment.

### File manifest

| File | Change |
| ---- | ------ |
| `main-playbook.yml` | Existing tasks tagged `e2e`; new `k8s` block added |

### WBS

1. Add `tags: [build, e2e]` to all pre-existing tasks.
2. Append a `block` tagged `k8s` with steps 1–6 above,
   wrapped in `block/always` for cleanup parity.
3. Run `ansible-lint main-playbook.yml`; fix any warnings.

### Verification

- `ansible-lint main-playbook.yml` — no errors.
- `ansible-playbook main-playbook.yml --tags k8s` completes without
  failures.
- `kubectl get pods -n waddle` — all pods Running.
- `curl http://<node-ip>:30080` — page loads with upload form.
- `ansible-playbook main-playbook.yml --tags e2e` — all hurl tests pass.
