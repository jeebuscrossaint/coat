'use strict';

const SCHEMES = {
  'coat': {
    name: 'Coat',
    variant: 'dark',
    base00: '0f0f13', base01: '16161c', base02: '1e1e27', base03: '2a2a38',
    base04: '6868a0', base05: 'd4d4e2', base06: 'e8e8f0', base07: 'f0f0f8',
    base08: 'f87171', base09: 'f09060', base0A: 'f0c060', base0B: '6ef0a0',
    base0C: '60d0f0', base0D: '8b8cf8', base0E: 'c080f0', base0F: 'f080c0',
  },
  'gruvbox-dark-hard': {
    name: 'Gruvbox Dark',
    variant: 'dark',
    base00: '1d2021', base01: '3c3836', base02: '504945', base03: '665c54',
    base04: 'bdae93', base05: 'd5c4a1', base06: 'ebdbb2', base07: 'fbf1c7',
    base08: 'fb4934', base09: 'fe8019', base0A: 'fabd2f', base0B: 'b8bb26',
    base0C: '8ec07c', base0D: '83a598', base0E: 'd3869b', base0F: 'd65d0e',
  },
  'gruvbox-light': {
    name: 'Gruvbox Light',
    variant: 'light',
    base00: 'fbf1c7', base01: 'ebdbb2', base02: 'd5c4a1', base03: 'bdae93',
    base04: 'a89984', base05: '7c6f64', base06: '504945', base07: '282828',
    base08: 'cc241d', base09: 'd65d0e', base0A: 'd79921', base0B: '98971a',
    base0C: '689d6a', base0D: '458588', base0E: 'b16286', base0F: '9d0006',
  },
  'nord': {
    name: 'Nord',
    variant: 'dark',
    base00: '2e3440', base01: '3b4252', base02: '434c5e', base03: '4c566a',
    base04: 'd8dee9', base05: 'e5e9f0', base06: 'eceff4', base07: '8fbcbb',
    base08: 'bf616a', base09: 'd08770', base0A: 'ebcb8b', base0B: 'a3be8c',
    base0C: '88c0d0', base0D: '81a1c1', base0E: 'b48ead', base0F: '5e81ac',
  },
  'one-dark': {
    name: 'One Dark',
    variant: 'dark',
    base00: '282c34', base01: '353b45', base02: '3e4451', base03: '545862',
    base04: '565c64', base05: 'abb2bf', base06: 'b6bdca', base07: 'c8ccd4',
    base08: 'e06c75', base09: 'd19a66', base0A: 'e5c07b', base0B: '98c379',
    base0C: '56b6c2', base0D: '61afef', base0E: 'c678dd', base0F: 'be5046',
  },
  'dracula': {
    name: 'Dracula',
    variant: 'dark',
    base00: '282a36', base01: '343746', base02: '44475a', base03: '6272a4',
    base04: '8be9fd', base05: 'f8f8f2', base06: 'f8f8f2', base07: 'ffffff',
    base08: 'ff5555', base09: 'ffb86c', base0A: 'f1fa8c', base0B: '50fa7b',
    base0C: '8be9fd', base0D: '6272a4', base0E: 'ff79c6', base0F: 'bd93f9',
  },
  'catppuccin-mocha': {
    name: 'Catppuccin Mocha',
    variant: 'dark',
    base00: '1e1e2e', base01: '181825', base02: '313244', base03: '45475a',
    base04: '585b70', base05: 'cdd6f4', base06: 'f5c2e7', base07: 'b4befe',
    base08: 'f38ba8', base09: 'fab387', base0A: 'f9e2af', base0B: 'a6e3a1',
    base0C: '94e2d5', base0D: '89b4fa', base0E: 'cba4f7', base0F: 'f2cdcd',
  },
  'catppuccin-latte': {
    name: 'Catppuccin Latte',
    variant: 'light',
    base00: 'eff1f5', base01: 'e6e9ef', base02: 'ccd0da', base03: 'bcc0cc',
    base04: 'acb0be', base05: '4c4f69', base06: 'dc8a78', base07: '7287fd',
    base08: 'd20f39', base09: 'fe640b', base0A: 'df8e1d', base0B: '40a02b',
    base0C: '179299', base0D: '1e66f5', base0E: '8839ef', base0F: 'dd7878',
  },
  'tokyo-night': {
    name: 'Tokyo Night',
    variant: 'dark',
    base00: '1a1b26', base01: '24283b', base02: '292e42', base03: '565f89',
    base04: '737aa2', base05: 'a9b1d6', base06: 'cbccd1', base07: 'c0caf5',
    base08: 'f7768e', base09: 'ff9e64', base0A: 'e0af68', base0B: '9ece6a',
    base0C: '73daca', base0D: '7aa2f7', base0E: 'bb9af7', base0F: 'f7768e',
  },
  'monokai': {
    name: 'Monokai',
    variant: 'dark',
    base00: '272822', base01: '383830', base02: '49483e', base03: '75715e',
    base04: 'a59f85', base05: 'f8f8f2', base06: 'f5f4f1', base07: 'f9f8f5',
    base08: 'f92672', base09: 'fd971f', base0A: 'f4bf75', base0B: 'a6e22e',
    base0C: 'a1efe4', base0D: '66d9e8', base0E: 'ae81ff', base0F: 'cc6633',
  },
  'solarized-dark': {
    name: 'Solarized Dark',
    variant: 'dark',
    base00: '002b36', base01: '073642', base02: '586e75', base03: '657b83',
    base04: '839496', base05: '93a1a1', base06: 'eee8d5', base07: 'fdf6e3',
    base08: 'dc322f', base09: 'cb4b16', base0A: 'b58900', base0B: '859900',
    base0C: '2aa198', base0D: '268bd2', base0E: '6c71c4', base0F: 'd33682',
  },
  'solarized-light': {
    name: 'Solarized Light',
    variant: 'light',
    base00: 'fdf6e3', base01: 'eee8d5', base02: '93a1a1', base03: '839496',
    base04: '657b83', base05: '586e75', base06: '073642', base07: '002b36',
    base08: 'dc322f', base09: 'cb4b16', base0A: 'b58900', base0B: '859900',
    base0C: '2aa198', base0D: '268bd2', base0E: '6c71c4', base0F: 'd33682',
  },
  'everforest-dark': {
    name: 'Everforest Dark',
    variant: 'dark',
    base00: '272e33', base01: '2e383c', base02: '374145', base03: '543a48',
    base04: '859289', base05: 'd3c6aa', base06: 'e9e8d2', base07: 'fff9e8',
    base08: 'e67e80', base09: 'e69875', base0A: 'dbbc7f', base0B: 'a7c080',
    base0C: '83c092', base0D: '7fbbb3', base0E: 'd699b6', base0F: '859289',
  },
  'rose-pine': {
    name: 'Rosé Pine',
    variant: 'dark',
    base00: '191724', base01: '1f1d2e', base02: '26233a', base03: '6e6a86',
    base04: '908caa', base05: 'e0def4', base06: 'e0def4', base07: '524f67',
    base08: 'eb6f92', base09: 'f6c177', base0A: 'f6c177', base0B: '9ccfd8',
    base0C: '9ccfd8', base0D: '31748f', base0E: 'c4a7e7', base0F: 'eb6f92',
  },
  'rose-pine-dawn': {
    name: 'Rosé Pine Dawn',
    variant: 'light',
    base00: 'faf4ed', base01: 'fffaf3', base02: 'f2e9e1', base03: '9893a5',
    base04: '797593', base05: '575279', base06: '575279', base07: '26233a',
    base08: 'b4637a', base09: 'ea9d34', base0A: 'd7827e', base0B: '286983',
    base0C: '56949f', base0D: '286983', base0E: '907aa9', base0F: 'cecacd',
  },
  'material-palenight': {
    name: 'Material Palenight',
    variant: 'dark',
    base00: '292d3e', base01: '444267', base02: '32374d', base03: '959dcb',
    base04: '676e95', base05: 'a6accd', base06: '717cb4', base07: 'c7c7c7',
    base08: 'f07178', base09: 'f78c6c', base0A: 'ffcb6b', base0B: 'c3e88d',
    base0C: '89ddff', base0D: '82aaff', base0E: 'c792ea', base0F: 'ff5370',
  },
  'tomorrow-night': {
    name: 'Tomorrow Night',
    variant: 'dark',
    base00: '1d1f21', base01: '282a2e', base02: '373b41', base03: '969896',
    base04: 'b4b7b4', base05: 'c5c8c6', base06: 'e0e0e0', base07: 'ffffff',
    base08: 'cc6666', base09: 'de935f', base0A: 'f0c674', base0B: 'b5bd68',
    base0C: '8abeb7', base0D: '81a2be', base0E: 'b294bb', base0F: 'a3685a',
  },
  'ocean': {
    name: 'Ocean',
    variant: 'dark',
    base00: '2b303b', base01: '343d46', base02: '4f5b66', base03: '65737e',
    base04: 'a7adba', base05: 'c0c5ce', base06: 'dfe1e8', base07: 'eff1f5',
    base08: 'bf616a', base09: 'd08770', base0A: 'ebcb8b', base0B: 'a3be8c',
    base0C: '96b5b4', base0D: '8fa1b3', base0E: 'b48ead', base0F: 'ab7967',
  },
};

// ── Helpers ──────────────────────────────────────────────────

function darken(hex, factor) {
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  const toHex = n => Math.round(Math.max(0, Math.min(255, n * factor)))
    .toString(16).padStart(2, '0');
  return '#' + toHex(r) + toHex(g) + toHex(b);
}

// ── Apply a scheme ────────────────────────────────────────────

function applyTheme(name) {
  const s = SCHEMES[name];
  if (!s) return;

  const r = document.documentElement;
  const set = (v, c) => r.style.setProperty(v, '#' + c);

  set('--bg',         s.base00);
  set('--surface',    s.base01);
  set('--surface2',   s.base02);
  set('--border',     s.base02);
  set('--muted',      s.base03);
  set('--text',       s.base05);
  set('--accent',     s.base0D);
  r.style.setProperty('--accent-dim', darken(s.base0D, 0.55));
  set('--green',      s.base0B);
  set('--red',        s.base08);
  set('--yellow',     s.base0A);
  set('--code-bg',    s.base00);

  localStorage.setItem('coat-docs-theme', name);

  document.querySelectorAll('.theme-card').forEach(card => {
    card.classList.toggle('active', card.dataset.scheme === name);
  });
}

// ── Build picker modal ────────────────────────────────────────

function buildPicker() {
  const modal = document.createElement('div');
  modal.id = 'theme-modal';
  modal.setAttribute('role', 'dialog');
  modal.setAttribute('aria-modal', 'true');
  modal.setAttribute('aria-label', 'Choose a theme');

  const accentKeys = ['base08','base09','base0A','base0B','base0C','base0D','base0E','base0F'];
  const saved = localStorage.getItem('coat-docs-theme') || 'coat';

  const cards = Object.entries(SCHEMES).map(([key, s]) => {
    const swatches = accentKeys
      .map(k => `<span class="swatch" style="background:#${s[k]}"></span>`)
      .join('');
    const isActive = key === saved ? ' active' : '';
    return `
      <div class="theme-card${isActive}"
           data-scheme="${key}"
           tabindex="0"
           role="button"
           aria-label="${s.name}"
           style="background:#${s.base00};border-color:#${s.base02}"
           onclick="applyTheme('${key}')"
           onkeydown="if(event.key==='Enter'||event.key===' ')applyTheme('${key}')">
        <div class="theme-swatches">${swatches}</div>
        <div class="theme-name" style="color:#${s.base05}">${s.name}</div>
        <div class="theme-variant" style="color:#${s.base05}">${s.variant}</div>
      </div>`;
  }).join('');

  modal.innerHTML = `
    <div class="theme-modal-backdrop" onclick="closeThemePicker()"></div>
    <div class="theme-modal-panel">
      <div class="theme-modal-header">
        <span>Choose a theme <small>${Object.keys(SCHEMES).length} schemes</small></span>
        <button class="close-btn" onclick="closeThemePicker()">✕ close</button>
      </div>
      <div class="theme-grid">${cards}</div>
    </div>`;

  document.body.appendChild(modal);
}

function openThemePicker() {
  document.getElementById('theme-modal').classList.add('open');
  document.getElementById('theme-modal').querySelector('.theme-card.active')
    ?.scrollIntoView({ block: 'nearest' });
}

function closeThemePicker() {
  document.getElementById('theme-modal').classList.remove('open');
}

// ── Init ──────────────────────────────────────────────────────

document.addEventListener('DOMContentLoaded', () => {
  buildPicker();

  const saved = localStorage.getItem('coat-docs-theme');
  if (saved && SCHEMES[saved]) applyTheme(saved);

  document.addEventListener('keydown', e => {
    if (e.key === 'Escape') closeThemePicker();
  });
});
