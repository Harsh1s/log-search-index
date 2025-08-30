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

