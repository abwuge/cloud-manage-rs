import { icon } from './icons.js';

export function esc(value) {
  if (value === null || value === undefined) return '';
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

export function truncate(str, max = 30) {
  if (!str) return '';
  if (str.length <= max) return str;
  const keep = Math.max(6, Math.floor((max - 3) / 2));
  return `${str.slice(0, keep)}...${str.slice(-keep)}`;
}

export function formatDuration(ms) {
  const totalSec = Math.max(0, Math.floor(ms / 1000));
  const m = String(Math.floor(totalSec / 60)).padStart(2, '0');
  const s = String(totalSec % 60).padStart(2, '0');
  return `${m}:${s}`;
}

export function copyChip(value, label = value, title = value) {
  if (!value) return '<span class="vnic-sub">—</span>';
  return `<button type="button" class="copy-chip" data-copy="${esc(value)}" title="${esc(title)}">
    <span>${esc(label)}</span>
    <span class="copy-icon">${icon('copy', 12)}</span>
  </button>`;
}

function stateBadge(state) {
  const upper = (state || 'UNKNOWN').toUpperCase();
  let cls = 'badge-warn';
  if (upper === 'RUNNING') cls = 'badge-ok';
  else if (upper === 'STOPPED') cls = 'badge-neutral';
  else if (upper === 'TERMINATED' || upper === 'TERMINATING') cls = 'badge-danger';
  return `<span class="badge ${cls}"><span class="status-dot"></span>${esc(upper)}</span>`;
}

function formatSpec(inst) {
  const parts = [inst.shape];
  if (inst.ocpus != null) {
    const ocpus = Number.isInteger(inst.ocpus) ? inst.ocpus : inst.ocpus.toFixed(1);
    parts.push(`${ocpus} OCPU`);
  }
  if (inst.memory_in_gbs != null) {
    const mem = Number.isInteger(inst.memory_in_gbs) ? inst.memory_in_gbs : inst.memory_in_gbs.toFixed(1);
    parts.push(`${mem} GB`);
  }
  return parts.join(' · ');
}

function buildIpToDnsMap(dnsRecords) {
  const map = new Map();
  for (const r of dnsRecords || []) {
    if (r.record_type === 'A' && r.content) {
      if (!map.has(r.content)) map.set(r.content, []);
      map.get(r.content).push(r);
    }
  }
  return map;
}

function buildIpToVnicMap(instances) {
  const map = new Map();
  for (const inst of instances || []) {
    const instName = inst.display_name || truncate(inst.instance_id, 18);
    for (const v of inst.vnics || []) {
      if (v.public_ip) {
        const role = v.is_primary ? 'Primary' : (v.display_name || 'Secondary');
        map.set(v.public_ip, { instName, role, isPrimary: v.is_primary });
      }
    }
  }
  return map;
}

export function renderSidebar(state) {
  const runningCount = state.instances.filter((i) => i.lifecycle_state === 'RUNNING').length;
  const items = [
    { id: 'instances', label: 'Instances', icon: 'server', count: state.instances.length ? `${runningCount}/${state.instances.length}` : '' },
    { id: 'dns', label: 'DNS', icon: 'globe', count: state.dns.length ? String(state.dns.length) : '' },
    { id: 'snipe', label: 'Snipe', icon: 'target', pulse: state.snipe.running },
    { id: 'settings', label: 'Settings', icon: 'settings' },
  ];

  return `
    <div class="brand">
      <span class="brand-mark">${icon('cloud', 17)}</span>
      <span class="brand-title">Cloud Manage</span>
    </div>
    <nav class="nav-list" aria-label="Primary">
      ${items.map((item) => `
        <button type="button" class="nav-item ${state.tab === item.id ? 'active' : ''}" data-tab="${item.id}">
          ${icon(item.icon, 16)}
          <span>${item.label}</span>
          ${item.pulse ? '<span class="pulse-dot" title="Running"></span>' : (item.count ? `<span class="nav-count">${esc(item.count)}</span>` : '')}
        </button>
      `).join('')}
    </nav>
    <div class="sidebar-footer">
      <button type="button" class="env-pill" data-tab="settings" title="Oracle Cloud region">
        <span>OCI</span>
        <strong>${esc(state.config?.oracle?.region || '—')}</strong>
      </button>
      <button type="button" class="env-pill" data-tab="settings" title="Cloudflare DNS zone">
        <span>Zone</span>
        <strong>${esc(state.config?.cloudflare?.zone_name || '—')}</strong>
      </button>
    </div>
  `;
}

export function renderMobileTabs(state) {
  const items = [
    { id: 'instances', label: 'Instances', icon: 'server' },
    { id: 'dns', label: 'DNS', icon: 'globe' },
    { id: 'snipe', label: 'Snipe', icon: 'target' },
    { id: 'settings', label: 'Settings', icon: 'settings' },
  ];
  return items.map((item) => `
    <button type="button" class="btn btn-sm ${state.tab === item.id ? 'btn-primary' : 'btn-secondary'}" data-tab="${item.id}">
      ${icon(item.icon, 14)}
      <span>${item.label}</span>
    </button>
  `).join('');
}

export function renderTopbar(state) {
  const titles = {
    instances: 'Instances',
    dns: 'Cloudflare DNS',
    snipe: 'Snipe',
    settings: 'Settings',
  };

  let actions = '';
  if (state.tab === 'instances') {
    actions = `
      <button type="button" class="btn btn-secondary ${state.loading.instances ? 'spin' : ''}" data-action="reload-instances" ${state.loading.instances ? 'disabled' : ''}>
        ${icon('refresh', 14)}<span>Reload</span>
      </button>
      <button type="button" class="btn btn-primary" data-action="open-create">
        ${icon('plus', 14)}<span>Launch Instance</span>
      </button>
    `;
  } else if (state.tab === 'dns') {
    actions = `
      <button type="button" class="btn btn-secondary ${state.loading.dns ? 'spin' : ''}" data-action="reload-dns" ${state.loading.dns ? 'disabled' : ''}>
        ${icon('refresh', 14)}<span>Reload</span>
      </button>
      <button type="button" class="btn btn-primary" data-action="open-dns-add">
        ${icon('plus', 14)}<span>Add Record</span>
      </button>
    `;
  } else if (state.tab === 'snipe') {
    actions = state.snipe.running
      ? `<button type="button" class="btn btn-danger" data-action="snipe-stop">${icon('square', 14)}<span>Stop</span></button>`
      : `<button type="button" class="btn btn-primary" data-action="snipe-start">${icon('play', 14)}<span>Start Snipe</span></button>`;
  } else if (state.tab === 'settings') {
    actions = `
      <button type="button" class="btn btn-secondary" data-action="settings-reset" ${!state.settingsDirty || state.loading.configSave ? 'disabled' : ''}>Reset</button>
      <button type="button" class="btn btn-primary" data-action="settings-save" ${!state.settingsDirty || state.loading.configSave ? 'disabled' : ''}>
        ${icon('save', 14)}<span>${state.loading.configSave ? 'Saving…' : 'Save Changes'}</span>
      </button>
    `;
  }

  return `
    <div class="topbar-left">
      <h1 class="topbar-title">${titles[state.tab] || 'Cloud Manage'}</h1>
    </div>
    <div class="topbar-right">
      ${actions}
      <button type="button" class="btn btn-ghost btn-icon" data-action="toggle-theme" title="Switch theme">
        ${icon(state.theme === 'dark' ? 'sun' : 'moon', 15)}
      </button>
      ${state.authRequired && state.hasToken ? `
        <button type="button" class="btn btn-ghost btn-icon" data-action="sign-out" title="Sign out">
          ${icon('logout', 15)}
        </button>
      ` : ''}
    </div>
  `;
}

export function renderInstancesView(state) {
  const instances = state.instances;
  const totalVnics = instances.reduce((sum, i) => sum + (i.vnics?.length || 0), 0);
  const activePublicIps = instances.reduce(
    (sum, i) => sum + (i.vnics || []).filter((v) => !!v.public_ip).length,
    0,
  );
  const runningCount = instances.filter((i) => i.lifecycle_state === 'RUNNING').length;
  const ipToDns = buildIpToDnsMap(state.dns);
  let matchedDnsCount = 0;
  for (const inst of instances) {
    for (const v of inst.vnics || []) {
      if (v.public_ip && ipToDns.has(v.public_ip)) {
        matchedDnsCount += ipToDns.get(v.public_ip).length;
      }
    }
  }

  const kpis = `
    <div class="kpi-grid">
      <div class="kpi-card">
        <div class="kpi-label">Instances</div>
        <div class="kpi-value">${instances.length}</div>
        <div class="kpi-sub">${runningCount} running</div>
      </div>
      <div class="kpi-card">
        <div class="kpi-label">Public IPv4</div>
        <div class="kpi-value">${activePublicIps}</div>
        <div class="kpi-sub">${totalVnics} attached VNIC${totalVnics === 1 ? '' : 's'}</div>
      </div>
      <div class="kpi-card">
        <div class="kpi-label">Linked A Records</div>
        <div class="kpi-value">${matchedDnsCount}</div>
        <div class="kpi-sub">${esc(state.config?.cloudflare?.zone_name || 'Cloudflare not configured')}</div>
      </div>
      <div class="kpi-card">
        <div class="kpi-label">Region</div>
        <div class="kpi-value mono" style="font-size: 16px; margin-top: 7px;">${esc(state.config?.oracle?.region || '—')}</div>
        <div class="kpi-sub">${esc(state.config?.instance?.instance_type ? `Default: ${state.config.instance.instance_type.toUpperCase()}` : '—')}</div>
      </div>
    </div>
  `;

  const errorBanner = state.errors.instances ? `
    <div class="alert">
      <div class="alert-main">
        <span class="alert-icon">${icon('alert', 16)}</span>
        <div class="alert-body">
          <div class="alert-title">Failed to load Oracle Cloud instances</div>
          <div>${esc(state.errors.instances)}</div>
        </div>
      </div>
      <div class="alert-actions">
        <button type="button" class="btn btn-sm btn-secondary" data-action="reload-instances">Retry</button>
      </div>
    </div>
  ` : '';

  let listHtml = '';
  if (state.loading.instances && instances.length === 0) {
    listHtml = `
      <div class="card">
        <div class="section-body">
          <div class="skeleton-line" style="width: 34%;"></div>
          <div class="skeleton-line" style="width: 100%; height: 42px;"></div>
          <div class="skeleton-line" style="width: 100%; height: 42px;"></div>
        </div>
      </div>
    `;
  } else if (instances.length === 0 && !state.errors.instances) {
    listHtml = `
      <div class="card">
        <div class="empty-state">
          ${icon('server', 28)}
          <div>No active instances in the configured compartment.</div>
          <button type="button" class="btn btn-primary" data-action="open-create">${icon('plus', 14)}<span>Launch Instance</span></button>
        </div>
      </div>
    `;
  } else {
    listHtml = instances.map((inst) => renderInstanceCard(inst, ipToDns, state)).join('');
  }

  return `${kpis}${errorBanner}${listHtml}`;
}

function renderInstanceCard(inst, ipToDns, state) {
  const vnics = inst.vnics || [];
  const rows = vnics.length
    ? vnics.map((v) => {
      const dnsMatches = v.public_ip ? (ipToDns.get(v.public_ip) || []) : [];
      const isRefreshing = state.loading.refreshKey === v.private_ip_id;
      const roleBadge = v.is_primary
        ? '<span class="badge badge-accent">Primary</span>'
        : '<span class="badge badge-neutral">Secondary</span>';
      const secIpBadge = !v.is_primary_private_ip
        ? '<span class="badge badge-neutral">Secondary IP</span>'
        : '';
      const lifetimeBadge = v.public_ip_lifetime && v.public_ip_lifetime.toUpperCase() === 'RESERVED'
        ? '<span class="badge badge-warn">Reserved</span>'
        : '';
      const vnicName = v.display_name && v.display_name !== inst.display_name ? v.display_name : '';

      const publicIpCell = v.public_ip
        ? `<div class="ip-stack">
             <div class="ip-main">
               ${copyChip(v.public_ip)}
               ${lifetimeBadge}
             </div>
             ${v.ipv6_addresses?.length ? `<div class="ip-sub">${copyChip(v.ipv6_addresses[0])}</div>` : ''}
           </div>`
        : (v.public_ip_error
          ? `<span class="badge badge-warn" title="${esc(v.public_ip_error)}">Lookup failed</span>`
          : '<span class="vnic-sub">None</span>');

      const dnsCell = dnsMatches.length
        ? `<div class="dns-pill-list">${dnsMatches.map((rec) => `
             <button type="button" class="dns-pill" data-jump-dns="${esc(rec.name)}" title="${esc(`${rec.name} (${rec.proxied ? 'Proxied' : 'DNS only'})`)}">
               ${esc(rec.name)}
             </button>
           `).join('')}</div>`
        : '<span class="vnic-sub">—</span>';

      return `
        <tr>
          <td>
            <div class="vnic-cell">
              <div class="vnic-head">
                ${roleBadge}
                ${vnicName ? `<span class="mono" style="font-size: 12.5px;">${esc(vnicName)}</span>` : ''}
                ${secIpBadge}
              </div>
              <div class="vnic-sub mono">NIC ${v.nic_index ?? '—'} · ${esc(truncate(v.vnic_id, 22))}</div>
            </div>
          </td>
          <td>${publicIpCell}</td>
          <td>${copyChip(v.private_ip)}</td>
          <td>${dnsCell}</td>
          <td class="col-right">
            <button type="button"
                    class="btn btn-sm btn-secondary ${isRefreshing ? 'spin' : ''}"
                    data-action="open-refresh"
                    data-instance-id="${esc(inst.instance_id)}"
                    data-private-ip-id="${esc(v.private_ip_id)}"
                    ${!v.can_refresh || isRefreshing ? 'disabled' : ''}
                    title="${esc(v.refresh_disabled_reason || 'Rotate ephemeral public IPv4')}">
              ${icon('rotate', 13)}
              <span>${isRefreshing ? 'Refreshing…' : 'Refresh IP'}</span>
            </button>
          </td>
        </tr>
      `;
    }).join('')
    : `<tr><td colspan="5" style="text-align:center; color: var(--text-muted); padding: 20px;">No attached VNICs reported.</td></tr>`;

  return `
    <article class="card">
      <div class="card-header">
        <div class="card-title-group">
          ${stateBadge(inst.lifecycle_state)}
          <span class="instance-name">${esc(inst.display_name || '(unnamed)')}</span>
          ${vnics.length > 1 ? `<span class="badge badge-neutral">${vnics.length} VNICs</span>` : ''}
        </div>
        <div class="instance-meta">
          <span class="mono">${esc(formatSpec(inst))}</span>
          <span class="meta-sep">·</span>
          <span>${esc(inst.availability_domain)}</span>
        </div>
      </div>
      <div class="table-wrap">
        <table class="data-table">
          <thead>
            <tr>
              <th>Interface</th>
              <th>Public IP</th>
              <th>Private IPv4</th>
              <th>Cloudflare A</th>
              <th class="col-right">Action</th>
            </tr>
          </thead>
          <tbody>${rows}</tbody>
        </table>
      </div>
      <details class="details-drawer">
        <summary>
          <span class="chev">${icon('chevronRight', 13)}</span>
          <span>Technical IDs</span>
        </summary>
        <div class="details-grid">
          <div class="kv-row">
            <span class="kv-key">Instance OCID</span>
            <span class="kv-val">${copyChip(inst.instance_id, truncate(inst.instance_id, 30), inst.instance_id)}</span>
          </div>
          <div class="kv-row">
            <span class="kv-key">Created</span>
            <span class="kv-val mono">${esc(inst.time_created ? inst.time_created.replace('T', ' ').replace(/\.\d+Z$/, ' UTC') : '—')}</span>
          </div>
          ${vnics.map((v, idx) => `
            <div class="kv-row">
              <span class="kv-key">${v.is_primary ? 'Primary' : `Secondary #${idx}`} MAC</span>
              <span class="kv-val">${copyChip(v.mac_address || '', v.mac_address || '—', v.mac_address || '')}</span>
            </div>
            <div class="kv-row">
              <span class="kv-key">${v.is_primary ? 'Primary' : `Secondary #${idx}`} VNIC</span>
              <span class="kv-val">${copyChip(v.vnic_id, truncate(v.vnic_id, 28), v.vnic_id)}</span>
            </div>
          `).join('')}
        </div>
      </details>
    </article>
  `;
}

export function renderDnsView(state) {
  const counts = { ALL: state.dns.length };
  for (const r of state.dns) {
    counts[r.record_type] = (counts[r.record_type] || 0) + 1;
  }
  const types = ['ALL', ...Object.keys(counts).filter((k) => k !== 'ALL').sort()];

  const errorBanner = state.errors.dns ? `
    <div class="alert">
      <div class="alert-main">
        <span class="alert-icon">${icon('alert', 16)}</span>
        <div class="alert-body">
          <div class="alert-title">Failed to load Cloudflare DNS records</div>
          <div>${esc(state.errors.dns)}</div>
        </div>
      </div>
      <div class="alert-actions">
        <button type="button" class="btn btn-sm btn-secondary" data-tab="settings">Settings</button>
        <button type="button" class="btn btn-sm btn-secondary" data-action="reload-dns">Retry</button>
      </div>
    </div>
  ` : '';

  return `
    ${errorBanner}
    <div class="toolbar">
      <div class="search-box">
        <span class="search-icon">${icon('search', 14)}</span>
        <input id="dns-search-input" class="input" type="search" placeholder="Filter by record name or value…" value="${esc(state.dnsFilter.query)}" />
      </div>
      <div class="filter-pills">
        ${types.map((t) => `
          <button type="button" class="filter-pill ${state.dnsFilter.type === t ? 'active' : ''}" data-dns-type="${esc(t)}">
            <span>${t === 'ALL' ? 'All' : esc(t)}</span>
            <span class="tabular" style="opacity:0.7;">${counts[t] || 0}</span>
          </button>
        `).join('')}
      </div>
    </div>

    <div class="card">
      <div class="table-wrap">
        <table class="data-table">
          <thead>
            <tr>
              <th>Type</th>
              <th>Name</th>
              <th>Content</th>
              <th>TTL</th>
              <th>Proxy</th>
              <th class="col-right">Actions</th>
            </tr>
          </thead>
          <tbody id="dns-table-body">
            ${renderDnsRows(state)}
          </tbody>
        </table>
      </div>
    </div>
  `;
}

export function renderDnsRows(state) {
  if (state.loading.dns && state.dns.length === 0) {
    return `<tr><td colspan="6" style="padding: 28px; text-align:center; color: var(--text-muted);">Loading DNS records…</td></tr>`;
  }

  const q = state.dnsFilter.query.trim().toLowerCase();
  const type = state.dnsFilter.type;
  const filtered = state.dns.filter((r) => {
    if (type !== 'ALL' && r.record_type !== type) return false;
    if (q && !r.name.toLowerCase().includes(q) && !r.content.toLowerCase().includes(q)) return false;
    return true;
  });

  if (filtered.length === 0) {
    const msg = state.dns.length === 0 ? 'No DNS records found.' : 'No records match the current filter.';
    return `<tr><td colspan="6" style="padding: 32px; text-align:center; color: var(--text-muted);">${msg}</td></tr>`;
  }

  const ipToVnic = buildIpToVnicMap(state.instances);

  return filtered.map((r) => {
    const linked = r.record_type === 'A' ? ipToVnic.get(r.content) : null;
    const proxyable = ['A', 'AAAA', 'CNAME'].includes(r.record_type);
    const toggling = state.loading.dnsToggleId === r.id;

    return `
      <tr>
        <td><span class="badge badge-neutral mono">${esc(r.record_type)}</span></td>
        <td>${copyChip(r.name)}</td>
        <td>
          <div class="ip-main">
            ${copyChip(r.content)}
            ${linked ? `<span class="badge ${linked.isPrimary ? 'badge-accent' : 'badge-neutral'}">${esc(`${linked.instName} · ${linked.role}`)}</span>` : ''}
          </div>
        </td>
        <td class="mono">${r.ttl === 1 ? 'Auto' : esc(r.ttl)}</td>
        <td>
          ${proxyable
            ? `<button type="button" class="badge ${r.proxied ? 'badge-ok' : 'badge-neutral'}" data-action="toggle-proxy" data-dns-id="${esc(r.id)}" ${toggling ? 'disabled' : ''} title="Click to toggle Cloudflare proxy">
                 ${toggling ? '…' : (r.proxied ? 'Proxied' : 'DNS only')}
               </button>`
            : '<span class="vnic-sub">—</span>'}
        </td>
        <td class="col-right">
          <div style="display:inline-flex; gap: 4px;">
            <button type="button" class="btn btn-ghost btn-icon-sm" data-action="open-dns-edit" data-dns-id="${esc(r.id)}" title="Edit">${icon('pencil', 14)}</button>
            <button type="button" class="btn btn-danger btn-icon-sm" data-action="open-dns-delete" data-dns-id="${esc(r.id)}" title="Delete">${icon('trash', 14)}</button>
          </div>
        </td>
      </tr>
    `;
  }).join('');
}

export function renderSnipeView(state) {
  const cfg = state.config;
  const inst = cfg?.instance;
  const spec = inst
    ? (inst.instance_type === 'arm'
      ? `ARM Flex (${inst.arm_ocpus ?? 2} OCPU / ${inst.arm_memory_gb ?? 12} GB) · ${inst.display_name}`
      : `AMD Micro · ${inst.display_name}`)
    : 'Loading configuration…';

  const s = state.snipe;
  const statusBadge = s.running
    ? '<span class="badge badge-warn"><span class="status-dot"></span>Running</span>'
    : (s.successInstanceId
      ? '<span class="badge badge-ok"><span class="status-dot"></span>Launched</span>'
      : '<span class="badge badge-neutral">Idle</span>');

  const disabled = s.running ? 'disabled' : '';

  return `
    <div class="snipe-meta-bar">
      <div style="display:flex; align-items:center; gap: 10px; flex-wrap:wrap;">
        <span class="vnic-sub">Target</span>
        <strong class="mono">${esc(spec)}</strong>
        ${cfg?.oracle?.availability_domain ? `<span class="meta-sep">·</span><span class="vnic-sub">${esc(cfg.oracle.availability_domain)}</span>` : ''}
      </div>
      <button type="button" class="btn btn-sm btn-ghost" data-tab="settings">Edit target</button>
    </div>

    ${s.successInstanceId ? `
      <div class="alert alert-ok">
        <div class="alert-main">
          <span class="alert-icon">${icon('check', 16)}</span>
          <div class="alert-body">
            <div class="alert-title">Instance launched on attempt #${s.currentAttempt}</div>
            <div>${copyChip(s.successInstanceId)}</div>
          </div>
        </div>
        <div class="alert-actions">
          <button type="button" class="btn btn-sm btn-secondary" data-tab="instances">View Instances</button>
        </div>
      </div>
    ` : ''}

    <div class="card">
      <div class="section-body">
        <div class="form-grid-4">
          <label class="field">
            <span class="field-label">Min Delay (s)</span>
            <input class="input mono" type="number" step="0.5" min="0" data-snipe-field="minDelay" value="${esc(s.minDelay)}" ${disabled} />
          </label>
          <label class="field">
            <span class="field-label">Max Delay (s)</span>
            <input class="input mono" type="number" step="0.5" min="0" data-snipe-field="maxDelay" value="${esc(s.maxDelay)}" ${disabled} />
          </label>
          <label class="field">
            <span class="field-label">Max Attempts (0 = unlimited)</span>
            <input class="input mono" type="number" min="0" data-snipe-field="maxAttempts" value="${esc(s.maxAttempts)}" ${disabled} />
          </label>
          <div class="field" style="justify-content: flex-end; padding-bottom: 6px;">
            <label class="checkbox-row">
              <input type="checkbox" data-snipe-field="bypass" ${s.bypass ? 'checked' : ''} ${disabled} />
              <span>Retry non-retryable errors</span>
            </label>
          </div>
        </div>
      </div>
    </div>

    <div class="card">
      <div class="card-header">
        <div class="card-title-group">
          ${icon('terminal', 15)}
          <span style="font-weight:600;">Event Stream</span>
          <span id="snipe-status-badge">${statusBadge}</span>
        </div>
        <div style="display:flex; align-items:center; gap: 12px; font-size: 12.5px; color: var(--text-secondary);">
          <span id="snipe-metrics" class="mono">${renderSnipeMetrics(s)}</span>
          <button type="button" class="btn btn-sm btn-ghost" data-action="snipe-clear" ${s.log.length === 0 ? 'disabled' : ''}>Clear</button>
        </div>
      </div>
      <div class="progress-track"><div id="snipe-progress" class="progress-fill" style="width: ${s.waitProgress || 0}%;"></div></div>
      <div id="snipe-terminal" class="snipe-terminal">${renderSnipeLogLines(s)}</div>
    </div>
  `;
}

export function renderSnipeMetrics(s) {
  const parts = [`Attempts: ${s.currentAttempt}`];
  if (s.startedAt) {
    const elapsed = (s.endedAt || Date.now()) - s.startedAt;
    parts.push(`Elapsed: ${formatDuration(elapsed)}`);
  }
  if (s.running && s.nextAttemptAt) {
    const rem = Math.max(0, (s.nextAttemptAt - Date.now()) / 1000);
    parts.push(`Next: ${rem.toFixed(1)}s`);
  }
  return parts.join(' · ');
}

export function renderSnipeLogLines(s) {
  if (s.log.length === 0) {
    return '<div style="color: var(--text-muted);">Ready. Start Snipe to stream launch attempts.</div>';
  }
  return s.log.map((line) => `
    <div class="log-line log-${esc(line.level)}">
      <span class="log-time">${esc(line.time)}</span>
      <span>${esc(line.text)}</span>
    </div>
  `).join('');
}

export function renderSettingsView(state) {
  const form = state.settingsForm;
  if (!form || !state.config) {
    return `
      <div class="card">
        <div class="empty-state">Loading settings…</div>
      </div>
    `;
  }

  const oracle = state.config.oracle;
  const isArm = form.instance.instance_type === 'arm';

  return `
    <div class="settings-grid">
      <section class="card">
        <div class="card-header">
          <div class="section-title">${icon('server', 15)}<span>Instance Defaults</span></div>
        </div>
        <div class="section-body">
          <div class="form-grid-2">
            <label class="field">
              <span class="field-label">Instance Type</span>
              <select class="input" data-cfg="instance.instance_type">
                <option value="amd" ${!isArm ? 'selected' : ''}>AMD Micro (E2.1.Micro)</option>
                <option value="arm" ${isArm ? 'selected' : ''}>ARM Flex (A1.Flex)</option>
              </select>
            </label>
            <label class="field">
              <span class="field-label">Boot Volume (GB)</span>
              <input class="input mono" type="number" min="47" data-cfg="instance.boot_volume_size_gb" value="${esc(form.instance.boot_volume_size_gb)}" />
            </label>
            <label class="field col-span-2">
              <span class="field-label">Display Name</span>
              <input class="input" type="text" data-cfg="instance.display_name" value="${esc(form.instance.display_name)}" />
            </label>
            ${isArm ? `
              <label class="field">
                <span class="field-label">ARM OCPUs (1–4)</span>
                <select class="input mono" data-cfg="instance.arm_ocpus">
                  ${[1, 2, 3, 4].map((n) => `<option value="${n}" ${Number(form.instance.arm_ocpus) === n ? 'selected' : ''}>${n} OCPU</option>`).join('')}
                </select>
              </label>
              <label class="field">
                <span class="field-label">ARM Memory (GB)</span>
                <input class="input mono" type="number" value="${esc(form.instance.arm_memory_gb)}" disabled />
              </label>
            ` : ''}
          </div>
        </div>
      </section>

      <section class="card">
        <div class="card-header">
          <div class="section-title">${icon('network', 15)}<span>Network Defaults</span></div>
        </div>
        <div class="section-body">
          <div style="display:flex; gap: 20px; flex-wrap:wrap;">
            <label class="checkbox-row">
              <input type="checkbox" data-cfg="network.assign_public_ip" ${form.network.assign_public_ip ? 'checked' : ''} />
              <span>Assign Public IPv4</span>
            </label>
            <label class="checkbox-row">
              <input type="checkbox" data-cfg="network.assign_ipv6" ${form.network.assign_ipv6 ? 'checked' : ''} />
              <span>Assign IPv6</span>
            </label>
          </div>
          <div class="form-grid-2">
            <label class="field">
              <span class="field-label">Private IPv4</span>
              <input class="input mono" type="text" placeholder="Auto" data-cfg="network.private_ip" value="${esc(form.network.private_ip)}" />
            </label>
            <label class="field">
              <span class="field-label">Hostname Label</span>
              <input class="input mono" type="text" placeholder="Optional" data-cfg="network.hostname_label" value="${esc(form.network.hostname_label)}" />
            </label>
            <label class="field col-span-2">
              <span class="field-label">IPv6 Address</span>
              <input class="input mono" type="text" placeholder="Auto" data-cfg="network.ipv6_address" value="${esc(form.network.ipv6_address)}" ${!form.network.assign_ipv6 ? 'disabled' : ''} />
            </label>
          </div>
        </div>
      </section>

      <section class="card">
        <div class="card-header">
          <div class="section-title">${icon('target', 15)}<span>Snipe Defaults</span></div>
        </div>
        <div class="section-body">
          <div class="form-grid-3">
            <label class="field">
              <span class="field-label">Min Delay (s)</span>
              <input class="input mono" type="number" step="0.5" min="0" data-cfg="snipe.min_delay_secs" value="${esc(form.snipe.min_delay_secs)}" />
            </label>
            <label class="field">
              <span class="field-label">Max Delay (s)</span>
              <input class="input mono" type="number" step="0.5" min="0" data-cfg="snipe.max_delay_secs" value="${esc(form.snipe.max_delay_secs)}" />
            </label>
            <label class="field">
              <span class="field-label">Max Attempts</span>
              <input class="input mono" type="number" min="0" data-cfg="snipe.max_attempts" value="${esc(form.snipe.max_attempts)}" />
            </label>
          </div>
        </div>
      </section>

      <section class="card">
        <div class="card-header">
          <div class="section-title">
            ${icon('globe', 15)}
            <span>Cloudflare DNS</span>
            <span class="badge ${state.config.cloudflare.api_token_set ? 'badge-ok' : 'badge-danger'}">
              ${state.config.cloudflare.api_token_set ? 'Token set' : 'Token missing'}
            </span>
          </div>
        </div>
        <div class="section-body">
          <div class="form-grid-2">
            <label class="field">
              <span class="field-label">Zone Name</span>
              <input class="input mono" type="text" placeholder="example.com" data-cfg="cloudflare.zone_name" value="${esc(form.cloudflare.zone_name)}" />
            </label>
            <label class="field">
              <span class="field-label">Default Record</span>
              <input class="input mono" type="text" placeholder="Optional" data-cfg="cloudflare.record_name" value="${esc(form.cloudflare.record_name)}" />
            </label>
            <label class="field col-span-2">
              <span class="field-label">API Token (Bearer)</span>
              <input class="input mono" type="password" placeholder="${state.config.cloudflare.api_token_set ? 'Leave blank to keep existing token' : 'Enter DNS Edit API token'}" data-cfg="cloudflare.api_token" value="${esc(form.cloudflare.api_token)}" />
            </label>
          </div>
        </div>
      </section>
    </div>

    <section class="card">
      <div class="card-header">
        <div class="section-title">${icon('cloud', 15)}<span>Oracle Cloud Connection</span></div>
      </div>
      <div class="section-body">
        <div class="details-grid" style="padding: 0;">
          <div class="kv-row"><span class="kv-key">Region</span><span class="kv-val">${copyChip(oracle.region)}</span></div>
          <div class="kv-row"><span class="kv-key">Availability Domain</span><span class="kv-val">${copyChip(oracle.availability_domain)}</span></div>
          <div class="kv-row"><span class="kv-key">Compartment</span><span class="kv-val">${copyChip(oracle.compartment_id, truncate(oracle.compartment_id, 28), oracle.compartment_id)}</span></div>
          <div class="kv-row"><span class="kv-key">Subnet</span><span class="kv-val">${copyChip(oracle.subnet_id, truncate(oracle.subnet_id, 28), oracle.subnet_id)}</span></div>
          <div class="kv-row"><span class="kv-key">AMD Image</span><span class="kv-val">${copyChip(oracle.image_id_amd, truncate(oracle.image_id_amd, 28), oracle.image_id_amd)}</span></div>
          <div class="kv-row"><span class="kv-key">ARM Image</span><span class="kv-val">${copyChip(oracle.image_id_arm, truncate(oracle.image_id_arm, 28), oracle.image_id_arm)}</span></div>
        </div>
      </div>
    </section>
  `;
}

export function renderModal(state) {
  if (state.showAuthModal) {
    return `
      <div class="modal-backdrop">
        <div class="modal-dialog" role="dialog" aria-modal="true" style="max-width: 390px;">
          <div class="modal-header">
            <h2 class="modal-title">${icon('lock', 16)}<span>Sign In</span></h2>
          </div>
          <form id="auth-form">
            <div class="modal-body">
              <label class="field">
                <span class="field-label">Bearer Token</span>
                <input id="auth-token-input" class="input mono" type="password" autocomplete="current-password" required autofocus value="${esc(state.authInput)}" />
              </label>
              ${state.authError ? `<div style="color: var(--danger); font-size: 13px;">${esc(state.authError)}</div>` : ''}
            </div>
            <div class="modal-footer">
              <button type="submit" class="btn btn-primary" ${state.loading.auth ? 'disabled' : ''}>${state.loading.auth ? 'Verifying…' : 'Sign In'}</button>
            </div>
          </form>
        </div>
      </div>
    `;
  }

  const m = state.modal;
  if (!m) return '';

  if (m.type === 'create') {
    const inst = state.config?.instance;
    const summary = inst
      ? (inst.instance_type === 'arm'
        ? `ARM Flex (${inst.arm_ocpus ?? 2} OCPU / ${inst.arm_memory_gb ?? 12} GB) · ${inst.boot_volume_size_gb} GB`
        : `AMD Micro · ${inst.boot_volume_size_gb} GB`)
      : 'Configured template';
    return `
      <div class="modal-backdrop" data-close-modal="1">
        <div class="modal-dialog" role="dialog" aria-modal="true">
          <div class="modal-header">
            <h2 class="modal-title">${icon('server', 16)}<span>Launch Instance</span></h2>
            <button type="button" class="btn btn-ghost btn-icon-sm" data-action="close-modal">${icon('x', 15)}</button>
          </div>
          <form id="create-form">
            <div class="modal-body">
              <div class="summary-box">
                <div class="kv-row" style="border:none; padding:0;"><span class="kv-key">Template</span><span class="kv-val mono">${esc(summary)}</span></div>
                <div class="kv-row" style="border:none; padding:0;"><span class="kv-key">AD</span><span class="kv-val mono">${esc(state.config?.oracle?.availability_domain || '—')}</span></div>
              </div>
              <label class="field">
                <span class="field-label">Display Name Override</span>
                <input id="create-name-input" class="input" type="text" placeholder="${esc(inst?.display_name || '')}" value="${esc(m.displayName || '')}" autofocus />
              </label>
            </div>
            <div class="modal-footer">
              <button type="button" class="btn btn-secondary" data-action="close-modal">Cancel</button>
              <button type="submit" class="btn btn-primary" ${state.loading.create ? 'disabled' : ''}>${state.loading.create ? 'Launching…' : 'Launch'}</button>
            </div>
          </form>
        </div>
      </div>
    `;
  }

  if (m.type === 'refresh') {
    const { inst, vnic, updateDns } = m;
    const matchingDns = vnic.public_ip
      ? state.dns.filter((r) => r.record_type === 'A' && r.content === vnic.public_ip)
      : [];
    const role = vnic.is_primary ? 'Primary' : 'Secondary';
    return `
      <div class="modal-backdrop" data-close-modal="1">
        <div class="modal-dialog" role="dialog" aria-modal="true">
          <div class="modal-header">
            <h2 class="modal-title">${icon('rotate', 16)}<span>Refresh Public IPv4</span></h2>
            <button type="button" class="btn btn-ghost btn-icon-sm" data-action="close-modal">${icon('x', 15)}</button>
          </div>
          <div class="modal-body">
            <div class="summary-box">
              <div class="kv-row" style="border:none; padding:0;">
                <span class="kv-key">Instance</span>
                <span class="kv-val" style="color:var(--text); font-weight:600;">${esc(inst.display_name || truncate(inst.instance_id, 24))}</span>
              </div>
              <div class="kv-row" style="border:none; padding:0;">
                <span class="kv-key">VNIC</span>
                <span class="kv-val">
                  <span class="badge ${vnic.is_primary ? 'badge-accent' : 'badge-neutral'}">${role}</span>
                  ${vnic.display_name ? `<span class="mono" style="margin-left:6px;">${esc(vnic.display_name)}</span>` : ''}
                </span>
              </div>
              <div class="kv-row" style="border:none; padding:0;">
                <span class="kv-key">Private IPv4</span>
                <span class="kv-val mono">${esc(vnic.private_ip)}</span>
              </div>
              <div class="kv-row" style="border:none; padding:0;">
                <span class="kv-key">Current Public IP</span>
                <span class="kv-val mono" style="color:var(--text);">${esc(vnic.public_ip || 'None')}</span>
              </div>
            </div>

            <div style="font-size: 13px; color: var(--text-secondary);">
              Releases the current ephemeral IPv4 on this VNIC and allocates a new one. Active connections to this IP will drop.
            </div>

            ${vnic.public_ip ? `
              <div style="display:flex; flex-direction:column; gap:6px;">
                <label class="checkbox-row">
                  <input type="checkbox" id="refresh-update-dns" ${updateDns ? 'checked' : ''} />
                  <span>Update matching Cloudflare A records</span>
                </label>
                <div class="field-hint mono" style="padding-left: 23px;">
                  ${matchingDns.length
                    ? matchingDns.map((r) => esc(r.name)).join(', ')
                    : 'No Cloudflare A records currently point to this IP.'}
                </div>
              </div>
            ` : ''}
          </div>
          <div class="modal-footer">
            <button type="button" class="btn btn-secondary" data-action="close-modal">Cancel</button>
            <button type="button" class="btn btn-primary ${state.loading.refreshKey ? 'spin' : ''}" data-action="confirm-refresh" ${state.loading.refreshKey ? 'disabled' : ''}>
              ${icon('rotate', 14)}<span>${state.loading.refreshKey ? 'Refreshing…' : 'Refresh IP'}</span>
            </button>
          </div>
        </div>
      </div>
    `;
  }

  if (m.type === 'dns') {
    const f = m.form;
    const isEdit = !!f.id;
    const proxyable = ['A', 'AAAA', 'CNAME'].includes(f.record_type);
    const knownIps = [];
    if (f.record_type === 'A') {
      for (const inst of state.instances) {
        for (const v of inst.vnics || []) {
          if (v.public_ip) {
            knownIps.push({
              ip: v.public_ip,
              label: `${inst.display_name || truncate(inst.instance_id, 12)} · ${v.is_primary ? 'Primary' : (v.display_name || 'Secondary')}`,
            });
          }
        }
      }
    }

    return `
      <div class="modal-backdrop" data-close-modal="1">
        <div class="modal-dialog" role="dialog" aria-modal="true">
          <div class="modal-header">
            <h2 class="modal-title">${icon('globe', 16)}<span>${isEdit ? 'Edit DNS Record' : 'Add DNS Record'}</span></h2>
            <button type="button" class="btn btn-ghost btn-icon-sm" data-action="close-modal">${icon('x', 15)}</button>
          </div>
          <form id="dns-form">
            <div class="modal-body">
              <div class="form-grid-2">
                <label class="field">
                  <span class="field-label">Type</span>
                  <select class="input mono" id="dns-f-type">
                    ${['A', 'AAAA', 'CNAME', 'TXT', 'MX', 'NS'].map((t) => `<option value="${t}" ${f.record_type === t ? 'selected' : ''}>${t}</option>`).join('')}
                  </select>
                </label>
                <label class="field">
                  <span class="field-label">TTL (1 = Auto)</span>
                  <input class="input mono" type="number" min="1" id="dns-f-ttl" value="${esc(f.ttl)}" />
                </label>
                <label class="field col-span-2">
                  <span class="field-label">Name</span>
                  <input class="input mono" type="text" id="dns-f-name" required value="${esc(f.name)}" placeholder="${esc(state.config?.cloudflare?.zone_name ? `host.${state.config.cloudflare.zone_name}` : 'app.example.com')}" />
                </label>
                <label class="field col-span-2">
                  <span class="field-label">Content</span>
                  <input class="input mono" type="text" id="dns-f-content" required value="${esc(f.content)}" placeholder="203.0.113.10" />
                </label>
                ${knownIps.length ? `
                  <div class="field col-span-2">
                    <span class="field-label">Use Instance IP</span>
                    <div class="quick-ip-row">
                      ${knownIps.map((item) => `
                        <button type="button" class="quick-ip-btn mono" data-fill-ip="${esc(item.ip)}">${esc(item.label)}: ${esc(item.ip)}</button>
                      `).join('')}
                    </div>
                  </div>
                ` : ''}
                ${proxyable ? `
                  <div class="col-span-2">
                    <label class="checkbox-row">
                      <input type="checkbox" id="dns-f-proxied" ${f.proxied ? 'checked' : ''} />
                      <span>Cloudflare Proxy</span>
                    </label>
                  </div>
                ` : ''}
              </div>
            </div>
            <div class="modal-footer">
              <button type="button" class="btn btn-secondary" data-action="close-modal">Cancel</button>
              <button type="submit" class="btn btn-primary" ${state.loading.dnsSave ? 'disabled' : ''}>${state.loading.dnsSave ? 'Saving…' : 'Save'}</button>
            </div>
          </form>
        </div>
      </div>
    `;
  }

  if (m.type === 'dns-delete') {
    const r = m.record;
    return `
      <div class="modal-backdrop" data-close-modal="1">
        <div class="modal-dialog" role="dialog" aria-modal="true" style="max-width: 420px;">
          <div class="modal-header">
            <h2 class="modal-title">${icon('trash', 16)}<span>Delete DNS Record</span></h2>
            <button type="button" class="btn btn-ghost btn-icon-sm" data-action="close-modal">${icon('x', 15)}</button>
          </div>
          <div class="modal-body">
            <div class="summary-box mono">
              <div>${esc(r.record_type)} ${esc(r.name)}</div>
              <div style="color: var(--text-secondary);">${esc(r.content)}</div>
            </div>
          </div>
          <div class="modal-footer">
            <button type="button" class="btn btn-secondary" data-action="close-modal">Cancel</button>
            <button type="button" class="btn btn-danger" data-action="confirm-dns-delete" ${state.loading.dnsDelete ? 'disabled' : ''}>
              ${state.loading.dnsDelete ? 'Deleting…' : 'Delete'}
            </button>
          </div>
        </div>
      </div>
    `;
  }

  return '';
}

export function renderToasts(toasts) {
  return toasts.map((t) => {
    const kind = t.kind === 'error' ? 'error' : (t.kind === 'ok' ? 'ok' : 'info');
    const ic = kind === 'error' ? 'alert' : (kind === 'ok' ? 'check' : 'info');
    return `
      <div class="toast toast-${kind}" role="status">
        <span class="toast-icon">${icon(ic, 16)}</span>
        <div class="toast-msg">${esc(t.message)}</div>
        <button type="button" class="btn btn-ghost btn-icon-sm" data-dismiss-toast="${t.id}" aria-label="Dismiss">${icon('x', 13)}</button>
      </div>
    `;
  }).join('');
}
