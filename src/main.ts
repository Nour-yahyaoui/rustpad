import './styles/main.css';
import {
  AI_PROVIDERS,
  CLI_RECIPES,
  CRAFT_PILLARS,
  CRATE_LEDGER,
  DOWNLOADS,
  ENTERPRISE_BENCHMARKS,
  HERO_METRICS,
  RELEASE_VERSION,
  SCOPE_MATRIX,
  STUDIO_SPECIMENS,
  TOUR_STEPS,
} from './data/projectData';

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function initTheme(): void {
  const btn = document.getElementById('theme-toggle');
  const metaTheme = document.querySelector<HTMLMetaElement>('meta[name="theme-color"]');

  const applyMetaColor = (theme: string) => {
    if (metaTheme) {
      metaTheme.content = theme === 'light' ? '#f5f2eb' : '#090a0d';
    }
  };

  applyMetaColor(document.documentElement.getAttribute('data-theme') || 'dark');

  btn?.addEventListener('click', () => {
    const current = document.documentElement.getAttribute('data-theme') === 'light' ? 'light' : 'dark';
    const next = current === 'dark' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', next);
    applyMetaColor(next);
    try {
      localStorage.setItem('rustpad-theme', next);
    } catch {
      // Ignore storage restrictions
    }
  });
}

function initMobileTopMenu(): () => void {
  const toggleBtn = document.getElementById('mobile-menu-toggle');
  const menuLabel = document.getElementById('mobile-menu-label');
  const menuSheet = document.getElementById('mobile-top-menu');
  const backdrop = document.getElementById('mobile-menu-backdrop');

  const setOpen = (open: boolean) => {
    if (!toggleBtn || !menuSheet || !backdrop) return;
    toggleBtn.setAttribute('aria-expanded', open ? 'true' : 'false');
    toggleBtn.setAttribute(
      'aria-label',
      open ? 'Close top navigation menu' : 'Open top navigation menu',
    );
    if (menuLabel) {
      menuLabel.textContent = open ? 'Close' : 'Menu';
    }
    menuSheet.hidden = !open;
    backdrop.hidden = !open;
    document.body.classList.toggle('menu-locked', open);
  };

  toggleBtn?.addEventListener('click', () => {
    const isOpen = toggleBtn.getAttribute('aria-expanded') === 'true';
    setOpen(!isOpen);
  });

  backdrop?.addEventListener('click', () => setOpen(false));

  document.querySelectorAll<HTMLElement>('[data-close-menu]').forEach((el) => {
    el.addEventListener('click', () => setOpen(false));
  });

  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      setOpen(false);
    }
  });

  window.addEventListener('resize', () => {
    if (window.innerWidth >= 1024) {
      setOpen(false);
    }
  });

  return () => setOpen(false);
}

function initOsSmartDownload(): void {
  const ua = navigator.userAgent.toLowerCase();
  const isLinuxDesktop = ua.includes('linux') && !ua.includes('android');
  if (!isLinuxDesktop) return;

  const primaryBtn = document.querySelector<HTMLAnchorElement>('#hero-primary-dl');
  const primaryLabel = document.getElementById('hero-primary-dl-label');
  const secondaryBtn = document.querySelector<HTMLAnchorElement>('#hero-secondary-dl');
  const mobilePrimary = document.querySelector<HTMLAnchorElement>('#mobile-primary-dl');

  if (primaryBtn && primaryLabel && secondaryBtn) {
    primaryBtn.href = DOWNLOADS.linuxBin;
    primaryLabel.textContent = 'Download Linux x86_64';
    secondaryBtn.href = DOWNLOADS.windowsExe;
    secondaryBtn.innerHTML = '<span>Download Windows x64 (.exe)</span>';
  }
  if (mobilePrimary) {
    mobilePrimary.href = DOWNLOADS.linuxBin;
    mobilePrimary.textContent = `Download Linux x86_64 — ${RELEASE_VERSION}`;
  }
}

function renderMetrics(): void {
  const grid = document.getElementById('metrics-grid');
  if (!grid) return;
  grid.innerHTML = HERO_METRICS.map(
    (m) => `
      <article class="metric-card">
        <span class="metric-val">${escapeHtml(m.value)}</span>
        <span class="metric-lbl">${escapeHtml(m.label)}</span>
        <span class="metric-sub">${escapeHtml(m.detail)}</span>
      </article>
    `,
  ).join('');
}

function initStudioSwitchboard(): (specimenId: string) => void {
  const tabsEl = document.getElementById('specimen-tabs');
  const imgEl = document.querySelector<HTMLImageElement>('#specimen-image');
  const titleEl = document.getElementById('specimen-window-title');
  const badgeEl = document.getElementById('specimen-badge');
  const summaryEl = document.getElementById('specimen-summary');
  const highlightsEl = document.getElementById('specimen-highlights');

  const selectSpecimen = (id: string) => {
    const spec = STUDIO_SPECIMENS.find((s) => s.id === id) ?? STUDIO_SPECIMENS[0];
    if (!spec) return;

    tabsEl?.querySelectorAll<HTMLButtonElement>('.specimen-tab').forEach((btn) => {
      const active = btn.dataset.specimen === spec.id;
      btn.setAttribute('aria-selected', active ? 'true' : 'false');
    });

    if (imgEl) {
      imgEl.src = spec.image;
      imgEl.alt = spec.alt;
    }
    if (titleEl) titleEl.textContent = spec.windowTitle;
    if (badgeEl) badgeEl.textContent = spec.badge;
    if (summaryEl) summaryEl.textContent = spec.summary;
    if (highlightsEl) {
      highlightsEl.innerHTML = spec.highlights
        .map(
          (h) => `
            <div class="highlight-chip">
              <strong>${escapeHtml(h.tag)}</strong>
              <span>${escapeHtml(h.detail)}</span>
            </div>
          `,
        )
        .join('');
    }
  };

  if (tabsEl) {
    tabsEl.innerHTML = STUDIO_SPECIMENS.map(
      (s, idx) => `
        <button
          type="button"
          role="tab"
          class="specimen-tab"
          data-specimen="${escapeHtml(s.id)}"
          aria-selected="${idx === 0 ? 'true' : 'false'}"
        >
          <span class="specimen-tab-idx">${escapeHtml(s.index)}</span>
          <span>${escapeHtml(s.label)}</span>
        </button>
      `,
    ).join('');

    tabsEl.querySelectorAll<HTMLButtonElement>('.specimen-tab').forEach((btn) => {
      btn.addEventListener('click', () => {
        if (btn.dataset.specimen) {
          selectSpecimen(btn.dataset.specimen);
        }
      });
    });
  }

  selectSpecimen(STUDIO_SPECIMENS[0].id);
  return selectSpecimen;
}

function renderEnterpriseMatrix(): void {
  const mobileWrap = document.getElementById('benchmark-cards-mobile');
  const tableBody = document.getElementById('benchmark-table-body');

  if (mobileWrap) {
    mobileWrap.innerHTML = ENTERPRISE_BENCHMARKS.map(
      (row) => `
        <article class="crate-card-m">
          <div class="crate-card-m-head">
            <span class="crate-name-pill">${escapeHtml(row.capability)}</span>
            <span class="crate-std-pill">v2.0 ENTERPRISE</span>
          </div>
          <p><strong>RustPad v2.0:</strong> ${escapeHtml(row.rustpad)}</p>
          <code class="crate-mod-path">Cloud: ${escapeHtml(row.cloudSaas)} · Electron: ${escapeHtml(row.electronApps)} · Legacy: ${escapeHtml(row.legacySuites)}</code>
        </article>
      `,
    ).join('');
  }

  if (tableBody) {
    tableBody.innerHTML = ENTERPRISE_BENCHMARKS.map(
      (row) => `
        <tr>
          <td><strong>${escapeHtml(row.capability)}</strong></td>
          <td><span class="crate-name-pill">${escapeHtml(row.rustpad)}</span></td>
          <td>${escapeHtml(row.cloudSaas)}</td>
          <td>${escapeHtml(row.electronApps)}</td>
          <td>${escapeHtml(row.legacySuites)}</td>
        </tr>
      `,
    ).join('');
  }
}

function renderCraftPillars(): void {
  const grid = document.getElementById('craft-grid');
  if (!grid) return;
  grid.innerHTML = CRAFT_PILLARS.map(
    (c) => `
      <article class="craft-card">
        <div class="craft-card-top">
          <span class="craft-idx">${escapeHtml(c.index)}</span>
          <span class="craft-tag">${escapeHtml(c.tag)}</span>
        </div>
        <h3>${escapeHtml(c.title)}</h3>
        <p>${escapeHtml(c.summary)}</p>
        <ul class="craft-bullets">
          ${c.bullets.map((b) => `<li>${escapeHtml(b)}</li>`).join('')}
        </ul>
      </article>
    `,
  ).join('');
}

function initAiProviderConsole(): void {
  const tabsEl = document.getElementById('ai-provider-tabs');
  const modePill = document.getElementById('ai-mode-pill');
  const modelTag = document.getElementById('ai-model-tag');
  const nameEl = document.getElementById('ai-provider-name');
  const notesEl = document.getElementById('ai-provider-notes');
  const endpointEl = document.getElementById('ai-endpoint-val');
  const transportEl = document.getElementById('ai-transport-val');
  const cliCodeEl = document.getElementById('ai-cli-code');
  const copyBtn = document.querySelector<HTMLButtonElement>('#ai-copy-btn');

  const selectProvider = (id: string) => {
    const p = AI_PROVIDERS.find((item) => item.id === id) ?? AI_PROVIDERS[0];
    if (!p) return;

    tabsEl?.querySelectorAll<HTMLButtonElement>('.ai-tab-btn').forEach((btn) => {
      btn.setAttribute('aria-selected', btn.dataset.provider === p.id ? 'true' : 'false');
    });

    if (modePill) modePill.textContent = p.mode;
    if (modelTag) modelTag.textContent = `Default model: ${p.defaultModel}`;
    if (nameEl) nameEl.textContent = p.name;
    if (notesEl) notesEl.textContent = p.notes;
    if (endpointEl) endpointEl.textContent = p.endpoint;
    if (transportEl) transportEl.textContent = p.transport;
    if (cliCodeEl) cliCodeEl.textContent = `$ ${p.cliExample}`;
    if (copyBtn) {
      copyBtn.dataset.copy = p.cliExample;
      copyBtn.textContent = 'Copy Command';
    }
  };

  if (tabsEl) {
    tabsEl.innerHTML = AI_PROVIDERS.map(
      (p, idx) => `
        <button
          type="button"
          role="tab"
          class="ai-tab-btn"
          data-provider="${escapeHtml(p.id)}"
          aria-selected="${idx === 0 ? 'true' : 'false'}"
        >
          ${escapeHtml(p.name)}
        </button>
      `,
    ).join('');

    tabsEl.querySelectorAll<HTMLButtonElement>('.ai-tab-btn').forEach((btn) => {
      btn.addEventListener('click', () => {
        if (btn.dataset.provider) selectProvider(btn.dataset.provider);
      });
    });
  }

  selectProvider(AI_PROVIDERS[0].id);
}

function renderCrateArchitecture(): void {
  const mobileWrap = document.getElementById('crate-cards-mobile');
  const tableBody = document.getElementById('crate-table-body');

  if (mobileWrap) {
    mobileWrap.innerHTML = CRATE_LEDGER.map(
      (c) => `
        <article class="crate-card-m">
          <div class="crate-card-m-head">
            <span class="crate-name-pill">${escapeHtml(c.crateName)}</span>
            <span class="crate-std-pill">${escapeHtml(c.standard)}</span>
          </div>
          <p>${escapeHtml(c.responsibility)}</p>
          <code class="crate-mod-path">${escapeHtml(c.keyModules)}</code>
        </article>
      `,
    ).join('');
  }

  if (tableBody) {
    tableBody.innerHTML = CRATE_LEDGER.map(
      (c) => `
        <tr>
          <td><span class="crate-name-pill">${escapeHtml(c.crateName)}</span></td>
          <td>${escapeHtml(c.layer)}</td>
          <td><span class="crate-std-pill">${escapeHtml(c.standard)}</span></td>
          <td>${escapeHtml(c.responsibility)}</td>
          <td><code class="crate-mod-path">${escapeHtml(c.keyModules)}</code></td>
        </tr>
      `,
    ).join('');
  }
}

function renderCliAndScope(): void {
  const cliGrid = document.getElementById('cli-grid');
  if (cliGrid) {
    cliGrid.innerHTML = CLI_RECIPES.map(
      (r) => `
        <article class="cli-card">
          <div class="cli-card-head">
            <span class="cli-card-title">${escapeHtml(r.label)}</span>
            <span class="crate-std-pill">${escapeHtml(r.badge)}</span>
          </div>
          <div class="cli-code-row">
            <code>$ ${escapeHtml(r.command)}</code>
            <button type="button" class="copy-chip" data-copy="${escapeHtml(r.command)}">Copy</button>
          </div>
          <p>${escapeHtml(r.detail)}</p>
        </article>
      `,
    ).join('');
  }

  const builtFor = document.getElementById('scope-built-for');
  const nonGoals = document.getElementById('scope-non-goals');
  if (builtFor) {
    builtFor.innerHTML = SCOPE_MATRIX.builtFor.map((item) => `<li>${escapeHtml(item)}</li>`).join('');
  }
  if (nonGoals) {
    nonGoals.innerHTML = SCOPE_MATRIX.nonGoals.map((item) => `<li>${escapeHtml(item)}</li>`).join('');
  }
}

function initCopyButtons(): void {
  document.addEventListener('click', (e) => {
    const target = (e.target as HTMLElement | null)?.closest<HTMLButtonElement>('[data-copy]');
    if (!target) return;
    const text = target.dataset.copy ?? '';
    if (!text) return;

    const original = target.textContent ?? 'Copy';
    navigator.clipboard
      ?.writeText(text)
      .then(() => {
        target.textContent = 'Copied ✓';
        setTimeout(() => {
          target.textContent = original;
        }, 1600);
      })
      .catch(() => {
        target.textContent = 'Copied ✓';
        setTimeout(() => {
          target.textContent = original;
        }, 1600);
      });
  });
}

function initGuidedTour(
  closeMobileMenu: () => void,
  selectSpecimen: (id: string) => void,
): void {
  const tourBar = document.getElementById('tour-bar');
  const stepBadge = document.getElementById('tour-step-badge');
  const titleEl = document.getElementById('tour-title');
  const bodyEl = document.getElementById('tour-body');
  const prevBtn = document.querySelector<HTMLButtonElement>('#tour-prev');
  const nextBtn = document.querySelector<HTMLButtonElement>('#tour-next');
  const closeBtn = document.getElementById('tour-close');

  let currentIdx = 0;
  let highlightedEl: Element | null = null;

  const clearRing = () => {
    if (highlightedEl) {
      highlightedEl.classList.remove('tour-focus-ring');
      highlightedEl = null;
    }
  };

  const renderStep = () => {
    const step = TOUR_STEPS[currentIdx];
    if (!step || !tourBar) return;
    clearRing();

    if (step.specimenId) {
      selectSpecimen(step.specimenId);
    }

    const target = document.querySelector(step.targetSelector);
    if (target) {
      target.classList.add('tour-focus-ring');
      highlightedEl = target;
      target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    if (stepBadge) stepBadge.textContent = step.step;
    if (titleEl) titleEl.textContent = step.title;
    if (bodyEl) bodyEl.textContent = step.body;
    if (prevBtn) prevBtn.disabled = currentIdx === 0;
    if (nextBtn) {
      nextBtn.textContent = currentIdx === TOUR_STEPS.length - 1 ? 'Finish ✓' : 'Next →';
    }
  };

  const stopTour = () => {
    clearRing();
    if (tourBar) tourBar.hidden = true;
  };

  document.querySelectorAll<HTMLElement>('[data-start-tour]').forEach((btn) => {
    btn.addEventListener('click', () => {
      closeMobileMenu();
      currentIdx = 0;
      if (tourBar) tourBar.hidden = false;
      renderStep();
    });
  });

  prevBtn?.addEventListener('click', () => {
    if (currentIdx > 0) {
      currentIdx -= 1;
      renderStep();
    }
  });

  nextBtn?.addEventListener('click', () => {
    if (currentIdx < TOUR_STEPS.length - 1) {
      currentIdx += 1;
      renderStep();
    } else {
      stopTour();
    }
  });

  closeBtn?.addEventListener('click', stopTour);
}

function initUxSandbox(): void {
  const shell = document.getElementById('ux-sandbox-shell');
  if (!shell) return;

  const phraseBtns = Array.from(
    shell.querySelectorAll<HTMLButtonElement>('[data-ux-phrase]'),
  );
  const fmtBtns = Array.from(
    shell.querySelectorAll<HTMLButtonElement>('[data-ux-fmt]'),
  );
  const aiPolishBtn = document.getElementById('ux-ai-polish');
  const focusToggleBtn = document.getElementById('ux-focus-toggle');
  const exitFocusBtn = document.getElementById('ux-exit-focus') as HTMLButtonElement | null;
  const acceptBtn = document.getElementById('ux-accept-clause') as HTMLButtonElement | null;
  const rejectBtn = document.getElementById('ux-reject-clause') as HTMLButtonElement | null;
  const resetBtn = document.getElementById('ux-reset-clause') as HTMLButtonElement | null;
  const delRun = document.getElementById('ux-del-run');
  const insRun = document.getElementById('ux-ins-run');
  const gutterBar = document.getElementById('ux-gutter-bar');
  const redlineStatus = document.getElementById('ux-redline-status');
  const toastEl = document.getElementById('ux-sheet-toast');

  let activeIdx = 0;
  let focusMode = false;
  const polishedAlts: [string, string][] = [
    [
      'sub-millisecond native TrueType sheets',
      'zero-jitter 120Hz subpixel TrueType pages',
    ],
    [
      'every AI prompt strictly air-gapped on localhost',
      '100% sovereign inference locked to 127.0.0.1',
    ],
  ];
  const toggleIdx = [0, 0];

  const syncToolbarState = () => {
    const active = phraseBtns[activeIdx];
    if (!active) return;
    phraseBtns.forEach((btn, idx) => {
      btn.classList.toggle('is-selected', idx === activeIdx);
    });
    const map: Record<string, boolean> = {
      bold: active.classList.contains('is-bold'),
      italic: active.classList.contains('is-italic'),
      underline: active.classList.contains('is-under'),
      highlight: active.classList.contains('is-hl'),
    };
    fmtBtns.forEach((btn) => {
      const k = btn.dataset.uxFmt || '';
      btn.setAttribute('aria-pressed', map[k] ? 'true' : 'false');
    });
  };

  phraseBtns.forEach((btn, idx) => {
    btn.addEventListener('click', () => {
      activeIdx = idx;
      syncToolbarState();
      const words = (btn.textContent || '').trim().split(/\s+/).length;
      if (toastEl) {
        toastEl.textContent = `Selected ${words} words — Floating Ivory Bar synced to TextSpan #${idx + 1}`;
      }
    });
  });

  fmtBtns.forEach((btn) => {
    btn.addEventListener('click', () => {
      const active = phraseBtns[activeIdx];
      if (!active) return;
      const fmt = btn.dataset.uxFmt;
      if (fmt === 'bold') active.classList.toggle('is-bold');
      if (fmt === 'italic') active.classList.toggle('is-italic');
      if (fmt === 'underline') active.classList.toggle('is-under');
      if (fmt === 'highlight') active.classList.toggle('is-hl');
      syncToolbarState();
      if (toastEl) {
        toastEl.textContent = `Applied ${fmt?.toUpperCase()} strictly to selected substring (Paragraph::transform_range)`;
      }
    });
  });

  aiPolishBtn?.addEventListener('click', () => {
    const active = phraseBtns[activeIdx];
    if (!active) return;
    toggleIdx[activeIdx] = toggleIdx[activeIdx] === 0 ? 1 : 0;
    const pair = polishedAlts[activeIdx];
    if (pair) {
      active.textContent = pair[toggleIdx[activeIdx]];
    }
    if (toastEl) {
      toastEl.textContent =
        'Polished selected substring via local nour-code / Ollama (0 cloud bytes)';
    }
  });

  const setFocus = (next: boolean) => {
    focusMode = next;
    shell.classList.toggle('is-focus-mode', focusMode);
    if (exitFocusBtn) exitFocusBtn.hidden = !focusMode;
    if (toastEl) {
      toastEl.textContent = focusMode
        ? 'Entered Distraction-Free Writer Focus Mode — calm Linen & Paper'
        : 'Restored Studio Chrome & Legal Redline Review Bar';
    }
  };

  focusToggleBtn?.addEventListener('click', () => setFocus(!focusMode));
  exitFocusBtn?.addEventListener('click', () => setFocus(false));

  acceptBtn?.addEventListener('click', () => {
    if (delRun) delRun.hidden = true;
    insRun?.classList.add('is-committed');
    if (gutterBar) gutterBar.hidden = true;
    if (acceptBtn) acceptBtn.hidden = true;
    if (rejectBtn) rejectBtn.hidden = true;
    if (resetBtn) resetBtn.hidden = false;
    if (redlineStatus) {
      redlineStatus.textContent = 'LEGAL REDLINE · Clause #1 accepted ("fourteen (14) business days")';
    }
    if (toastEl) {
      toastEl.textContent = 'Accepted revision clause #1 — logged to SHA-256 AuditTrail';
    }
  });

  rejectBtn?.addEventListener('click', () => {
    if (insRun) insRun.hidden = true;
    delRun?.classList.add('is-restored');
    if (gutterBar) gutterBar.hidden = true;
    if (acceptBtn) acceptBtn.hidden = true;
    if (rejectBtn) rejectBtn.hidden = true;
    if (resetBtn) resetBtn.hidden = false;
    if (redlineStatus) {
      redlineStatus.textContent = 'LEGAL REDLINE · Clause #1 rejected (reverted to "thirty (30) calendar days")';
    }
    if (toastEl) {
      toastEl.textContent = 'Rejected revision clause #1 — original counterparty text restored';
    }
  });

  resetBtn?.addEventListener('click', () => {
    if (delRun) {
      delRun.hidden = false;
      delRun.classList.remove('is-restored');
    }
    if (insRun) {
      insRun.hidden = false;
      insRun.classList.remove('is-committed');
    }
    if (gutterBar) gutterBar.hidden = false;
    if (acceptBtn) acceptBtn.hidden = false;
    if (rejectBtn) rejectBtn.hidden = false;
    resetBtn.hidden = true;
    if (redlineStatus) {
      redlineStatus.textContent = 'LEGAL REDLINE · Clause #1 pending review';
    }
    if (toastEl) {
      toastEl.textContent = 'Restored ECMA-376 <w:del> + <w:ins> redline clause #1';
    }
  });

  syncToolbarState();
  initUxTableCreator(toastEl);
}

interface SandboxTableModel {
  rows: string[][];
  alignRightCols: number[];
  zebra: boolean;
  themeIdx: number;
}

const SANDBOX_THEMES: { id: string; label: string }[] = [
  { id: 'linen', label: 'Linen' },
  { id: 'cobalt', label: 'Cobalt' },
  { id: 'sage', label: 'Sage' },
  { id: 'terracotta', label: 'Terracotta' },
];

function initUxTableCreator(toastEl: HTMLElement | null): void {
  const triggerBtn = document.getElementById('ux-table-trigger') as HTMLButtonElement | null;
  const inlineOpenBtn = document.getElementById('ux-tbl-open-picker') as HTMLButtonElement | null;
  const popover = document.getElementById('ux-table-popover') as HTMLElement | null;
  const gridWrap = document.getElementById('ux-grid-cells') as HTMLElement | null;
  const gridReadout = document.getElementById('ux-grid-readout');
  const quickForm = document.getElementById('ux-table-quick-form') as HTMLFormElement | null;
  const quickInput = document.getElementById('ux-table-quick-input') as HTMLInputElement | null;
  const studioEl = document.getElementById('ux-table-studio') as HTMLElement | null;
  const tableEl = document.getElementById('ux-live-table') as HTMLTableElement | null;
  const coordEl = document.getElementById('ux-table-coord');
  const addRowBtn = document.getElementById('ux-tbl-add-row');
  const addColBtn = document.getElementById('ux-tbl-add-col');
  const zebraBtn = document.getElementById('ux-tbl-zebra') as HTMLButtonElement | null;
  const themeBtn = document.getElementById('ux-tbl-theme') as HTMLButtonElement | null;

  if (!tableEl || !studioEl) return;

  const model: SandboxTableModel = {
    rows: [
      ['Deliverable', 'Owner', 'Status'],
      ['8×8 Visual Table Picker', 'Desktop UX', 'Shipped ✓'],
      ['Slash /table & TSV Paste', 'Doc Model', '60–120Hz Live'],
    ],
    alignRightCols: [],
    zebra: false,
    themeIdx: 0,
  };

  let activeR = 1;
  let activeC = 0;
  let hoverR = 3;
  let hoverC = 3;

  const setToast = (msg: string) => {
    if (toastEl) toastEl.textContent = msg;
  };

  const closePopover = () => {
    if (!popover) return;
    popover.hidden = true;
    triggerBtn?.setAttribute('aria-expanded', 'false');
  };

  const togglePopover = () => {
    if (!popover) return;
    const willOpen = popover.hidden;
    popover.hidden = !willOpen;
    triggerBtn?.setAttribute('aria-expanded', willOpen ? 'true' : 'false');
    if (willOpen) {
      paintHoverGrid(hoverR, hoverC);
    }
  };

  const updateCoordLabel = () => {
    const rCount = model.rows.length;
    const cCount = model.rows[0]?.length ?? 1;
    if (coordEl) {
      coordEl.textContent = `TABLE R${activeR + 1}·C${activeC + 1} · ${rCount}×${cCount}`;
    }
  };

  const focusCell = (r: number, c: number) => {
    const cell = tableEl.querySelector<HTMLElement>(
      `[data-r="${r}"][data-c="${c}"]`,
    );
    if (!cell) return;
    cell.focus();
    const sel = window.getSelection();
    if (sel) {
      const range = document.createRange();
      range.selectNodeContents(cell);
      range.collapse(false);
      sel.removeAllRanges();
      sel.addRange(range);
    }
  };

  const renderTable = (focusAfter?: { r: number; c: number }) => {
    const theme = SANDBOX_THEMES[model.themeIdx] ?? SANDBOX_THEMES[0];
    studioEl.dataset.theme = theme.id;
    studioEl.classList.toggle('is-zebra', model.zebra);
    if (zebraBtn) zebraBtn.setAttribute('aria-pressed', model.zebra ? 'true' : 'false');
    if (themeBtn) themeBtn.textContent = `Theme: ${theme.label} ↻`;

    tableEl.innerHTML = model.rows
      .map((row, rIdx) => {
        const tag = rIdx === 0 ? 'th' : 'td';
        const cellsHtml = row
          .map((val, cIdx) => {
            const isRight = model.alignRightCols.includes(cIdx);
            const isPlaceholder = rIdx === 0 && val === `Column ${cIdx + 1}`;
            const cls = [
              isRight ? 'is-right' : '',
              isPlaceholder ? 'is-placeholder' : '',
            ]
              .filter(Boolean)
              .join(' ');
            return `<${tag} contenteditable="true" spellcheck="false" data-r="${rIdx}" data-c="${cIdx}" class="${cls}">${escapeHtml(val)}</${tag}>`;
          })
          .join('');
        return `<tr>${cellsHtml}</tr>`;
      })
      .join('');

    updateCoordLabel();

    tableEl.querySelectorAll<HTMLElement>('[data-r][data-c]').forEach((cell) => {
      const r = Number(cell.dataset.r || '0');
      const c = Number(cell.dataset.c || '0');

      cell.addEventListener('focus', () => {
        activeR = r;
        activeC = c;
        updateCoordLabel();
        if (r === 0 && model.rows[0]?.[c] === `Column ${c + 1}`) {
          const sel = window.getSelection();
          if (sel) {
            const range = document.createRange();
            range.selectNodeContents(cell);
            sel.removeAllRanges();
            sel.addRange(range);
          }
        }
      });

      cell.addEventListener('input', () => {
        const txt = (cell.textContent || '').replace(/\n/g, ' ');
        if (model.rows[r]) {
          model.rows[r][c] = txt;
        }
        cell.classList.toggle('is-placeholder', r === 0 && txt === `Column ${c + 1}`);
      });

      cell.addEventListener('keydown', (e: KeyboardEvent) => {
        if (e.key === 'Tab') {
          e.preventDefault();
          const cols = model.rows[0]?.length ?? 1;
          if (e.shiftKey) {
            if (c > 0) {
              focusCell(r, c - 1);
            } else if (r > 0) {
              focusCell(r - 1, cols - 1);
            }
          } else if (c + 1 < cols) {
            focusCell(r, c + 1);
          } else if (r + 1 < model.rows.length) {
            focusCell(r + 1, 0);
          } else {
            model.rows.push(Array.from({ length: cols }, () => ''));
            const newR = model.rows.length - 1;
            activeR = newR;
            activeC = 0;
            renderTable({ r: newR, c: 0 });
            setToast(`Tab at last cell auto-added Row ${newR + 1} — keep typing smoothly!`);
          }
        } else if (e.key === 'Enter') {
          e.preventDefault();
          const cols = model.rows[0]?.length ?? 1;
          if (r + 1 < model.rows.length) {
            focusCell(r + 1, c);
          } else {
            model.rows.push(Array.from({ length: cols }, () => ''));
            const newR = model.rows.length - 1;
            renderTable({ r: newR, c });
            setToast(`Added Row ${newR + 1} below`);
          }
        }
      });
    });

    if (focusAfter) {
      focusCell(focusAfter.r, focusAfter.c);
    }
  };

  const applyCustomGrid = (rows: number, cols: number) => {
    const r = Math.max(1, Math.min(8, rows));
    const c = Math.max(1, Math.min(6, cols));
    model.alignRightCols = [];
    model.zebra = false;
    model.rows = Array.from({ length: r }, (_, rIdx) =>
      Array.from({ length: c }, (__, cIdx) => (rIdx === 0 ? `Column ${cIdx + 1}` : '')),
    );
    activeR = 0;
    activeC = 0;
    closePopover();
    renderTable({ r: 0, c: 0 });
    setToast(`Created ${r}×${c} table — type header & press Tab across cells!`);
  };

  const applyPreset = (id: string) => {
    if (id === 'invoice') {
      model.themeIdx = 1;
      model.zebra = false;
      model.alignRightCols = [1, 2, 3];
      model.rows = [
        ['Deliverable / Item', 'Qty', 'Unit Price', 'Line Total'],
        ['Core Rust Architecture & UX', '1', '$1,200.00', '$1,200.00'],
        ['Security & PDF/A-2b Audit', '1', '$650.00', '$650.00'],
        ['Total Due (USD)', '', '', '$1,850.00'],
      ];
    } else if (id === 'tracker') {
      model.themeIdx = 2;
      model.zebra = true;
      model.alignRightCols = [];
      model.rows = [
        ['Milestone / Task', 'Owner', 'Status', 'Target'],
        ['Visual 8×8 Table Creator', 'Desktop UX', 'Complete', 'Week 1'],
        ['Slash & Pipe Auto-Tables', 'Doc Model', 'In Progress', 'Week 2'],
        ['OOXML & PDF Verification', 'SecOps', 'Planned', 'Week 3'],
      ];
    } else if (id === 'compare') {
      model.themeIdx = 1;
      model.zebra = true;
      model.alignRightCols = [];
      model.rows = [
        ['Dimension', 'Legacy Suites', 'RustPad v2.3'],
        ['Table Creation', 'Nested ribbon dialogs', 'Visual grid, /table & TSV'],
        ['Frame Latency', '35–90 ms DOM reflow', '< 1 ms native VSync'],
        ['Runtime Footprint', '340 MB – 2 GB', '~3.9 MB · 0 crates'],
      ];
    } else {
      model.themeIdx = 3;
      model.zebra = false;
      model.alignRightCols = [];
      model.rows = [
        ['Advantages / Pros', 'Trade-Offs / Considerations'],
        ['Instant 60–120Hz native execution', 'Focused scope (no VBA macros)'],
        ['100% offline & zero external deps', 'Desktop-first (Win32 & X11)'],
      ];
    }
    activeR = 1;
    activeC = 0;
    closePopover();
    renderTable({ r: 1, c: 0 });
    setToast(`Inserted styled '${id.toUpperCase()}' preset — click any cell or press Tab!`);
  };

  // Build 6 × 6 Interactive Hover/Tap Visual Grid
  const GRID_N = 6;
  const paintHoverGrid = (selR: number, selC: number) => {
    hoverR = selR;
    hoverC = selC;
    if (gridReadout) gridReadout.textContent = `${selR} × ${selC} Table`;
    gridWrap?.querySelectorAll<HTMLButtonElement>('.ux-grid-cell').forEach((btn) => {
      const r = Number(btn.dataset.gr || '1');
      const c = Number(btn.dataset.gc || '1');
      const active = r <= selR && c <= selC;
      btn.classList.toggle('is-active', active);
      btn.classList.toggle('is-header', active && r === 1);
    });
  };

  if (gridWrap) {
    let html = '';
    for (let r = 1; r <= GRID_N; r += 1) {
      for (let c = 1; c <= GRID_N; c += 1) {
        html += `<button type="button" class="ux-grid-cell" data-gr="${r}" data-gc="${c}" aria-label="Insert ${r} by ${c} table"></button>`;
      }
    }
    gridWrap.innerHTML = html;
    gridWrap.querySelectorAll<HTMLButtonElement>('.ux-grid-cell').forEach((btn) => {
      const r = Number(btn.dataset.gr || '3');
      const c = Number(btn.dataset.gc || '3');
      btn.addEventListener('pointerenter', () => paintHoverGrid(r, c));
      btn.addEventListener('click', () => applyCustomGrid(r, c));
    });
    paintHoverGrid(3, 3);
  }

  popover?.querySelectorAll<HTMLButtonElement>('[data-ux-preset]').forEach((btn) => {
    btn.addEventListener('click', () => {
      if (btn.dataset.uxPreset) applyPreset(btn.dataset.uxPreset);
    });
  });

  quickForm?.addEventListener('submit', (e) => {
    e.preventDefault();
    const raw = (quickInput?.value || '').trim();
    if (!raw) return;
    const lower = raw.toLowerCase();
    if (lower.startsWith('|') && raw.split('|').filter((s) => s.trim().length > 0).length >= 2) {
      const headers = raw
        .replace(/^\|/, '')
        .replace(/\|$/, '')
        .split('|')
        .map((s) => s.trim() || 'Column');
      model.alignRightCols = [];
      model.rows = [headers, headers.map(() => '')];
      closePopover();
      if (quickInput) quickInput.value = '';
      renderTable({ r: 1, c: 0 });
      setToast(`Created ${headers.length}-column table from pipe headers — type Row 2!`);
      return;
    }
    if (lower.includes('invoice') || lower.includes('pricing')) {
      if (quickInput) quickInput.value = '';
      applyPreset('invoice');
      return;
    }
    if (lower.includes('tracker') || lower.includes('roadmap')) {
      if (quickInput) quickInput.value = '';
      applyPreset('tracker');
      return;
    }
    if (lower.includes('compare')) {
      if (quickInput) quickInput.value = '';
      applyPreset('compare');
      return;
    }
    if (lower.includes('pros')) {
      if (quickInput) quickInput.value = '';
      applyPreset('pros');
      return;
    }
    const dimMatch = lower.match(/(\d+)\s*[x×*\s]\s*(\d+)/);
    if (dimMatch) {
      if (quickInput) quickInput.value = '';
      applyCustomGrid(Number(dimMatch[1]), Number(dimMatch[2]));
      return;
    }
    if (quickInput) quickInput.value = '';
    applyCustomGrid(3, 3);
  });

  triggerBtn?.addEventListener('click', (e) => {
    e.stopPropagation();
    togglePopover();
  });
  inlineOpenBtn?.addEventListener('click', (e) => {
    e.stopPropagation();
    togglePopover();
  });

  document.addEventListener('click', (e) => {
    if (!popover || popover.hidden) return;
    const target = e.target as Node | null;
    if (target && !popover.contains(target) && target !== triggerBtn && target !== inlineOpenBtn) {
      closePopover();
    }
  });

  addRowBtn?.addEventListener('click', () => {
    const cols = model.rows[0]?.length ?? 1;
    model.rows.push(Array.from({ length: cols }, () => ''));
    const nr = model.rows.length - 1;
    renderTable({ r: nr, c: 0 });
    setToast(`Added Row ${nr + 1} — start typing`);
  });

  addColBtn?.addEventListener('click', () => {
    const curCols = model.rows[0]?.length ?? 1;
    if (curCols >= 6) {
      setToast('Sandbox preview caps columns at 6 (native desktop supports up to 16)');
      return;
    }
    model.rows.forEach((row, rIdx) => {
      row.push(rIdx === 0 ? `Column ${curCols + 1}` : '');
    });
    renderTable({ r: 0, c: curCols });
    setToast(`Added Column ${curCols + 1}`);
  });

  zebraBtn?.addEventListener('click', () => {
    model.zebra = !model.zebra;
    renderTable();
    setToast(model.zebra ? 'Enabled alternating Zebra row striping' : 'Disabled Zebra striping');
  });

  themeBtn?.addEventListener('click', () => {
    model.themeIdx = (model.themeIdx + 1) % SANDBOX_THEMES.length;
    renderTable();
    const theme = SANDBOX_THEMES[model.themeIdx];
    setToast(`Applied '${theme?.label}' table palette (round-trips to .docx & .pdf)`);
  });

  // Interactive .docx Figure & Image Studio controls (50%/75%/100%, Left/Center/Right, Toggle)
  const figStudio = document.getElementById('ux-figure-studio');
  const figFrame = document.getElementById('ux-fig-frame');
  const figCoord = document.getElementById('ux-fig-coord');
  const figToggleBtn = document.querySelector<HTMLButtonElement>('#ux-figure-toggle');
  let figPct = 75;
  let figAlign = 'center';

  const syncFigureUi = () => {
    if (figFrame) figFrame.style.width = `${figPct}%`;
    if (figStudio) figStudio.dataset.align = figAlign;
    const wPt = Math.round((480 * figPct) / 100);
    const hPt = Math.round(wPt * 0.45);
    if (figCoord) {
      figCoord.textContent = `IMAGE · ${wPt}×${hPt} pt · ${figPct}% (PNG/JPEG/BMP)`;
    }
    figStudio?.querySelectorAll<HTMLButtonElement>('[data-ux-fig-scale]').forEach((b) => {
      b.classList.toggle('ux-tbar-accent', Number(b.dataset.uxFigScale) === figPct);
    });
    figStudio?.querySelectorAll<HTMLButtonElement>('[data-ux-fig-align]').forEach((b) => {
      b.classList.toggle('ux-tbar-accent', b.dataset.uxFigAlign === figAlign);
    });
  };

  figStudio?.querySelectorAll<HTMLButtonElement>('[data-ux-fig-scale]').forEach((btn) => {
    btn.addEventListener('click', () => {
      figPct = Number(btn.dataset.uxFigScale || '75');
      syncFigureUi();
      setToast(`Scaled .docx figure to ${figPct}% page width (aspect-ratio locked)`);
    });
  });

  figStudio?.querySelectorAll<HTMLButtonElement>('[data-ux-fig-align]').forEach((btn) => {
    btn.addEventListener('click', () => {
      figAlign = btn.dataset.uxFigAlign || 'center';
      syncFigureUi();
      setToast(`Aligned .docx figure ${figAlign.toUpperCase()} (<w:jc w:val="${figAlign}"/>)`);
    });
  });

  figToggleBtn?.addEventListener('click', () => {
    if (!figStudio) return;
    const nextHidden = !figStudio.hidden;
    figStudio.hidden = nextHidden;
    figToggleBtn.setAttribute('aria-pressed', nextHidden ? 'false' : 'true');
    setToast(
      nextHidden
        ? 'Hid sample .docx figure (click 🖼 Figure Studio to re-insert)'
        : 'Inserted .docx figure (<pic:pic> + word/media/image1.png) — try 50%/75%/100%!',
    );
  });

  renderTable();
}

function bootstrap(): void {
  initTheme();
  const closeMobileMenu = initMobileTopMenu();
  initOsSmartDownload();
  renderMetrics();
  initUxSandbox();
  renderEnterpriseMatrix();
  const selectSpecimen = initStudioSwitchboard();
  renderCraftPillars();
  initAiProviderConsole();
  renderCrateArchitecture();
  renderCliAndScope();
  initCopyButtons();
  initGuidedTour(closeMobileMenu, selectSpecimen);
}

bootstrap();
