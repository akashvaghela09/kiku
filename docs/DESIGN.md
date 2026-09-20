# kiku - Design Direction

**Version** 1.0 · **Status** Proposed, opinionated · **Scope** Overlay, Main window, Settings, Onboarding
**Stack** Tauri 2 · React · TypeScript · Tailwind · Lucide only · Light + dark following OS

---

## 0. The one-sentence design thesis

> kiku is a **system instrument**, not an app. The user's attention belongs to the document they are
> dictating into; kiku's entire visual job is to confirm, peripherally and without being looked at,
> that it is listening - and then to get out of the way.

Everything below is derived from that sentence. Where a decision could go either way, it goes toward
*less presence*.

---

## 1. Design principles

### P1. The overlay is read peripherally, never focally.
A user in flow does not look at the overlay. They perceive a change in the corner of their eye and
keep talking. Anything that requires foveal vision to decode - text, small icons, numeric levels,
multi-colour states - has failed.

**Implication:** the listening state contains **zero text and zero icons**. It is a shape and a
motion. State is encoded in *silhouette* (width), *motion signature* (moving vs. pulsing vs. static),
and *one* colour shift - in that priority order. If the overlay were rendered at 25% scale and blurred
by 4px, a user must still be able to name the state. That is the acceptance test for every overlay
state we ship.

### P2. Latency is a design material; perceived latency is the one we control.
Speech recognition takes what it takes. The *feedback* must not. The gap between physical key-down and
first pixel is the single number that decides whether kiku feels premium or cheap.

**Implication:** the overlay OS window is created at app launch and kept alive and hidden forever -
never created on demand (X11 window creation is 30-120ms; that is 4-7 dropped frames the user reads as
lag). Showing it is `setVisible(true)` plus one CSS class toggle. Budget: **≤ 16ms key-down → first
paint**. Correspondingly, the `processing` state is never allowed to be instantaneous-and-invisible:
if the transcript returns in under 180ms we still hold `processing` for 180ms, because a state that
flashes for 3 frames reads as a glitch, not as speed.

### P3. Every pixel of chrome is a CPU cycle stolen from the recogniser.
The webview and the ASR run on the same machine at the same moment. A 32px backdrop blur on a
400×400 region is not free; neither are eleven simultaneous keyframe animations.

**Implication:** a hard rule - **only `opacity` and `transform` animate**, with exactly one sanctioned
exception (the overlay capsule's `width`, on a ≤260px element). No `backdrop-filter` anywhere. No
animated `box-shadow`, `filter`, `border-radius`, or `height`. Maximum **two** concurrent animations
process-wide while `listening` is active. The main window's render loops pause on blur.

### P4. Trust is the feature; state it in words, don't imply it in vibes.
"Local-first, no cloud" is the entire reason someone chooses kiku over the incumbent. Privacy claims
communicated only through aesthetic restraint are not communicated.

**Implication:** every place where the app touches the network or the disk says so, inline, in plain
sentences, at the point of the control - not in an About box. The update-check toggle carries the line
*"Checks a version number once a day. Sends nothing about you."* The history section carries
*"Text only. Audio is discarded the moment it is transcribed."* This is copy as UI, and the copy is
part of the design spec, not filler.

### P5. Small surface, deep polish - refuse the second thing.
Four surfaces, roughly twenty settings, one primary interaction. The owner has rejected feature creep;
design must hold that line by making the existing set *feel finished* rather than by adding.

**Implication:** a deliberately tiny component set (14 primitives, §6) and a rule that no surface gets
a control the other surfaces cannot reuse. Concretely: no dashboard, no stats, no streaks, no "recent
apps", no AI post-processing UI, no vocabulary manager, no cloud sync affordance - not even a disabled
one. **One exception I am arguing for, in §11.2: inline editing of a transcript.** It is error
recovery for the product's core failure mode, not a feature.

---

## 2. Design tokens

### 2.1 Brand constants

Sampled from the icon; these three are fixed and never re-derived.

```
--brand-cyan   #22A7CC   left hemisphere of the mark
--brand-teal   #2A94B2   right hemisphere of the mark
--brand-white  #FFFFFF   face + waves
```

These are **mark colours**. `#22A7CC` is a fill and graphic colour; it is *not* a text colour on white
(2.8:1 against `#FFFFFF`) and white text on it fails too. The UI accent ramp below exists precisely so
nobody reaches for the brand hex when they need contrast.

### 2.2 Accent ramp (`cyan`)

`500` is the brand. Everything else is derived by lightness, hue held between 194° and 198° so the ramp
reads as one family with the mark.

| Step | Hex | Use |
|---|---|---|
| 50  | `#EDFAFE` | accent wash (selected row, light) |
| 100 | `#D0F2FC` | accent wash hover, light |
| 200 | `#A5E6F8` | subtle accent border, light |
| 300 | `#6BD3EF` | accent border, dark |
| 400 | `#38BDE0` | **accent text/icon on dark**, focus ring on dark |
| 500 | `#22A7CC` | **brand fill**, waveform bars, mark |
| 600 | `#1C8FAF` | hover of 700, large-text accent on light |
| 700 | `#17738F` | **primary button fill (light)**, accent text on light |
| 800 | `#185E74` | primary button pressed (light) |
| 900 | `#184E60` | accent wash on dark |
| 950 | `#0D3140` | accent wash on dark, subtle |

### 2.3 Neutral ramp (`ink`) - hue-tinted ~205° so neutrals belong to the cyan family

| Step | Hex | | Step | Hex |
|---|---|---|---|---|
| 0   | `#FFFFFF` | | 600 | `#53616C` |
| 25  | `#FBFCFD` | | 700 | `#3F4B55` |
| 50  | `#F6F8FA` | | 800 | `#2B343C` |
| 100 | `#EDF1F4` | | 850 | `#222A31` |
| 200 | `#DDE4E9` | | 900 | `#191F25` |
| 300 | `#C3CDD5` | | 925 | `#14191E` |
| 400 | `#94A3AE` | | 950 | `#10151A` |
| 500 | `#64727E` | | 1000| `#0A0E12` |

### 2.4 Status ramps (minimal - two steps each, light fill + dark fill)

```
success   light #0F7A55   dark #3DDC97   wash-light #E6F7F0   wash-dark #08281D
warning   light #9A5B00   dark #F5B547   wash-light #FDF3E2   wash-dark #2E1F06
danger    light #B4291B   dark #FF7A6B   wash-light #FDECEA   wash-dark #2E110D
```

Note the deliberate absence of an "info" colour: info *is* the accent. One fewer semantic to misuse.

### 2.5 CSS custom properties

```css
:root {
  /* ---- fixed brand ---- */
  --brand-cyan: #22A7CC;
  --brand-teal: #2A94B2;

  /* ---- surfaces ---- */
  --surface:          #FFFFFF;  /* window background */
  --surface-raised:   #FFFFFF;  /* cards, popovers - same, separated by border not fill */
  --surface-sunken:   #F6F8FA;  /* list wells, code/transcript blocks, inputs */
  --surface-hover:    #F6F8FA;
  --surface-active:   #EDF1F4;
  --surface-selected: #EDFAFE;  /* cyan-50 */
  --surface-overlay-scrim: rgb(16 21 26 / 0.40);

  /* ---- borders ---- */
  --border:        #DDE4E9;  /* default hairline */
  --border-strong: #C3CDD5;  /* inputs, focusable controls at rest */
  --border-subtle: #EDF1F4;  /* list dividers */
  --border-accent: #A5E6F8;

  /* ---- text ---- */
  --text-primary:   #191F25;  /* 17.0:1 on surface */
  --text-secondary: #53616C;  /*  6.4:1 on surface */
  --text-muted:     #64727E;  /*  5.0:1 on surface */
  --text-disabled:  #94A3AE;  /*  2.7:1 - non-text / disabled only */
  --text-on-accent: #FFFFFF;  /*  5.4:1 on --accent */
  --text-accent:    #17738F;  /*  5.6:1 on surface */

  /* ---- interactive accent ---- */
  --accent:         #17738F;
  --accent-hover:   #1C8FAF;
  --accent-active:  #185E74;
  --accent-graphic: #22A7CC;  /* waveform, mark, non-text marks only */
  --accent-wash:    #EDFAFE;

  /* ---- status ---- */
  --success: #0F7A55;  --success-wash: #E6F7F0;
  --warning: #9A5B00;  --warning-wash: #FDF3E2;
  --danger:  #B4291B;  --danger-wash:  #FDECEA;  --danger-hover: #8F2015;

  /* ---- focus ---- */
  --ring:        #1C8FAF;
  --ring-offset: var(--surface);

  /* ---- elevation (static; never animated) ---- */
  --shadow-sm: 0 1px 2px rgb(16 21 26 / 0.06);
  --shadow-md: 0 2px 8px rgb(16 21 26 / 0.08), 0 1px 2px rgb(16 21 26 / 0.06);
  --shadow-lg: 0 8px 24px rgb(16 21 26 / 0.12), 0 2px 6px rgb(16 21 26 / 0.08);

  /* ---- radii ---- */
  --r-sm: 4px; --r-md: 6px; --r-lg: 8px; --r-xl: 12px; --r-pill: 9999px;
}

/* Dark values live in one place and are applied by two selectors.
   Tauri exposes the OS theme; the app writes data-theme="light|dark" on <html>,
   and the media query covers the frames before hydration so there is no flash.
   The explicit attribute always wins because it is listed last. */
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) { /* ...identical block to the one below... */ }
}
```

```css
:root[data-theme="dark"] {
  --surface:          #14191E;
  --surface-raised:   #191F25;
  --surface-sunken:   #10151A;
  --surface-hover:    #1E252C;
  --surface-active:   #222A31;
  --surface-selected: #0D3140;
  --surface-overlay-scrim: rgb(10 14 18 / 0.56);

  --border:        #2B343C;
  --border-strong: #3F4B55;
  --border-subtle: #222A31;
  --border-accent: #184E60;

  --text-primary:   #E6EDF3;  /* 15.0:1 on surface */
  --text-secondary: #A3B1BC;  /*  8.1:1 */
  --text-muted:     #7A8894;  /*  4.9:1 */
  --text-disabled:  #53616C;  /*  2.4:1 - non-text only */
  --text-on-accent: #08191F;  /* dark ink on light-cyan fill: 9.6:1 */
  --text-accent:    #38BDE0;  /*  8.1:1 */

  --accent:         #38BDE0;
  --accent-hover:   #6BD3EF;
  --accent-active:  #22A7CC;
  --accent-graphic: #38BDE0;
  --accent-wash:    #0D3140;

  --success: #3DDC97;  --success-wash: #08281D;
  --warning: #F5B547;  --warning-wash: #2E1F06;
  --danger:  #FF7A6B;  --danger-wash:  #2E110D;  --danger-hover: #FF9A8E;

  --ring:        #38BDE0;
  --ring-offset: var(--surface);

  --shadow-sm: 0 1px 2px rgb(0 0 0 / 0.40);
  --shadow-md: 0 2px 8px rgb(0 0 0 / 0.44), 0 1px 2px rgb(0 0 0 / 0.36);
  --shadow-lg: 0 8px 24px rgb(0 0 0 / 0.52), 0 2px 6px rgb(0 0 0 / 0.40);
}
```

**Overlay palette - deliberately theme-independent.** See §5.2 for the justification.

```css
:root {
  --ov-bg:        rgb(16 21 26 / 0.94);   /* ink-950 @ 94% */
  --ov-hairline:  rgb(255 255 255 / 0.10);
  --ov-bar:       #38BDE0;                /* cyan-400 - reads on light and dark backdrops */
  --ov-bar-idle:  rgb(56 189 224 / 0.45);
  --ov-text:      #E6EDF3;
  --ov-danger:    #FF7A6B;
  --ov-success:   #3DDC97;
  --ov-shadow:    0 8px 28px rgb(0 0 0 / 0.38);
}
```

### 2.6 Contrast audit

Measured WCAG 2.1 contrast, sRGB.

| Pair | Light | Dark | Verdict |
|---|---|---|---|
| text-primary / surface | 17.0 | 15.0 | AAA |
| text-secondary / surface | 6.4 | 8.1 | AA (AAA for ≥18.66px) |
| text-muted / surface | 5.0 | 4.9 | AA - this is the floor; never go lighter for real text |
| text-accent / surface | 5.6 | 8.1 | AA |
| text-on-accent / accent | 5.4 | 9.6 | AA |
| border-strong / surface | 2.1 | 2.0 | AA non-text (≥3:1 required only for *focusable* boundaries - see below) |
| ring / surface | 3.9 | 6.3 | AA non-text |
| ov-text / ov-bg | 13.1 | - | AAA |
| ov-bar / ov-bg | 8.4 | - | AAA |

Two rules that fall out of this table and must be enforced in review:

1. **`--brand-cyan` (#22A7CC) may never carry text and may never be a text-bearing fill.** It is 2.8:1
   with white. It exists for the mark and the waveform bars, both of which sit on near-black.
2. **`--text-disabled` is not text.** A disabled control's *label* uses `--text-muted` at 60% opacity
   against a `--surface-active` fill, which keeps it at ~3.0:1 - legally disabled controls are exempt,
   but an unreadable disabled label is a support ticket. Never render information only as disabled.

Control boundaries that are focusable (inputs, the hotkey capture field, select triggers) use
`--border-strong` *plus* a `--surface-sunken` fill, so the control is identified by fill-contrast
(1.06:1 fill delta is not enough alone) **and** border. Where a control is borderless (ghost buttons),
it must reveal a ≥3:1 boundary on hover and focus.

### 2.7 Tailwind theme

**Tailwind v4** (`app.css`):

```css
@import "tailwindcss";

@theme {
  --color-cyan-50:  #EDFAFE;  --color-cyan-500: #22A7CC;
  --color-cyan-100: #D0F2FC;  --color-cyan-600: #1C8FAF;
  --color-cyan-200: #A5E6F8;  --color-cyan-700: #17738F;
  --color-cyan-300: #6BD3EF;  --color-cyan-800: #185E74;
  --color-cyan-400: #38BDE0;  --color-cyan-900: #184E60;
                              --color-cyan-950: #0D3140;

  --color-ink-0:   #FFFFFF;   --color-ink-600:  #53616C;
  --color-ink-25:  #FBFCFD;   --color-ink-700:  #3F4B55;
  --color-ink-50:  #F6F8FA;   --color-ink-800:  #2B343C;
  --color-ink-100: #EDF1F4;   --color-ink-850:  #222A31;
  --color-ink-200: #DDE4E9;   --color-ink-900:  #191F25;
  --color-ink-300: #C3CDD5;   --color-ink-925:  #14191E;
  --color-ink-400: #94A3AE;   --color-ink-950:  #10151A;
  --color-ink-500: #64727E;   --color-ink-1000: #0A0E12;

  /* semantic aliases - components use ONLY these */
  --color-surface:        var(--surface);
  --color-surface-raised: var(--surface-raised);
  --color-surface-sunken: var(--surface-sunken);
  --color-surface-hover:  var(--surface-hover);
  --color-surface-active: var(--surface-active);
  --color-surface-selected: var(--surface-selected);
  --color-border:         var(--border);
  --color-border-strong:  var(--border-strong);
  --color-border-subtle:  var(--border-subtle);
  --color-fg:             var(--text-primary);
  --color-fg-secondary:   var(--text-secondary);
  --color-fg-muted:       var(--text-muted);
  --color-fg-accent:      var(--text-accent);
  --color-fg-on-accent:   var(--text-on-accent);
  --color-accent:         var(--accent);
  --color-accent-hover:   var(--accent-hover);
  --color-accent-wash:    var(--accent-wash);
  --color-success:        var(--success);
  --color-warning:        var(--warning);
  --color-danger:         var(--danger);

  --font-sans: ui-sans-serif, -apple-system, BlinkMacSystemFont,
               "Segoe UI Variable Text", "Segoe UI",
               Inter, Cantarell, Ubuntu, "Noto Sans", sans-serif;
  --font-mono: ui-monospace, "SF Mono", "Cascadia Mono", "JetBrains Mono",
               Menlo, Consolas, "DejaVu Sans Mono", monospace;

  --text-2xs: 10px;  --text-2xs--line-height: 14px;  --text-2xs--letter-spacing: 0.04em;
  --text-xs:  11px;  --text-xs--line-height:  16px;
  --text-sm:  12px;  --text-sm--line-height:  18px;
  --text-ui:  13px;  --text-ui--line-height:  20px;
  --text-base:14px;  --text-base--line-height:21px;
  --text-lg:  16px;  --text-lg--line-height:  24px;
  --text-xl:  20px;  --text-xl--line-height:  28px;  --text-xl--letter-spacing: -0.01em;
  --text-2xl: 24px;  --text-2xl--line-height: 32px;  --text-2xl--letter-spacing: -0.015em;

  --radius-sm: 4px; --radius-md: 6px; --radius-lg: 8px; --radius-xl: 12px;

  --shadow-sm: var(--shadow-sm);
  --shadow-md: var(--shadow-md);
  --shadow-lg: var(--shadow-lg);

  --ease-out:   cubic-bezier(0.22, 1, 0.36, 1);
  --ease-in:    cubic-bezier(0.55, 0, 1, 0.45);
  --ease-inout: cubic-bezier(0.65, 0, 0.35, 1);
}
```

**Tailwind v3** (`tailwind.config.ts`) - same values, for reference:

```ts
import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: ["class", '[data-theme="dark"]'],
  theme: {
    extend: {
      colors: {
        cyan: { 50:"#EDFAFE",100:"#D0F2FC",200:"#A5E6F8",300:"#6BD3EF",400:"#38BDE0",
                500:"#22A7CC",600:"#1C8FAF",700:"#17738F",800:"#185E74",900:"#184E60",950:"#0D3140" },
        ink:  { 0:"#FFFFFF",25:"#FBFCFD",50:"#F6F8FA",100:"#EDF1F4",200:"#DDE4E9",300:"#C3CDD5",
                400:"#94A3AE",500:"#64727E",600:"#53616C",700:"#3F4B55",800:"#2B343C",850:"#222A31",
                900:"#191F25",925:"#14191E",950:"#10151A",1000:"#0A0E12" },
        surface:   { DEFAULT:"var(--surface)", raised:"var(--surface-raised)",
                     sunken:"var(--surface-sunken)", hover:"var(--surface-hover)",
                     active:"var(--surface-active)", selected:"var(--surface-selected)" },
        border:    { DEFAULT:"var(--border)", strong:"var(--border-strong)", subtle:"var(--border-subtle)" },
        fg:        { DEFAULT:"var(--text-primary)", secondary:"var(--text-secondary)",
                     muted:"var(--text-muted)", accent:"var(--text-accent)", "on-accent":"var(--text-on-accent)" },
        accent:    { DEFAULT:"var(--accent)", hover:"var(--accent-hover)", wash:"var(--accent-wash)" },
        success:"var(--success)", warning:"var(--warning)", danger:"var(--danger)",
      },
      fontSize: {
        "2xs":["10px",{lineHeight:"14px",letterSpacing:"0.04em"}],
        xs:["11px",{lineHeight:"16px"}],  sm:["12px",{lineHeight:"18px"}],
        ui:["13px",{lineHeight:"20px"}],  base:["14px",{lineHeight:"21px"}],
        lg:["16px",{lineHeight:"24px"}],
        xl:["20px",{lineHeight:"28px",letterSpacing:"-0.01em"}],
        "2xl":["24px",{lineHeight:"32px",letterSpacing:"-0.015em"}],
      },
      borderRadius: { sm:"4px", md:"6px", lg:"8px", xl:"12px" },
      transitionTimingFunction: {
        out:"cubic-bezier(0.22,1,0.36,1)", in:"cubic-bezier(0.55,0,1,0.45)",
        inout:"cubic-bezier(0.65,0,0.35,1)",
      },
      transitionDuration: { instant:"80ms", fast:"140ms", base:"200ms", slow:"320ms" },
    },
  },
} satisfies Config;
```

---

## 3. Type and spacing

### 3.1 Font stacks

One stack, ordered so each OS hits its native UI face first. **Do not ship a webfont** - a 200KB
variable font download on a "no internet at runtime" product is both a contradiction and a first-paint
cost.

| OS | Resolves to | Note |
|---|---|---|
| macOS | `-apple-system` → SF Pro Text | `ui-sans-serif` first for future-proofing |
| Windows 11 | `Segoe UI Variable Text` | falls back to `Segoe UI` on Win10 |
| Linux | `Inter` → `Cantarell` → `Ubuntu` → `Noto Sans` | Inter is commonly present on dev machines; Cantarell/Ubuntu cover GNOME/Ubuntu defaults |

```
font-sans: ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI Variable Text",
           "Segoe UI", Inter, Cantarell, Ubuntu, "Noto Sans", sans-serif;
font-mono: ui-monospace, "SF Mono", "Cascadia Mono", "JetBrains Mono", Menlo, Consolas,
           "DejaVu Sans Mono", monospace;
```

Mono is used for exactly two things: keyboard chips (`<Kbd>`) and file sizes/durations in tabular
positions. Transcript text is **sans** - it is prose, not code, and mono would make it look like
output rather than like the user's own words.

### 3.2 Type scale

Base UI size is **13px**, body/transcript is **14px**. Desktop utilities read denser than web apps;
13px UI at 1× matches Segoe UI's native 12-13 and SF's 13.

| Token | Size / LH | Weight | Use |
|---|---|---|---|
| `2xs` | 10 / 14, +0.04em, uppercase | 600 | section eyebrows, badge text, "PAUSED" |
| `xs`  | 11 / 16 | 400/500 | timestamps, durations, helper text under fields |
| `sm`  | 12 / 18 | 400/500 | secondary metadata, date group headers |
| `ui`  | 13 / 20 | 400/500 | **default UI**: labels, buttons, menu items, inputs |
| `base`| 14 / 21 | 400 | **transcript text**, settings descriptions, onboarding body |
| `lg`  | 16 / 24 | 600 | settings section titles, dialog titles |
| `xl`  | 20 / 28, −0.01em | 600 | screen titles ("History", "Settings") |
| `2xl` | 24 / 32, −0.015em | 600 | onboarding headlines only |

**Weights: 400, 500, 600 only.** No 700 - bold in a small utility reads as shouting, and Segoe UI
Variable's 700 is noticeably heavier than SF's, which breaks cross-platform parity. No 300 - it fails
contrast at 11-13px on Windows' greyscale AA.

Transcript text gets `max-width: 68ch` and `text-wrap: pretty` where supported.

### 3.3 Spacing scale

4px base. Only these values exist; anything else is a bug.

```
0 · 1=4 · 2=8 · 3=12 · 4=16 · 5=20 · 6=24 · 7=28 · 8=32 · 10=40 · 12=48 · 16=64 · 20=80
```

Anchor measurements that everything else hangs off:

| Thing | Value |
|---|---|
| Window edge gutter (main, settings) | 24 |
| Section vertical rhythm (settings) | 32 between groups, 16 between rows |
| Control height - default | 32 |
| Control height - small | 26 |
| Control height - large (onboarding CTAs, hotkey field) | 40 |
| List row height - collapsed transcript | 64 |
| Top bar height | 56 |
| Icon sizes | 14 (inline), 16 (default), 18 (primary action), 20 (brand mark), 24 (empty state) |
| Minimum hit target | 28×28 (desktop pointer; 32×32 preferred) |

---

## 4. Motion

### 4.1 Tokens

```css
:root {
  --dur-instant: 80ms;   /* hover, focus ring, press - must feel un-animated */
  --dur-fast:   140ms;   /* exits, icon swaps, toast in/out */
  --dur-base:   200ms;   /* size and position changes, overlay width */
  --dur-slow:   320ms;   /* route change, onboarding step advance */
  --dur-hold-min: 180ms; /* minimum time any overlay state is allowed to be visible */

  --ease-out:    cubic-bezier(0.22, 1, 0.36, 1);   /* entrances: fast start, soft land */
  --ease-in:     cubic-bezier(0.55, 0, 1, 0.45);   /* exits: soft start, fast leave */
  --ease-inout:  cubic-bezier(0.65, 0, 0.35, 1);   /* size/position, bidirectional */
  --ease-linear: linear;                            /* continuous: progress, waveform, dots */
}
```

The asymmetry is intentional and is the difference between "considered" and "generic": things arrive
slower than they leave (200ms in, 140ms out). Exits that linger feel like the app is arguing.

### 4.2 When motion is used - and when it is not

**Motion is used only to:**
1. Explain a spatial or causal relationship (the overlay arrived from the screen edge; this panel came
   from that button).
2. Represent continuous data (the waveform; a download progress bar).
3. Absorb latency (processing dots).

**Motion is never used to:**
- Decorate a state that is already unambiguous. Hover is a colour change in 80ms, not a lift.
- Reveal list content. Transcript rows appear instantly. Staggered list entry animations cost N
  animations and buy nothing in a list the user is scanning.
- Draw attention to something the user already asked for (opening Settings is a 320ms cross-fade +
  8px slide, not a bounce).
- Celebrate. There is no success confetti, no spring, no overshoot anywhere in this product. The one
  place with any character is the overlay's arrival, and it is a 0.96→1 scale - not a bounce.

**Hard performance rule:** only `opacity` and `transform` are animated, plus the single sanctioned
exception of the overlay capsule's `width`. Elements that animate carry `will-change` *only while
animating* (added on state entry, removed on `transitionend`) - a permanent `will-change: transform` on
15 waveform bars pins 15 compositor layers for the lifetime of the process.

### 4.3 Reduced motion

`@media (prefers-reduced-motion: reduce)`:
- All durations → `1ms` except the waveform and progress bars, which are **data, not decoration**, and
  keep animating at a reduced 10Hz update (see §5.7).
- Overlay entrance/exit become opacity-only (no translate, no scale), 100ms.
- Processing dots become a static `···` with a single-step opacity pulse at 1Hz, or if the user has
  also disabled sound feedback, the capsule simply shows a static accent-tinted state.
- The error shake is removed entirely; the error state is conveyed by width, colour rail, and text.

---

## 5. Overlay specification

> This is the product. Everything else is a settings panel.

### 5.1 Concept: **the capsule**

A single dark pill, bottom-centre, containing a **centre-out mirrored waveform**. New audio enters at
the centre bar and propagates outward to both edges, each older sample one bar further out, tapering
in amplitude as it goes.

Two reasons this specific form, and not the alternatives:

1. **It is the icon, animated.** The mark is a face on the left and three arcs radiating right - sound
   leaving a source and spreading. The centre-out waveform is that same idea rendered symmetrically.
   A left-to-right scrolling waveform (the default choice, used by nearly every competitor) reads as a
   *recording timeline* - a thing with a length, a thing being captured and stored. kiku stores no
   audio. Radiating reads as *transmission*, which is what is actually happening.
2. **It is readable at a glance with no fixation point.** A scrolling waveform has a "now" edge you
   must find. A symmetric one has its now-point at the geometric centre of the capsule, which is where
   your peripheral vision already resolves the shape's centroid. You perceive "loud" vs "quiet" from
   the silhouette without looking at anything.

The capsule is the *only* overlay form. Every state is the same pill at a different width with
different contents. There is no second shape, no expanding panel, no corner toast.

### 5.2 Theme: the overlay is always dark. In both OS themes.

This is a deliberate deviation from the "follow the OS theme" constraint, and I want it argued
explicitly rather than discovered in review.

The overlay does not sit on kiku's surface; it sits on **an arbitrary application's** surface - a dark
IDE, a white document, a photograph, a video call. "Following the OS theme" is a guarantee about
matching *our* windows, and the overlay has no windows to match. What it must do is be legible against
anything, and a near-black capsule with a light-cyan waveform and a hairline highlight is legible on
every backdrop in a way a white capsule is not (white-on-white documents are the single most common
dictation target).

There is a second argument: **recognition**. This shape is the product's signature. A signature that
changes colour depending on a setting is a weaker signature.

If this is overruled, the fallback is a light variant `--ov-bg-light: rgb(255 255 255 / 0.96)` with
`--ov-bar: #17738F` and `--ov-hairline: rgb(16 21 26 / 0.10)`, and the shadow strengthened to
`0 8px 28px rgb(16 21 26 / 0.22)`. I do not recommend shipping it.

**No `backdrop-filter`.** Beyond the CPU argument: on Linux/X11 a transparent Tauri window's webview
cannot sample the desktop behind it, so `backdrop-filter` produces *nothing* there while costing real
milliseconds on macOS and Windows. A platform-inconsistent effect that costs CPU is the worst of both.
The 94% opaque fill is the design, not a compromise.

### 5.3 Window geometry

```
OS window ("kiku-overlay"):  320 × 96 logical px - FIXED, never resized
  transparent: true          decorations: false        shadow: false (we draw our own)
  always_on_top: true        skip_taskbar: true        focusable: false
  resizable: false           visible_on_all_workspaces: true
  set_ignore_cursor_events(true)   // click-through
  created at app launch, hidden; NEVER created on demand
```

The window is oversized relative to the capsule on purpose: the capsule's widest state is 260px and
its shadow bleeds ~28px, so 320×96 contains every state plus shadow plus the 8px entry translate. **The
OS window never changes size** - resizing a window per state causes visible flicker on X11 and a
compositor round-trip on every state change. Only the CSS capsule inside it animates.

`html, body` are `background: transparent; overflow: hidden; user-select: none;` and the capsule is
centred with flexbox. `pointer-events: none` on everything as a second line of defence behind
`set_ignore_cursor_events`.

### 5.4 Screen position

```
x = workArea.x + (workArea.width  - 320) / 2          // horizontally centred
y = workArea.y +  workArea.height - 96 - 40           // 40px above the work area bottom
```

- Anchored to the **work area**, not the display bounds - this is what keeps the capsule off the macOS
  Dock and the Windows taskbar. Getting this wrong is the most common shipped bug in this product
  category.
- On the display containing the **currently focused window**, resolved at key-down; falls back to the
  display containing the cursor, then the primary display.
- Bottom-centre, not a corner: corners are where OS notifications live (top-right on macOS, bottom-right
  on Windows), and colliding with the notification stack is both visually noisy and functionally bad.
  Bottom-centre is also the least content-destructive band in text editors, chat apps, and browsers,
  where the caret is typically upper-to-mid screen.
- **Rejected: anchoring to the text caret.** Tempting, and it is what a naive spec asks for. It is not
  reliably queryable cross-platform (X11 has no general caret API; Win32 needs UIA per-app; macOS needs
  AX and fails in Electron apps), and it would place the capsule directly over the region the user is
  about to write into. Bottom-centre is both more implementable and more correct.
- Multi-monitor: recompute on every show. Do not cache.

### 5.5 States

Height is **44px in every state**. Radius is always `22px` (a true capsule). Only width changes.

| State | Width | Contents | Duration |
|---|---|---|---|
| `hidden` | - | OS window hidden | - |
| `listening` | **168** | waveform, 15 bars | while hotkey held |
| `processing` | **132** | 3 pulsing dots | until transcript, min 180ms |
| `done` | **108** | `Check` 18px, `--ov-success` | hold 420ms, then exit |
| `cancelled` | **44** (circle) | `X` 14px at 60% opacity | hold 200ms, then exit |
| `error` | **260** | 3px danger rail + `TriangleAlert` 16px + one line of text | hold 3200ms, or until hotkey |

Full state machine:

```
             hotkey down                 hotkey up                 transcript ok
  hidden ─────────────────▶ listening ──────────────▶ processing ─────────────────▶ done
     ▲                          │                         │                           │
     │                          │ release < 400ms         │ failure                   │ 420ms
     │                          ▼                         ▼                           │
     │                     cancelled ◀──── Esc ──── ── error ──────────────────────────┤
     └──────────────────────────┴─────────────────────────┴───────────────────────────┘
                                          exit (140ms)
```

Notes on the edges:
- **`hidden → listening` fires on key *down*, not on audio-stream-open.** The user gets feedback in
  one frame; the mic opens behind it. If the mic fails to open within 300ms, transition to `error`.
- **A release under 400ms is `cancelled`, not `done`.** That is an accidental keypress; sending an
  empty or one-syllable transcript into the user's document is worse than doing nothing. Cancelled is
  visually distinct from done (circle + X, no green) so the user learns the difference without being
  told.
- **`Esc` during `listening` or `processing` cancels.** The overlay is click-through, so the keyboard
  is the only cancel channel - this is non-optional.
- **Repeated `error` (3 in a row) raises the main window** to the relevant settings section. A
  click-through overlay cannot host troubleshooting; after three failures the user needs a surface they
  can actually interact with.

### 5.6 Waveform - implementation spec

**Geometry**

```
capsule            168 × 44, radius 22, padding 0 14px
content box        140 × 20, vertically centred
bars               15 bars × 4px wide, 5px gap  → 15·4 + 14·5 = 130px, centred in the 140px box
bar radius         2px (fully rounded caps)
bar base height    20px (the DOM height; scaled down, never up)
bar min visual     4px  → minimum scaleY = 0.20
bar max visual     20px → maximum scaleY = 1.00
transform-origin   center
colour             --ov-bar (#38BDE0); idle floor uses --ov-bar-idle
```

Index `i ∈ [0..14]`, centre index `c = 7`, ring distance `d = |i − c| ∈ [0..7]`.

**Amplitude pipeline** (Rust side → event → React side)

1. Audio thread computes per-block RMS at ~**30Hz** (one value per ~33ms of audio). Do not emit at
   60Hz; the visual gains nothing and you double the IPC traffic during ASR.
2. Convert to perceptual level. Linear RMS is useless - speech lives in a narrow band near the top of
   it and the bars would barely move.
   ```
   db    = 20 * log10(rms + 1e-7)
   level = clamp((db - (-58)) / ((-8) - (-58)), 0, 1)     // -58 dBFS → 0,  -8 dBFS → 1
   ```
   `-58` is the noise floor; `-8` is a loud speaking voice close to a laptop mic. These two numbers are
   the entire "feel" of the waveform and should be the only tunables. Emit `level` as an `f32` over
   `emit_to("kiku-overlay", "audio-level", level)`.
3. **Smoothing (JS, per animation frame).** Fast attack, slow release - the standard VU ballistic, and
   the reason a good meter looks alive instead of twitchy.
   ```ts
   const k = target > current ? 0.55 : 0.14;   // ~3 frames to rise, ~9 to fall @60fps
   current += (target - current) * k;
   ```
4. **Ring buffer.** Keep `history: Float32Array(8)` (centre + 7 rings). Push the current smoothed
   level into `history[0]` every **70ms**, shifting the rest outward. Visible window =
   8 × 70ms ≈ **560ms** of recent speech - long enough to read as a shape, short enough to feel
   immediate.
5. **Per-bar height.**
   ```ts
   const falloff = 1 - d * 0.055;                       // outermost bar at 0.615
   const v       = history[d] * falloff;
   const scaleY  = 0.20 + v * 0.80;                     // 0.20 .. 1.00
   ```
   The falloff is what makes the shape *radiate* rather than pump uniformly: energy visibly decays as
   it travels outward, mirroring the mark's shortening arcs.

**Rendering**

15 absolutely-positioned `<div>`s, each `4px × 20px`, animated with `transform: scaleY(k)` only.

- `transform` on a promoted layer is compositor-only: **zero layout, zero paint** per frame. This is
  strictly cheaper than an SVG whose `rect` `height`/`y` attributes are rewritten (which repaints), and
  vastly cheaper than canvas.
- Write the 15 transforms inside one `requestAnimationFrame` callback, in one synchronous pass, with
  no reads interleaved. Cache the `HTMLElement` refs in an array at mount; never query the DOM in the
  loop.
- Add `will-change: transform` on the bar container's children when entering `listening`, remove it on
  entering `processing`. Never leave it on.
- **Frame budget: ≤ 0.6ms of JS per frame, ≤ 1.5ms total frame cost.** If a profiler shows more, the
  bug is a React re-render - the rAF loop must write to the DOM imperatively and must not call
  `setState`. State-machine transitions use React; the 60fps loop does not.

**Idle behaviour (listening, but silence)**

All bars settle to `scaleY: 0.20` (a 130×4px dotted-dash line). Do **not** leave it dead - dead reads
as crashed. Instead: one animation, on the bar *container*, `opacity: 0.55 ⇄ 0.85` over **2400ms**,
`ease-inout`, infinite. One property, one element, one composited animation - the cost is a rounding
error, and the capsule breathes.

The breathing animation is paused whenever `max(history) > 0.08` and resumed after 900ms of continued
silence, so it never fights with real audio.

### 5.7 Reduced-motion waveform

The waveform is information (am I being heard?), so it does not simply stop. Under
`prefers-reduced-motion: reduce`:
- Update at **10Hz** instead of 60Hz (a `setInterval`, not rAF).
- Replace the 15-bar mirrored display with **a single centred bar** whose *width* maps to level -
  60px at silence, 130px at full - because a horizontally growing meter has far less apparent motion
  than 15 independently scaling elements.
- Remove the idle breathing pulse; use a static `--ov-bar-idle` fill instead.

### 5.8 Motion per transition (exact)

```
hidden → listening
  window.show() on key-down, same tick
  capsule:  opacity 0→1, translateY 8px→0, scale 0.96→1
  duration: 180ms   easing: --ease-out
  bars:     start at scaleY 0.20; first real level lands ~2 frames later

listening → processing
  1. bars collapse:  all scaleY → 0.20   |  140ms  --ease-inout
  2. cross-fade:     bars opacity 1→0 (100ms) / dots opacity 0→1 (100ms, +60ms delay)
  3. capsule width:  168 → 132           |  200ms  --ease-inout   (runs concurrently with 1-2)

processing (loop)
  3 dots · 4px · 5px gap · --ov-bar
  each dot: opacity 0.28 ⇄ 1.0, 900ms loop, stagger 120ms, --ease-inout
  NOT a spinner. A spinner reads "web page loading". Dots read "thinking / typing",
  which is the correct mental model for a transcription that is about to become text.

processing → done
  capsule width 132 → 108                | 200ms --ease-inout
  dots opacity → 0 (100ms), Check icon opacity 0→1 + scale 0.7→1 (160ms --ease-out, +60ms delay)
  hold 420ms

any → exit
  opacity 1→0, translateY 0→6px, scale 1→0.98
  duration 140ms  easing --ease-in
  window.hide() on transitionend + 20ms   (the +20ms avoids a hide-before-paint flash on X11)

* → error
  capsule width → 260                    | 200ms --ease-inout
  left rail 3px --ov-danger fades in     | 140ms
  icon + text fade in                    | 140ms, +80ms delay
  shake: translateX keyframes 0,-3,3,-2,2,0 over 260ms - EXACTLY ONCE, never looping,
         suppressed entirely under reduced-motion

listening/processing → cancelled
  capsule width → 44 (circle)            | 160ms --ease-inout
  contents cross-fade to X 14px @60%     | 100ms
  hold 200ms, then exit
```

### 5.9 Sound feedback

Because the overlay is click-through and non-focusable, it cannot be announced to a screen reader and
cannot be interacted with. **Sound is therefore the primary non-visual state channel, not a nicety -
it ships default-ON**, and this should be stated in the settings copy.

```
start   660 Hz sine, 55ms, 6ms attack / 20ms release
end     880 Hz sine, 55ms
error   descending 520→390 Hz, 140ms
cancel  (silent - absence of the end tone is the signal)
```

All four are generated in-process, not loaded as assets. Default volume 45%.

---

## 6. Component inventory

Fourteen primitives. Every surface is assembled from these; nothing else gets written. The test for
adding a fifteenth is: *does it appear on at least two surfaces, or is it the signature of one?*

```ts
// ───────────────────────────────── 1. Button ─────────────────────────────────
type ButtonProps = {
  variant?: "primary" | "secondary" | "ghost" | "danger";  // default "secondary"
  size?: "sm" | "md" | "lg";                                // 26 / 32 / 40 px tall
  icon?: LucideIcon;                     // leading
  trailingIcon?: LucideIcon;
  iconOnly?: boolean;                    // square; REQUIRES `label`
  label?: string;                        // visible text, or aria-label when iconOnly
  loading?: boolean;                     // swaps icon slot for dots, keeps width
  destructive?: boolean;                 // adds confirm-on-first-click behaviour
  fullWidth?: boolean;
} & React.ButtonHTMLAttributes<HTMLButtonElement>;

// ───────────────────────────────── 2. Input ──────────────────────────────────
type InputProps = {
  size?: "sm" | "md";
  icon?: LucideIcon;                     // leading
  clearable?: boolean;                   // renders X when value is non-empty
  invalid?: boolean;
  onClear?: () => void;
} & React.InputHTMLAttributes<HTMLInputElement>;
// Search field = <Input icon={Search} clearable />. No separate SearchInput.

// ───────────────────────────────── 3. Select ─────────────────────────────────
type SelectOption<T> = {
  value: T; label: string; description?: string;
  icon?: LucideIcon; badge?: string; disabled?: boolean;
};
type SelectProps<T> = {
  value: T;
  options: SelectOption<T>[];
  onChange: (v: T) => void;
  placeholder?: string;
  size?: "sm" | "md";
  // renderOption is the escape hatch that lets the model picker (with size + accuracy
  // metadata) reuse this instead of becoming a bespoke component.
  renderOption?: (o: SelectOption<T>) => React.ReactNode;
};
// Custom listbox, not <select>: native select styling is unfixable across the three OSes
// and cannot show the two-line mic/model options.

// ───────────────────────────────── 4. Toggle ─────────────────────────────────
type ToggleProps = {
  checked: boolean;
  onChange: (v: boolean) => void;
  size?: "sm" | "md";                    // 16×28 / 20×34
  disabled?: boolean;
  "aria-label"?: string;                 // required when used without a <SettingRow> label
};

// ───────────────────────────────── 5. Slider ─────────────────────────────────
type SliderProps = {
  value: number; min?: number; max?: number; step?: number;
  onChange: (v: number) => void;
  onCommit?: (v: number) => void;        // fires on pointer-up - plays the test sound once
  leadingIcon?: LucideIcon; trailingIcon?: LucideIcon;
  formatValue?: (v: number) => string;   // "45%"
};

// ───────────────────────────────── 6. Row ────────────────────────────────────
// The single most important primitive. SettingRow and TranscriptRow are both this.
type RowProps = {
  as?: "div" | "li" | "button";
  leading?: React.ReactNode;             // icon, checkbox, avatar slot
  title: React.ReactNode;
  description?: React.ReactNode;
  trailing?: React.ReactNode;            // control, actions, metadata
  align?: "center" | "start";            // "start" for multi-line transcripts
  density?: "comfortable" | "compact";   // 48 / 36 px min height
  interactive?: boolean;                 // hover + focus-visible + cursor
  selected?: boolean;
  onActivate?: () => void;               // click + Enter + Space
};

// ───────────────────────────────── 7. Panel ──────────────────────────────────
type PanelProps = {
  title?: string;
  eyebrow?: string;                      // 2xs uppercase
  description?: string;
  icon?: LucideIcon;
  footer?: React.ReactNode;
  tone?: "default" | "accent" | "warning" | "danger";  // tinted wash + border
  children: React.ReactNode;
};
// Used for: each settings group, each onboarding card, the macOS permission callout.

// ───────────────────────────────── 8. Badge ──────────────────────────────────
type BadgeProps = {
  tone?: "neutral" | "accent" | "success" | "warning" | "danger";
  variant?: "solid" | "soft" | "outline";  // default "soft"
  icon?: LucideIcon;
  children: React.ReactNode;
};
// "650 MB" · "Recommended" · "PAUSED" · "Granted" · "3 selected"

// ───────────────────────────────── 9. Kbd ────────────────────────────────────
type KbdProps = {
  keys: string[];                        // ["Ctrl","Shift","Space"] - normalised per OS
  size?: "sm" | "md";
  tone?: "neutral" | "accent";
};
// Renders ⌘ ⌥ ⌃ ⇧ on macOS, Ctrl/Alt/Shift/Win elsewhere. Owns the OS key-name mapping
// so no other component ever has to know about it.

// ──────────────────────────────── 10. Progress ───────────────────────────────
type ProgressProps = {
  value?: number;                        // 0..1; omit for indeterminate
  label?: string;
  detail?: string;                       // "312 MB of 648 MB · 4.1 MB/s"
  tone?: "accent" | "success" | "danger";
  size?: "sm" | "md";                    // 3 / 6 px track
};

// ──────────────────────────────── 11. EmptyState ─────────────────────────────
type EmptyStateProps = {
  icon: LucideIcon;
  title: string;
  description?: React.ReactNode;
  action?: React.ReactNode;              // a <Button>
  compact?: boolean;                     // for "no search results" inside a populated list
};

// ──────────────────────────────── 12. Dialog ─────────────────────────────────
type DialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  tone?: "default" | "danger";
  confirmLabel?: string; cancelLabel?: string;
  onConfirm: () => void;
  children?: React.ReactNode;
};
// One dialog component. Used only for destructive confirmation. There are no other modals.

// ──────────────────────────────── 13. Toast ──────────────────────────────────
type ToastProps = {
  tone?: "neutral" | "success" | "danger";
  icon?: LucideIcon;
  message: string;
  action?: { label: string; onClick: () => void };   // "Undo"
  duration?: number;                     // default 3200ms
};
// Main window only. Bottom-centre, 1 at a time, replaces rather than stacks.

// ──────────────────────────────── 14. WaveBars ───────────────────────────────
type WaveBarsProps = {
  /** 0..1 ring buffer, newest at index 0. Length sets the bar count: 2n-1 bars. */
  history: Float32Array;
  bars?: number;                         // default 15 (odd only)
  barWidth?: number;                     // default 4
  gap?: number;                          // default 5
  height?: number;                       // default 20
  color?: string;                        // default var(--ov-bar)
  idle?: boolean;                        // breathing pulse
  reducedMotion?: boolean;               // switches to single width-mapped bar
};
// The signature component. Reused at height={12} bars={9} in onboarding's mic test and
// in the Settings > Microphone live-level preview - which is exactly why it is a primitive
// and not overlay-internal code.
```

**Deliberately NOT components:** Tabs (the IA has none), Tooltip-as-a-component (use `title` for
icon-only buttons plus `aria-label`; a custom tooltip layer is a positioning library we do not need),
Card (that is `Panel`), IconButton (that is `Button iconOnly`), SearchInput (that is `Input`),
Switch/Checkbox split (checkbox exists only inside `Row leading` for bulk select and is 24 lines of
local JSX).

**Non-visual shared modules:** `useTheme()`, `useHotkeyCapture()`, `useOverlayState()`,
`formatRelativeTime()`, `formatDuration()`, `osKeyName()`.

---

## 7. Layout wireframes

### 7.1 Overlay

```
   ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐
   │  OS window "kiku-overlay" · 320 × 96 · transparent       │
   │  click-through · always-on-top · non-activating · fixed   │
   │                                                           │
   │        ╭───────────────────────────────────╮              │   LISTENING
   │        │      ▁ ▃ ▅ ▇ █ █ █ ▇ ▅ ▃ ▁        │  44px h      │   capsule 168 × 44, r22
   │        ╰───────────────────────────────────╯              │   15 bars · 4w · 5gap · 130 total
   │         └14┘└────── 140 content ──────┘└14┘               │   centre-out, newest at middle
   └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
                              ▲
                     40px above work-area bottom, horizontally centred


            ╭───────────────────────╮          PROCESSING · 132 × 44
            │        ●  ◐  ○        │          3 dots · 4px · 5gap · 120ms stagger
            ╰───────────────────────╯

               ╭─────────────────╮             DONE · 108 × 44 · hold 420ms
               │        ✓        │             Check 18px · --ov-success
               ╰─────────────────╯

                     ╭─────╮                   CANCELLED · 44 × 44 circle · hold 200ms
                     │  ✕  │
                     ╰─────╯

   ╭───────────────────────────────────────────────────────────╮   ERROR · 260 × 44 · hold 3200ms
   ┃ ⚠  Microphone is in use by another app                    │   3px danger rail (left edge)
   ╰───────────────────────────────────────────────────────────╯   icon 16 · text 13/20 --ov-text
   └3┘└10┘└16┘└10┘└──────────── ~190 ───────────┘└14┘


   ── full-screen context ────────────────────────────────────────────────────
   ┌───────────────────────────────────────────────────────────────────────┐
   │  whatever app the user is actually in                                 │
   │                                                                       │
   │      The quick brown fox jumps over the lazy▏                         │
   │                                                                       │
   │                                                                       │
   │                        ╭──────────────────╮                           │ ← 40px above
   │                        │  ▁▃▅▇█▇▅▃▁       │                           │   work area
   └────────────────────────╰──────────────────╯───────────────────────────┘
   ▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔ dock / taskbar ▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔
```

### 7.2 Main window - History

Default **880 × 620**, min **640 × 460**. Resizable. Remembers size and position.

```
┌──────────────────────────────────────────────────────────────────────────────────┐ 880
│ ░ native title bar (macOS: hiddenInset, traffic lights inset 12/18) ░            │
├──────────────────────────────────────────────────────────────────────────────────┤
│  ◉ kiku    ┌──────────────────────────────────────────┐   ⏸        ⚙             │ 56
│   20px     │ 🔍  Search transcripts…               ✕ │   32       32             │
│            └──────────────────────────────────────────┘                          │
│            └──────────── flex, max 420 ───────────────┘  └─ 8 gap ─┘   └ 24 gut ─┘
├──────────────────────────────────────────────────────────────────────────────────┤ 1px border
│                                                                                  │
│  TODAY                                                            4 transcripts  │ 32  (2xs/600 uppercase, muted)
│  ┌────────────────────────────────────────────────────────────────────────────┐  │
│  │ ☐  Let's push the release to Thursday so QA has a full day with the        │  │
│  │    build. I'll let Maya know in standup.                                   │  │ 64  row, comfortable
│  │    9:42 AM · 6.2s · 21 words                              ⧉  🗑            │  │     hover: surface-hover
│  └────────────────────────────────────────────────────────────────────────────┘  │     actions fade in 80ms
│  ├──────────────────────────────── 1px border-subtle ───────────────────────────┤ │
│  │ ☑  Can you take a look at the auth middleware? The refresh token…          │  │ 64  selected: surface-selected
│  │    9:31 AM · 3.8s · 14 words                              ⧉  🗑            │  │     + 2px accent left rail
│  ├────────────────────────────────────────────────────────────────────────────┤  │
│  │ ☐  Reminder to book the flights before Friday.                            │  │ 64  short entries collapse
│  │    8:58 AM · 1.9s · 7 words                               ⧉  🗑            │  │     to a single line
│  └────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
│  YESTERDAY                                                       12 transcripts  │ 32
│  ┌────────────────────────────────────────────────────────────────────────────┐  │
│  │ ☐  …                                                                       │  │
│                                    ⋮                                             │
│                                                                                  │
│  ┌──────────────────────────────────────────────────────────────────────────┐    │ ← selection bar
│  │  3 selected        Deselect all          ⧉ Copy all      🗑 Delete        │    │   slides up 200ms
│  └──────────────────────────────────────────────────────────────────────────┘    │   48 tall, floating
└──────────────────────────────────────────────────────────────────────────────────┘   16 above bottom
   └ 24 ┘└──────────────────── content max-width 832 ─────────────────────┘└ 24 ┘


  ── row anatomy, expanded (double-click / Space) ────────────────────────────
  ┌────────────────────────────────────────────────────────────────────────────┐
  │ ☐  Let's push the release to Thursday so QA has a full day with the build.  │  text becomes
  │    I'll let Maya know in standup, and I'll update the milestone in Linear   │  user-selectable
  │    once that's confirmed. Also worth flagging to design.                    │  + editable (§11.2)
  │                                                                             │
  │    9:42 AM · 6.2s · 38 words                    ⧉ Copy   ✎ Edit   🗑 Delete │  40
  └────────────────────────────────────────────────────────────────────────────┘

  ── first run / empty ───────────────────────────────────────────────────────
  ┌──────────────────────────────────────────────────────────────────────────────┐
  │                                                                              │
  │                              ◉  (mark, 48px, 40% opacity)                    │
  │                                                                              │
  │                          Nothing here yet                    xl/600          │
  │            Hold  ⌃ Right Ctrl  anywhere and start talking.   base, secondary │
  │                  └── Kbd, accent tone ──┘                                    │
  │                                                                              │
  │                     Your transcripts will appear here.       sm, muted       │
  │                     Text only - audio is never stored.                       │
  └──────────────────────────────────────────────────────────────────────────────┘

  ── history paused ──────────────────────────────────────────────────────────
  Top bar ⏸ turns into ▶ with an accent-tinted background, and a 28px bar appears
  under the header:  ⏸ PAUSED · New transcripts aren't being saved.   [ Resume ]
```

**Why this and not a grid of cards.** The dominant reuse action is "copy the thing I just dictated,
again" - the target is almost always in the top three rows, found by recency, not by browsing. A card
grid optimises for browsing a corpus; kiku's history is a *recency stack*. So: dense rows, day groups,
newest first, and the row itself is the copy target - **clicking anywhere on a row copies it** and the
row flashes `surface-selected` for 400ms with the trailing icon swapping `Copy → Check`. Double-click
or `Space` expands for reading and selecting sub-text. That turns the single most common action into a
zero-aim click, which a card grid cannot.

### 7.3 Settings

Same **880 × 620** window, same chrome, full-window route (not a modal - the hotkey capture field must
own the keyboard, and trapping keys inside a modal that also wants Escape is a fight you lose).

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│ ░ native title bar ░                                                             │
├──────────────────────────────────────────────────────────────────────────────────┤
│  ←  Settings                                                     kiku 1.2.0      │ 56
├───────────────────────┬──────────────────────────────────────────────────────────┤
│   180                 │                                                          │
│  ┌─────────────────┐  │  DICTATION                                        2xs    │ scroll-spy nav
│  │▌⌨  Dictation    │  │  ┌────────────────────────────────────────────────────┐  │ active: accent text
│  │ 🎤 Microphone   │  │  │  Hotkey                                            │  │ + 2px accent rail
│  │ ⚙  Model        │  │  │  Hold to talk, release to insert.        base/muted│  │
│  │ 🔊 Sound        │  │  │                                                    │  │
│  │ 🕐 History      │  │  │  ┌──────────────────────────────┐  ┌────────────┐  │  │
│  │ 🛡  Permissions │  │  │  │   ⌃  Right Ctrl              │  │  ↺ Reset   │  │  │ 40 tall capture field
│  │ ⬇  Updates      │  │  │  └──────────────────────────────┘  └────────────┘  │  │ click → "Press keys…"
│  │ ℹ  About        │  │  │                                                    │  │
│  └─────────────────┘  │  │  ⚠ Already used by macOS for Input Sources.  danger │  │ inline conflict error
│                       │  │    Try ⌥ Right Option or ⌘ Right Command.           │  │ + suggestion chips
│  sticky, own scroll   │  └────────────────────────────────────────────────────┘  │
│                       │                                                      32  │
│                       │  MICROPHONE                                              │
│                       │  ┌────────────────────────────────────────────────────┐  │
│                       │  │  Input device                    [ MacBook Pro  ▾] │  │ 48 Row + Select
│                       │  │  ────────────────────────────────────────────────  │  │
│                       │  │  Level            ▁▃▅▇█▇▅▃▁   (WaveBars h=12 n=9)  │  │ 48 live, only while
│                       │  └────────────────────────────────────────────────────┘  │    this section visible
│                       │                                                      32  │
│                       │  MODEL                                                   │
│                       │  ┌────────────────────────────────────────────────────┐  │
│                       │  │ ◉  Accurate                   [Recommended] 648 MB │  │ 64 radio rows
│                       │  │    Best quality. ~1.2 s on your machine.           │  │    measured, not claimed
│                       │  │ ────────────────────────────────────────────────── │  │
│                       │  │ ○  Fast                                    197 MB  │  │ 64
│                       │  │    Lower accuracy on names and jargon. ~0.4 s.     │  │
│                       │  │                                        ⬇ Download  │  │
│                       │  └────────────────────────────────────────────────────┘  │
│                       │                                                      32  │
│                       │  SOUND                                                   │
│                       │  │  Sound feedback                              [ ●━]│  │ 48
│                       │  │  Plays a tone when kiku starts and stops.          │  │
│                       │  │  Volume       🔈 ━━━━━━━●━━━━━━━ 🔊         45%    │  │ 48
│                       │                                                          │
│                       │  HISTORY                                                 │
│                       │  │  Save transcripts                            [ ●━]│  │ 48
│                       │  │  Text only. Audio is discarded immediately.        │  │
│                       │  │  Keep for                      [ Forever      ▾ ]  │  │ 48
│                       │  │  ────────────────────────────────────────────────  │  │
│                       │  │  1,284 transcripts · 412 KB      [ Delete all… ]   │  │ 48 danger ghost
│                       │                                                          │
│                       │  PERMISSIONS              (macOS: always shown;           │
│                       │  ┌────────────────────────  Win/Linux: mic only) ──────┐  │
│                       │  │ 🛡 Microphone                    ✓ Granted  success │  │ 48
│                       │  │ 🛡 Accessibility                 ⚠ Needed           │  │ 64 warning-tone Panel
│                       │  │    kiku types the text for you. macOS requires      │  │
│                       │  │    Accessibility for that.       [ Open Settings ↗] │  │
│                       │  └────────────────────────────────────────────────────┘  │
│                       │                                                          │
│                       │  UPDATES                                                 │
│                       │  │  Check for updates                           [ ●━]│  │ 48
│                       │  │  Checks a version number once a day.               │  │
│                       │  │  Sends nothing about you.              muted, 11px │  │
│                       │  │                  Last checked 2 hours ago  [Check] │  │
└───────────────────────┴──────────────────────────────────────────────────────────┘
  └ 24 ┘└ 180 ┘└ 32 ┘└──────────── content, max 560 ────────────┘└ flex ┘└ 24 ┘
```

**IA decision: sticky left section-nav + one continuous scroll.** Justification, and the two rejected
options:

- **Rejected - tabs.** Eight groups is too many for a tab strip in an 880px window, and tabs impose a
  "which tab is X in?" lookup cost on a user who visits settings perhaps four times ever. Worse, tabs
  hide the macOS Accessibility warning behind a click, and that warning is the single thing most likely
  to be blocking a new user.
- **Rejected - bare single scroll, no nav.** Honest and minimal, but the content is ~2.5 viewport
  heights and the two settings people actually return to (hotkey, microphone) are at the top while the
  one they need in an emergency (permissions) is at the bottom. Scrolling to find it is a tax.
- **Chosen - nav + scroll.** You get tabs' direct access *and* scroll's full-overview scannability.
  The nav is scroll-spy-linked, so it doubles as a map of the entire product's surface area - which,
  for a single-purpose tool, is itself a reassurance: *this is all there is.* It also degrades
  gracefully to a single column below 720px width, where the nav collapses into a row of chips.

Every group is a `<Panel>`, every line is a `<Row>`. Settings contributes **zero** new components.

### 7.4 Onboarding

Dedicated window, **580 × 540**, not resizable, centred, no title bar text. Four steps. It is short
enough that a step counter is the only progress affordance needed - no progress bar, no sidebar.

```
STEP 1 - WELCOME
┌────────────────────────────────────────────────────────────┐ 580
│                                                            │
│                         ◉  64px                            │ 80 top pad
│                                                            │
│                  Talk instead of typing.        2xl/600    │
│                                                            │
│      Hold a key, say what you mean, and kiku types it      │ base/secondary
│      wherever your cursor is. Everything runs on this      │ max 44ch, centred
│      computer - nothing is sent anywhere.                  │
│                                                            │
│      ┌────────────────────────────────────────────────┐    │
│      │ 🔒  No account. No cloud. No telemetry.        │    │ Panel tone="accent"
│      └────────────────────────────────────────────────┘    │ 48
│                                                            │
│                                                            │
├────────────────────────────────────────────────────────────┤
│  1 of 4                                     [ Get started ]│ 64 footer, lg button
└────────────────────────────────────────────────────────────┘

STEP 2 - CHOOSE A MODEL
┌────────────────────────────────────────────────────────────┐
│  Pick a speech model                            xl/600     │ 32 pad
│  You can change this later in Settings.      base/muted    │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ ◉  Accurate                    [Recommended] 648 MB  │  │ 88  selected: accent
│  │    Handles names, jargon, and accents well.          │  │     border + wash
│  │    About a second to transcribe a sentence.          │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ ○  Fast                                     197 MB   │  │ 88
│  │    Quicker and lighter. Misses unusual words         │  │
│  │    more often. Good on older machines.               │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  Downloaded once, then used offline forever.   xs/muted    │
├────────────────────────────────────────────────────────────┤
│  2 of 4                              [ Back ]  [ Continue ]│
└────────────────────────────────────────────────────────────┘

STEP 3 - SETTING UP  (download and permissions run CONCURRENTLY)
┌────────────────────────────────────────────────────────────┐
│  Setting up                                     xl/600     │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ ⬇  Downloading Accurate                              │  │
│  │    ████████████████████░░░░░░░░░░░░░  58%            │  │ 6px track
│  │    376 MB of 648 MB · 4.1 MB/s · about 1 min left    │  │ xs/muted
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  While that downloads:                        sm/secondary │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ 🎤 Microphone                          ✓ Granted     │  │ 56  live checklist,
│  │    So kiku can hear you.                             │  │     resolves in place
│  ├──────────────────────────────────────────────────────┤  │
│  │ 🛡 Accessibility        [ Open System Settings  ↗ ]  │  │ 72  macOS only
│  │    So kiku can type into other apps.                 │  │
│  │    Find kiku in the list and switch it on.           │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
├────────────────────────────────────────────────────────────┤
│  3 of 4                                        [ Continue ]│  disabled until
└────────────────────────────────────────────────────────────┘  download done + mic granted

STEP 4 - TRY IT
┌────────────────────────────────────────────────────────────┐
│  Try it now                                     xl/600     │
│                                                            │
│  Hold  ┌──────────────┐  and say something.                │
│        │ ⌃ Right Ctrl │  Kbd, accent, lg                   │
│        └──────────────┘                                    │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                                                      │  │ 132  live practice field
│  │   ▁ ▃ ▅ ▇ █ ▇ ▅ ▃ ▁      ← WaveBars, h=20, n=15     │  │      real focus, real paste
│  │                                                      │  │      target - the hotkey
│  │   your words appear here▏                            │  │      actually works here
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ✓ Nice. That's the whole product.            success, sm  │  appears after first success
│                                                            │
│  Change the hotkey any time in Settings.       xs/muted    │
├────────────────────────────────────────────────────────────┤
│  4 of 4                              [ Skip ]  [ Done ]    │  "Done" becomes primary
└────────────────────────────────────────────────────────────┘  only after a success
```

Two things worth defending in step 3: the download starts the instant the model is chosen (step 2 →
3 transition), and permission prompts run *during* it. The 648MB download is the long pole of
onboarding; making the user grant permissions first and *then* wait is a self-inflicted 2-minute dead
screen. Second: the "Continue" gate requires mic + download but **not** macOS Accessibility, because
that grant requires leaving the app and some users will come back later - blocking on it strands them
in the wizard. The main window shows a persistent warning banner until it is granted.

---

## 8. Icon set (Lucide)

Lucide only, 16px default, `stroke-width: 1.75` (Lucide's 2 is heavy at 16px on a dense UI; 1.75 with
these stroke colours is the sweet spot), `stroke-linecap: round`.

| Where | Action / meaning | Icon | Notes |
|---|---|---|---|
| **Overlay** | listening | *(none - waveform)* | zero icons in the signature state |
| | processing | *(none - 3 dots)* | not `Loader2`; a spinner reads "web page" |
| | done | `Check` | 18px, `--ov-success` |
| | cancelled | `X` | 14px, 60% opacity |
| | error | `TriangleAlert` | v0.400+ name; `AlertTriangle` on older |
| **Main - top bar** | search | `Search` | leading, inside Input |
| | clear search | `X` | trailing, `clearable` |
| | pause history | `Pause` | → `Play` when paused, accent tint |
| | open settings | `Settings2` | sliders form - more instrument than gear |
| **Main - rows** | copy | `Copy` | → `Check` for 1200ms on success |
| | delete one | `Trash2` | |
| | edit transcript | `Pencil` | see §11.2 |
| | expand / collapse | `ChevronRight` / `ChevronDown` | only on hover; rows expand on click too |
| | row selected | `SquareCheckBig` / `Square` | in `Row leading` |
| **Main - bulk** | select all | `ListChecks` | |
| | copy selected | `Copy` | |
| | delete selected | `Trash2` | danger variant |
| | empty state | *(brand mark)* | not a Lucide icon - the product's own face |
| | no search results | `SearchX` | compact EmptyState |
| **Settings nav** | dictation | `Keyboard` | |
| | microphone | `Mic` | `MicOff` when no device |
| | model | `Cpu` | not `Brain` - this is a technical product |
| | sound | `Volume2` | `VolumeX` when muted |
| | history | `History` | |
| | permissions | `ShieldCheck` / `ShieldAlert` | state-dependent in the nav itself |
| | updates | `ArrowDownToLine` | `RefreshCw` for the manual "Check now" button |
| | about | `Info` | |
| **Settings controls** | reset hotkey | `RotateCcw` | |
| | recording a hotkey | `Circle` (filled, accent) | pulses at 1Hz while capturing |
| | download model | `Download` | → `Check` when present locally |
| | delete model | `Trash2` | |
| | open OS settings | `ExternalLink` | always trailing, 14px |
| | select chevron | `ChevronDown` | |
| | slider ends | `Volume1` / `Volume2` | |
| | back to history | `ArrowLeft` | |
| **Onboarding** | privacy callout | `Lock` | not `ShieldCheck` - lock = data, shield = permission |
| | step complete | `Check` | success tone |
| | downloading | `Download` | |
| | skip | *(text only)* | |
| **Toasts** | copied | `Check` | |
| | deleted | `Trash2` | with Undo action |
| | error | `TriangleAlert` | |

Icons never appear alone as the sole carrier of meaning except where the action is universal (copy,
delete, search, close) and then always with an `aria-label` and a native `title`.

---

## 9. Accessibility

### 9.1 Focus

- `:focus-visible` only - never `:focus`. A mouse user clicking a row must not get a ring.
- The ring is a **double box-shadow**, so it reads on every background including accent fills:
  ```css
  outline: none;
  box-shadow: 0 0 0 2px var(--ring-offset), 0 0 0 4px var(--ring);
  ```
  On `Button variant="primary"`, `--ring-offset` is locally overridden to the button's own fill so the
  gap reads as a notch rather than a halo.
- Ring contrast: 3.9:1 (light) / 6.3:1 (dark) against `--surface`. Both clear WCAG 2.2 SC 1.4.11.
- Focus is never trapped except inside `Dialog`, where it is trapped, restores on close, and `Escape`
  closes.
- The window's initial focus on opening History is the **search input**; on opening Settings it is the
  first nav item.

### 9.2 Keyboard

**Main window**

| Key | Action |
|---|---|
| `/` or `Ctrl/⌘ F` | focus search |
| `↑ ↓` | move row focus (roving tabindex - the list is one tab stop) |
| `Enter` | copy focused transcript |
| `Space` | expand / collapse focused transcript |
| `Ctrl/⌘ C` | copy focused (or all selected) |
| `Delete` / `Backspace` | delete focused (or all selected), with confirm |
| `X` or `Ctrl/⌘ Click` | toggle selection of focused row |
| `Ctrl/⌘ A` | select all visible (respects the active search filter) |
| `Escape` | clear selection → clear search → close window, in that order |
| `Ctrl/⌘ ,` | open Settings |
| `Ctrl/⌘ W` | close window (app keeps running in tray) |

**Settings**

- Nav is a `<nav>` of anchor buttons, in tab order, `aria-current="true"` on the active section; each
  section is a `<section aria-labelledby>`.
- The hotkey capture field is a `<button role="button" aria-describedby>` that, on activation, enters
  capture mode: it takes a document-level `keydown` capture listener, calls `preventDefault()` on
  everything, announces "Press a key combination" via `aria-live="assertive"`, and commits on the
  `keyup` of the first non-modifier. `Escape` exits capture without committing - which means Escape
  cannot be bound as a hotkey, and that is the right trade.
- Sliders are real `<input type="range">` with `aria-valuetext="45 percent"`; `←/→` = 5%, `Home/End` =
  min/max.

**Overlay** - has no keyboard focus by construction (non-activating). The only key it responds to is
the global `Escape` handler that cancels dictation, registered as a temporary global shortcut for the
duration of `listening`/`processing` and unregistered immediately after.

### 9.3 Screen readers

The overlay is a non-activating, click-through OS window and is **not reachable by any assistive
technology** on any of the three platforms. Pretending otherwise with ARIA on it is theatre. So:

1. **Sound feedback is the accessible state channel and ships default-ON.** The settings copy says so:
   *"Tones tell you when kiku starts and stops listening. If you turn these off, the floating indicator
   is the only signal."*
2. When the main window is open, a visually-hidden `aria-live="polite"` region in it mirrors overlay
   state ("Listening", "Transcribing", "Inserted 21 words", "Microphone unavailable"). Errors use
   `assertive`.
3. The inserted text itself lands in the target app, where the user's own screen reader announces it
   natively. That is the real feedback loop and it already works.

Elsewhere: transcript rows are `<li>` with an accessible name of `"{time}, {duration}, {first 60
chars}"`; the copy button's label is `"Copy transcript from 9:42 AM"`, not `"Copy"`. Toasts are
`role="status"`. The destructive `Dialog` is `role="alertdialog"`.

### 9.4 Reduced motion

Covered in §4.3 and §5.7. The principle worth restating: **motion that carries data keeps running at a
reduced rate; motion that carries only polish stops entirely.** The waveform is the former.

### 9.5 Other

- Respect `prefers-contrast: more`: promote `--border` → `--border-strong`, raise `--text-muted` to
  `--text-secondary`, thicken the focus ring to 3px, and raise the overlay fill to 100% opacity.
- Support OS text scaling up to 150% - all heights in §3.3 are minimums (`min-height`), never fixed
  `height`, except the overlay capsule, which is fixed by design because it must not grow into the
  user's content.
- No colour-only encoding anywhere. `done` is a checkmark *and* green; `error` is a triangle, a rail,
  *and* red; the selected row is an accent rail *and* a filled checkbox.
- All animation respects `prefers-reduced-transparency` by raising overlay opacity to 100%.

---

## 10. Copy and tone

Short, lowercase-product-name, no exclamation marks, no "Oops", no "we". Say what happened and what to
do. Examples that should be lifted verbatim:

| Situation | Copy |
|---|---|
| Overlay error - mic busy | `Microphone is in use by another app` |
| Overlay error - no permission | `kiku needs microphone access` |
| Overlay error - too quiet | `Didn't catch that` |
| Overlay error - paste blocked (macOS) | `Grant Accessibility to type for you` |
| Hotkey conflict | `Already used by macOS for Input Sources.` + two suggestion chips |
| Delete confirm | `Delete 3 transcripts? This can't be undone.` |
| History toggle | `Text only. Audio is discarded the moment it's transcribed.` |
| Update toggle | `Checks a version number once a day. Sends nothing about you.` |
| Empty history | `Nothing here yet` |

---

## 11. Where I disagree with the brief

### 11.1 The overlay should not follow the OS theme
Argued in full in §5.2. Short form: the overlay's backdrop is not our window, it is an arbitrary app.
Ship one dark capsule.

### 11.2 "Text only, no audio" needs inline editing to be survivable
This is my one argued-for addition, and I want to be clear it is error recovery, not a feature.

Speech recognition gets names, jargon, and numbers wrong. With no stored audio, a wrong transcript is
**unrecoverable** - the user cannot re-listen, cannot re-run it on the better model, cannot verify. The
history list then becomes a graveyard of subtly-wrong text that the user must either retype or
re-dictate. That is the product's core failure mode and there is currently no UI for it.

The fix is ~40 lines: make the expanded row's text a `contentEditable` / `<textarea>` that saves on
blur, with a `Pencil` affordance. No new component (it is a `Row` in an expanded state), no new
concept, no network, no settings. It makes "copy again later" reliable, which is the entire reason
history exists. **I would not ship 1.0 without it.**

### 11.3 Two models named by size is asking the user a question they can't answer
"650MB vs 200MB" is a storage question. The user's actual question is "will it get my colleague's name
right, and how long will I wait?" Ship the labels **Accurate** and **Fast**, with size as secondary
metadata, and - after the first ten dictations - replace the estimated latency with the *measured*
median on that machine ("~0.9 s on your machine"). Measured numbers are the most credible thing a
local-first app can show, and they cost one rolling average.

### 11.4 Bottom-centre will collide with things, and the fix is non-obvious
Two failure modes that must be handled or the overlay looks broken:
- **Work area, not display bounds** (§5.4). Using display bounds puts the capsule behind the Dock.
- **Tiling window managers on X11** (i3, sway/X11, awesome) will tile an always-on-top window unless it
  is correctly hinted. Set `_NET_WM_WINDOW_TYPE_UTILITY` plus `_NET_WM_STATE_ABOVE` and
  `skip_taskbar`, and if the WM tiles it anyway, detect the size change and fall back to a compact
  mode. A Linux-first developer audience will hit this on day one.

### 11.5 Push-to-talk on a chord is ergonomically wrong
A held three-key chord for the product's only interaction is a repetitive-strain design. The default
should be a **single right-side modifier held down**: `Right Ctrl` on Windows/Linux, `Right ⌘` on
macOS. Caveats to handle: on many European layouts `Right Alt` is AltGr and must never be offered; on
macOS, capturing a bare modifier requires an event tap, which is the same Accessibility permission we
already request, so it is free. I am **not** recommending a toggle-to-talk mode - that is the feature
creep the owner rightly rejected, and hold-to-talk with a good cancel (§5.5) covers the long-dictation
case adequately.

### 11.6 The daily update check is the one place the privacy story can leak
It is a request to a server carrying an IP address and a version string. That is fine, and it is far
better than the alternative of never patching. But it must be stated in the UI (§10), the toggle must
be genuinely honoured, and the request must carry **no** unique identifier, no install ID, and no
user-agent beyond `kiku/1.2.0`. If a crash reporter is ever added, this document should be revisited;
as of 1.0, there isn't one, and that should stay true.

### 11.7 A click-through overlay cannot host an error the user must act on
Three consecutive failures should raise the main window at the relevant settings section (§5.5). A
260px capsule holding an error for 3.2 seconds is adequate for "didn't catch that" and completely
inadequate for "Accessibility permission was revoked by a macOS update". Build the escalation path.

---

*End of document.*
