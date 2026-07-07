---
name: reasons-mcp
description: Long-term semantic memory backed by the reasons MCP server — a justification-based truth maintenance system (JTMS) that stores what you learned, WHY each belief holds, and auto-flips conclusions when a premise is retracted, challenged, or contradicted. Context is working memory and ends with the session; reasons is where knowledge survives. USE THIS at the START of any task in a domain you may have worked before (search before re-deriving), DURING work whenever you learn a durable fact, decide something, or get corrected (record immediately), and at session END (curate). Also use when the user says "remember this," asks what you learned before, or the task involves claims with sources, default rules with exceptions ("birds fly unless penguin"), conflicting evidence, argument mapping, or "what if we drop this assumption." Trigger on mentions of reasons MCP, belief graphs, truth maintenance, or JTMS — even without these exact words.
---

# Reasons MCP: Long-Term Semantic Memory with Justifications

## What this is

Your context window is **working memory**: fast, rich, and gone when the session ends or compacts. The reasons server is **long-term semantic memory**: a persistent store of justified beliefs that survives session boundaries, model swaps, and compaction.

It is not a note-taking tool or a knowledge graph. It is a **JTMS** (justification-based truth maintenance system — Doyle, de Kleer): every belief is **IN** (currently believed) or **OUT** (not currently believed), and that status is *computed* from the justification graph, not set by hand. Retract one premise and everything that depended on it flips OUT automatically. That is the value: conclusions stay honest as evidence changes, and any belief can answer "why do you hold this?" via its chain.

Memory that is written but never read is a log, not memory. The protocol below exists to close the loop.

## The session lifecycle protocol

**START — read before you think.**
1. `domains` — see what belief databases exist. Pick the domain for this project if one is configured.
2. `search` the store for the task's key entities and terms (2–4 searches, `depth=1`). Past sessions may have already learned what you are about to re-derive.
3. For any load-bearing hit, call `explain` before trusting it — confirm it is IN and see what it rests on. Never treat an OUT belief as false (see Gotchas); treat it as "reasons currently insufficient."

**DURING — write the moment you learn.**
- Learned a durable fact from a tool output, file, or experiment → `add` a premise immediately, with `source`. Do not batch it for later; later steps may depend on it and later may not come.
- Reached a conclusion by combining existing beliefs → `add` a derived node with `sl` pointing at the belief IDs you combined. This preserves the "how did I get here" chain for `explain`.
- The user corrected you, or new evidence contradicts a stored belief → do not silently overwrite. `challenge` the old belief with the reason (preferred — keeps both the belief and the objection visible), or `retract` if it should simply leave the active set. If two premises genuinely conflict, record a `nogood` and check what backtracking dropped.
- **Before every `add`: search for near-duplicates.** Different sessions mint different vocabulary for the same fact; parallel phrasings later become undetected contradictions. If a near-duplicate exists, extend it, cite it, or challenge it — do not mint a twin.

**END — curate before the window closes.**
1. `list --by_impact true` — review the load-bearing beliefs you touched; `explain` or `tree` your key conclusions to confirm they hold together.
2. Delete-by-retraction anything that was session scaffolding rather than durable knowledge (see "What not to record").
3. If you are reporting results to the user, the belief IDs behind your conclusions are your audit trail — offer them.

## Writing beliefs a future reader can use

Every belief you store will be read by an agent (possibly you) with **none of this session's context**. Write for that reader:

- **Self-contained text.** No pronouns, no "the bug," no "this repo." Bad: `fixed the connection issue`. Good: `acme-api: connection resets under load were caused by keepalive timeout 5s < LB idle timeout 60s; fix is keepalive 75s`. Name the entities explicitly.
- **Provenance classes in `source`.** Distinguish how you know, because these decay differently and future contradiction-handling must know which side is a measurement:
  - `observed:` — you ran it / read it directly (`observed: pytest output 2026-07-07`, `observed: file src/config.py`)
  - `told:` — user or another agent asserted it (`told: user, 2026-07-07`)
  - `inferred:` — your own reasoning (usually better expressed as a derived node with `sl`, so the inference is inspectable)
  - Use `source_url` for anything fetchable; include dates for anything that can go stale.
- **No totalizing claims.** "X never works," "always do Y," "every Z fails" — store the bounded version you actually observed (`X failed in configurations A and B`). Universal quantifiers from single observations are how memory rots.
- **One fact per node.** Conjunctions bundling unrelated claims cannot be individually retracted later.
- **Stable, prefixed IDs.** `projectname-topic-claim` (e.g., `acme-api-keepalive-timeout-fix`). The prefix is your namespace within a shared domain and prevents cross-project ID collisions.

## What NOT to record

The store persists and may be shared across sessions, agents, and readers. Never store: secrets, tokens, passwords, PII, or private user details; ephemeral session state ("currently editing file X"); speculation phrased as fact (store it as a challenge-able hypothesis with `inferred:` provenance, or not at all); duplicate phrasings of existing beliefs. The value of the store is curation — the justified absence of most possible beliefs.

## Core concepts

- **Node**: a belief with an ID and text. **Premise** = asserted directly, no justification. **Derived** = believed only because of its justification — an `sl` (support list) of antecedent node IDs that must all be IN.
- **IN / OUT**: computed truth value. OUT ≠ false; it means "not currently justified."
- **`unless` (default reasoning)**: a justification may include an outlist — nodes that, if IN, knock this node OUT. Encodes "true by default, unless exception." Example: `tweety-flies` with `sl=[is-a-bird,birds-fly-by-default]`, `unless=[is-a-penguin]`.
- **Cascading propagation**: retract/challenge/defend recomputes every downstream dependent. Always assume ripples.
- **Contradiction (`nogood`)**: declares a set of nodes mutually inconsistent. If all are IN, dependency-directed backtracking retracts the least-entrenched premise to restore consistency.
- **Domains**: separate belief databases, configured in `~/.reasons/domains.toml`. **You cannot create one by naming it in a tool call** (fails with "unknown domain"). Prefer one domain per project where a human has provisioned it — small, concentrated stores retrieve better than one giant mixed store. Where you cannot get a domain, use ID prefixes as the namespace.

## Tool reference

| Tool | Purpose | Key params |
|---|---|---|
| `add` | Create a node. Omit `sl` for a premise; include it for derived. | `node_id`, `text`, `sl`, `unless`, `label`, `source`, `source_url` |
| `show` | Full detail on one node. | `node_id` |
| `explain` | Human-readable trace of why a node is IN/OUT. Use before `tree` when you need reasoning, not shape. | `node_id` |
| `tree` | Justification graph visualization. `up` = depends on, `down` = dependents, `both`. | `node_id`, `direction`, `max_depth` |
| `list` | Enumerate nodes; filter by `status`, `premises`, `has_dependents`; `by_impact` sorts most-depended-on first (find load-bearing assumptions). | — |
| `search` | Full-text search with neighbor expansion (`depth`). First move of every session. | `query`, `depth` |
| `challenge` | Attack a belief: creates a challenge node forcing the target OUT. | `target_id`, `reason`, `challenge_id?` |
| `defend` | Counter a challenge, restoring the target to IN. | `challenge_id`, `target_id`, `reason`, `defense_id?` |
| `retract` | Manually mark a node OUT. Cascades. | `node_id`, `reason` |
| `assert_node` | Restore a retracted node to IN. Cascades. | `node_id` |
| `nogood` | Declare nodes mutually contradictory; triggers backtracking if all IN. | `node_ids` (list) |
| `domains` | List configured domains. Call first in any session. | — |

Every tool except `domains` accepts optional `domain`.

### Full parameter schemas (skip tools/list discovery)

All params are strings unless noted; `?` marks optional.

- **domains()** — no params
- **search(query, format?="markdown", depth?=1, domain?)** — `depth` (int): neighbor-expansion hops
- **show(node_id, domain?)**
- **explain(node_id, domain?)**
- **tree(node_id, direction?="up", max_depth?, domain?)** — `direction`: "up"|"down"|"both"
- **list(status?, premises?=false, has_dependents?=false, by_impact?=false, domain?)** — the three flags are bools
- **add(node_id, text, sl?, unless?, source?, source_url?, label?, domain?)** — `sl`/`unless`: comma-separated node IDs, no spaces
- **retract(node_id, reason?, domain?)**
- **assert_node(node_id, domain?)**
- **challenge(target_id, reason, challenge_id?, domain?)**
- **defend(target_id, challenge_id, reason, defense_id?, domain?)**
- **nogood(node_ids, domain?)** — JSON array of strings

## Workflow patterns

**`unless` vs. `challenge`/`defend` for exceptions.** The difference is *when you learn about the exception*:
- Exception known at creation time (modeling a policy with a known carve-out) → bake it in with `unless` at `add` time. Cheaper and more direct.
- Exception discovered later (evidence arrives, an objection surfaces mid-conversation) → there is no edit tool to retrofit an outlist; use `challenge` to inject a defeater against the existing node, and `defend` if the objection itself falls. Real workflows mostly discover exceptions as they go, so default to `challenge`/`defend`.

**Default rule with a known exception:**
1. `add` supporting premises (`is-a-bird`, `birds-fly-by-default`).
2. `add` the exception premise (`is-a-penguin`) — now or when it becomes relevant.
3. `add` the conclusion with `sl` on the supports and `unless` on the exception.
4. If the exception is IN, the conclusion computes OUT automatically; confirm with `explain`.

**Conflicting evidence:**
1. `add` both claims as premises (both IN initially), each with its provenance class.
2. `nogood` with both IDs. Backtracking retracts the less-entrenched one — read the returned message to see which and why.
3. **Check the survivor's provenance.** If backtracking kept an `inferred:` belief and dropped an `observed:` one, override it: `retract` the inference, `assert_node` the observation. Entrenchment is a heuristic; observations should outrank inferences.

**Debate / argument mapping:**
1. `add` each claim with a clear `source`.
2. `challenge` for each rebuttal; `defend` when a rebuttal is answered.
3. `tree` with `direction: both` to see the full exchange. Challenges and defenses are themselves nodes and can be challenged in turn — nested objections are expected.

**Sanity-check before relying on a conclusion:** `explain` (reasoning) or `tree` (shape) on any derived node before treating its status as final — especially after `challenge`, `defend`, `retract`, or `nogood`, which can cascade further than expected. `list --by_impact true` shows where scrutiny pays most.

## Gotchas

- `node_id` must be unique per domain. Collision behavior on `add` (error vs. overwrite) is server-version dependent — treat a collision as potentially destructive and always `search` first. This is one more reason the dedup-before-add rule is mandatory, not advisory.
- Unknown `domain` fails; domains are created by editing `~/.reasons/domains.toml` and restarting the server, not by tool call.
- **OUT does not mean disproven.** Never narrate an OUT node to the user as false; run `explain` and describe the actual justification state.
- Prefer `challenge` over silent `retract` when evidence turns against a belief — it preserves *why* the belief fell, which is itself knowledge the next session needs.
