# Forkd — operator plan

Updated 2026-08-13. Runtime home for HyperAgent plugin lanes.

## Goal

Give HyperAgent seats a warmed, secret-free, isolated runtime so they can
iterate Stack Admin plugins, then Activate (Campaign Manager, canvasser, site
admin) and Aguatech (field owner, field technician), in parallel, on HyperAgent
credits.

## Current runtime

- Host: mac-mint, controller `forkd-controller.service` on `127.0.0.1:8889`
- Binary: patched v0.5.3; upstream PR #303 serializes registry persistence
- Snapshot tag `stack-admin-tasks-c41d006` is ready at
  `/home/merlin/.local/share/forkd/snapshots/stack-admin-tasks-c41d006`
  (4.0 GiB memory, squashfs root, Vite + effect + Chromium were live in the
  parent before pause)
- Next: HyperAgent `workbench lane create` from that tag, then effect relay
  and a paid Tasks builder

## Open runtime defects

1. Squashfs/RO cmdline omits `init=/forkd-init.sh` — capsule currently
   symlinks `/sbin/init`. Upstream should pass `init=` on the RO path too.
2. `sudo forkd snapshot` writes `$HOME` as `/root`. Documented workaround:
   set `HOME`/`XDG_DATA_HOME` to merlin. A `--snapshot-dir` flag would be
   the real fix.
3. Warmup first-line stdout handshake is fragile. Capsule now keeps child
   stdout off that pipe; Forkd should keep reading until `{ready:true}`.

## Branching

- `main` — upstream
- `pr-303` — registry persistence (installed on mac-mint)
- `merlin/operator-skill` — this skill and plan; do not push to deeplethe
