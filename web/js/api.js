const TOKEN_KEY = 'cloudManageToken';

export class UnauthorizedError extends Error {
  constructor() {
    super('unauthorized');
    this.name = 'UnauthorizedError';
  }
}

export class ApiClient {
  constructor({ onUnauthorized } = {}) {
    this.token = localStorage.getItem(TOKEN_KEY) || null;
    this.onUnauthorized = onUnauthorized || (() => {});
  }

  setToken(token) {
    this.token = token || null;
    if (this.token) {
      localStorage.setItem(TOKEN_KEY, this.token);
    } else {
      localStorage.removeItem(TOKEN_KEY);
    }
  }

  async getAuthStatus() {
    const res = await fetch('/api/auth-status');
    if (!res.ok) return { auth_required: false };
    return res.json();
  }

  async verifyToken(candidate) {
    const res = await fetch('/api/config', {
      headers: { Authorization: `Bearer ${candidate}` },
    });
    if (res.status === 401) {
      return { ok: false, error: 'Invalid token.' };
    }
    if (!res.ok) {
      return { ok: false, error: `Server returned HTTP ${res.status}.` };
    }
    const config = await res.json();
    return { ok: true, config };
  }

  async request(path, opts = {}) {
    const headers = new Headers(opts.headers || {});
    if (opts.body !== undefined && !headers.has('Content-Type')) {
      headers.set('Content-Type', 'application/json');
    }
    if (this.token) {
      headers.set('Authorization', `Bearer ${this.token}`);
    }

    const res = await fetch(path, {
      ...opts,
      headers,
      body: opts.body !== undefined ? JSON.stringify(opts.body) : undefined,
    });

    if (res.status === 401) {
      this.setToken(null);
      this.onUnauthorized();
      throw new UnauthorizedError();
    }

    const text = await res.text();
    let data = null;
    if (text) {
      try {
        data = JSON.parse(text);
      } catch {
        data = { error: text };
      }
    }

    if (!res.ok) {
      throw new Error(data?.error || `${res.status} ${res.statusText}`);
    }
    return data;
  }

  getConfig() {
    return this.request('/api/config');
  }

  updateConfig(payload) {
    return this.request('/api/config', { method: 'PUT', body: payload });
  }

  listInstances() {
    return this.request('/api/instances');
  }

  createInstance(displayName) {
    return this.request('/api/instances', {
      method: 'POST',
      body: { display_name: displayName || null },
    });
  }

  refreshInstanceIp(instanceId, { vnicId, privateIpId, updateDns }) {
    return this.request(`/api/instances/${encodeURIComponent(instanceId)}/refresh-ip`, {
      method: 'POST',
      body: {
        vnic_id: vnicId || null,
        private_ip_id: privateIpId || null,
        update_dns: !!updateDns,
      },
    });
  }

  listDns() {
    return this.request('/api/dns');
  }

  upsertDns(payload) {
    return this.request('/api/dns', { method: 'POST', body: payload });
  }

  updateDns(id, payload) {
    return this.request(`/api/dns/${encodeURIComponent(id)}`, { method: 'PUT', body: payload });
  }

  deleteDns(id) {
    return this.request(`/api/dns/${encodeURIComponent(id)}`, { method: 'DELETE' });
  }

  openSnipeStream({ minDelay, maxDelay, maxAttempts, bypass }) {
    const params = new URLSearchParams({
      min_delay: String(minDelay),
      max_delay: String(maxDelay),
      max_attempts: String(maxAttempts),
      bypass: String(!!bypass),
    });
    if (this.token) {
      params.set('token', this.token);
    }
    return new EventSource(`/api/snipe/stream?${params.toString()}`);
  }
}
