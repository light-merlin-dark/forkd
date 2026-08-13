# Forkd runtime

Local checkout of [deeplethe/forkd](https://github.com/deeplethe/forkd) plus
Merlin's operator skill. This is the Firecracker/KVM substrate HyperAgent
lanes run on. It is **not** the HyperAgent product CLI and **not** the
remote seats.

## Skill entrypoints

- `$forkd` — this runtime. Load `skills/forkd/SKILL.md` (or
  `skill-sync resolve forkd`).
- `$hyperagent` — product loop: lanes, relay, seats, briefs, dispatch.

## Read first

1. `skills/forkd/SKILL.md`
2. `docs/plan.md`
3. HyperAgent `docs/runbooks/forkd-lanes.md` for the lane/relay contract

## Rules

- mac-mint only. Production has no `/dev/kvm`.
- Controller is loopback-only. Never bind LAN or a tunnel.
- Snapshot as `sudo HOME=/home/merlin XDG_DATA_HOME=/home/merlin/.local/share`
  so the controller can see the tag.
- Fix runtime defects here or upstream. Fix seat/product defects in
  HyperAgent or the capsule.
- Public Forkd PRs carry the Merlin signature. Operator files on
  `merlin/operator-skill` stay local.
