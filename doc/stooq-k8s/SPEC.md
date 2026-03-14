# Spec: stooq-k8s

## Objective

Configure the Kubernetes `waddle-ws` deployment to run with the `stooq` quote
provider. Deliver a production-ready example symbol map for five real securities
(IBM, MSFT, EOAN/Xetra, BP/LSE, CAP/Paris), mount it into the pod via a
Kubernetes ConfigMap volume, and wire the required environment variables. Also
fix the E2E (Docker) test path by supplying `WADDLE_QUOTES_PROVIDER=mock`.

## Tech Stack

No new Rust crates or npm packages. Infrastructure additions only:

- Kubernetes ConfigMap (YAML) — embeds the TOML symbol map
- Volume and VolumeMount in the Deployment

## Core Features

### Feature 1 — Example symbol map TOML (`k8s/stooq-symbol-map.toml`)

| Symbol | Exchange       | Stooq suffix | Divisor      |
|--------|----------------|--------------|--------------|
| IBM    | US             | .US          | —            |
| MSFT   | US             | .US          | —            |
| EOAN   | Xetra          | .DE          | —            |
| BP     | LSE            | .UK          | 100 (GBX→GBP)|
| CAP    | Euronext Paris | .FR          | —            |

### Feature 2 — ConfigMap (`k8s/stooq-configmap.yaml`)

Embeds the TOML file as a ConfigMap key in namespace `waddle`.

### Feature 3 — Deployment update (`k8s/waddle-ws.yaml`)

Add `WADDLE_QUOTES_PROVIDER=stooq` and
`STOOQ_SYMBOL_MAP=/etc/waddle/stooq-symbol-map.toml` env vars; mount ConfigMap
as a read-only volume at `/etc/waddle/`.

### Feature 4 — Ansible playbook update (`main-playbook.yml`)

Apply ConfigMap before `waddle-ws.yaml` (tag `k8s`). Add
`WADDLE_QUOTES_PROVIDER=mock` to the E2E Docker container env (tag `e2e`).

## Public Interface and Data Structures

### `k8s/stooq-symbol-map.toml`

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

### `k8s/stooq-configmap.yaml`

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: stooq-symbol-map
  namespace: waddle
data:
  stooq-symbol-map.toml: |
    <contents of k8s/stooq-symbol-map.toml>
```

### `k8s/waddle-ws.yaml` additions (env section)

```yaml
- name: WADDLE_QUOTES_PROVIDER
  value: stooq
- name: STOOQ_SYMBOL_MAP
  value: /etc/waddle/stooq-symbol-map.toml
```

### `k8s/waddle-ws.yaml` additions (volumes)

```yaml
volumes:
  - name: stooq-symbol-map
    configMap:
      name: stooq-symbol-map
volumeMounts:
  - name: stooq-symbol-map
    mountPath: /etc/waddle
    readOnly: true
```

### `main-playbook.yml` E2E Docker env addition

```yaml
env:
  WADDLE_QUOTES_PROVIDER: mock
```

## Acceptance Criteria

1. `--tags k8s` playbook run: deployment reaches `Available`; no fatal startup
   errors in logs.
2. `STOOQ_SYMBOL_MAP` resolves to a parseable TOML at
   `/etc/waddle/stooq-symbol-map.toml` inside the pod.
3. ConfigMap contains all five symbols with correct suffixes and the `divisor`
   field for BP.
4. `--tags e2e` playbook run: all hurl tests pass with mock provider.
5. `kubectl apply --dry-run=client` validates all new/modified manifests.
6. `ansible-lint main-playbook.yml` — no errors.
7. `markdownlint-cli2 doc/stooq-k8s/SPEC.md` — passes.

## Constraints

- No Rust source or `Cargo.toml` changes.
- Symbol map is immutable after pod startup (no hot-reload).
- Network access to `stooq.com` from within the kind cluster is assumed
  available.
- E2E tests continue to use mock provider (no live network access required).
- Currency conversion is out of scope.
- No PVC or external volume — ConfigMap inline is sufficient.
