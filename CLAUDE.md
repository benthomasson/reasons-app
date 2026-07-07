# Reasons.app

Cargo workspace: `reasons-core` (library), `reasons-cli` (binary), `src-tauri` (Tauri app).

## Using reasons databases

When you need to interact with a reasons database, prefer the CLI over MCP tools — it's faster (one Bash call vs multiple MCP round trips).

### CLI (preferred in Claude Code)

```bash
# Search
reasons --db ~/reasons.db search "query"

# Add a premise
reasons --db ~/reasons.db add node-id "Belief text" --source "where I learned this"

# Add a derived belief
reasons --db ~/reasons.db add derived-id "Derived text" --sl antecedent-a,antecedent-b

# Show, explain, tree
reasons --db ~/reasons.db show node-id
reasons --db ~/reasons.db explain node-id
reasons --db ~/reasons.db tree node-id --direction both

# List with filters
reasons --db ~/reasons.db list --by-impact
reasons --db ~/reasons.db list --status OUT
reasons --db ~/reasons.db list --premises

# Retract and assert
reasons --db ~/reasons.db retract node-id --reason "why"
reasons --db ~/reasons.db assert node-id

# Challenge and defend
reasons --db ~/reasons.db challenge target-id --reason "objection"
reasons --db ~/reasons.db defend target-id --challenge-id challenge-target-id --reason "rebuttal"

# Nogood
reasons --db ~/reasons.db nogood node-a node-b
```

### Common mistakes to avoid

- **node_id format**: use kebab-case, no spaces, no special characters. Good: `climate-change-real`. Bad: `Climate Change is Real`.
- **Check before adding**: run `reasons --db DB search "topic"` before `add` to avoid duplicate nodes.
- **sl format**: comma-separated, no spaces. `--sl node-a,node-b` not `--sl "node-a, node-b"`.
- **Retract vs challenge**: use `retract` when a premise is simply wrong. Use `challenge` when you want to record WHY it's wrong (keeps both the belief and the objection visible).
- **OUT means "not justified"**: not "proven false". When reporting OUT nodes, explain the justification state.

### MCP tool schemas (skip discovery, call directly)

All tools accept optional `domain` (string). Omit for default domain.

- **domains()** — list configured domains. No params.
- **search(query: string, format?: string="markdown", depth?: int=1)** — full-text search with neighbor expansion
- **show(node_id: string)** — node details, justifications, dependents
- **explain(node_id: string)** — trace why a node is IN or OUT
- **tree(node_id: string, direction?: string="up", max_depth?: int)** — dependency tree ("up", "down", "both")
- **list(status?: string, premises?: bool=false, has_dependents?: bool=false, by_impact?: bool=false)** — list nodes with filters
- **add(node_id: string, text: string, sl?: string, unless?: string, source?: string, source_url?: string, label?: string)** — create node. `sl` and `unless` are comma-separated node IDs, no spaces.
- **retract(node_id: string, reason?: string)** — mark OUT with cascade
- **assert_node(node_id: string)** — restore to IN with cascade
- **challenge(target_id: string, reason: string, challenge_id?: string)** — attack a belief
- **defend(target_id: string, challenge_id: string, reason: string, defense_id?: string)** — counter a challenge
- **nogood(node_ids: string[])** — record contradiction, auto-backtrack

### Domain databases

Configured in `~/.reasons/domains.toml`. The CLI `--db` flag targets a specific file. The MCP server serves all configured domains.

## Development

- `cargo tauri dev` — run the menu bar app (must be from repo root)
- `cargo build` — build all crates
- The Tauri app is windowless (menu bar only, `LSUIElement`)
- MCP server runs on `localhost:6519` (HTTP) or stdio (CLI)
