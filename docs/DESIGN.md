# OneCorp design

The rules every screen follows, so the app feels like one product. Read this before adding a screen.

## Feel

Quiet and calm. You run a company, the app shouldn't shout. The team's faces and desks bring the warmth. Everything around them stays plain.

## System

- **Type.** Bricolage Grotesque for titles, always weight 700. IBM Plex Sans for text, IBM Plex Mono for keys, times and code. Page titles 18px, card titles 15px, body 14px, meta 12 to 13px.
- **Section labels.** 11px, uppercase, `.label`. Used for sidebar groups, board columns, channels and side sections. One style, everywhere.
- **Radii.** `--r-sm` 8px for controls, `--r-md` 12px for cards, `--r-lg` 16px for big surfaces. Nothing else.
- **Cards.** Surface background, 1px `--line` border, no shadow. Never a card inside a card. Side columns are plain sections with a label, not boxes.
- **Color.** Accent for links, keys and the selected thing. Orange (`--signal`) only for what blocks on you. Green only for working or success. No colored rings around cards to get attention.
- **Buttons.** One primary button per view at most. Secondary actions are plain buttons. Free text and rare actions stay folded behind a small link until asked for.
- **Icons.** One inline stroke set (`ICON` in app.js), 16px, 1.8 stroke. Add new ones in the same style, never mix icon families.
- **Where things live.** The sidebar holds pages only. Anything about you or the app (profile, settings, forgetting your profile) is in your menu at the bottom. Settings is a sheet over the app, never a page. The top bar says where you are and holds New ticket, nothing else.
- **What goes where.** The Company page is the office: your team, what needs you, what's happening. Planning and tickets live on the Board, decisions in the Inbox, knowledge in the Handbook.
- **Empty states.** A soft icon, a short headline that feels good ("You're all caught up"), one line on what shows up here. No dashed placeholder boxes.
- **Motion.** Quick and small, with one easing (`--ease`). Pages ease in when you switch to them, menus and popovers scale in, sheets rise, buttons press. Nothing animates on a background refresh, and everything turns off with Reduce Motion.
- **Focus screens.** Setup and onboarding are full screen with nothing else around. No sidebar for things that don't exist yet.
- **Copy.** Short sentences, plain words, no terminal commands in the app. Say what happens next.
