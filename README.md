# My Always Unfinished Transaction System

## What I want to do

- Load a transaction log from a csv file - done
- Evaluate how the portfolio performs:
  - Calculate daily return: `(Value(t) - Value(t-1)) / Value(t-1)`. Utilize
    static content, and if changes occur between periods, calculate `t-1`
    using `t` content. If it changed in between, the `t-1` value is calculated
    using `t` content.
  - Draw a chart. Using annualized values and SMA smoothing.

## Techstack

- Rust
- Webservice with axum

## The plan

### Transaction log - done

- Import from CSV
- Display

- [x] Make Docker build locally
- [x] Do it with ansible
- [x] Learn naming and tagging docker images in ansible
- [x] Add cleanup code to the playbook.
- [x] Review ai transaction to yaml code
- [x] learn server state
- [x] Create get transactions handler
- [x] upload transactions
- [x] create a e2e test to see if transactions works.

### Portfolio

- Replay transaction log a show the current one

- [ ] create 'portfolio' handler

### Evaluate portfolio

## Ideas

### Smaller image

It uses debian slim and tini now. Consider ideas:

- Add tini to distroless
- Add ctrl-c handler to rust code
- Musl build

## Deployment

### Prerequisites

- `kind` cluster named `kind` running locally
- Docker daemon running
- `ansible-playbook`, `kubectl`, `kind` on PATH

### Deploy to the cluster

```bash
ansible-playbook main-playbook.yml --tags k8s
```

Builds both images, loads them into kind, applies the k8s manifests, and
waits for rollout. Both pods should reach Running in under a minute.

### Access the frontend

```bash
NODE_IP=$(kubectl get node kind-control-plane \
  -o jsonpath='{.status.addresses[?(@.type=="InternalIP")].address}')
echo "http://$NODE_IP:30080"
```

Open the URL in a browser. Select a semicolon-delimited CSV file and click
**Upload** to load transactions. The portfolio table updates automatically.

### Check status

```bash
kubectl get pods -n waddle
kubectl logs -n waddle deployment/waddle-ws
kubectl logs -n waddle deployment/waddle-frontend
```

### Tear down

```bash
kubectl delete namespace waddle
```

### Run backend E2E tests (Docker, no k8s)

```bash
ansible-playbook main-playbook.yml --tags e2e
```

Spins up the `waddle-ws` container, runs all hurl tests, then removes the
container.

---

## Dependencies

- docker collection
- requests package
