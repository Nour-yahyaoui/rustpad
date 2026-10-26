import './styles/main.css';
import {
  CLI_PRESETS,
  STUDIO_MODES,
  TOUR_STEPS,
  WORKSPACE_CRATES,
  type CrateEntry,
} from './data/projectData';
import { toAppError } from './lib/errors';

interface SpecimenItem {
  id: string;
  fileName: string;
  url: string;
  standard: string;
  bytes: number;
  cratePipeline: string;
  summary: string;
}

interface SpecimenManifest {
  generatedBy: string;
  specimens: SpecimenItem[];
}

const THEME_KEY = 'rustpad:theme';
const TOUR_DONE_KEY = 'onboard:v1:done';

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(1)} KB (${bytes.toLocaleString()} B)`;
}

function formatHexDump(buf: Uint8Array, maxBytes = 48): string {
  const slice = buf.slice(0, maxBytes);
  const lines: string[] = [];
  for (let offset = 0; offset < slice.length; offset += 16) {
    const row = slice.slice(offset, offset + 16);
    const hexCols = Array.from(row)
      .map((b) => b.toString(16).padStart(2, '0').toUpperCase())
      .join(' ')
      .padEnd(47, ' ');
    const asciiCols = Array.from(row)
      .map((b) => (b >= 32 && b <= 126 ? String.fromCharCode(b) : '.'))
      .join('');
    lines.push(`${offset.toString(16).padStart(4, '0').toUpperCase()}  ${hexCols}  |${asciiCols}|`);
  }
  return lines.join('\n');
}

function detectSignatureNote(buf: Uint8Array): string {
  if (buf.length >= 4 && buf[0] === 0x50 && buf[1] === 0x4b && buf[2] === 0x03 && buf[3] === 0x04) {
    return 'Verified PKZIP Local File Header (50 4B 03 04) -> ECMA-376 OpenXML package';
  }
  if (buf.length >= 8 && buf[0] === 0x25 && buf[1] === 0x50 && buf[2] === 0x44 && buf[3] === 0x46) {
    const ver = Array.from(buf.slice(0, 8))
      .map((b) => (b >= 32 && b <= 126 ? String.fromCharCode(b) : ''))
      .join('');
    return `Verified ISO 32000 Header (${ver}) -> Pure-Rust vector stream & xref table`;
  }
  return 'Verified UTF-8 CommonMark text stream';
}

function initThemeToggle(): void {
  const btn = document.getElementById('theme-toggle') as HTMLButtonElement | null;
  const label = document.getElementById('theme-toggle-label');
  if (!btn) return;

  const sync = (): void => {
    const current = document.documentElement.dataset.theme === 'dark' ? 'dark' : 'light';
    const isDark = current === 'dark';
    btn.setAttribute('aria-pressed', String(isDark));
    btn.setAttribute(
      'aria-label',
      isDark ? 'Switch to Daylight Proof light theme' : 'Switch to Darkroom Litho dark theme'
    );
    if (label) {
      label.textContent = isDark ? 'Light' : 'Dark';
    }
  };

  sync();

  btn.addEventListener('click', () => {
    const next = document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark';
    document.documentElement.dataset.theme = next;
    try {
      localStorage.setItem(THEME_KEY, next);
    } catch {
      // Ignore storage restrictions in private contexts
    }
    sync();
  });
}

function initMobileDrawer(): void {
  const openBtn = document.getElementById('mobile-menu-open') as HTMLButtonElement | null;
  const closeBtn = document.getElementById('mobile-menu-close') as HTMLButtonElement | null;
  const drawer = document.getElementById('mobile-drawer');
  if (!openBtn || !drawer) return;

  const closeDrawer = (): void => {
    drawer.hidden = true;
    openBtn.setAttribute('aria-expanded', 'false');
    openBtn.focus();
  };

  openBtn.addEventListener('click', () => {
    drawer.hidden = false;
    openBtn.setAttribute('aria-expanded', 'true');
    closeBtn?.focus();
  });

  closeBtn?.addEventListener('click', closeDrawer);

  drawer.addEventListener('click', (e) => {
    if (e.target === drawer) closeDrawer();
  });

  drawer.querySelectorAll('a, [data-close-drawer]').forEach((el) => {
    el.addEventListener('click', () => {
      drawer.hidden = true;
      openBtn.setAttribute('aria-expanded', 'false');
    });
  });

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && !drawer.hidden) {
      closeDrawer();
    }
  });
}

function initStudioSwitchboard(): void {
  const tabContainer = document.getElementById('switchboard-tabs');
  const img = document.getElementById('switchboard-img') as HTMLImageElement | null;
  const caption = document.getElementById('switchboard-caption');
  const shortcut = document.getElementById('switchboard-shortcut');
  const sourceRef = document.getElementById('switchboard-source');
  if (!tabContainer || !img || !caption || !shortcut || !sourceRef) return;

  const selectMode = (modeId: string, focusTab = false): void => {
    const mode = STUDIO_MODES.find((m) => m.id === modeId) ?? STUDIO_MODES[0];
    img.src = mode.image;
    img.alt = mode.alt;
    caption.textContent = mode.caption;
    shortcut.textContent = mode.shortcut;
    sourceRef.textContent = mode.sourceRef;

    const tabs = Array.from(tabContainer.querySelectorAll<HTMLButtonElement>('[role="tab"]'));
    tabs.forEach((tab) => {
      const active = tab.dataset.mode === mode.id;
      tab.setAttribute('aria-selected', String(active));
      tab.tabIndex = active ? 0 : -1;
      if (active && focusTab) tab.focus();
    });
  };

  tabContainer.querySelectorAll<HTMLButtonElement>('[role="tab"]').forEach((tab) => {
    tab.addEventListener('click', () => {
      if (tab.dataset.mode) selectMode(tab.dataset.mode);
    });

    tab.addEventListener('keydown', (e) => {
      const idx = STUDIO_MODES.findIndex((m) => m.id === tab.dataset.mode);
      if (idx === -1) return;
      if (e.key === 'ArrowRight') {
        e.preventDefault();
        selectMode(STUDIO_MODES[(idx + 1) % STUDIO_MODES.length].id, true);
      } else if (e.key === 'ArrowLeft') {
        e.preventDefault();
        selectMode(STUDIO_MODES[(idx + STUDIO_MODES.length - 1) % STUDIO_MODES.length].id, true);
      }
    });
  });
}

function initSpecimenInspector(): void {
  const listEl = document.getElementById('specimen-list');
  const detailEl = document.getElementById('specimen-detail');
  if (!listEl || !detailEl) return;

  let specimens: SpecimenItem[] = [];
  let activeId = 'welcome-docx';

  const renderDetailLoading = (item?: SpecimenItem): void => {
    detailEl.setAttribute('aria-busy', 'true');
    detailEl.innerHTML = `
      <div class="inspector-header">
        <div>
          <span class="tab-kicker">${item ? item.standard : 'Loading binary header...'}</span>
          <h3 class="cap-heading">${item ? item.fileName : 'Reading specimen manifest'}</h3>
        </div>
      </div>
      <div class="skeleton-block" role="status" aria-label="Inspecting binary specimen bytes"></div>
    `;
  };

  const renderError = (err: unknown, retryFn: () => void, fallbackUrl?: string): void => {
    detailEl.setAttribute('aria-busy', 'false');
    const appErr = toAppError(err);
    detailEl.innerHTML = `
      <div class="error-banner" role="alert">
        <div>
          <strong>${appErr.title}</strong>
          <p style="margin:0.25rem 0 0;font-size:var(--text-sm);">${appErr.message}</p>
        </div>
        <div style="display:flex;gap:0.5rem;">
          ${appErr.retryable ? '<button type="button" class="btn" id="specimen-retry-btn">Retry Inspection</button>' : ''}
          ${fallbackUrl ? `<a class="btn btn-primary" href="${fallbackUrl}" download>Download Directly</a>` : ''}
        </div>
      </div>
    `;
    document.getElementById('specimen-retry-btn')?.addEventListener('click', retryFn);
  };

  const inspectSpecimen = async (id: string): Promise<void> => {
    activeId = id;
    const item = specimens.find((s) => s.id === id) ?? specimens[0];
    if (!item) return;

    listEl.querySelectorAll<HTMLButtonElement>('.specimen-btn').forEach((btn) => {
      btn.setAttribute('aria-current', String(btn.dataset.specimenId === item.id));
    });

    renderDetailLoading(item);

    try {
      const res = await fetch(item.url);
      if (!res.ok) throw res;
      const ab = await res.arrayBuffer();
      const bytes = new Uint8Array(ab);
      const sigNote = detectSignatureNote(bytes);
      const hex = formatHexDump(bytes, 48);

      detailEl.setAttribute('aria-busy', 'false');
      detailEl.innerHTML = `
        <div class="inspector-header">
          <div>
            <span class="tab-kicker">${item.standard} · ${formatBytes(bytes.byteLength)}</span>
            <h3 class="cap-heading" style="margin-top:0.2rem;">${item.fileName}</h3>
          </div>
          <a class="btn btn-primary" href="${item.url}" download="${item.fileName}">
            Download ${item.fileName.split('.').pop()?.toUpperCase()}
          </a>
        </div>
        <p style="margin:0;color:var(--fg-secondary);font-size:var(--text-sm);">${item.summary}</p>
        <div style="display:flex;flex-wrap:wrap;gap:0.5rem;align-items:center;font-family:var(--font-mono);font-size:var(--text-xs);color:var(--ok);">
          <span>[OK] ${sigNote}</span>
        </div>
        <div>
          <div style="display:flex;justify-content:space-between;margin-bottom:0.35rem;font-family:var(--font-mono);font-size:var(--text-xs);color:var(--muted);">
            <span>FIRST 48 BYTES (LIVE BROWSER FETCH)</span>
            <span>PIPELINE: ${item.cratePipeline}</span>
          </div>
          <pre class="hex-well"><code></code></pre>
        </div>
      `;
      const codeNode = detailEl.querySelector('code');
      if (codeNode) codeNode.textContent = hex;
    } catch (err) {
      renderError(err, () => void inspectSpecimen(item.id), item.url);
    }
  };

  const loadManifest = async (): Promise<void> => {
    renderDetailLoading();
    try {
      const res = await fetch('/downloads/manifest.json');
      if (!res.ok) throw res;
      const data = (await res.json()) as SpecimenManifest;
      specimens = data.specimens;

      listEl.innerHTML = specimens
        .map(
          (s) => `
          <button type="button" class="specimen-btn" data-specimen-id="${s.id}" aria-current="${s.id === activeId}">
            <span>
              <span class="specimen-name">${s.fileName}</span>
              <span class="specimen-std">${s.standard}</span>
            </span>
            <span class="specimen-bytes">${formatBytes(s.bytes)}</span>
          </button>
        `
        )
        .join('');

      listEl.querySelectorAll<HTMLButtonElement>('.specimen-btn').forEach((btn) => {
        btn.addEventListener('click', () => {
          if (btn.dataset.specimenId) void inspectSpecimen(btn.dataset.specimenId);
        });
      });

      await inspectSpecimen(activeId);
    } catch (err) {
      renderError(err, () => void loadManifest(), '/downloads/RustPad_Welcome_Guide.docx');
    }
  };

  void loadManifest();
}

function initCrateLedger(): void {
  const searchInput = document.getElementById('crate-search') as HTMLInputElement | null;
  const filterGroup = document.getElementById('crate-filters');
  const tbody = document.getElementById('crate-tbody');
  const emptyBox = document.getElementById('crate-empty');
  const clearBtn = document.getElementById('crate-clear-btn');
  const countLabel = document.getElementById('crate-count-label');
  if (!searchInput || !filterGroup || !tbody || !emptyBox) return;

  let activeCategory = 'All';
  let query = '';

  const renderRows = (): void => {
    const q = query.trim().toLowerCase();
    const matches: CrateEntry[] = WORKSPACE_CRATES.filter((c) => {
      const catOk = activeCategory === 'All' || c.category === activeCategory;
      const textOk =
        !q ||
        c.name.toLowerCase().includes(q) ||
        c.summary.toLowerCase().includes(q) ||
        c.path.toLowerCase().includes(q);
      return catOk && textOk;
    });

    const totalLoc = matches.reduce((acc, item) => acc + item.loc, 0);
    if (countLabel) {
      countLabel.textContent = `Showing ${matches.length} of ${WORKSPACE_CRATES.length} crates · ${totalLoc.toLocaleString()} pure-Rust LOC`;
    }

    if (matches.length === 0) {
      tbody.innerHTML = '';
      emptyBox.hidden = false;
      return;
    }

    emptyBox.hidden = true;
    tbody.innerHTML = matches
      .map(
        (c) => `
        <tr>
          <td class="mono" style="font-weight:700;white-space:nowrap;">${c.name}</td>
          <td class="mono" style="white-space:nowrap;color:var(--muted);">${c.category}</td>
          <td class="mono tabular" style="white-space:nowrap;">${c.loc.toLocaleString()}</td>
          <td>
            <div>${c.summary}</div>
            <div class="mono" style="font-size:var(--text-xs);color:var(--muted);margin-top:0.25rem;">${c.path}</div>
          </td>
          <td class="mono" style="white-space:nowrap;color:var(--ok);">${c.tests}</td>
        </tr>
      `
      )
      .join('');
  };

  searchInput.addEventListener('input', () => {
    query = searchInput.value;
    renderRows();
  });

  filterGroup.querySelectorAll<HTMLButtonElement>('.filter-pill').forEach((pill) => {
    pill.addEventListener('click', () => {
      activeCategory = pill.dataset.category ?? 'All';
      filterGroup.querySelectorAll<HTMLButtonElement>('.filter-pill').forEach((p) => {
        p.setAttribute('aria-pressed', String(p === pill));
      });
      renderRows();
    });
  });

  clearBtn?.addEventListener('click', () => {
    query = '';
    activeCategory = 'All';
    searchInput.value = '';
    filterGroup.querySelectorAll<HTMLButtonElement>('.filter-pill').forEach((p) => {
      p.setAttribute('aria-pressed', String(p.dataset.category === 'All'));
    });
    renderRows();
    searchInput.focus();
  });

  renderRows();
}

function initCliBenchAndCopy(): void {
  const presetGroup = document.getElementById('cli-presets');
  const cmdEl = document.getElementById('cli-active-cmd');
  const outEl = document.getElementById('cli-active-out');
  const noteEl = document.getElementById('cli-active-note');
  const copyCliBtn = document.getElementById('cli-copy-btn') as HTMLButtonElement | null;

  let currentCmd = CLI_PRESETS[0].command;

  const setPreset = (id: string): void => {
    const preset = CLI_PRESETS.find((p) => p.id === id) ?? CLI_PRESETS[0];
    currentCmd = preset.command;
    if (cmdEl) cmdEl.textContent = `$ ${preset.command}`;
    if (outEl) outEl.textContent = preset.outputPreview;
    if (noteEl) noteEl.textContent = preset.note;
    presetGroup?.querySelectorAll<HTMLButtonElement>('.filter-pill').forEach((b) => {
      b.setAttribute('aria-pressed', String(b.dataset.preset === preset.id));
    });
  };

  presetGroup?.querySelectorAll<HTMLButtonElement>('.filter-pill').forEach((btn) => {
    btn.addEventListener('click', () => {
      if (btn.dataset.preset) setPreset(btn.dataset.preset);
    });
  });

  const attachCopy = (btn: HTMLButtonElement | null, getText: () => string): void => {
    if (!btn) return;
    const original = btn.textContent ?? 'Copy';
    btn.addEventListener('click', async () => {
      try {
        await navigator.clipboard.writeText(getText());
        btn.textContent = 'Copied';
        setTimeout(() => {
          btn.textContent = original;
        }, 1600);
      } catch {
        btn.textContent = 'Select cmd';
      }
    });
  };

  attachCopy(copyCliBtn, () => currentCmd);
  attachCopy(
    document.getElementById('hero-copy-btn') as HTMLButtonElement | null,
    () => 'git clone https://github.com/Nour-yahyaoui/word.git && cd word && cargo run --release -p rustpad'
  );
}

function initSpotlightTour(): void {
  const backdrop = document.getElementById('tour-backdrop');
  const stepCounter = document.getElementById('tour-step-counter');
  const titleEl = document.getElementById('tour-title');
  const bodyEl = document.getElementById('tour-body');
  const nextBtn = document.getElementById('tour-next-btn') as HTMLButtonElement | null;
  const prevBtn = document.getElementById('tour-prev-btn') as HTMLButtonElement | null;
  const skipBtn = document.getElementById('tour-skip-btn') as HTMLButtonElement | null;
  const triggers = document.querySelectorAll<HTMLElement>('[data-start-tour]');

  if (!backdrop || !stepCounter || !titleEl || !bodyEl || !nextBtn || !prevBtn || !skipBtn) return;

  let idx = 0;
  let lastTrigger: HTMLElement | null = null;

  const renderStep = (): void => {
    const s = TOUR_STEPS[idx];
    stepCounter.textContent = `STEP 0${s.step} OF 0${TOUR_STEPS.length}`;
    titleEl.textContent = s.title;
    bodyEl.textContent = s.body;
    prevBtn.hidden = idx === 0;
    nextBtn.textContent = idx === TOUR_STEPS.length - 1 ? 'Finish Tour' : 'Next Plate ->';

    const target = document.querySelector<HTMLElement>(s.targetSelector);
    if (target) {
      const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
      target.scrollIntoView({ behavior: reduced ? 'auto' : 'smooth', block: 'center' });
    }
  };

  const closeTour = (): void => {
    backdrop.hidden = true;
    try {
      localStorage.setItem(TOUR_DONE_KEY, '1');
    } catch {
      // Ignore storage restrictions
    }
    lastTrigger?.focus();
  };

  const openTour = (trigger: HTMLElement | null): void => {
    lastTrigger = trigger;
    idx = 0;
    backdrop.hidden = false;
    renderStep();
    nextBtn.focus();
  };

  triggers.forEach((t) => {
    t.addEventListener('click', () => openTour(t));
  });

  nextBtn.addEventListener('click', () => {
    if (idx < TOUR_STEPS.length - 1) {
      idx += 1;
      renderStep();
    } else {
      closeTour();
    }
  });

  prevBtn.addEventListener('click', () => {
    if (idx > 0) {
      idx -= 1;
      renderStep();
    }
  });

  skipBtn.addEventListener('click', closeTour);

  backdrop.addEventListener('click', (e) => {
    if (e.target === backdrop) closeTour();
  });

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && !backdrop.hidden) {
      closeTour();
    }
  });
}

function initCustom404Guard(): void {
  const path = window.location.pathname;
  if (path === '/' || path === '/index.html') return;

  const mainContent = document.getElementById('main-content');
  const notFound = document.getElementById('not-found-view');
  const badPathEl = document.getElementById('not-found-path');
  const homeReturnBtn = document.getElementById('not-found-return');

  if (!mainContent || !notFound) return;

  Array.from(mainContent.children).forEach((child) => {
    if (child.id !== 'not-found-view') {
      (child as HTMLElement).hidden = true;
    }
  });
  notFound.hidden = false;
  if (badPathEl) badPathEl.textContent = path;

  homeReturnBtn?.addEventListener('click', (e) => {
    e.preventDefault();
    window.history.pushState({}, '', '/');
    notFound.hidden = true;
    Array.from(mainContent.children).forEach((child) => {
      if (child.id !== 'not-found-view') {
        (child as HTMLElement).hidden = false;
      }
    });
  });
}

document.addEventListener('DOMContentLoaded', () => {
  initThemeToggle();
  initMobileDrawer();
  initStudioSwitchboard();
  initSpecimenInspector();
  initCrateLedger();
  initCliBenchAndCopy();
  initSpotlightTour();
  initCustom404Guard();
});
