# SPEC: Frontend

## Objective

Create a minimal static web frontend that allows users to upload a transaction
CSV file and display the resulting portfolio. The frontend is served by nginx,
which also reverse-proxies the backend API. Both services are deployed to a
local Kubernetes cluster.

## Tech Stack

- **HTML5 + vanilla JS** — no build toolchain; a single `index.html` file
- **Pico.css** — classless, minimal CSS library (~30 KB); downloaded locally,
  no CDN references at runtime
- **nginx** (`nginx:alpine`) — static file server and reverse proxy
- **Kubernetes** — local cluster (e.g., k3s or minikube) for deployment
- **Docker** — container image for the nginx + frontend bundle
- **Ansible** — extended `main-playbook.yml` orchestrates image build and
  k8s apply

## Core Features

1. **CSV upload** — a file-picker and submit button PUT the selected CSV to
   `PUT /transactions` with the `X-Overwrite: confirm` header.
2. **Portfolio display** — immediately after a successful upload, and on page
   load, `GET /portfolio` is called and the YAML response is rendered as an
   HTML table (symbol | quantity columns).
3. **Status feedback** — a status line shows success or error messages for
   both the upload and the fetch operations.
4. **nginx proxy + static serving** — nginx serves the frontend at `/` and
   forwards `/transactions` and `/portfolio` to the waddle-ws ClusterIP
   service; no CORS headers are required.
5. **Kubernetes deployment** — k8s manifests for both waddle-ws and the
   frontend (nginx) are applied to the local cluster.

## Data Structures

### Upload request (`PUT /transactions`)

Multipart form data with a single field `transaction_log` carrying the CSV
file; header `X-Overwrite: confirm` is always sent.

### Portfolio response (`GET /portfolio`)

Existing YAML mapping (symbol → net quantity):

```yaml
FOO: 3
BAR: -1
```

Rendered in the browser as an HTML table:

| Symbol | Quantity |
|--------|----------|
| FOO    | 3        |
| BAR    | −1       |

### Error response

Any non-2xx HTTP status from the backend is surfaced to the user as a plain
text status message (HTTP status code + response body excerpt).

## File Manifest

| Path | Description |
| ---- | ----------- |
| `frontend/index.html` | Single-page frontend (HTML + inline JS) |
| `frontend/pico.min.css` | Downloaded Pico.css (no CDN reference) |
| `nginx/nginx.conf` | nginx config: static root + proxy rules |
| `Dockerfile.frontend` | `nginx:alpine` image bundling static files |
| `k8s/waddle-ws.yaml` | Deployment + ClusterIP Service for waddle-ws |
| `k8s/frontend.yaml` | Deployment + NodePort Service for nginx frontend |
| `k8s/namespace.yaml` | `waddle` namespace |

`main-playbook.yml` gains a new tag (`k8s`) and tasks to build the frontend
image and apply the k8s manifests.

## Acceptance Criteria

1. Navigating to the frontend URL in a modern desktop browser renders a page
   with a file-upload control and an empty portfolio table placeholder.
2. Selecting a valid CSV and clicking **Upload** sends `PUT /transactions`
   with `X-Overwrite: confirm`; the page displays a success message.
3. After a successful upload, the portfolio table is populated automatically
   from `GET /portfolio`.
4. If the backend returns an error, a descriptive status message is shown and
   no stale portfolio data is rendered.
5. On page load, if `GET /portfolio` returns 200, the table is pre-populated;
   if it returns 404, the table area shows "No data loaded yet."
6. All static assets (CSS, any JS) are served locally — zero external network
   requests from the browser.
7. nginx forwards `/transactions` and `/portfolio` to the waddle-ws service
   without exposing the backend port directly.
8. `kubectl get pods -n waddle` shows all pods Running after
   `ansible-playbook main-playbook.yml --tags k8s`.
9. All existing hurl E2E tests continue to pass unchanged.

## Constraints

- No npm, webpack, or any JS build toolchain.
- No JavaScript frameworks (React, Vue, etc.) — vanilla JS only.
- Pico.css is the only external CSS library; it must be vendored into
  `frontend/`.
- nginx base image: `nginx:alpine` (consistent with existing docker-compose).
- Docker base image for waddle-ws remains `debian:bookworm-slim`.
- k8s manifests target API versions stable in Kubernetes 1.29+.
- Ansible lint must pass on all modified/added playbook tasks.
- markdownlint must pass on all markdown files.
- The inline JS in `index.html` must not use `eval` or dynamic `innerHTML`
  injection with untrusted data.
