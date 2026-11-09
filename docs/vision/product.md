# Product

## Screens

1. **Office.** A floor of desks, one per employee. Monitors scroll code while they work, a "zz" floats over idle ones, blocked ones raise a speech bubble. Live activity feed on the side.
2. **Team.** Employee cards with status, current ticket, runtime and spend. Daily standup on top.
3. **Org chart.** You at the top, leads under you, the team under them. Questions go up the chart: engineers ask the CTO first, and only what leads can't decide reaches your inbox.
4. **Board.** Kanban. Tickets move by themselves. Click one for its thread, branch and artifacts.
5. **Chat.** Slack-style channels and DMs. @mention someone to get a reply.
6. **Inbox.** Everything that needs you: merges, questions, budget raises. One click to answer.
7. **Hiring.** Describe a role, get 3 candidates at different prices and runtimes. Give one a test task, then hire.
8. **Employee drawer.** Bio, personality, stats, runtime and model, permissions, live terminal, memory.

## Identity

Employees should feel like people who stick around.

- **Face.** Generated from a seed, so every hire gets a stable face for free. Skin, hair, glasses, headphones, hat.
- **Mood.** The face follows state: focused while working, worried when blocked, asleep when idle, smiling when something ships.
- **Personality.** A few trait sliders (Chatty, Bold, Picky), a quirk, a desk description, a handful of lines in their own voice.
- **Memory.** Past decisions, your preferences, repo facts. This is what makes them feel real after a few weeks.
- **Track record.** Tickets shipped, cost, review score, mistakes.
- **Real accounts (later).** GitHub bot user, email, Slack or Discord bot, so their work shows up under their name.

Rule: personality shapes tone in chat, standups and bubbles. It never goes into the prompt for actual work. Keeps it cheap and keeps quality up.

## Monitoring

- Status at a glance on every screen: working, blocked, idle, paused.
- Spend per employee and per company, live, with a cap.
- Live terminal per employee.
- Every event logged: prompts, tool calls, outputs, cost. Click any line to see the full context.
- One "Pause company" button that stops everything and all spending.

## Fun

- Daily standup written by the PM, readable in 20 seconds.
- Speech bubbles in each employee's voice.
- Confetti when something ships.
- Weekly report card: shipped, cost per ticket, most expensive mistake.
- Replay a day on a timeline.
- Later: moods that build over time, work anniversaries, employee of the week, unlockable desk items, optional sounds.
- Shareable company templates: `app new --from github.com/someone/indie-saas`.

## Review is the real product

The human is the bottleneck. Six agents can open more PRs than one person can read. The app wins or loses on how fast you can review and decide:

- Every PR comes with a short summary, what changed, how it was tested, and a preview link.
- The reviewer agent goes first, so you only see work that already passed a check.
- Inbox items have a default and a deadline ("I'll go with 15 minutes if you don't answer by 14:00").
