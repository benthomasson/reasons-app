---
name: reasons-mcp
description: Use the reasons MCP server to build and maintain a justification-based truth maintenance system (JTMS) — a graph of beliefs that tracks WHY each claim holds and automatically re-derives what's true when an assumption changes. Reach for this whenever a task involves tracking claims alongside their justifications and sources, modeling default rules with exceptions ("birds fly unless the bird is a penguin"), handling evidence that conflicts or contradicts, mapping out an argument or debate so each side's points and rebuttals are explicit, running "what if we drop this assumption" scenarios, or maintaining any knowledge base where conclusions need to automatically flip when an underlying premise is retracted, challenged, or found to contradict something else. Also trigger this whenever the user mentions the reasons MCP, a "belief graph," "truth maintenance," "JTMS," or asks to track hypotheses/claims with confidence that updates as new information arrives — even if they don't use these exact words.
---

# Reasons MCP: Justification-Based Truth Maintenance

## What this actually is

The reasons MCP server is not a note-taking or knowledge-graph tool — it's a **JTMS** (justification-based truth maintenance system), the classic AI technique (Doyle, de Kleer) for keeping a set of beliefs consistent as evidence changes. The core idea: every belief node is either **IN** (currently believed) or **OUT** (not currently believed), and that status is *derived*, not something you set directly. You add premises and rules; the server computes IN/OUT by propagating through the justification graph, and it keeps that propagation correct automatically whenever something upstream changes.

This matters because it's fundamentally different from just appending facts to a list. If you retract a premise, everything that depended on it flips OUT automatically — you never have to manually track "what else does this affect." That's the entire value of the tool: use it for anything where conclusions are contingent and evidence is going to shift.

## Core concepts

- **Node**: a belief with an ID and text. Two kinds:
  - **Premise**: asserted directly, no justification (e.g., a fact, an assumption, a piece of evidence).
  - **Derived**: believed only because of its justification — an `sl` (support list) of antecedent node IDs that must all be IN.
- **IN / OUT**: a node's truth value. IN = currently believed; OUT = not currently believed (not the same as "false" — it just means the reasons in its favor aren't currently holding).
- **`unless` (default reasoning)**: a justification can include an outlist — node IDs that, if IN, knock this node OUT. This is how you encode "true by default, unless there's a specific exception." Example: "Tweety flies" is justified by `sl=[is-a-bird, birds-fly-by-default]`, `unless=[is-a-penguin]`. If `is-a-penguin` later becomes IN, "Tweety flies" automatically flips OUT — no manual bookkeeping needed.
- **Cascading propagation**: retracting, challenging, or defending a node doesn't just change that node — it recomputes every downstream dependent. Always assume effects ripple; that's the point of the system.
- **Contradiction (`nogood`)**: when two or more IN nodes can't both be true, recording a nogood tells the system so. If it finds all of them currently IN, it runs dependency-directed backtracking and retracts the least-entrenched premise among them to restore consistency — this is a form of automatic conflict resolution, not just a flag.
- **Domains**: separate belief databases. Nodes in one domain are invisible to another. Domains are configured in `~/.reasons/domains.toml` — you cannot create a new one just by naming it in a tool call (attempting to do so errors with "unknown domain"). Call `domains` first if you're unsure what's available; default is usually called `default`.

## Tool reference

| Tool | Purpose | Key params |
|---|---|---|
| `add` | Create a node. Omit `sl` for a premise; include it for a derived node. | `node_id`, `text`, `sl` (comma-separated antecedents), `unless` (comma-separated outlist), `label`, `source`, `source_url` |
| `show` | Full detail on one node: text, justifications, timestamps. | `node_id` |
| `explain` | Human-readable trace of *why* a node is IN or OUT. Use this before `tree` when you just need the reasoning, not the shape. | `node_id` |
| `tree` | Box-drawing visualization of the justification graph. `direction: up` = what this depends on, `down` = what depends on this, `both`. | `node_id`, `direction`, `max_depth` |
| `list` | Enumerate nodes, filterable by `status` (IN/OUT), `premises` (only unjustified nodes), `has_dependents`, sortable `by_impact` (most depended-on first — good for finding load-bearing assumptions). | — |
| `search` | Full-text search with neighbor expansion (`depth`). Good for finding a node when you don't remember its exact ID. | `query`, `depth` |
| `challenge` | Attack a belief: creates a challenge node that forces the target OUT. Use for "actually, here's a reason to doubt this." | `target_id`, `reason`, `challenge_id` (optional) |
| `defend` | Counter a challenge: creates a defense node that knocks the challenge back down, restoring the original target to IN. Use for "the objection doesn't hold up, here's why." | `challenge_id`, `target_id`, `reason`, `defense_id` (optional) |
| `retract` | Manually mark a node OUT (e.g., "we no longer believe this premise"). Cascades to dependents. | `node_id`, `reason` |
| `assert_node` | Re-assert a previously retracted node, restoring it to IN with cascading propagation. | `node_id` |
| `nogood` | Declare a set of nodes as mutually contradictory. If all are IN, triggers automatic backtracking to retract the weakest premise. | `node_ids` (list) |
| `domains` | List configured domains/databases and their file paths. Call this first when working with non-default domains. | — |

Every tool except `domains` accepts an optional `domain` parameter to target a non-default belief database.

## Workflow patterns

**Choosing `unless` vs. `challenge`/`defend` for exceptions.** Both encode "this doesn't hold after all, unless something defeats the defeater" — the difference is *when you learn about the exception*:
- If you already know the exception condition when you create the node (e.g., you're modeling a policy that has a known carve-out), bake it in with `unless` at `add` time. This is the cheaper, more direct option.
- If the node already exists and new information surfaces afterward that should knock it down (evidence comes in later, someone raises an objection mid-conversation), there is no "edit" tool to retroactively add an outlist entry — use `challenge` instead. It effectively injects a new defeater against an existing node without needing to recreate it, and `defend` lets you knock the challenge back down if the objection itself doesn't hold up. In practice, most real workflows *discover* exceptions as they go, so reach for `challenge`/`defend` unless you're confident you know every exception up front.

**Default rule with an exception known upfront** (the standard JTMS pattern — "X, unless Y"):
1. `add` the premises (e.g., `is-a-bird`, `birds-fly-by-default`).
2. `add` the exception premise (e.g., `is-a-penguin`) — even if you don't yet know if it applies, or add it later when it becomes relevant.
3. `add` the derived conclusion with `sl` pointing at the supporting premises and `unless` pointing at the exception.
4. If the exception premise is IN, the conclusion computes as OUT automatically. Use `explain` to confirm why.

**Debate / argument mapping:**
1. `add` each claim as a premise or derived node with a clear `source`.
2. Use `challenge` for each rebuttal against a claim, with a `reason` describing the objection.
3. Use `defend` when a rebuttal itself is answered, restoring the original claim.
4. Use `tree` with `direction: both` to see the full back-and-forth at a glance.

**Conflicting evidence:**
1. `add` both conflicting claims as premises (they'll both be IN at first).
2. Call `nogood` with both node IDs. If both are genuinely IN, the system automatically backtracks and retracts the less-entrenched one — check the returned message to see which one it dropped and why.
3. If the automatic choice is wrong, `retract` the other one manually and `assert_node` to restore your preferred claim.

**Sanity-checking before you rely on a conclusion:**
Always call `explain` (for the reasoning) or `tree` (for the shape) on a derived node before treating its IN/OUT status as final — especially after any `challenge`, `defend`, `retract`, or `nogood` call, since those can cascade further than expected. `list --by_impact true` is useful for finding out which premises the most conclusions depend on, i.e. where to focus scrutiny.

## Recording your own work as you go

While working on a task, treat the default domain as a running log of what you learn, not just a place to model someone else's argument:

- **Learn something new (single fact)** → `add` it as a premise with `source` set to where you learned it (file path, URL, tool output). Don't wait until the end of the task to log it — add it the moment you learn it, since later steps may depend on it.
- **Learn something new by combining two or more existing beliefs** → `add` a derived node with `sl` pointing at the belief IDs that led you there. This makes the inference itself inspectable later via `explain`/`tree`, instead of losing the "how did I get here" chain.
- **Find something that contradicts an existing belief** → don't just overwrite it:
  - If you're now confident the old belief was wrong and don't expect to need it back, `challenge` it with a `reason` explaining the contradiction (preferred — keeps the old belief and the objection both visible), or `retract` it if it should simply be gone from the active set.
  - If the contradiction is fully resolved by adding the new information rather than a specific rebuttal, `challenge`/`defend` is more honest than silently retracting — it preserves *why* the old belief no longer holds.
- **Before wrapping up or reporting results**, `list --by_impact true` or `tree` on your key conclusions to sanity-check the beliefs you built up during the task actually hold together, and to give the human a way to audit your reasoning after the fact.

## Gotchas

- `node_id` must be unique per domain; reusing an ID for `add` will likely error or overwrite — use `search` or `list` first if unsure whether something already exists.
- Passing a `domain` that hasn't been configured on the server fails with "unknown domain" — this system does not create domains on the fly. To add a new domain, edit `~/.reasons/domains.toml` and restart the MCP server.
- OUT does not mean "proven false" — it means "not currently justified." Don't narrate an OUT node to the user as disproven; explain the actual justification state with `explain`.
- `challenge`/`defend` are themselves nodes in the graph (they get their own IDs), so they show up in `list` and can be challenged/defended in turn — nested objections are supported and expected in longer debates.
