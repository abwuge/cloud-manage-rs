import { ApiClient, UnauthorizedError } from './api.js';
import { icon } from './icons.js';
import {
  renderDnsRows,
  renderDnsView,
  renderInstancesView,
  renderMobileTabs,
  renderModal,
  renderSettingsView,
  renderSidebar,
  renderSnipeLogLines,
  renderSnipeMetrics,
  renderSnipeView,
  renderToasts,
  renderTopbar,
} from './views.js';

const THEME_KEY = 'cloudManageTheme';

const state = {
  tab: 'instances',
  theme: document.documentElement.getAttribute('data-theme') || 'dark',
  authRequired: false,
  hasToken: false,
  showAuthModal: false,
  authInput: '',
  authError: '',

  config: null,
  settingsForm: null,
  settingsDirty: false,

  instances: [],
  dns: [],
  dnsFilter: { query: '', type: 'ALL' },

  errors: { instances: null, dns: null },
  loading: {
    auth: false,
    instances: false,
    dns: false,
    create: false,
    refreshKey: null,
    dnsSave: false,
    dnsDelete: false,
    dnsToggleId: null,
    configSave: false,
  },

  modal: null,
  toasts: [],
  toastSeq: 0,

  snipe: {
    running: false,
    source: null,
    minDelay: 5,
    maxDelay: 30,
    maxAttempts: 0,
    bypass: false,
    currentAttempt: 0,
    startedAt: null,
    endedAt: null,
    nextAttemptAt: null,
    waitTotalMs: 0,
    waitProgress: 0,
    successInstanceId: null,
    log: [],
  },
};

let pollTimer = null;
let snipeTickTimer = null;

const api = new ApiClient({
  onUnauthorized() {
    state.hasToken = false;
    state.authRequired = true;
    state.showAuthModal = true;
    state.authError = 'Session expired. Enter bearer token again.';
    stopSnipe(false);
    renderAll();
  },
});
state.hasToken = !!api.token;

function mountShell() {
  const root = document.getElementById('app');
  root.innerHTML = `
    <div class="app-shell">
      <aside id="sidebar" class="sidebar"></aside>
      <div class="main-area">
        <header id="topbar" class="topbar"></header>
        <div id="mobile-tabs" class="mobile-tabs"></div>
        <main id="view-root" class="content"></main>
      </div>
    </div>
    <div id="modal-root"></div>
    <div id="toast-root" class="toast-stack"></div>
  `;
}

function renderChrome() {
  document.getElementById('sidebar').innerHTML = renderSidebar(state);
  document.getElementById('mobile-tabs').innerHTML = renderMobileTabs(state);
  document.getElementById('topbar').innerHTML = renderTopbar(state);
}

function renderView() {
  const root = document.getElementById('view-root');
  if (state.tab === 'instances') {
    root.innerHTML = renderInstancesView(state);
  } else if (state.tab === 'dns') {
    root.innerHTML = renderDnsView(state);
  } else if (state.tab === 'snipe') {
    root.innerHTML = renderSnipeView(state);
    scrollTerminalToBottom();
  } else if (state.tab === 'settings') {
    root.innerHTML = renderSettingsView(state);
  }
}

function renderModalRoot() {
  document.getElementById('modal-root').innerHTML = renderModal(state);
}

function renderToastRoot() {
  document.getElementById('toast-root').innerHTML = renderToasts(state.toasts);
}

function renderAll() {
  renderChrome();
  renderView();
  renderModalRoot();
  renderToastRoot();
}

function toast(message, kind = 'info') {
  const id = ++state.toastSeq;
  state.toasts.push({ id, message, kind });
  renderToastRoot();
  window.setTimeout(() => dismissToast(id), kind === 'error' ? 6000 : 3800);
}

function dismissToast(id) {
  const next = state.toasts.filter((t) => t.id !== id);
  if (next.length !== state.toasts.length) {
    state.toasts = next;
    renderToastRoot();
  }
}

function cloneConfigToForm(cfg) {
  return {
    instance: {
      instance_type: cfg.instance.instance_type || 'amd',
      display_name: cfg.instance.display_name || '',
      arm_ocpus: cfg.instance.arm_ocpus ?? 2,
      arm_memory_gb: cfg.instance.arm_memory_gb ?? 12,
      boot_volume_size_gb: cfg.instance.boot_volume_size_gb ?? 47,
    },
    network: {
      assign_public_ip: !!cfg.network.assign_public_ip,
      assign_ipv6: !!cfg.network.assign_ipv6,
      private_ip: cfg.network.private_ip || '',
      ipv6_address: cfg.network.ipv6_address || '',
      hostname_label: cfg.network.hostname_label || '',
    },
    snipe: {
      min_delay_secs: cfg.snipe.min_delay_secs ?? 5,
      max_delay_secs: cfg.snipe.max_delay_secs ?? 30,
      max_attempts: cfg.snipe.max_attempts ?? 0,
    },
    cloudflare: {
      zone_name: cfg.cloudflare.zone_name || '',
      record_name: cfg.cloudflare.record_name || '',
      api_token: '',
    },
  };
}

function applyConfig(cfg) {
  state.config = cfg;
  if (!state.settingsDirty) {
    state.settingsForm = cloneConfigToForm(cfg);
  }
  if (!state.snipe.running) {
    state.snipe.minDelay = cfg.snipe.min_delay_secs;
    state.snipe.maxDelay = cfg.snipe.max_delay_secs;
    state.snipe.maxAttempts = cfg.snipe.max_attempts;
  }
}

async function loadConfig() {
  try {
    const cfg = await api.getConfig();
    applyConfig(cfg);
  } catch (err) {
    if (!(err instanceof UnauthorizedError)) {
      toast(`Config load failed: ${err.message}`, 'error');
    }
  }
}

async function loadInstances({ silent = false } = {}) {
  if (!silent) {
    state.loading.instances = true;
    renderChrome();
    if (state.tab === 'instances' && state.instances.length === 0) renderView();
  }
  try {
    state.instances = await api.listInstances();
    state.errors.instances = null;
  } catch (err) {
    if (!(err instanceof UnauthorizedError)) {
      state.errors.instances = err.message;
      if (!silent) toast(`Instances: ${err.message}`, 'error');
    }
  } finally {
    state.loading.instances = false;
    renderChrome();
    if (state.tab === 'instances' || state.tab === 'dns') renderView();
  }
}

async function loadDns({ silent = false } = {}) {
  if (!silent) {
    state.loading.dns = true;
    renderChrome();
    if (state.tab === 'dns' && state.dns.length === 0) renderView();
  }
  try {
    state.dns = await api.listDns();
    state.errors.dns = null;
  } catch (err) {
    if (!(err instanceof UnauthorizedError)) {
      state.errors.dns = err.message;
      if (!silent && state.tab === 'dns') toast(`DNS: ${err.message}`, 'error');
    }
  } finally {
    state.loading.dns = false;
    renderChrome();
    if (state.tab === 'dns' || state.tab === 'instances') renderView();
  }
}

async function refreshAllData() {
  await Promise.all([loadConfig(), loadInstances(), loadDns({ silent: true })]);
  renderAll();
}

function startPolling() {
  if (pollTimer) return;
  pollTimer = window.setInterval(() => {
    if (!state.showAuthModal && state.tab === 'instances' && !state.modal) {
      loadInstances({ silent: true });
    }
  }, 30000);
}

function switchTab(nextTab) {
  if (!nextTab || state.tab === nextTab) return;
  state.tab = nextTab;
  renderChrome();
  renderView();
}

function toggleTheme() {
  state.theme = state.theme === 'dark' ? 'light' : 'dark';
  document.documentElement.setAttribute('data-theme', state.theme);
  localStorage.setItem(THEME_KEY, state.theme);
  renderChrome();
}

async function copyText(text, btn) {
  try {
    await navigator.clipboard.writeText(text);
    if (btn) {
      btn.classList.add('copied');
      const iconEl = btn.querySelector('.copy-icon');
      if (iconEl) iconEl.innerHTML = icon('check', 12);
      window.setTimeout(() => {
        btn.classList.remove('copied');
        if (iconEl) iconEl.innerHTML = icon('copy', 12);
      }, 1400);
    }
  } catch {
    toast('Clipboard copy failed', 'error');
  }
}

/* ---------- Modal Actions ---------- */

function closeModal() {
  state.modal = null;
  renderModalRoot();
}

function openCreateModal() {
  state.modal = { type: 'create', displayName: '' };
  renderModalRoot();
}

async function submitCreateInstance() {
  const input = document.getElementById('create-name-input');
  const displayName = input ? input.value.trim() : '';
  state.loading.create = true;
  renderModalRoot();
  try {
    const res = await api.createInstance(displayName);
    toast(`Instance launched: ${res.instance_id}`, 'ok');
    state.modal = null;
    renderModalRoot();
    window.setTimeout(() => loadInstances(), 1500);
  } catch (err) {
    if (!(err instanceof UnauthorizedError)) {
      toast(`Launch failed: ${err.message}`, 'error');
    }
  } finally {
    state.loading.create = false;
    renderModalRoot();
  }
}

function openRefreshModal(instanceId, privateIpId) {
  const inst = state.instances.find((i) => i.instance_id === instanceId);
  const vnic = inst?.vnics?.find((v) => v.private_ip_id === privateIpId);
  if (!inst || !vnic) return;
  const hasMatchingDns = vnic.public_ip
    ? state.dns.some((r) => r.record_type === 'A' && r.content === vnic.public_ip)
    : false;
  state.modal = { type: 'refresh', inst, vnic, updateDns: hasMatchingDns };
  renderModalRoot();
}

async function confirmRefreshIp() {
  if (!state.modal || state.modal.type !== 'refresh') return;
  const { inst, vnic } = state.modal;
  const cb = document.getElementById('refresh-update-dns');
  const updateDns = cb ? cb.checked : false;

  state.loading.refreshKey = vnic.private_ip_id;
  renderModalRoot();
  if (state.tab === 'instances') renderView();

  try {
    const res = await api.refreshInstanceIp(inst.instance_id, {
      vnicId: vnic.vnic_id,
      privateIpId: vnic.private_ip_id,
      updateDns,
    });
    let msg = `Public IP updated: ${res.new_public_ip}`;
    if (res.dns_updated?.length) {
      msg += ` (${res.dns_updated.length} DNS record${res.dns_updated.length === 1 ? '' : 's'} synced)`;
    }
    toast(msg, 'ok');
    state.modal = null;
    await loadInstances({ silent: true });
    if (res.dns_updated?.length) {
      await loadDns({ silent: true });
    }
  } catch (err) {
    if (!(err instanceof UnauthorizedError)) {
      toast(`Refresh failed: ${err.message}`, 'error');
    }
  } finally {
    state.loading.refreshKey = null;
    renderAll();
  }
}

function openDnsModal(record = null) {
  state.modal = {
    type: 'dns',
    form: record
      ? {
          id: record.id,
          record_type: record.record_type,
          name: record.name,
          content: record.content,
          ttl: record.ttl || 1,
          proxied: !!record.proxied,
        }
      : {
          id: null,
          record_type: 'A',
          name: state.config?.cloudflare?.record_name || '',
          content: '',
          ttl: 1,
          proxied: false,
        },
  };
  renderModalRoot();
}

async function submitDnsForm() {
  if (!state.modal || state.modal.type !== 'dns') return;
  const recordType = document.getElementById('dns-f-type').value;
  const name = document.getElementById('dns-f-name').value.trim();
  const content = document.getElementById('dns-f-content').value.trim();
  const ttl = Number(document.getElementById('dns-f-ttl').value) || 1;
  const proxiedEl = document.getElementById('dns-f-proxied');
  const proxyable = ['A', 'AAAA', 'CNAME'].includes(recordType);
  const payload = {
    type: recordType,
    name,
    content,
    ttl,
    proxied: proxyable ? !!proxiedEl?.checked : null,
  };

  state.loading.dnsSave = true;
  renderModalRoot();
  try {
    if (state.modal.form.id) {
      await api.updateDns(state.modal.form.id, payload);
      toast('DNS record updated', 'ok');
    } else {
      await api.upsertDns(payload);
      toast('DNS record saved', 'ok');
    }
    state.modal = null;
    await loadDns({ silent: true });
  } catch (err) {
    if (!(err instanceof UnauthorizedError)) {
      toast(`DNS save failed: ${err.message}`, 'error');
    }
  } finally {
    state.loading.dnsSave = false;
    renderAll();
  }
}

async function toggleDnsProxy(id) {
  const rec = state.dns.find((r) => r.id === id);
  if (!rec) return;
  state.loading.dnsToggleId = id;
  updateDnsTableOnly();
  try {
    const updated = await api.updateDns(rec.id, {
      type: rec.record_type,
      name: rec.name,
      content: rec.content,
      ttl: rec.ttl || 1,
      proxied: !rec.proxied,
    });
    state.dns = state.dns.map((r) => (r.id === id ? updated : r));
    toast(`${updated.name}: ${updated.proxied ? 'Proxied' : 'DNS only'}`, 'ok');
  } catch (err) {
    if (!(err instanceof UnauthorizedError)) {
      toast(`Proxy update failed: ${err.message}`, 'error');
    }
  } finally {
    state.loading.dnsToggleId = null;
    updateDnsTableOnly();
  }
}

function openDnsDeleteModal(id) {
  const record = state.dns.find((r) => r.id === id);
  if (!record) return;
  state.modal = { type: 'dns-delete', record };
  renderModalRoot();
}

async function confirmDnsDelete() {
  if (!state.modal || state.modal.type !== 'dns-delete') return;
  const record = state.modal.record;
  state.loading.dnsDelete = true;
  renderModalRoot();
  try {
    await api.deleteDns(record.id);
    toast(`Deleted ${record.name}`, 'ok');
    state.modal = null;
    await loadDns({ silent: true });
  } catch (err) {
    if (!(err instanceof UnauthorizedError)) {
      toast(`Delete failed: ${err.message}`, 'error');
    }
  } finally {
    state.loading.dnsDelete = false;
    renderAll();
  }
}

function updateDnsTableOnly() {
  const tbody = document.getElementById('dns-table-body');
  if (tbody) {
    tbody.innerHTML = renderDnsRows(state);
  }
}

/* ---------- Snipe ---------- */

function appendSnipeLog(text, level = 'info') {
  const time = new Date().toTimeString().slice(0, 8);
  state.snipe.log.push({ time, text, level });
  if (state.snipe.log.length > 500) {
    state.snipe.log.splice(0, 100);
  }
  updateSnipeLiveDom();
}

function scrollTerminalToBottom() {
  const el = document.getElementById('snipe-terminal');
  if (el) el.scrollTop = el.scrollHeight;
}

function updateSnipeLiveDom() {
  if (state.tab !== 'snipe') return;
  const term = document.getElementById('snipe-terminal');
  if (term) {
    term.innerHTML = renderSnipeLogLines(state.snipe);
    scrollTerminalToBottom();
  }
  const metrics = document.getElementById('snipe-metrics');
  if (metrics) metrics.textContent = renderSnipeMetrics(state.snipe);
  const progress = document.getElementById('snipe-progress');
  if (progress) progress.style.width = `${state.snipe.waitProgress || 0}%`;
}

function startSnipeTicker() {
  stopSnipeTicker();
  snipeTickTimer = window.setInterval(() => {
    const s = state.snipe;
    if (!s.running) {
      stopSnipeTicker();
      return;
    }
    if (s.nextAttemptAt && s.waitTotalMs > 0) {
      const remaining = Math.max(0, s.nextAttemptAt - Date.now());
      s.waitProgress = Math.min(100, Math.max(0, ((s.waitTotalMs - remaining) / s.waitTotalMs) * 100));
    } else {
      s.waitProgress = 0;
    }
    const metrics = document.getElementById('snipe-metrics');
    if (metrics) metrics.textContent = renderSnipeMetrics(s);
    const progress = document.getElementById('snipe-progress');
    if (progress) progress.style.width = `${s.waitProgress}%`;
  }, 150);
}

function stopSnipeTicker() {
  if (snipeTickTimer) {
    window.clearInterval(snipeTickTimer);
    snipeTickTimer = null;
  }
}

function startSnipe() {
  const s = state.snipe;
  if (s.running) return;
  s.log = [];
  s.currentAttempt = 0;
  s.startedAt = Date.now();
  s.endedAt = null;
  s.nextAttemptAt = null;
  s.waitTotalMs = 0;
  s.waitProgress = 0;
  s.successInstanceId = null;
  s.running = true;

  const es = api.openSnipeStream({
    minDelay: s.minDelay,
    maxDelay: s.maxDelay,
    maxAttempts: s.maxAttempts,
    bypass: s.bypass,
  });
  s.source = es;
  startSnipeTicker();
  renderAll();

  es.addEventListener('started', (ev) => {
    const d = JSON.parse(ev.data);
    appendSnipeLog(
      `Started: delay ${d.min_delay}–${d.max_delay}s, max ${d.max_attempts === 0 ? 'unlimited' : d.max_attempts}${d.bypass ? ', bypass enabled' : ''}`,
    );
  });

  es.addEventListener('attempt_start', (ev) => {
    const d = JSON.parse(ev.data);
    s.currentAttempt = d.attempt;
    s.nextAttemptAt = null;
    s.waitProgress = 0;
    appendSnipeLog(`[#${d.attempt}] Launching…`);
  });

  es.addEventListener('attempt_error', (ev) => {
    const d = JSON.parse(ev.data);
    appendSnipeLog(
      `[#${d.attempt}] ${d.message}${d.retryable ? '' : ' (non-retryable)'}`,
      d.retryable ? 'warn' : 'error',
    );
  });

  es.addEventListener('waiting', (ev) => {
    const d = JSON.parse(ev.data);
    s.waitTotalMs = d.delay_secs * 1000;
    s.nextAttemptAt = Date.now() + s.waitTotalMs;
    appendSnipeLog(`Waiting ${d.delay_secs.toFixed(1)}s before next attempt…`);
  });

  es.addEventListener('success', (ev) => {
    const d = JSON.parse(ev.data);
    s.successInstanceId = d.instance_id;
    appendSnipeLog(`Launched on attempt #${d.attempt}: ${d.instance_id}`, 'success');
    toast(`Instance launched: ${d.instance_id}`, 'ok');
    stopSnipe(false);
    window.setTimeout(() => loadInstances({ silent: true }), 1500);
  });

  es.addEventListener('stopped', (ev) => {
    const d = JSON.parse(ev.data);
    appendSnipeLog(`Stopped: ${d.reason}`, 'warn');
    stopSnipe(false);
  });

  es.onerror = () => {
    if (s.running) {
      appendSnipeLog('Stream disconnected', 'warn');
      stopSnipe(false);
    }
  };
}

function stopSnipe(logStop = true) {
  const s = state.snipe;
  const wasRunning = s.running;
  if (s.source) {
    s.source.close();
    s.source = null;
  }
  s.running = false;
  s.nextAttemptAt = null;
  s.waitProgress = 0;
  if (wasRunning) {
    s.endedAt = Date.now();
    if (logStop) appendSnipeLog('Stopped by user', 'warn');
  }
  stopSnipeTicker();
  renderChrome();
  if (state.tab === 'snipe') renderView();
}

/* ---------- Settings ---------- */

function handleSettingsFieldChange(path, target) {
  if (!state.settingsForm) return;
  const [section, key] = path.split('.');
  let val;
  if (target.type === 'checkbox') {
    val = target.checked;
  } else if (target.type === 'number') {
    val = target.value === '' ? '' : Number(target.value);
  } else {
    val = target.value;
  }

  state.settingsForm[section][key] = val;

  let structuralChange = false;
  if (path === 'instance.instance_type') {
    structuralChange = true;
  } else if (path === 'instance.arm_ocpus') {
    const ocpus = Number(val) || 1;
    state.settingsForm.instance.arm_ocpus = ocpus;
    state.settingsForm.instance.arm_memory_gb = ocpus * 6;
    structuralChange = true;
  } else if (path === 'network.assign_ipv6') {
    structuralChange = true;
  }

  if (!state.settingsDirty) {
    state.settingsDirty = true;
    renderChrome();
  }
  if (structuralChange) {
    renderView();
  }
}

async function saveSettings() {
  if (!state.settingsForm) return;
  const f = state.settingsForm;
  state.loading.configSave = true;
  renderChrome();
  try {
    const payload = {
      instance: {
        instance_type: f.instance.instance_type,
        display_name: f.instance.display_name,
        arm_ocpus: f.instance.instance_type === 'arm' ? Number(f.instance.arm_ocpus) : null,
        arm_memory_gb: f.instance.instance_type === 'arm' ? Number(f.instance.arm_memory_gb) : null,
        boot_volume_size_gb: Number(f.instance.boot_volume_size_gb),
      },
      network: {
        assign_public_ip: !!f.network.assign_public_ip,
        assign_ipv6: !!f.network.assign_ipv6,
        private_ip: f.network.private_ip || null,
        ipv6_address: f.network.ipv6_address || null,
        hostname_label: f.network.hostname_label || null,
      },
      snipe: {
        min_delay_secs: Number(f.snipe.min_delay_secs),
        max_delay_secs: Number(f.snipe.max_delay_secs),
        max_attempts: Number(f.snipe.max_attempts),
      },
      cloudflare: {
        zone_name: f.cloudflare.zone_name,
        record_name: f.cloudflare.record_name || null,
        api_token: f.cloudflare.api_token || null,
      },
    };

    const updated = await api.updateConfig(payload);
    state.settingsDirty = false;
    applyConfig(updated);
    toast('Settings saved', 'ok');
    loadDns({ silent: true });
  } catch (err) {
    if (!(err instanceof UnauthorizedError)) {
      toast(`Save failed: ${err.message}`, 'error');
    }
  } finally {
    state.loading.configSave = false;
    renderAll();
  }
}

/* ---------- Auth ---------- */

async function submitAuthForm() {
  const input = document.getElementById('auth-token-input');
  const candidate = (input ? input.value : '').trim();
  if (!candidate) return;
  state.authInput = candidate;
  state.authError = '';
  state.loading.auth = true;
  renderModalRoot();
  try {
    const res = await api.verifyToken(candidate);
    if (!res.ok) {
      state.authError = res.error;
      return;
    }
    api.setToken(candidate);
    state.hasToken = true;
    state.authInput = '';
    state.showAuthModal = false;
    applyConfig(res.config);
    startPolling();
    await Promise.all([loadInstances(), loadDns({ silent: true })]);
  } catch (err) {
    state.authError = `Network error: ${err.message}`;
  } finally {
    state.loading.auth = false;
    renderAll();
  }
}

function signOut() {
  api.setToken(null);
  state.hasToken = false;
  state.config = null;
  state.settingsForm = null;
  state.settingsDirty = false;
  state.instances = [];
  state.dns = [];
  stopSnipe(false);
  state.showAuthModal = true;
  state.authError = '';
  renderAll();
}

/* ---------- Event Wiring ---------- */

function bindEvents() {
  document.addEventListener('click', (ev) => {
    const dismissBtn = ev.target.closest('[data-dismiss-toast]');
    if (dismissBtn) {
      dismissToast(Number(dismissBtn.dataset.dismissToast));
      return;
    }

    const copyBtn = ev.target.closest('[data-copy]');
    if (copyBtn) {
      copyText(copyBtn.dataset.copy, copyBtn);
      return;
    }

    const jumpDns = ev.target.closest('[data-jump-dns]');
    if (jumpDns) {
      state.dnsFilter.query = jumpDns.dataset.jumpDns;
      state.dnsFilter.type = 'ALL';
      switchTab('dns');
      return;
    }

    const tabBtn = ev.target.closest('[data-tab]');
    if (tabBtn) {
      switchTab(tabBtn.dataset.tab);
      return;
    }

    const dnsTypeBtn = ev.target.closest('[data-dns-type]');
    if (dnsTypeBtn) {
      state.dnsFilter.type = dnsTypeBtn.dataset.dnsType;
      renderView();
      return;
    }

    const fillIpBtn = ev.target.closest('[data-fill-ip]');
    if (fillIpBtn) {
      const contentEl = document.getElementById('dns-f-content');
      if (contentEl) {
        contentEl.value = fillIpBtn.dataset.fillIp;
        contentEl.focus();
      }
      return;
    }

    if (ev.target.dataset?.closeModal === '1') {
      closeModal();
      return;
    }

    const actionEl = ev.target.closest('[data-action]');
    if (!actionEl) return;
    const action = actionEl.dataset.action;

    if (action === 'toggle-theme') toggleTheme();
    else if (action === 'sign-out') signOut();
    else if (action === 'reload-instances') loadInstances();
    else if (action === 'reload-dns') loadDns();
    else if (action === 'open-create') openCreateModal();
    else if (action === 'open-refresh') openRefreshModal(actionEl.dataset.instanceId, actionEl.dataset.privateIpId);
    else if (action === 'confirm-refresh') confirmRefreshIp();
    else if (action === 'open-dns-add') openDnsModal(null);
    else if (action === 'open-dns-edit') {
      const rec = state.dns.find((r) => r.id === actionEl.dataset.dnsId);
      if (rec) openDnsModal(rec);
    } else if (action === 'open-dns-delete') openDnsDeleteModal(actionEl.dataset.dnsId);
    else if (action === 'confirm-dns-delete') confirmDnsDelete();
    else if (action === 'toggle-proxy') toggleDnsProxy(actionEl.dataset.dnsId);
    else if (action === 'snipe-start') startSnipe();
    else if (action === 'snipe-stop') stopSnipe(true);
    else if (action === 'snipe-clear') {
      state.snipe.log = [];
      renderView();
    } else if (action === 'settings-save') saveSettings();
    else if (action === 'settings-reset') {
      if (state.config) {
        state.settingsForm = cloneConfigToForm(state.config);
        state.settingsDirty = false;
        renderChrome();
        renderView();
      }
    } else if (action === 'close-modal') closeModal();
  });

  document.addEventListener('input', (ev) => {
    const target = ev.target;
    if (target.id === 'dns-search-input') {
      state.dnsFilter.query = target.value;
      updateDnsTableOnly();
      return;
    }

    if (target.dataset?.snipeField) {
      const field = target.dataset.snipeField;
      state.snipe[field] = target.type === 'checkbox' ? target.checked : Number(target.value);
      return;
    }

    if (target.dataset?.cfg && target.type !== 'checkbox' && target.tagName !== 'SELECT') {
      handleSettingsFieldChange(target.dataset.cfg, target);
    }
  });

  document.addEventListener('change', (ev) => {
    const target = ev.target;
    if (target.id === 'dns-f-type' && state.modal?.type === 'dns') {
      state.modal.form.record_type = target.value;
      state.modal.form.name = document.getElementById('dns-f-name')?.value ?? state.modal.form.name;
      state.modal.form.content = document.getElementById('dns-f-content')?.value ?? state.modal.form.content;
      state.modal.form.ttl = Number(document.getElementById('dns-f-ttl')?.value) || 1;
      renderModalRoot();
      return;
    }

    if (target.dataset?.snipeField && target.type === 'checkbox') {
      state.snipe[target.dataset.snipeField] = target.checked;
      return;
    }

    if (target.dataset?.cfg && (target.type === 'checkbox' || target.tagName === 'SELECT')) {
      handleSettingsFieldChange(target.dataset.cfg, target);
    }
  });

  document.addEventListener('submit', (ev) => {
    if (ev.target.id === 'auth-form') {
      ev.preventDefault();
      submitAuthForm();
    } else if (ev.target.id === 'create-form') {
      ev.preventDefault();
      submitCreateInstance();
    } else if (ev.target.id === 'dns-form') {
      ev.preventDefault();
      submitDnsForm();
    }
  });

  document.addEventListener('keydown', (ev) => {
    if (ev.key === 'Escape' && state.modal && !state.showAuthModal) {
      closeModal();
    }
  });
}

async function init() {
  mountShell();
  bindEvents();
  renderAll();

  try {
    const status = await api.getAuthStatus();
    state.authRequired = !!status.auth_required;
  } catch {
    state.authRequired = false;
  }

  if (state.authRequired && !api.token) {
    state.showAuthModal = true;
    renderAll();
    return;
  }

  await refreshAllData();
  startPolling();
}

init();
