# ACCEPTANCE: Frontend

Reference spec: `doc/frontend/SPEC.md`
Date: 2026-02-28

## Results Summary

| # | Acceptance Criterion | Result |
| - | -------------------- | ------ |
| 1 | Page renders with upload control and portfolio table | PASS |
| 2 | Upload sends `PUT /transactions`; success message shown | PASS |
| 3 | Portfolio table populated after upload | PASS |
| 4 | Backend error shows descriptive status message | PASS |
| 5 | Page load: 200 → pre-populated; 404 → "No data loaded yet" | PASS |
| 6 | Zero external network requests (all assets served locally) | PASS |
| 7 | nginx forwards `/transactions` and `/portfolio` to waddle-ws | PASS |
| 8 | All pods Running after `--tags k8s` | PASS |
| 9 | All existing hurl E2E tests pass unchanged | PASS |

Overall: **PASS**

---

## Test Logs

### AC8 — k8s deployment (`ansible-playbook --tags k8s`)

```text
TASK [Build waddle-ws image for k8s]    changed: [localhost]
TASK [Build frontend image]             changed: [localhost]
TASK [Load waddle-ws image into kind]   changed: [localhost]
TASK [Load frontend image into kind]    changed: [localhost]
TASK [Apply k8s namespace]              changed: [localhost]
TASK [Apply k8s manifests]              changed: [localhost]
TASK [Wait for waddle-ws rollout]       ok: [localhost]
TASK [Wait for waddle-frontend rollout] ok: [localhost]

PLAY RECAP: ok=8  changed=6  failed=0
```

```text
$ kubectl get pods -n waddle -o wide
NAME                              READY  STATUS   AGE  NODE
waddle-frontend-68b7697b6d-6jhgr  1/1    Running  10s  kind-control-plane
waddle-ws-845dbbd4bc-4tmtf        1/1    Running  42s  kind-control-plane
```

### AC7 — nginx proxy / AC1 / AC3 / AC5 — frontend behaviour

Node IP: `172.18.0.2`, NodePort: `30080`

```text
$ curl -o /dev/null -w "%{http_code}" http://172.18.0.2:30080/
200

$ curl -o /dev/null -w "%{http_code}" http://172.18.0.2:30080/portfolio
404   # AC5: 404 before any upload

$ curl -X PUT -H "X-Overwrite: confirm" \
    -F "transaction_log=@hurl/1trans.csv;type=text/csv" \
    -o /dev/null -w "%{http_code}" \
    http://172.18.0.2:30080/transactions
200   # AC2: upload success via nginx proxy

$ curl http://172.18.0.2:30080/portfolio
FOO: 1    # AC3: portfolio populated; AC7: proxy working

$ curl -o /dev/null -w "%{http_code}" \
    http://172.18.0.2:30080/pico.min.css
200   # AC6: CSS served locally from nginx
```

### AC6 — zero external URLs in `frontend/index.html`

```text
$ grep -Eo 'https?://[^"'"'"' ]+' frontend/index.html
(no output)   # PASS
```

### AC9 — unit tests (`cargo test --lib`)

```text
running 7 tests
test transactions::test::portfolio_yaml_multiple_symbols ... ok
test transactions::test::portfolio_yaml_accumulated      ... ok
test transactions::test::portfolio_yaml_single           ... ok
test transactions::test::test_invalid_csv_fails          ... ok
test transactions::test::test_from_csv                   ... ok
test transactions::test::test_wrong_delimiter_fails      ... ok
test transactions::test::test_missing_required_fields    ... ok

test result: ok. 7 passed; 0 failed
```

### AC9 — E2E tests (`ansible-playbook --tags e2e`)

```text
TASK [Build release binary]   changed: [localhost]
TASK [Build test image]       changed: [localhost]
TASK [Spin up the container]  changed: [localhost]
TASK [Wait for the service]   ok: [localhost]
TASK [Test with hurl]         ok: [localhost]
TASK [Cleanup]                changed: [localhost]

PLAY RECAP: ok=6  changed=4  failed=0

hurl (--jobs 1, explicit order):
  empty_transactions.hurl:  Success (1 request)
  overwrite_rejected.hurl:  Success (2 requests)
  trans.hurl:               Success (2 requests)
  portfolio.hurl:           Success (2 requests)
  Executed: 4 files / 7 requests — 0 failures
```

---

## Issues Found During Acceptance

See `doc/frontend/BUILD-ISSUES.md` for full details.

| ID | Issue | Fix |
| -- | ----- | --- |
| BI-01 | `.dockerignore` blocked build context | Added exception rules |
| BI-02 | nginx exits standalone (no `waddle-ws` DNS) | Accepted; k8s-only |
| BI-03 | Wrong cluster tool (k3s instead of kind) | `kind load docker-image` |
| BI-04 | hurl v6 parallel mode broke e2e order | `--jobs 1` + explicit order |
| BI-05 | `frontend.yaml` applied before namespace | Apply namespace first |
