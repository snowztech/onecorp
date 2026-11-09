# Market

## Would people use it?

Yes, there's real demand. Paperclip got a lot of attention fast, and many developers already run several Claude Code or Codex sessions by hand, juggling terminals and worktrees. They want one place to run and watch them.

Likely users:

- **Solo founders and indie hackers** who want a team they can't afford to hire.
- **Developers with paid subscriptions** who want those subscriptions working overnight and while they're in meetings.
- **Small agencies** running several client projects at once.

The trap: org-sim tools get stars, then get abandoned when the novelty wears off. People keep using it only if it reliably ships code they'd merge. The fun gets them in, the output keeps them.

## Versus Paperclip

To check as Paperclip evolves. From what's known:

| | Paperclip | This |
|---|---|---|
| Goal | Autonomous "zero-human" company | One human with a small AI team |
| Form | Server plus web dashboard | Native desktop app |
| Setup | Node, database, adapters | Download and open |
| Config | In the app and database | `company.yaml` in git |
| Focus | Org structure and autonomy | Review speed, cost visibility, polish, personality |

Not better yet. Better only if:

1. Setup takes under 2 minutes.
2. You always know who's doing what and what it costs.
3. Reviewing their work is faster than doing it yourself.
4. It's genuinely pleasant to open every day.

The positioning gap is real: Paperclip pushes autonomy, this one keeps the founder in charge. Most people don't trust agents to run unsupervised yet.

## Risks

- **Platform risk.** Anthropic, OpenAI and others keep shipping multi-agent features inside their own tools. Being runtime-agnostic and owning the "company" layer (tickets, memory, review, cost) is the defense.
- **Paperclip catches up.** They have a head start and a community. Polish and DX are hard to copy quickly, but not impossible.
- **Agent quality ceiling.** Multi-agent teams still make messes. The review flow and small teams have to absorb that.
- **Cost surprises.** One runaway agent burning $50 kills trust. Hard caps are non-negotiable.
- **Scope.** Desktop app, adapters, MCP server, scheduler, UI. Easy to build half of everything. The MVP has to stay tiny.
