# BUILD-ISSUES: Frontend

## BI-01 `.dockerignore` blocked frontend build context

**Symptom**: `docker build -f Dockerfile.frontend` failed — `nginx/nginx.conf`
not found in build context.

**Root cause**: `.dockerignore` contained `**/*` with only
`!target/debug/waddle-ws` as an exception; all other files were excluded.

**Fix**: Added `!frontend/`, `!frontend/**`, `!nginx/`, and `!nginx/nginx.conf`
exception rules to `.dockerignore`.

---

## BI-04 hurl v6 parallel execution broke ordered e2e tests

**Symptom**: `hurl --test hurl/` fails on `overwrite_rejected.hurl` and
`empty_transactions.hurl` because hurl v6 defaults to parallel file execution
in `--test` mode, and the test files are order-dependent (some must run before
any data is loaded).

**Root cause**: hurl v6 changed `--test` to imply `--parallel`. Files also
run in filesystem (inode) order rather than alphabetically when given a
directory, so `trans.hurl` (which loads data) runs before the tests that
require an empty server.

**Fix**: Pass `--jobs 1` to force sequential execution and list the hurl files
explicitly in dependency order:
`empty_transactions.hurl` → `overwrite_rejected.hurl` → `trans.hurl` →
`portfolio.hurl`.

---

## BI-03 Wrong cluster import tool — k3s instead of kind

**Symptom**: Playbook used `k3s ctr images import` which is not available; the
local cluster is `kind` (cluster name: `kind`, node: `kind-control-plane`).

**Fix**: Replaced the save-to-tarball + `k3s ctr images import` steps with
`kind load docker-image <image>`, which loads images directly from the Docker
daemon into kind nodes without an intermediate tarball.

---

## BI-02 nginx exits immediately when `waddle-ws` is not resolvable

**Symptom**: `docker run localhost/waddle-frontend:latest` exits immediately
with `host not found in upstream "waddle-ws"`.

**Root cause**: nginx resolves all `proxy_pass` upstreams at startup time.
Outside a cluster, `waddle-ws` does not exist in DNS, so nginx refuses to
start.

**Status**: Known, accepted. The image is designed for k8s deployment where
the `waddle-ws` ClusterIP Service is resolvable via cluster DNS. Standalone
smoke-testing of the frontend image must be done with docker-compose (both
services running) or inside the k8s cluster. The Feature 3 verification step
in `PLAN.md` should use docker-compose for standalone verification, not a bare
`docker run`.
