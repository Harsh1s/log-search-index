# logdive Landing Page — Design Spec

**Date:** 2026-06-04  
**Status:** Approved  
**Author:** Arya Gorjipour (via Claude Code brainstorming)

---

## 1. Goal

Build a production-quality SPA landing page for the logdive project, deployable to `https://aryagorjipour.github.io/logdive/`. Pixel-faithful to the Claude Design handoff (`LogDive Landing-handoff.zip`), with all content updated to v0.2.1 and docs strictly accurate to the real README/CHANGELOG.

---

## 2. Repository Location

**`landing/` subdirectory** inside the existing `Aryagorjipour/logdive` repo.

- Astro `base: '/logdive'` config handles the path prefix.
- GitHub Actions workflow at `.github/workflows/landing.yml` triggers on `push` to `main` when `landing/**` changes.
- Deploys `landing/dist/` to the `gh-pages` branch.

---

## 3. Tech Stack

| Concern | Choice | Rationale |
|---|---|---|
| Framework | Astro 5 | Static output, island architecture, no framework overhead |
| Language | TypeScript | Type-safe data, component props |
| Styling | Vanilla CSS (custom properties) | Preserve handoff design system exactly |
| Interactive bits | Astro `<script>` islands | ThemeToggle, CodeBlock, Tabs, mobile nav — no React/Vue |
| Content | Inline Astro + `src/data/*.ts` | Docs in page files; roadmap/version as typed TS data |
| Fonts | Google Fonts (IBM Plex Sans + JetBrains Mono) | Matches handoff exactly |
| Deployment | `withastro/action@v3` + `gh-pages` branch | Standard Astro GH Pages pattern |

---

## 4. Directory Structure

```
landing/
  astro.config.ts
  package.json
  tsconfig.json
  src/
    data/
      roadmap.ts          # typed roadmap data + CURRENT_VERSION constant
    layouts/
      Base.astro           # <html>, <head>, theme pre-paint script, font import
    components/
      Header.astro         # sticky nav, theme toggle, mobile menu
      Footer.astro         # logo, license, version badge, links
      CodeBlock.astro      # <pre> + copy-to-clipboard island
      Tabs.astro           # installation method tabs island
      ThemeToggle.astro    # localStorage theme toggle island
      RoadmapStatus.astro  # renders Now/Next/Later lanes from data/roadmap.ts
      TerminalPreview.astro # query language demo terminal
    pages/
      index.astro          # → /logdive/
      docs.astro           # → /logdive/docs
      about.astro          # → /logdive/about
    styles/
      tokens.css           # CSS custom properties (design tokens)
      global.css           # reset, base, typography
      components.css       # header, footer, buttons, code, terminal, stat grid, etc.
  public/
    (no binary assets; favicon is inline SVG data URI)

# GitHub Actions workflow lives at REPO ROOT (not inside landing/):
# .github/workflows/landing.yml
```

---

## 5. Design System

Preserved exactly from handoff. No deviations.

**Palette (light mode):**
- `--bg: #FBF9F6`
- `--surface: #FFFFFF`
- `--border: #E8E3DC`
- `--text: #0E1419`
- `--text-muted: #5B5A57`
- `--brand: #0A2540`
- `--accent: #B85D44` (terracotta — CTAs, logo dot, active states, accent highlights)
