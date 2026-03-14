<!-- markdownlint-disable MD013 MD024 -->
# PLAN: stooq-k8s — Kubernetes Deployment of Stooq Provider

## Feature 1 — `k8s/stooq-symbol-map.toml`

### Target State

New TOML file with five securities. Parsed by `StooqQuotes::from_toml_file` at
container startup via the `STOOQ_SYMBOL_MAP` env var.

### Architecture

The file uses the `[symbols.SYMBOL]` table structure consumed by
`toml::from_str::<SymbolMap>`. `suffix` is required; `divisor` is optional
(defaults to 1 when absent). Stored in `k8s/` so it travels with the
Kubernetes manifests and is embedded in the ConfigMap.

### File Manifest

| File | Change |
| --- | --- |
| `k8s/stooq-symbol-map.toml` | **Create** |

### WBS

1. Create `k8s/stooq-symbol-map.toml`:

```toml
[symbols.IBM]
suffix = ".US"

[symbols.MSFT]
suffix = ".US"

[symbols.EOAN]
suffix = ".DE"

[symbols.BP]
suffix = ".UK"
divisor = 100

[symbols.CAP]
suffix = ".FR"
```

### Verification

```bash
cargo test --lib stooq::
```

`StooqQuotes::from_toml_file` parses the file with no errors.

---

## Feature 2 — `k8s/stooq-configmap.yaml`

### Target State

Kubernetes ConfigMap in namespace `waddle` that embeds the TOML file as a
single key `stooq-symbol-map.toml`. Mounted read-only at `/etc/waddle/` in
the `waddle-ws` pod.

### Architecture

The ConfigMap is applied before `waddle-ws.yaml` so the volume reference is
satisfied when the Deployment rolls out. Embedding the TOML inline (not via
`binaryData` or a separate file reference) keeps the ConfigMap self-contained
and diff-friendly.

### File Manifest

| File | Change |
| --- | --- |
| `k8s/stooq-configmap.yaml` | **Create** |

### WBS

1. Create `k8s/stooq-configmap.yaml` with the TOML content inlined under the
   `data` key `stooq-symbol-map.toml`.

### Verification

```bash
kubectl apply --dry-run=client -f k8s/stooq-configmap.yaml
```

---

## Feature 3 — `k8s/waddle-ws.yaml` Update

### Target State

The `waddle-ws` Deployment gains two env vars (`WADDLE_QUOTES_PROVIDER`,
`STOOQ_SYMBOL_MAP`) and a read-only volume mount of the ConfigMap at
`/etc/waddle/`.

### Architecture

`WADDLE_QUOTES_PROVIDER=stooq` tells `main()` to construct a `StooqQuotes`
provider. `STOOQ_SYMBOL_MAP=/etc/waddle/stooq-symbol-map.toml` points to the
file injected by the ConfigMap volume. The volume is declared in `pod.spec.volumes`
and referenced in `containers[0].volumeMounts`.

### File Manifest

| File | Change |
| --- | --- |
| `k8s/waddle-ws.yaml` | **Modify** |

### WBS

1. In the `env:` section of the `waddle-ws` container, add after `RUST_LOG`:

```yaml
- name: WADDLE_QUOTES_PROVIDER
  value: stooq
- name: STOOQ_SYMBOL_MAP
  value: /etc/waddle/stooq-symbol-map.toml
```

1. In the container spec, add `volumeMounts:`:

```yaml
volumeMounts:
  - name: stooq-symbol-map
    mountPath: /etc/waddle
    readOnly: true
```

1. In the pod `spec:`, add `volumes:` (sibling of `containers:`):

```yaml
volumes:
  - name: stooq-symbol-map
    configMap:
      name: stooq-symbol-map
```

### Verification

```bash
kubectl apply --dry-run=client -f k8s/waddle-ws.yaml
```

---

## Feature 4 — `main-playbook.yml` Update

### Target State

Two changes:

- `--tags k8s`: ConfigMap is applied before `waddle-ws.yaml`
- `--tags e2e`: Docker container receives `WADDLE_QUOTES_PROVIDER: mock` so
  the E2E tests do not make live HTTP requests to Stooq

### Architecture

The `mock` provider requires no TOML file; it returns default prices for all
symbols. Setting the env var in the `docker_container` task keeps the E2E path
isolated from the Stooq network dependency. The new "Apply stooq ConfigMap"
task is inserted between "Wait for namespace to be active" and "Apply k8s
manifests" so the ConfigMap exists before the Deployment references it.

### File Manifest

| File | Change |
| --- | --- |
| `main-playbook.yml` | **Modify** |

### WBS

1. Add a new task between "Wait for namespace to be active" and "Apply k8s
   manifests":

```yaml
- name: Apply stooq ConfigMap
  kubernetes.core.k8s:
    src: "{{ playbook_dir }}/k8s/stooq-configmap.yaml"
    state: present
  tags: [k8s]
```

1. In the "Spin up the container" task (`docker_container`), add `env:`:

```yaml
env:
  WADDLE_QUOTES_PROVIDER: mock
```

### Verification

```bash
ansible-lint main-playbook.yml
```

---

## Files Created / Modified

| Action | Path |
| --- | --- |
| **Create** | `doc/stooq-k8s/PLAN.md` |
| **Create** | `k8s/stooq-symbol-map.toml` |
| **Create** | `k8s/stooq-configmap.yaml` |
| **Modify** | `k8s/waddle-ws.yaml` |
| **Modify** | `main-playbook.yml` |

---

## Verification (End-to-End)

```bash
# Manifest validation
kubectl apply --dry-run=client -f k8s/stooq-configmap.yaml
kubectl apply --dry-run=client -f k8s/waddle-ws.yaml

# Ansible lint
ansible-lint main-playbook.yml

# Markdown lint
markdownlint-cli2 doc/stooq-k8s/PLAN.md

# E2E (Docker + mock provider)
ansible-playbook main-playbook.yml --tags e2e

# K8s deployment (requires running kind cluster)
ansible-playbook main-playbook.yml --tags k8s
```
