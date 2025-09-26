# logdive Landing Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a production-quality Astro 5 SPA landing page in `landing/` subdirectory, deployable to `https://aryagorjipour.github.io/logdive/`, pixel-faithful to the Claude Design handoff with all content updated to v0.2.1.

**Architecture:** Astro 5 static site in `landing/` with `base: '/logdive'`. Pages (`index`, `docs`, `about`) share a `Base.astro` layout. Interactive bits (theme toggle, copy buttons, tabs, mobile nav, docs scroll-spy) are vanilla TypeScript Astro `<script>` islands. Roadmap and version data live in `src/data/roadmap.ts` — rendered at build time, zero client JS for the status section.

**Tech Stack:** Astro 5, TypeScript, Bun (package manager + runtime), Vanilla CSS (custom properties), IBM Plex Sans + JetBrains Mono (Google Fonts), GitHub Actions (`oven-sh/setup-bun@v2`) → `gh-pages` branch deploy.

**Branch:** `feat/landing-page`

**Spec:** `docs/superpowers/specs/2026-06-04-logdive-landing-design.md`

---

## File Map

```
landing/
  astro.config.ts
  package.json
  tsconfig.json
  bun.lockb                          (auto-generated)
  src/
    data/
      roadmap.ts                     version constants + typed roadmap data
    layouts/
      Base.astro                     <html>, <head>, theme pre-paint, fonts, global CSS
    components/
      Header.astro                   sticky nav, logo, mobile toggle, theme toggle
      Footer.astro                   logo, license, version badge, footer links
      CodeBlock.astro                <pre> wrapper + copy-to-clipboard <script>
      Tabs.astro                     tab buttons + panels + <script>
      TerminalPreview.astro          query demo terminal block (static)
      RoadmapStatus.astro            Now/Next/Later lanes rendered at build time
    pages/
      index.astro                    home page (7 sections)
      docs.astro                     docs page (sidebar + main content)
      about.astro                    about page
    styles/
      tokens.css                     CSS custom properties, spacing scale
      global.css                     reset, base, typography, layout helpers
      components.css                 header, footer, buttons, code, terminal, stat grid, pillars, tabs, compare, status, docs
.github/
  workflows/
    landing.yml                      build + deploy on push to main, paths: landing/**
```

---

## Task 1: Scaffold Astro project with Bun

**Files:**
- Create: `landing/package.json`
- Create: `landing/astro.config.ts`
- Create: `landing/tsconfig.json`

- [ ] **Step 1: Initialise Astro with Bun**

```bash
cd /home/arysmart/Projects/Rust/logdive
mkdir landing && cd landing
bun create astro@latest . --template minimal --typescript strict --no-git --install
```

Expected output: Astro scaffold in `landing/`, `bun.lockb` created.

- [ ] **Step 2: Replace `astro.config.ts` with correct config**

```typescript
// landing/astro.config.ts
import { defineConfig } from 'astro/config';

export default defineConfig({
  site: 'https://aryagorjipour.github.io',
  base: '/logdive',
  output: 'static',
  build: {
    assets: '_assets',
  },
});
```

- [ ] **Step 3: Replace `tsconfig.json`**

```json
{
  "extends": "astro/tsconfigs/strict",
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@data/*": ["src/data/*"],
      "@components/*": ["src/components/*"],
      "@layouts/*": ["src/layouts/*"]
    }
  }
}
```

- [ ] **Step 4: Verify build runs clean**

```bash
cd landing
bun run build
```

Expected: `dist/` created, no errors.

- [ ] **Step 5: Commit scaffold**

```bash
cd /home/arysmart/Projects/Rust/logdive
git add landing/
git commit -m "feat(landing): scaffold Astro 5 project with Bun"
```

---

## Task 2: CSS design system

**Files:**
- Create: `landing/src/styles/tokens.css`
- Create: `landing/src/styles/global.css`
- Create: `landing/src/styles/components.css`

- [ ] **Step 1: Create `tokens.css`**

```css
/* landing/src/styles/tokens.css */
@import url('https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap');

:root {
  --bg: #FBF9F6;
  --surface: #FFFFFF;
  --surface-hover: #F4F1EA;
  --border: #E8E3DC;
  --text: #0E1419;
  --text-muted: #5B5A57;
  --brand: #0A2540;
  --accent: #B85D44;
  --success: #1F7A3A;
  --warn: #A0660A;
  --error: #A02818;

  --radius-sm: 6px;
  --radius-md: 8px;
  --radius-lg: 12px;

  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-6: 24px;
  --space-8: 32px;
  --space-12: 48px;
  --space-16: 64px;
  --space-24: 96px;
  --space-32: 128px;

  --shadow-card: 0 1px 2px rgba(14,20,25,0.04), 0 4px 12px rgba(14,20,25,0.04);
  --ease: cubic-bezier(0.32, 0.72, 0, 1);

  --font-sans: 'IBM Plex Sans', sans-serif;
  --font-mono: 'JetBrains Mono', monospace;

  --content-width: 1120px;
}

@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --bg: #0B1014;
    --surface: #141A20;
    --surface-hover: #1B232B;
    --border: #1F2730;
    --text: #E8E4DC;
    --text-muted: #8B8D90;
    --brand: #E8E4DC;
    --accent: #D9714E;
    --success: #3FC771;
    --warn: #D49B36;
    --error: #E66956;
    --shadow-card: 0 1px 2px rgba(0,0,0,0.4), 0 4px 12px rgba(0,0,0,0.3);
  }
}

[data-theme="dark"] {
  --bg: #0B1014;
  --surface: #141A20;
  --surface-hover: #1B232B;
  --border: #1F2730;
  --text: #E8E4DC;
  --text-muted: #8B8D90;
  --brand: #E8E4DC;
  --accent: #D9714E;
  --success: #3FC771;
  --warn: #D49B36;
  --error: #E66956;
  --shadow-card: 0 1px 2px rgba(0,0,0,0.4), 0 4px 12px rgba(0,0,0,0.3);
}
```

- [ ] **Step 2: Create `global.css`**

```css
/* landing/src/styles/global.css */
*, *::before, *::after { box-sizing: border-box; }

html { scroll-behavior: smooth; }

body {
  margin: 0;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-sans);
  font-weight: 400;
  font-size: 16px;
  line-height: 1.5;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-rendering: optimizeLegibility;
}

a {
  color: var(--text);
  text-decoration: none;
  transition: opacity 200ms var(--ease);
}
a:hover { opacity: 0.7; }
a.link-underline { border-bottom: 1px solid var(--border); }
a.link-underline:hover { border-bottom-color: var(--accent); opacity: 1; }

p { margin: 0 0 var(--space-4) 0; text-wrap: pretty; }
p:last-child { margin-bottom: 0; }

h1, h2, h3, h4 {
  font-family: var(--font-sans);
  font-weight: 600;
  letter-spacing: -0.02em;
  margin: 0;
  color: var(--text);
  text-wrap: balance;
}

h1 { font-size: 64px; line-height: 1.05; }
h2 { font-size: 36px; line-height: 1.15; }
h3 { font-size: 20px; line-height: 1.3; letter-spacing: -0.01em; }

@media (max-width: 720px) {
  h1 { font-size: 44px; }
  h2 { font-size: 28px; }
}

code, pre, .mono { font-family: var(--font-mono); font-feature-settings: "calt" 1; }

hr { border: 0; border-top: 1px solid var(--border); margin: 0; }
button { font-family: inherit; }

.wrap {
  max-width: var(--content-width);
  margin: 0 auto;
  padding: 0 var(--space-6);
}

section { padding: var(--space-24) 0; }
section.tight { padding: var(--space-16) 0; }

@media (max-width: 720px) {
  section { padding: var(--space-16) 0; }
  section.tight { padding: var(--space-12) 0; }
}

.eyebrow {
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-muted);
  margin-bottom: var(--space-4);
  display: block;
}

.muted { color: var(--text-muted); }
.accent { color: var(--accent); }

.section-head { margin-bottom: var(--space-12); max-width: 56ch; }
.section-head h2 { margin-bottom: var(--space-3); }
.section-head p { color: var(--text-muted); font-size: 16px; }

@keyframes om-rise {
  from { opacity: 0; transform: translateY(8px); }
  to   { opacity: 1; transform: translateY(0); }
}
main > section,
.docs > * {
  animation: om-rise 400ms var(--ease) both;
}
main > section:nth-child(1), .docs > *:nth-child(1) { animation-delay: 0ms; }
main > section:nth-child(2), .docs > *:nth-child(2) { animation-delay: 80ms; }
main > section:nth-child(3) { animation-delay: 160ms; }
main > section:nth-child(4) { animation-delay: 240ms; }
main > section:nth-child(5) { animation-delay: 320ms; }
main > section:nth-child(n+6) { animation-delay: 320ms; }

@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation: none !important;
    transition: none !important;
    scroll-behavior: auto !important;
  }
}
```

- [ ] **Step 3: Create `components.css`**

```css
/* landing/src/styles/components.css */

/* --- header --- */
.site-header {
  position: sticky; top: 0; z-index: 50;
  height: 64px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  backdrop-filter: saturate(140%) blur(8px);
}
.site-header .wrap {
  height: 100%; display: flex; align-items: center; gap: var(--space-8);
}
.logo { display: flex; align-items: center; gap: var(--space-3); color: var(--text); }
.logo svg { display: block; flex: none; }
.wordmark {
  font-family: var(--font-sans); font-weight: 600;
  font-size: 20px; letter-spacing: -0.02em; color: var(--text);
}
.nav { margin-left: auto; display: flex; align-items: center; gap: var(--space-6); }
.nav a { font-size: 14px; font-weight: 500; color: var(--text-muted); }
.nav a:hover { color: var(--text); opacity: 1; }
.nav a.is-active { color: var(--text); }
.icon-btn {
  appearance: none; background: transparent;
  border: 1px solid var(--border); color: var(--text-muted);
  width: 36px; height: 36px; border-radius: var(--radius-sm);
  display: inline-flex; align-items: center; justify-content: center;
  cursor: pointer;
  transition: opacity 200ms var(--ease), border-color 200ms var(--ease);
}
.icon-btn:hover { color: var(--text); border-color: var(--text-muted); }
.icon-btn svg { width: 16px; height: 16px; }
.theme-toggle .sun { display: none; }
.theme-toggle .moon { display: block; }
[data-theme="dark"] .theme-toggle .sun { display: block; }
[data-theme="dark"] .theme-toggle .moon { display: none; }
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) .theme-toggle .sun { display: block; }
  :root:not([data-theme="light"]) .theme-toggle .moon { display: none; }
}
.mobile-toggle { display: none; }
@media (max-width: 720px) {
  .nav {
    display: none; position: absolute; top: 64px; left: 0; right: 0;
    background: var(--surface); border-bottom: 1px solid var(--border);
    flex-direction: column; align-items: stretch; gap: 0; padding: var(--space-2) 0;
  }
  .nav.is-open { display: flex; }
  .nav a { padding: var(--space-3) var(--space-6); border-bottom: 1px solid var(--border); }
  .nav a:last-child { border-bottom: 0; }
  .mobile-toggle { display: inline-flex; margin-left: auto; }
  .theme-toggle { margin-left: var(--space-2); }
}

/* --- buttons --- */
.btn {
  display: inline-flex; align-items: center; justify-content: center;
  gap: var(--space-2); height: 44px; padding: 0 var(--space-6);
  border-radius: var(--radius-sm); font-family: var(--font-sans);
  font-size: 14px; font-weight: 500; letter-spacing: -0.005em;
  cursor: pointer; border: 1px solid transparent;
  transition: opacity 200ms var(--ease), border-color 200ms var(--ease);
  white-space: nowrap;
}
.btn-primary { background: var(--accent); color: #FFFFFF; }
.btn-primary:hover { opacity: 0.9; }
.btn-ghost { background: transparent; border-color: var(--border); color: var(--text); }
.btn-ghost:hover { border-color: var(--text-muted); opacity: 1; }

/* --- hero --- */
.hero { position: relative; padding-top: var(--space-24); padding-bottom: var(--space-24); overflow: hidden; }
.hero-lines { position: absolute; inset: 0; pointer-events: none; z-index: 0; }
.hero-lines::before {
  content: "";
  position: absolute; inset: 0;
  background-image: repeating-linear-gradient(to bottom, transparent 0, transparent 79px, var(--text) 79px, var(--text) 80px);
  opacity: 0.06;
}
.hero .wrap { position: relative; z-index: 1; }
.hero h1 { max-width: 14ch; margin-bottom: var(--space-6); }
.hero-sub {
  font-size: 20px; line-height: 1.5; color: var(--text-muted);
  max-width: 56ch; margin-bottom: var(--space-8);
}
.hero-cta { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-3); }
.install-code {
  display: inline-flex; align-items: center; height: 44px;
  padding: 0 var(--space-3) 0 var(--space-4); gap: var(--space-3);
  background: var(--surface); border: 1px solid var(--border);
  border-radius: var(--radius-sm); font-family: var(--font-mono);
  font-size: 14px; color: var(--text); position: relative;
}
.install-code .prompt { color: var(--text-muted); margin-right: var(--space-2); }
.install-code .copy-btn { position: static; }

/* --- code blocks --- */
.code-block {
  position: relative; background: var(--surface);
  border: 1px solid var(--border); border-radius: var(--radius-md);
  padding: var(--space-4); font-family: var(--font-mono);
  font-size: 14px; line-height: 1.6; color: var(--text); overflow-x: auto;
}
.code-block pre {
  margin: 0; font-family: inherit; font-size: inherit;
  line-height: inherit; color: inherit; white-space: pre;
}
.code-block .prompt { color: var(--text-muted); user-select: none; }
.code-block .kw { color: var(--brand); font-weight: 500; }
[data-theme="dark"] .code-block .kw,
:root:not([data-theme="light"]) .code-block .kw { color: var(--text); }
.code-block .cm { color: var(--text-muted); }
.code-block .str { color: var(--success); }
.code-block .accent-tok { color: var(--accent); }
.copy-btn {
  position: absolute; top: var(--space-2); right: var(--space-2);
  appearance: none; background: var(--bg); border: 1px solid var(--border);
  border-radius: var(--radius-sm); color: var(--text-muted);
  font-family: var(--font-mono); font-size: 11px; letter-spacing: 0.04em;
  padding: 4px 8px; cursor: pointer;
  transition: opacity 200ms var(--ease), color 200ms var(--ease), border-color 200ms var(--ease);
  text-transform: uppercase;
}
.copy-btn:hover { color: var(--text); border-color: var(--text-muted); }
.copy-btn.is-copied { color: var(--success); border-color: var(--success); }
.code-inline {
  font-family: var(--font-mono); font-size: 0.9em;
  background: var(--surface); border: 1px solid var(--border);
  border-radius: 4px; padding: 1px 6px; color: var(--text);
}

/* --- terminal preview --- */
.terminal {
  background: var(--surface); border: 1px solid var(--border);
  border-radius: var(--radius-md); overflow: hidden; box-shadow: var(--shadow-card);
}
.terminal-head {
  padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border);
  display: flex; align-items: center; gap: var(--space-2);
  font-family: var(--font-mono); font-size: 12px; color: var(--text-muted);
}
.terminal-dots { display: flex; gap: 6px; margin-right: var(--space-3); }
.terminal-dots span { width: 10px; height: 10px; border-radius: 50%; background: var(--border); }
.terminal-body {
  padding: var(--space-6); font-family: var(--font-mono);
  font-size: 14px; line-height: 1.7;
}
.terminal-body .note {
  color: var(--text-muted); font-size: 13px; margin-top: 2px;
  margin-bottom: var(--space-4); padding-left: 18px;
}
.terminal-body .note:last-child { margin-bottom: 0; }
.terminal-body .cmd { color: var(--text); }
.terminal-body .cmd .prompt { color: var(--text-muted); margin-right: 8px; }
.terminal-body .cmd .accent-tok { color: var(--accent); }

/* --- stat grid --- */
.stat-grid {
  display: grid; grid-template-columns: repeat(4, 1fr); gap: var(--space-4);
}
@media (max-width: 960px) { .stat-grid { grid-template-columns: repeat(2, 1fr); } }
@media (max-width: 520px) { .stat-grid { grid-template-columns: 1fr; } }
.stat-card {
  background: var(--surface); border: 1px solid var(--border);
  border-radius: var(--radius-md); padding: var(--space-6);
}
.stat-num {
  font-family: var(--font-sans); font-weight: 600; font-size: 36px;
  letter-spacing: -0.03em; line-height: 1; color: var(--text);
  margin-bottom: var(--space-3); display: flex; align-items: baseline; gap: 4px;
}
.stat-num .unit { font-size: 18px; font-weight: 500; letter-spacing: -0.01em; color: var(--text-muted); }
.stat-desc { font-size: 13px; line-height: 1.45; color: var(--text-muted); }

/* --- feature pillars --- */
.pillar-grid {
  display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--space-4);
}
@media (max-width: 880px) { .pillar-grid { grid-template-columns: 1fr; } }
.pillar {
  background: var(--surface); border: 1px solid var(--border);
  border-radius: var(--radius-md); padding: var(--space-6);
}
.pillar svg { color: var(--text); margin-bottom: var(--space-4); display: block; }
.pillar h3 { margin-bottom: var(--space-2); }
.pillar p { color: var(--text-muted); font-size: 14px; line-height: 1.55; }

/* --- tabs --- */
.tabs {
  display: flex; gap: 0; border-bottom: 1px solid var(--border); margin-bottom: var(--space-4);
}
.tab-btn {
  appearance: none; background: transparent; border: 0;
  border-bottom: 1px solid transparent; margin-bottom: -1px;
  padding: var(--space-3) var(--space-4); font-family: var(--font-mono);
  font-size: 13px; color: var(--text-muted); cursor: pointer;
  transition: color 200ms var(--ease), border-color 200ms var(--ease);
}
.tab-btn:hover { color: var(--text); }
.tab-btn.is-active { color: var(--text); border-bottom-color: var(--accent); }
.tab-panel { display: none; }
.tab-panel.is-active { display: block; }

/* --- comparison --- */
.compare {
  background: var(--surface); border: 1px solid var(--border);
  border-radius: var(--radius-md); padding: var(--space-8);
}
.compare p { font-size: 16px; line-height: 1.65; }
.compare p + p { margin-top: var(--space-4); }
.compare strong { color: var(--text); font-weight: 600; }
.compare .honest {
  margin-top: var(--space-6); padding-top: var(--space-6);
  border-top: 1px solid var(--border); color: var(--text-muted); font-size: 14px;
}
.compare .honest strong { color: var(--accent); font-weight: 500; }

/* --- project status section --- */
#status > header { margin-bottom: var(--space-12); max-width: 56ch; }
#status > header h2 { margin-bottom: var(--space-3); }
#status .meta {
  font-size: 14px; color: var(--text-muted);
  display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-2);
}
#status .meta .sep { opacity: 0.6; }
#status .meta a { color: var(--text-muted); border-bottom: 1px solid var(--border); padding-bottom: 1px; }
#status .meta a:hover { color: var(--text); border-bottom-color: var(--accent); opacity: 1; }

.status-lanes { display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--space-8); }
@media (max-width: 900px) { .status-lanes { grid-template-columns: 1fr; } }

.status-lane {
  background: var(--surface); border: 1px solid var(--border);
  border-radius: var(--radius-md); padding: var(--space-6);
}
@media (max-width: 600px) { .status-lane { padding: var(--space-4); } }
.status-lane > h3 {
  font-size: 22px; font-weight: 600; letter-spacing: -0.02em;
  margin: 0 0 var(--space-2) 0; color: var(--text);
}
.status-lane > p.lane-desc {
  font-size: 13px; color: var(--text-muted); margin: 0 0 var(--space-6) 0; line-height: 1.5;
}
.status-lane > ul {
  list-style: none; margin: 0; padding: 0;
  display: flex; flex-direction: column; gap: var(--space-4);
}
.status-lane li {
  padding: var(--space-3); margin: calc(var(--space-3) * -1);
  border-radius: var(--radius-sm); transition: background-color 200ms var(--ease);
}
.status-lane li + li { margin-top: var(--space-1); }
.status-lane li:hover { background: var(--surface-hover); }
.status-lane li > a,
.status-lane li > .item-title {
  display: inline-block; font-size: 16px; font-weight: 500; color: var(--text);
  line-height: 1.35; letter-spacing: -0.005em;
  text-decoration: underline; text-decoration-color: transparent;
  text-decoration-thickness: 2px; text-underline-offset: 4px;
  transition: text-decoration-color 200ms var(--ease), opacity 200ms var(--ease);
}
.status-lane li > .item-title { text-decoration: none; }
.status-lane li > a:hover { text-decoration-color: var(--accent); opacity: 1; }
.status-lane li .item-meta {
  font-family: var(--font-mono); font-size: 13px; line-height: 1.4; color: var(--text-muted); margin-top: 4px;
}
.status-lane li .item-desc {
  font-size: 14px; line-height: 1.5; color: var(--text); margin: var(--space-2) 0 0 0;
  display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden;
}
.status-lane li.lane-empty {
  font-style: italic; color: var(--text-muted); font-size: 14px; margin: 0; padding: 0;
}
.status-lane li.lane-empty:hover { background: transparent; }

.status-shipped { margin-top: var(--space-16); }
.status-shipped > h3 {
  font-size: 22px; font-weight: 600; letter-spacing: -0.02em; margin: 0 0 var(--space-2) 0; color: var(--text);
}
.status-shipped .shipped-meta { font-size: 13px; color: var(--text-muted); margin: 0 0 var(--space-6) 0; }
.status-shipped .shipped-meta a { color: var(--text-muted); border-bottom: 1px solid var(--border); }
.status-shipped .shipped-meta a:hover { color: var(--text); border-bottom-color: var(--accent); opacity: 1; }
#shipped-list { list-style: none; margin: 0; padding: 0; }
.status-shipped details { padding: var(--space-6) 0; border-bottom: 1px solid var(--border); }
#shipped-list > li:last-child details { border-bottom: 0; }
#shipped-list > li:first-child details { padding-top: 0; }
.status-shipped summary {
  list-style: none; cursor: pointer; display: flex; align-items: baseline;
  gap: var(--space-2); padding-right: 28px; position: relative; user-select: none;
  transition: opacity 200ms var(--ease);
}
.status-shipped summary::-webkit-details-marker { display: none; }
.status-shipped summary::marker { display: none; content: ""; }
.status-shipped summary:hover { opacity: 0.8; }
.status-shipped .release-version {
  font-size: 18px; font-weight: 600; letter-spacing: -0.02em; color: var(--text);
}
.status-shipped .release-date { font-size: 14px; color: var(--text-muted); }
.status-shipped summary::after {
  content: "›"; position: absolute; right: 4px; top: 50%; transform: translateY(-50%);
  font-size: 22px; line-height: 1; color: var(--text-muted); transition: transform 200ms var(--ease);
}
.status-shipped details[open] > summary::after { transform: translateY(-50%) rotate(90deg); }
.status-shipped .release-highlights {
  list-style: none; margin: var(--space-4) 0 0 0; padding: 0;
  display: flex; flex-direction: column; gap: var(--space-2);
}
.status-shipped .release-highlights li {
  position: relative; padding-left: var(--space-6);
  font-size: 15px; line-height: 1.55; color: var(--text);
}
.status-shipped .release-highlights li::before {
  content: ""; position: absolute; left: 0; top: 11px;
  width: 12px; height: 1px; background: var(--text-muted); opacity: 0.6;
}
.status-oos {
  margin-top: var(--space-12); font-size: 14px; color: var(--text-muted); max-width: 60ch;
}
.status-oos a { color: var(--text-muted); border-bottom: 1px solid var(--border); }
.status-oos a:hover { color: var(--text); border-bottom-color: var(--accent); opacity: 1; }

/* --- footer --- */
.site-footer {
  border-top: 1px solid var(--border); padding: var(--space-12) 0; background: var(--bg);
}
.site-footer .wrap {
  display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-6);
  font-size: 13px; color: var(--text-muted);
}
.site-footer .logo .wordmark { font-size: 16px; }
.site-footer .footer-links { margin-left: auto; display: flex; gap: var(--space-6); }
.site-footer .footer-links a { color: var(--text-muted); }
.site-footer .footer-links a:hover { color: var(--text); opacity: 1; }
.site-footer .version {
  font-family: var(--font-mono); font-size: 12px; padding: 2px 8px;
  border: 1px solid var(--border); border-radius: 999px; color: var(--text-muted);
}

/* --- about page --- */
.two-col { display: grid; grid-template-columns: 3fr 2fr; gap: var(--space-16); align-items: start; }
@media (max-width: 900px) { .two-col { grid-template-columns: 1fr; gap: var(--space-12); } }
.prose p { font-size: 17px; line-height: 1.65; color: var(--text); }
.prose p + p { margin-top: var(--space-4); }
.nongoals {
  background: var(--surface); border: 1px solid var(--border);
  border-radius: var(--radius-md); padding: var(--space-6);
}
.nongoals h3 { margin-bottom: var(--space-4); }
.nongoals ul { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: var(--space-3); }
.nongoals li {
  font-family: var(--font-mono); font-size: 13px; line-height: 1.5; color: var(--text);
  padding-left: 22px; position: relative;
}
.nongoals li::before {
  content: "×"; position: absolute; left: 0; color: var(--accent);
  font-family: var(--font-sans); font-weight: 500; font-size: 15px; top: -1px;
}

/* --- docs layout --- */
.docs {
  display: grid; grid-template-columns: 240px 1fr; gap: var(--space-12);
  padding: var(--space-12) var(--space-6); max-width: 1280px;
  margin: 0 auto; align-items: start;
}
@media (max-width: 900px) {
  .docs { grid-template-columns: 1fr; gap: var(--space-8); }
  .docs-nav { position: static !important; }
}
.docs-nav { position: sticky; top: 88px; align-self: start; font-size: 14px; }
.docs-nav h4 {
  font-family: var(--font-mono); font-size: 11px; text-transform: uppercase;
  letter-spacing: 0.08em; color: var(--text-muted); font-weight: 500;
  margin: var(--space-6) 0 var(--space-2) 0;
}
.docs-nav h4:first-child { margin-top: 0; }
.docs-nav ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }
.docs-nav a {
  display: block; padding: 6px 12px; margin-left: -12px; color: var(--text-muted);
  border-left: 2px solid transparent; font-size: 14px; line-height: 1.4;
  transition: color 200ms var(--ease), border-color 200ms var(--ease);
}
.docs-nav a:hover { color: var(--text); opacity: 1; }
.docs-nav a.is-active { color: var(--text); border-left-color: var(--accent); }

.docs-main { min-width: 0; max-width: 960px; }
.docs-main h2 {
  font-size: 28px; margin-top: var(--space-16); margin-bottom: var(--space-4); scroll-margin-top: 80px;
}
.docs-main > section:first-child h2 { margin-top: 0; }
.docs-main h3 { font-size: 17px; margin-top: var(--space-8); margin-bottom: var(--space-3); letter-spacing: -0.01em; }
.docs-main p { font-size: 15px; line-height: 1.65; color: var(--text); max-width: 70ch; }
.docs-main p.lede { color: var(--text-muted); font-size: 16px; }
.docs-main .code-block { margin: var(--space-4) 0; }
.docs-main ul { padding-left: var(--space-6); margin: 0 0 var(--space-4) 0; }
.docs-main ul li { margin-bottom: var(--space-1); font-size: 15px; color: var(--text); }

.kv-list {
  display: grid; grid-template-columns: 200px 1fr;
  gap: var(--space-2) var(--space-6); font-size: 14px; margin: var(--space-4) 0; max-width: 720px;
}
.kv-list dt {
  font-family: var(--font-mono); color: var(--text); padding: 6px 0; border-top: 1px solid var(--border);
}
.kv-list dd { margin: 0; color: var(--text-muted); padding: 6px 0; border-top: 1px solid var(--border); line-height: 1.5; }
.kv-list dt:first-of-type, .kv-list dd:nth-of-type(1) { border-top: 0; }
@media (max-width: 640px) {
  .kv-list { grid-template-columns: 1fr; gap: 0; }
  .kv-list dd { border-top: 0; padding-top: 0; padding-bottom: var(--space-3); }
}

.grammar {
  background: var(--surface); border: 1px solid var(--border); border-left: 2px solid var(--accent);
  border-radius: var(--radius-sm); padding: var(--space-4) var(--space-6);
  font-family: var(--font-mono); font-size: 13px; line-height: 1.7; color: var(--text);
  margin: var(--space-4) 0; white-space: pre; overflow-x: auto;
}
.grammar .nt { color: var(--accent); }
.grammar .tok { color: var(--text-muted); }
```

- [ ] **Step 4: Verify CSS files exist**

```bash
ls landing/src/styles/
```

Expected: `tokens.css  global.css  components.css`

- [ ] **Step 5: Commit CSS**

```bash
cd /home/arysmart/Projects/Rust/logdive
git add landing/src/styles/
git commit -m "feat(landing): add design system CSS (tokens, global, components)"
```

---

## Task 3: Data file — roadmap and version

**Files:**
- Create: `landing/src/data/roadmap.ts`

- [ ] **Step 1: Create `landing/src/data/roadmap.ts`**

```typescript
// landing/src/data/roadmap.ts

export const CURRENT_VERSION = '0.2.1';
export const IN_PROGRESS_VERSION = '0.3.0';
export const ROADMAP_UPDATED = '2026-06-01';

export type ItemKind = 'feature' | 'perf' | 'infra' | 'docs';

export interface RoadmapItem {
  title: string;
  kind: ItemKind;
  issue?: number;
  description?: string;
  note?: string;
  versionTarget?: string;
}

export interface ShippedRelease {
  version: string;
  date: string;
  highlights: string[];
}

export const roadmapNow: RoadmapItem[] = [
  {
    title: 'Parenthesised query expressions',
    kind: 'feature',
    issue: 51,
    description: 'Grouping with ( ) so NOT and OR compose cleanly without precedence surprises.',
  },
  {
    title: 'Distroless Docker image',
    kind: 'infra',
    description: 'Replace debian:bookworm-slim runtime with distroless/cc for a smaller attack surface.',
  },
  {
    title: 'Generated columns',
    kind: 'feature',
    description: 'Index derived fields at ingest time to accelerate repeated queries on computed values.',
  },
];

export const roadmapNext: RoadmapItem[] = [
  {
    title: 'Windows support for --follow mode',
    kind: 'infra',
    versionTarget: '0.4.0',
    description: 'Rotation and truncation detection on NTFS using ReadDirectoryChangesW.',
  },
  {
    title: 'Structured output formats: yaml, csv',
    kind: 'feature',
    issue: 39,
    versionTarget: '0.3.0',
  },
  {
    title: 'Configurable retention by source',
    kind: 'feature',
    issue: 44,
    versionTarget: '0.4.0',
    description: 'Let prune --older-than vary per source tag instead of one global cutoff.',
  },
];

export const roadmapLater: RoadmapItem[] = [
  {
    title: 'Authentication for the HTTP API',
    kind: 'infra',
    note: 'waiting on feedback',
    description: 'Currently a non-goal. Reconsidering only if the localhost-only stance is causing real pain.',
  },
  {
    title: 'Multi-file ingest with glob patterns',
    kind: 'feature',
    issue: 33,
    note: 'considering',
  },
  {
    title: 'Aggregations: count, distinct, group-by',
    kind: 'feature',
    issue: 21,
    note: 'considering',
  },
  {
    title: 'Browser-based query UI',
    kind: 'feature',
    note: 'needs spec',
    description: 'Listed for completeness; explicit v1 non-goal. Would need a separate crate and a real design pass.',
  },
];

export const shipped: ShippedRelease[] = [
  {
    version: '0.2.1',
    date: '2026-06-01',
    highlights: [
      'Security test suite: SQL injection, LIKE wildcard escaping, resource exhaustion (1k-disjunct OR, 10 MB line).',
      'Functional tests: proptest property-based, cross-format dedup, concurrent CLI ingest, parser edge cases, follow-mode, API integration, prune boundary.',
      'Supply-chain hardening: cargo-deny, SBOM via cargo-cyclonedx, daily audit CI, CI permissions: contents: read.',
      'Allocation improvements: LogEntry::with_tag takes &str, entry_to_json_string avoids clone per HTTP row.',
    ],
  },
  {
    version: '0.2.0',
    date: '2026-05-15',
    highlights: [
      'Added OR to the query language — (level=error OR level=warn) AND service=payments.',
      'Ingestion now accepts logfmt and plain-text lines alongside JSON.',
      'New --follow mode tails files with rotation and truncation detection.',
      'Introduced the prune subcommand for time-based retention with --older-than.',
      'HTTP API gained /version and /capabilities endpoints, plus configurable CORS.',
      'Docker image is now multi-stage and multi-arch, down to ~9 MB compressed.',
    ],
  },
  {
    version: '0.1.0',
    date: '2026-04-19',
    highlights: [
      'Initial release with ingest, query, and stats subcommands on the CLI.',
      'SQLite-backed local indexing with blake3 content hashing for dedup.',
      'Typed query language supporting AND, =, !=, >, <, contains, last, and since.',
      'Read-only HTTP server exposing /query as NDJSON and /stats as JSON.',
    ],
  },
];
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd landing
bun run build
```

Expected: no TypeScript errors.

- [ ] **Step 3: Commit**

```bash
cd /home/arysmart/Projects/Rust/logdive
git add landing/src/data/roadmap.ts
git commit -m "feat(landing): add typed roadmap and version data"
```

---

## Task 4: Base layout + shared components

**Files:**
- Create: `landing/src/layouts/Base.astro`
- Create: `landing/src/components/Header.astro`
- Create: `landing/src/components/Footer.astro`

- [ ] **Step 1: Create `Base.astro`**

```astro
---
// landing/src/layouts/Base.astro
export interface Props {
  title: string;
  description: string;
  activeNav?: 'docs' | 'about';
}
const { title, description, activeNav } = Astro.props;
import { CURRENT_VERSION } from '@data/roadmap';
import '../styles/tokens.css';
import '../styles/global.css';
import '../styles/components.css';
import Header from '@components/Header.astro';
import Footer from '@components/Footer.astro';
---
<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>{title}</title>
  <meta name="description" content={description} />
  <link rel="icon" href="data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%230E1419' stroke-width='1.5'><line x1='3' y1='8' x2='21' y2='8' opacity='0.4'/><line x1='3' y1='14' x2='21' y2='14' opacity='0.4'/><line x1='3' y1='20' x2='21' y2='20' opacity='0.4'/><path d='M20 4 Q 14 6 11 14 T 4 22' stroke-width='2' stroke-linecap='round'/><circle cx='4' cy='22' r='1.5' fill='%23B85D44' stroke='none'/></svg>" />
  <script is:inline>
    (function(){
      try {
        var t = localStorage.getItem('logdive-theme');
        if (t === 'light' || t === 'dark') document.documentElement.setAttribute('data-theme', t);
      } catch(e){}
    })();
  </script>
</head>
<body>
  <Header activeNav={activeNav} />
  <slot />
  <Footer version={CURRENT_VERSION} />
</body>
</html>
```

- [ ] **Step 2: Create `Header.astro`**

```astro
---
// landing/src/components/Header.astro
export interface Props { activeNav?: 'docs' | 'about'; }
const { activeNav } = Astro.props;
const base = import.meta.env.BASE_URL;
---
<header class="site-header">
  <div class="wrap">
    <a class="logo" href={`${base}/`} aria-label="logdive home">
      <svg width="28" height="28" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <line x1="3" y1="8"  x2="21" y2="8"  stroke="currentColor" stroke-width="1.5" opacity="0.4"/>
        <line x1="3" y1="14" x2="21" y2="14" stroke="currentColor" stroke-width="1.5" opacity="0.4"/>
        <line x1="3" y1="20" x2="21" y2="20" stroke="currentColor" stroke-width="1.5" opacity="0.4"/>
        <path d="M20 4 Q 14 6 11 14 T 4 22" stroke="currentColor" stroke-width="2" stroke-linecap="round" fill="none"/>
        <circle cx="4" cy="22" r="1.5" fill="var(--accent)"/>
      </svg>
      <span class="wordmark">logdive</span>
    </a>
    <nav class="nav" id="primary-nav" aria-label="Primary">
      <a href={`${base}/docs`} class={activeNav === 'docs' ? 'is-active' : ''}>Docs</a>
      <a href={`${base}/about`} class={activeNav === 'about' ? 'is-active' : ''}>About</a>
      <a href="https://github.com/Aryagorjipour/logdive" rel="noopener">GitHub</a>
    </nav>
    <button class="icon-btn theme-toggle" type="button" aria-label="Toggle theme">
      <svg class="moon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>
      <svg class="sun" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"/></svg>
    </button>
    <button class="icon-btn mobile-toggle" type="button" aria-label="Open menu" aria-expanded="false" aria-controls="primary-nav">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"><line x1="4" y1="7" x2="20" y2="7"/><line x1="4" y1="12" x2="20" y2="12"/><line x1="4" y1="17" x2="20" y2="17"/></svg>
    </button>
  </div>
</header>

<script>
  const toggle = document.querySelector('.theme-toggle')!;
  const mobileToggle = document.querySelector('.mobile-toggle')!;
  const nav = document.querySelector('#primary-nav')!;

  function currentTheme(): string {
    const t = document.documentElement.getAttribute('data-theme');
    if (t === 'light' || t === 'dark') return t;
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }

  toggle.addEventListener('click', () => {
    const next = currentTheme() === 'dark' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', next);
    try { localStorage.setItem('logdive-theme', next); } catch {}
  });

  mobileToggle.addEventListener('click', () => {
    const open = nav.classList.toggle('is-open');
    mobileToggle.setAttribute('aria-expanded', String(open));
  });
</script>
```

- [ ] **Step 3: Create `Footer.astro`**

```astro
---
// landing/src/components/Footer.astro
export interface Props { version: string; }
const { version } = Astro.props;
const base = import.meta.env.BASE_URL;
---
<footer class="site-footer">
  <div class="wrap">
    <a class="logo" href={`${base}/`} aria-label="logdive home">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <line x1="3" y1="8"  x2="21" y2="8"  stroke="currentColor" stroke-width="1.5" opacity="0.4"/>
        <line x1="3" y1="14" x2="21" y2="14" stroke="currentColor" stroke-width="1.5" opacity="0.4"/>
        <line x1="3" y1="20" x2="21" y2="20" stroke="currentColor" stroke-width="1.5" opacity="0.4"/>
        <path d="M20 4 Q 14 6 11 14 T 4 22" stroke="currentColor" stroke-width="2" stroke-linecap="round" fill="none"/>
        <circle cx="4" cy="22" r="1.5" fill="var(--accent)"/>
      </svg>
      <span class="wordmark">logdive</span>
    </a>
    <span>MIT OR Apache-2.0</span>
    <span class="version">v{version}</span>
    <nav class="footer-links" aria-label="Footer">
      <a href="https://github.com/Aryagorjipour/logdive" rel="noopener">GitHub</a>
      <a href="https://github.com/Aryagorjipour" rel="noopener">@Aryagorjipour</a>
      <a href={`${base}/docs`}>Docs</a>
    </nav>
  </div>
</footer>
```

- [ ] **Step 4: Verify build**

```bash
cd landing && bun run build
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
cd /home/arysmart/Projects/Rust/logdive
git add landing/src/layouts/ landing/src/components/Header.astro landing/src/components/Footer.astro
git commit -m "feat(landing): add Base layout, Header, Footer components"
```

---

## Task 5: CodeBlock and Tabs components

**Files:**
- Create: `landing/src/components/CodeBlock.astro`
- Create: `landing/src/components/Tabs.astro`

- [ ] **Step 1: Create `CodeBlock.astro`**

```astro
---
// landing/src/components/CodeBlock.astro
export interface Props {
  id: string;
  maxWidth?: string;
}
const { id, maxWidth } = Astro.props;
---
<div class="code-block" style={maxWidth ? `max-width: ${maxWidth}` : undefined}>
  <button class="copy-btn" type="button" data-copy-target={`#${id}`}>Copy</button>
  <pre id={id}><slot /></pre>
</div>

<script>
  document.querySelectorAll<HTMLButtonElement>('.copy-btn[data-copy-target]').forEach(btn => {
    btn.addEventListener('click', () => {
      const selector = btn.getAttribute('data-copy-target')!;
      const node = document.querySelector<HTMLElement>(selector);
      const text = node?.innerText ?? '';
      if (!text) return;

      const write = navigator.clipboard?.writeText(text);
      if (write) {
        write.then(() => flash(btn));
      } else {
        const ta = document.createElement('textarea');
        ta.value = text;
        document.body.appendChild(ta);
        ta.select();
        try { document.execCommand('copy'); } catch {}
        document.body.removeChild(ta);
        flash(btn);
      }
    });
  });

  function flash(btn: HTMLButtonElement) {
    const orig = btn.textContent!;
    btn.classList.add('is-copied');
    btn.textContent = 'COPIED';
    setTimeout(() => { btn.classList.remove('is-copied'); btn.textContent = orig; }, 1400);
  }
</script>
```

- [ ] **Step 2: Create `Tabs.astro`**

```astro
---
// landing/src/components/Tabs.astro
export interface Tab { id: string; label: string; }
export interface Props { tabs: Tab[]; }
const { tabs } = Astro.props;
---
<div data-tabs>
  <div class="tabs" role="tablist">
    {tabs.map((tab, i) => (
      <button
        class={`tab-btn${i === 0 ? ' is-active' : ''}`}
        type="button"
        data-tab={tab.id}
        role="tab"
        aria-selected={i === 0 ? 'true' : 'false'}
        aria-controls={`panel-${tab.id}`}
      >
        {tab.label}
      </button>
    ))}
  </div>
  <slot />
</div>

<script>
  document.querySelectorAll<HTMLElement>('[data-tabs]').forEach(group => {
    const buttons = group.querySelectorAll<HTMLButtonElement>('.tab-btn');
    const panels = group.querySelectorAll<HTMLElement>('.tab-panel');
    buttons.forEach(btn => {
      btn.addEventListener('click', () => {
        const id = btn.getAttribute('data-tab')!;
        buttons.forEach(b => {
          b.classList.toggle('is-active', b === btn);
          b.setAttribute('aria-selected', String(b === btn));
        });
        panels.forEach(p => p.classList.toggle('is-active', p.getAttribute('data-panel') === id));
      });
    });
  });
</script>
```

- [ ] **Step 3: Verify build**

```bash
cd landing && bun run build
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
cd /home/arysmart/Projects/Rust/logdive
git add landing/src/components/CodeBlock.astro landing/src/components/Tabs.astro
git commit -m "feat(landing): add CodeBlock and Tabs interactive components"
```

---

## Task 6: TerminalPreview and RoadmapStatus components

**Files:**
- Create: `landing/src/components/TerminalPreview.astro`
- Create: `landing/src/components/RoadmapStatus.astro`

- [ ] **Step 1: Create `TerminalPreview.astro`**

```astro
---
// landing/src/components/TerminalPreview.astro
---
<div class="terminal" role="figure" aria-label="Example logdive queries">
  <div class="terminal-head">
    <span class="terminal-dots" aria-hidden="true"><span></span><span></span><span></span></span>
    <span>~/services/payments  —  logdive</span>
  </div>
  <div class="terminal-body">
    <div class="cmd">
      <span class="prompt">$</span>logdive query <span class="str">'level=error AND service=payments last 2h'</span>
    </div>
    <div class="note">All errors from the payments service in the last 2 hours.</div>

    <div class="cmd">
      <span class="prompt">$</span>logdive query <span class="str">'level=error OR level=warn'</span> --format json
    </div>
    <div class="note">Pipe results downstream — output is line-delimited JSON.</div>

    <div class="cmd">
      <span class="prompt">$</span>logdive query <span class="str">'message contains <span class="accent-tok">"timeout"</span> last 24h'</span>
    </div>
    <div class="note">Substring match on the message body. Highlighted token is the literal you're searching for.</div>
  </div>
</div>
```

- [ ] **Step 2: Create `RoadmapStatus.astro`**

```astro
---
// landing/src/components/RoadmapStatus.astro
import {
  roadmapNow, roadmapNext, roadmapLater, shipped,
  IN_PROGRESS_VERSION, ROADMAP_UPDATED,
  type RoadmapItem, type ShippedRelease,
} from '@data/roadmap';

function issueUrl(issue: number) {
  return `https://github.com/Aryagorjipour/logdive/issues/${issue}`;
}
---

<section id="status" aria-labelledby="status-h2">
  <div class="wrap">
    <header>
      <span class="eyebrow">Project status</span>
      <h2 id="status-h2">Project status.</h2>
      <p class="meta">
        <time datetime={ROADMAP_UPDATED}>Last updated {new Date(ROADMAP_UPDATED).toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' })}</time>
        <span class="sep" aria-hidden="true">·</span>
        <a href="https://github.com/Aryagorjipour/logdive/issues" rel="noopener">All issues on GitHub</a>
      </p>
    </header>

    <div class="status-lanes">
      <!-- NOW -->
      <article class="status-lane" aria-labelledby="lane-now-h">
        <h3 id="lane-now-h">Now</h3>
        <p class="lane-desc">Active development. Currently building v{IN_PROGRESS_VERSION}.</p>
        <ul>
          {roadmapNow.length === 0
            ? <li class="lane-empty">Nothing active right now.</li>
            : roadmapNow.map(item => (
              <li>
                {item.issue
                  ? <a href={issueUrl(item.issue)} rel="noopener">{item.title}</a>
                  : <span class="item-title">{item.title}</span>
                }
                {item.description && <p class="item-desc">{item.description}</p>}
              </li>
            ))
          }
        </ul>
      </article>

      <!-- NEXT -->
      <article class="status-lane" aria-labelledby="lane-next-h">
        <h3 id="lane-next-h">Next</h3>
        <p class="lane-desc">Planned for the next release or two. Likely to ship.</p>
        <ul>
          {roadmapNext.length === 0
            ? <li class="lane-empty">Nothing queued yet.</li>
            : roadmapNext.map(item => (
              <li>
                {item.issue
                  ? <a href={issueUrl(item.issue)} rel="noopener">{item.title}</a>
                  : <span class="item-title">{item.title}</span>
                }
                {item.versionTarget && <div class="item-meta">target: v{item.versionTarget}</div>}
                {item.description && <p class="item-desc">{item.description}</p>}
              </li>
            ))
          }
        </ul>
      </article>

      <!-- LATER -->
      <article class="status-lane" aria-labelledby="lane-later-h">
        <h3 id="lane-later-h">Later</h3>
        <p class="lane-desc">Under consideration. No timeline, may not happen.</p>
        <ul>
          {roadmapLater.length === 0
            ? <li class="lane-empty">Nothing under consideration.</li>
            : roadmapLater.map(item => (
              <li>
                {item.issue
                  ? <a href={issueUrl(item.issue)} rel="noopener">{item.title}</a>
                  : <span class="item-title">{item.title}</span>
                }
                {item.note && <div class="item-meta">{item.note}</div>}
                {item.description && <p class="item-desc">{item.description}</p>}
              </li>
            ))
          }
        </ul>
      </article>
    </div>

    <!-- SHIPPED -->
    <div class="status-shipped">
      <h3>Recently shipped</h3>
      <p class="shipped-meta">
        Full history in <a href="https://github.com/Aryagorjipour/logdive/blob/main/CHANGELOG.md" rel="noopener">CHANGELOG.md</a> on GitHub.
      </p>
      <ol id="shipped-list">
        {shipped.map((release, i) => (
          <li>
            <details open={i === 0}>
              <summary>
                <span class="release-version">v{release.version}</span>
                <span class="release-date">{new Date(release.date).toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' })}</span>
              </summary>
              <ul class="release-highlights">
                {release.highlights.map(h => <li>{h}</li>)}
              </ul>
            </details>
          </li>
        ))}
      </ol>
    </div>

    <p class="status-oos">
      Looking for something that isn't here? Check the <a href={`${import.meta.env.BASE_URL}/about#non-goals`}>v1 non-goals</a> — some things are intentionally out of scope.
    </p>
  </div>
</section>
```

- [ ] **Step 3: Verify build**

```bash
cd landing && bun run build
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
cd /home/arysmart/Projects/Rust/logdive
git add landing/src/components/TerminalPreview.astro landing/src/components/RoadmapStatus.astro
git commit -m "feat(landing): add TerminalPreview and RoadmapStatus components"
```

---

## Task 7: Home page (`index.astro`)

**Files:**
- Create: `landing/src/pages/index.astro`

- [ ] **Step 1: Create `landing/src/pages/index.astro`**

```astro
---
// landing/src/pages/index.astro
import Base from '@layouts/Base.astro';
import CodeBlock from '@components/CodeBlock.astro';
import Tabs from '@components/Tabs.astro';
import TerminalPreview from '@components/TerminalPreview.astro';
import RoadmapStatus from '@components/RoadmapStatus.astro';
import { CURRENT_VERSION } from '@data/roadmap';
const base = import.meta.env.BASE_URL;
---
<Base
  title="logdive — jq with memory."
  description="Self-hosted log query for engineers. Single binary, SQLite-backed, no daemon. Written in Rust. MIT OR Apache-2.0."
>
<main>

  <!-- 1. HERO -->
  <section class="hero">
    <div class="hero-lines" aria-hidden="true"></div>
    <div class="wrap">
      <span class="eyebrow">v{CURRENT_VERSION} · MIT OR Apache-2.0</span>
      <h1>jq with memory<span class="accent">.</span></h1>
      <p class="hero-sub">Self-hosted log query for engineers who don't want a Datadog bill. Single binary, SQLite-backed, no daemon, no agent, no account.</p>
      <div class="hero-cta">
        <div class="install-code">
          <span class="prompt">$</span>
          <span id="hero-install-text">cargo install logdive</span>
          <button class="copy-btn" type="button" data-copy-target="#hero-install-text" aria-label="Copy install command">Copy</button>
        </div>
        <a class="btn btn-ghost" href="https://github.com/Aryagorjipour/logdive" rel="noopener">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 .5C5.7.5.5 5.7.5 12c0 5.1 3.3 9.4 7.9 10.9.6.1.8-.2.8-.6v-2.1c-3.2.7-3.9-1.5-3.9-1.5-.5-1.4-1.3-1.7-1.3-1.7-1-.7.1-.7.1-.7 1.1.1 1.7 1.2 1.7 1.2 1 1.7 2.7 1.2 3.4.9.1-.7.4-1.2.7-1.5-2.6-.3-5.3-1.3-5.3-5.7 0-1.3.4-2.3 1.2-3.1-.1-.3-.5-1.5.1-3.1 0 0 1-.3 3.2 1.2.9-.3 1.9-.4 2.9-.4s2 .1 2.9.4c2.2-1.5 3.2-1.2 3.2-1.2.6 1.6.2 2.8.1 3.1.7.8 1.2 1.9 1.2 3.1 0 4.4-2.7 5.4-5.3 5.7.4.4.8 1.1.8 2.2v3.3c0 .3.2.7.8.6 4.6-1.5 7.9-5.8 7.9-10.9C23.5 5.7 18.3.5 12 .5z"/></svg>
          View on GitHub
        </a>
      </div>
    </div>
  </section>

  <!-- 2. QUERY LANGUAGE PREVIEW -->
  <section>
    <div class="wrap">
      <div class="section-head">
        <span class="eyebrow">The query language</span>
        <h2>Filters that read like English. Time ranges that just work.</h2>
        <p>Boolean expressions over fields, with first-class time windows and JSON path extraction. No regex required for the common case.</p>
      </div>
      <TerminalPreview />
    </div>
  </section>

  <!-- 3. PERFORMANCE -->
  <section>
    <div class="wrap">
      <div class="section-head">
        <span class="eyebrow">Performance</span>
        <h2>Fast enough that grep starts to feel slow.</h2>
        <p>Measured on the project's own criterion suite against a 100k-row corpus. Your hardware will vary; the shape of the numbers won't.</p>
      </div>
      <div class="stat-grid">
        <div class="stat-card">
          <div class="stat-num">210<span class="unit">k lines/s</span></div>
          <div class="stat-desc">Batched ingest throughput. Sustained, not peak.</div>
        </div>
        <div class="stat-card">
          <div class="stat-num">166<span class="unit">k lines/s</span></div>
          <div class="stat-desc">End-to-end parse + ingest, JSON in, indexed out.</div>
        </div>
        <div class="stat-card">
          <div class="stat-num">17<span class="unit">µs</span></div>
          <div class="stat-desc">Indexed-field query against 100k rows.</div>
        </div>
        <div class="stat-card">
          <div class="stat-num">3.6<span class="unit">ms</span></div>
          <div class="stat-desc">json_extract field query against 100k rows.</div>
        </div>
      </div>
      <p class="muted" style="margin-top: var(--space-6); font-size: 14px;">
        Full-table <code class="code-inline">CONTAINS</code> scans land at 36–40&thinsp;ms over 100k rows. Run <code class="code-inline">cargo bench</code> for your own baseline.
      </p>
    </div>
  </section>

  <!-- 4. PILLARS -->
  <section>
    <div class="wrap">
      <div class="section-head">
        <span class="eyebrow">What you get</span>
        <h2>Three things, done well.</h2>
      </div>
      <div class="pillar-grid">
        <div class="pillar">
          <svg width="32" height="32" viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true">
            <rect x="6" y="9" width="20" height="16" rx="1.5"/>
            <line x1="6" y1="14" x2="26" y2="14"/>
            <circle cx="10" cy="11.5" r="0.8" fill="currentColor"/>
            <circle cx="13" cy="11.5" r="0.8" fill="currentColor"/>
          </svg>
          <h3>Local-first</h3>
          <p>One binary, one SQLite file. No daemon to babysit, no cloud account to authorize, no agent to ship logs anywhere. Your data stays on the machine that produced it.</p>
        </div>
        <div class="pillar">
          <svg width="32" height="32" viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" aria-hidden="true">
            <path d="M6 22 L12 14 L17 19 L26 8"/>
            <path d="M20 8 L26 8 L26 14"/>
          </svg>
          <h3>Fast queries</h3>
          <p>SQLite handles the storage. blake3 deduplicates content hashes. <code class="code-inline">json_extract</code> reaches into structured payloads without re-parsing. Microsecond reads on indexed fields.</p>
        </div>
        <div class="pillar">
          <svg width="32" height="32" viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" aria-hidden="true">
            <path d="M8 8 L24 8"/><path d="M8 16 L20 16"/><path d="M8 24 L16 24"/>
            <path d="M28 6 L26 8 L28 10"/><path d="M28 14 L26 16 L28 18"/><path d="M28 22 L26 24 L28 26"/>
          </svg>
          <h3>Multi-format ingestion</h3>
          <p>JSON for the modern stack. logfmt for the old guard. Plain text for everything else. logdive sniffs the format per-line and normalizes into a single queryable shape.</p>
        </div>
      </div>
