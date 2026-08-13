---
name: forkd
description: "Operate Merlin's Forkd runtime on mac-mint — controller, snapshot,
  kernel/init contract, tap, and upstream PRs. Use when building or warming a
  HyperAgent lane capsule, when a squashfs parent boots systemd, when snapshot
  files land under /root, or when the user says $forkd / Forkd / Firecracker
  lane. This is the VM substrate, not the HyperAgent product CLI and not the
  remote seats."
model: inherit
metadata:
  skill-sync.visibility: routed
---

# Forkd

Source checkout: `/Users/merlin/_dev/forkd` (upstream `deeplethe/forkd`).
Installed controller: patched v0.5.3 from PR #303 on mac-mint.
Product loop: HyperAgent project (`$hyperagent`). Seats spend HyperAgent credits.

## Why this exists

HyperAgent seats must iterate plugin and app surfaces — see the page, see
tenant/fixture data, fix, HMR, prove — in parallel, without burning the host
allowance. Forkd is the isolated warmed runtime those seats sit in. Docker
stays the default until a real workload promotes it.

## Split

| Layer | Owns |
|---|---|
| **This repo / `$forkd`** | controller, snapshot, kernel cmdline, tap, binary, upstream PRs |
| **HyperAgent project** | `workbench lane`, effect relay, seats, briefs, dispatch |
| **Product capsule** | guest image, HMR, fixture vs production-truth, surface law |
| **HyperAgent seats** | the product work, paid on HyperAgent credits |

If the VM/init/snapshot is wrong, fix it here or upstream. If the seat cannot
see a plugin or land a PR, fix it in HyperAgent or the capsule.

## Host (mac-mint)

- `192.168.0.231`, VSSH user `merlin`
- Controller: `127.0.0.1:8889` only. Unit: `forkd-controller.service`
- Snapshot root: `/home/merlin/.local/share/forkd/snapshots`
- Kernel: `/var/lib/forkd/kernels/vmlinux`
- Tap: `forkd-tap0`

`sudo forkd snapshot` writes under `/root/.local/share/forkd`. Always:

```bash
sudo HOME=/home/merlin XDG_DATA_HOME=/home/merlin/.local/share \
  forkd snapshot --tag <tag> --kernel /var/lib/forkd/kernels/vmlinux \
  --rootfs <squashfs> --tap forkd-tap0 --boot-wait-secs 180 --mem-size-mib 4096
```

Then confirm `GET /v1/snapshots` lists the tag under the merlin snapshot root.

## Contracts paid for in this loop

- Squashfs/RO kernel cmdline does **not** pass `init=/forkd-init.sh`. Point
  `/sbin/init` and `/usr/sbin/init` at the capsule PID 1 or the parent boots
  systemd and never warms.
- Warmup handshake is one JSON line on the agent's stdout. Child processes
  must not inherit that stdout or `warmup_ready` stays false.
- A create/exec/destroy receipt is not a product gate. Seats still have to
  see the surface, compare data, and land a change.

## Granis admin session (HyperAgent.com)

The focused Chrome tab is often Cloudflare or Gmail, even when a HyperAgent
tab is already enabled. Never pair/session against the focused tab.

```bash
bun /Users/merlin/_dev/granis/granis-cli/granis.ts browser doctor --json
# pick enabledTabs[] where origin is https://hyperagent.com
bun /Users/merlin/_dev/granis/granis-cli/granis.ts browser session new \
  --expected-origin https://hyperagent.com \
  --instance <id> --window <windowId> --tab <tabId>
```

Durable trust ("Always allow local CLI sessions on this site") on the
HyperAgent origin makes `enable-tab` unattended. One session is enough for
`hyperagent -s <id>` admin calls.

## Health

```bash
vssh --host 192.168.0.231 --user merlin \
  'systemctl is-active forkd-controller.service; curl -fsS http://127.0.0.1:8889/healthz; curl -fsS http://127.0.0.1:8889/v1/sandboxes'
```

Idle inventory is `[]`. Stale registry, leftover Firecracker, or a snapshot
only under `/root` is a hard stop.

## Upstream

Public PRs to `deeplethe/forkd` carry the Merlin signature. Local operator
files (`skills/`, `AGENTS.md`, `docs/plan.md`) stay on `merlin/operator-skill`
and are not sent upstream. Runtime fixes that belong in Forkd itself go up
as focused PRs — current: #303 registry persistence.
