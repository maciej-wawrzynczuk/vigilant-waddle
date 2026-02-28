# BUILD-ISSUES: Frontend

## BI-01 `.dockerignore` blocked frontend build context

**Symptom**: `docker build -f Dockerfile.frontend` failed — `nginx/nginx.conf`
not found in build context.

**Root cause**: `.dockerignore` contained `**/*` with only
`!target/debug/waddle-ws` as an exception; all other files were excluded.

**Fix**: Added `!frontend/`, `!frontend/**`, `!nginx/`, and `!nginx/nginx.conf`
exception rules to `.dockerignore`.

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
