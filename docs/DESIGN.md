# OneCorp design

The rules every screen follows, so the app feels like one product. Read this before adding a screen.

## Feel

A developer tool first: neutral zinc greys, one crisp blue, tight type, mono labels, hairline borders. Quiet and calm. You run a company, the app shouldn't shout. The team's faces and desks bring the warmth. Everything around them stays plain.

## Brand

Our mascot is the OneCorp badge: an employee ID badge on its lanyard loop, with a face cut out of it. One color (`currentColor`), geometric, no gradients, so it sits in the monochrome UI like it belongs. It has moods like the team: happy, asleep, wink. It's the app icon (`docs/brand/icon.html`, rendered to `app/icons`), the mark next to the name, and the face of empty states. One source: `app/ui/mascot.js`. Use it where a moment deserves warmth, never as decoration on busy screens.

## System

- **Type.** Geist or Inter when installed, otherwise the system font. Titles weight 600 with tight tracking, never heavier. Mono for labels, keys, times and code. Page titles 18px, card titles 15px, body 14px, meta 12 to 13px.
- **Section labels.** 11px, uppercase, mono, `.label`. Used for sidebar groups, board columns, channels and side sections. One style, everywhere.
- **Radii.** `--r-sm` 6px for controls, `--r-md` 8px for cards, `--r-lg` 12px for big surfaces. Nothing else.
- **Cards.** Surface background, 1px `--line` border, no shadow. Never a card inside a card. Side columns are plain sections with a label, not boxes.
- **Color.** Accent for links, keys and the selected thing. Orange (`--signal`) only for what blocks on you. Green only for working or success. No colored rings around cards to get attention.
- **Buttons.** One primary button per view at most. Secondary actions are plain buttons. Free text and rare actions stay folded behind a small link until asked for.
- **Icons.** One inline stroke set (`ICON` in app.js), 16px, 1.8 stroke. Add new ones in the same style, never mix icon families.
- **Every object can be undone or removed where you see it.** People, tickets and companies each have their own quiet delete or let-go at the bottom of their drawer or sheet, behind an inline confirm that says what stays and what goes.
- **Where things live.** The sidebar holds pages only. Anything about you or the app (profile, settings, forgetting your profile) is in your menu at the bottom. Settings is a sheet over the app, never a page. The top bar says where you are and holds New ticket, nothing else.
- **What goes where.** The Company page is the office: your team, what needs you, what's happening. Planning and tickets live on the Board, decisions in the Inbox, knowledge in the Handbook.
- **Reassure, don't hide.** A summary that can be empty says so in a calm way ("All clear") instead of disappearing. Absence reads as broken; a check reads as good news.
- **Empty states.** A soft icon, a short headline that feels good ("You're all caught up"), one line on what shows up here. No dashed placeholder boxes.
- **Motion.** Quick and small, with one easing (`--ease`). Pages ease in when you switch to them, menus and popovers scale in, sheets rise, buttons press. Nothing animates on a background refresh, and everything turns off with Reduce Motion.
- **Focus screens.** Setup and onboarding are full screen with nothing else around: the steps on the left, the step on the right, over a faint grid. No sidebar for things that don't exist yet.
- **Copy.** Short sentences, plain words, no terminal commands in the app. Say what happens next.
