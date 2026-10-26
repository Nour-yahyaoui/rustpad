import './styles/main.css';
import {
  AI_PROVIDERS,
  CLI_RECIPES,
  CRAFT_PILLARS,
  CRATE_LEDGER,
  DOWNLOADS,
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

function bootstrap(): void {
  initTheme();
  const closeMobileMenu = initMobileTopMenu();
  initOsSmartDownload();
  renderMetrics();
  const selectSpecimen = initStudioSwitchboard();
  renderCraftPillars();
  initAiProviderConsole();
  renderCrateArchitecture();
  renderCliAndScope();
  initCopyButtons();
  initGuidedTour(closeMobileMenu, selectSpecimen);
}

bootstrap();
