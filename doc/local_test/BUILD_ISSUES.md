# Build Issues — Local Test Deployment

## Issue 1: Unnecessary inventory file for localhost deployment

**Problem:** Initial plan included creating an `inventory` file with `localhost ansible_connection=local`, but this is redundant when using `connection: local` directly in the playbook.

**Solution:** Removed inventory file from plan. Playbook uses `hosts: localhost` with `connection: local` directly, which is the standard Ansible pattern for local execution.

**Files affected:**
- `doc/local_test/PLAN.md` — removed inventory from all file manifests and verification steps
- `ansible.cfg` — kept minimal (only `host_key_checking = False`)

**Cycle/Stage/Phase:** local_test / Planning / Section 1
