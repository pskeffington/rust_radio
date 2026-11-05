# Codex agent instructions

## Central agent-skills doctrine (Codex development)

**Mandatory for all Codex development work.** Use the canonical [pskeffington/agent_skills](https://github.com/pskeffington/agent_skills) registry and preserve every repository-specific security, task, and release gate.

1. Before edits, inspect local and nested agent instructions, current roadmap/tasks, and tests. Resolve relevant agent roles and skill paths from the central registry. Record the **exact registry commit SHA** and skill revisions used; never invent missing skills.
2. Use specialized architect, implementation, test, and independent-review roles appropriate to the task. Do not substitute a generic coding pass when a relevant registered specialist exists.
3. Execute in isolated branches/worktrees. Document task ID, agent/skill invocations, registry revision, reproducible test commands, results, and blockers in the PR. Mentioning a skill without invocation evidence is not compliance.
4. Independently validate changes against existing CI, security and acceptance gates. If a necessary skill/tool is missing, record **BLOCKED-AGENT** rather than claiming successful compliance.
5. Submit sanitized reproducible failures and successful repairs to [FACTORY-002](https://github.com/pskeffington/agent_skills/issues/14) for independently evaluated improvement. Never share secrets, sensitive data, or private source into the public skills registry.
6. Once [FACTORY-003](https://github.com/pskeffington/agent_skills/issues/15) trusted invocation attestation and required `agent-compliance` status check are installed, require them for merge. Until verified, this file expresses policy, **not** technical enforcement.

**Precedence:** repository security/authorization and acceptance policies remain controlling; central skills supplement but never weaken them. No autonomous widening of permissions, bypass of CI, or unapproved production actions.
