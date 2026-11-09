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
- **Focus screens.** Setup and onboarding are full screen with nothing else around. No sidebar for things that don't exist yet.
- **Copy.** Short sentences, plain words, no terminal commands in the app. Say what happens next.
