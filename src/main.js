const { useEffect, useMemo, useState } = React;

const STATUS_LABELS = {
  active: "在线",
  expired: "需检查",
  error: "错误",
  unknown: "未知",
};

function normalizeId(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9-]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function deriveHost(url) {
  try {
    return new URL(url).hostname;
  } catch (e) {
    return "";
  }
}

function formatCountdown(seconds) {
  if (!seconds) return "--:--";
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

function hostLabel(tab) {
  try {
    return new URL(tab.url).hostname.replace(/^www\./, "");
  } catch (e) {
    return tab.expected_hostname || "invalid-url";
  }
}

function App() {
  const [config, setConfig] = useState(null);
  const [activeTabId, setActiveTabId] = useState(null);
  const [openedTabs, setOpenedTabs] = useState(new Set());
  const [tabStatuses, setTabStatuses] = useState({});
  const [keepAliveRunning, setKeepAliveRunning] = useState(false);
  const [countdown, setCountdown] = useState(0);
  const [showAddForm, setShowAddForm] = useState(false);
  const [form, setForm] = useState({ name: "", url: "", expected_hostname: "" });
  const [error, setError] = useState("");

  const intervalSecs = config?.keep_alive_interval_secs || 600;
  const tabs = config?.tabs || [];

  useEffect(() => {
    let cleanupStatus = null;
    let cleanupCycle = null;
    let cancelled = false;

    async function boot() {
      try {
        const loaded = await Bridge.loadConfig();
        if (cancelled) return;
        setConfig(loaded);

        const ids = await Bridge.getSiteWebviewIds();
        if (cancelled) return;
        setOpenedTabs(new Set(ids || []));
        setTabStatuses(
          Object.fromEntries((loaded.tabs || []).map((tab) => [tab.id, "unknown"])),
        );

        cleanupStatus = await Bridge.onSessionStatus((payload) => {
          setTabStatuses((prev) => ({ ...prev, [payload.id]: payload.status || "unknown" }));
          if (payload.status === "expired") notifyExpired(payload.id, loaded.tabs || []);
        });
        cleanupCycle = await Bridge.onCycleDone(() => {
          setCountdown(loaded.keep_alive_interval_secs || 600);
        });

        await Bridge.startKeepAlive(loaded.keep_alive_interval_secs || 600);
        if (!cancelled) {
          setKeepAliveRunning(true);
          setCountdown(loaded.keep_alive_interval_secs || 600);
        }
      } catch (e) {
        setError(e.message || String(e));
      }
    }

    boot();
    return () => {
      cancelled = true;
      cleanupStatus?.();
      cleanupCycle?.();
      Bridge.stopKeepAlive().catch(() => {});
    };
  }, []);

  useEffect(() => {
    if (!keepAliveRunning) return undefined;
    const timer = setInterval(() => {
      setCountdown((prev) => Math.max(0, prev - 1));
    }, 1000);
    return () => clearInterval(timer);
  }, [keepAliveRunning]);

  const activeTab = useMemo(
    () => tabs.find((tab) => tab.id === activeTabId),
    [tabs, activeTabId],
  );

  async function switchTab(tab) {
    setError("");
    try {
      if (!openedTabs.has(tab.id)) {
        await Bridge.createSiteWebview(tab.id, tab.url);
        setOpenedTabs((prev) => new Set([...prev, tab.id]));
      }
      await Bridge.showSiteWebview(tab.id);
      setActiveTabId(tab.id);
    } catch (e) {
      setError(e.message || String(e));
    }
  }

  async function toggleKeepAlive() {
    setError("");
    try {
      if (keepAliveRunning) {
        await Bridge.stopKeepAlive();
        setKeepAliveRunning(false);
        setCountdown(0);
      } else {
        await Bridge.startKeepAlive(intervalSecs);
        setKeepAliveRunning(true);
        setCountdown(intervalSecs);
      }
    } catch (e) {
      setError(e.message || String(e));
    }
  }

  async function pingActive() {
    if (!activeTab) return;
    try {
      const status = await Bridge.pingWebview(activeTab.id, activeTab.expected_hostname);
      setTabStatuses((prev) => ({ ...prev, [activeTab.id]: status }));
    } catch (e) {
      setError(e.message || String(e));
    }
  }

  async function addTab(event) {
    event.preventDefault();
    setError("");
    const url = form.url.trim();
    const host = form.expected_hostname.trim() || deriveHost(url);
    const id = normalizeId(form.name || host);

    if (!id || !form.name.trim() || !url || !host) {
      setError("请填写名称和有效 URL。");
      return;
    }

    const nextConfig = {
      ...config,
      tabs: [
        ...tabs,
        {
          id,
          name: form.name.trim(),
          url,
          expected_hostname: host,
        },
      ],
    };

    try {
      await Bridge.saveConfig(nextConfig);
      await Bridge.stopKeepAlive();
      await Bridge.startKeepAlive(nextConfig.keep_alive_interval_secs);
      setConfig(nextConfig);
      setTabStatuses((prev) => ({ ...prev, [id]: "unknown" }));
      setShowAddForm(false);
      setForm({ name: "", url: "", expected_hostname: "" });
    } catch (e) {
      setError(e.message || String(e));
    }
  }

  if (!config) {
    return (
      <div className="bar loading" data-tauri-drag-region>
        <div className="traffic-space" data-tauri-drag-region />
        <span data-tauri-drag-region>Session Keeper 正在启动</span>
      </div>
    );
  }

  return (
    <div className="bar" data-tauri-drag-region>
      <div className="traffic-space" data-tauri-drag-region />

      <div className="tabs">
        {tabs.map((tab) => {
          const status = tabStatuses[tab.id] || "unknown";
          return (
            <button
              key={tab.id}
              className={`tab ${activeTabId === tab.id ? "is-active" : ""}`}
              title={`${tab.name} · ${hostLabel(tab)} · ${STATUS_LABELS[status]}`}
              onClick={() => switchTab(tab)}
            >
              <span className={`status-dot ${status}`} />
              <span className="tab-name">{tab.name}</span>
            </button>
          );
        })}

        <button className="icon-button" title="添加站点" onClick={() => setShowAddForm(true)}>
          +
        </button>
      </div>

      {error ? <div className="error" title={error}>{error}</div> : null}

      <div className="actions">
        <button className="mini-button" title="检查当前标签" onClick={pingActive} disabled={!activeTab}>
          检查
        </button>
        <button
          className={`keepalive ${keepAliveRunning ? "running" : ""}`}
          title="切换保活心跳"
          onClick={toggleKeepAlive}
        >
          <span className="pulse" />
          <span>{keepAliveRunning ? formatCountdown(countdown) : "已停"}</span>
        </button>
      </div>

      {showAddForm ? (
        <form className="add-popover" onSubmit={addTab}>
          <input
            autoFocus
            placeholder="名称"
            value={form.name}
            onChange={(e) => setForm((prev) => ({ ...prev, name: e.target.value }))}
          />
          <input
            placeholder="https://example.com"
            value={form.url}
            onChange={(e) => {
              const url = e.target.value;
              setForm((prev) => ({
                ...prev,
                url,
                expected_hostname: prev.expected_hostname || deriveHost(url),
              }));
            }}
          />
          <input
            placeholder="expected host"
            value={form.expected_hostname}
            onChange={(e) => setForm((prev) => ({ ...prev, expected_hostname: e.target.value }))}
          />
          <div className="form-actions">
            <button type="button" onClick={() => setShowAddForm(false)}>取消</button>
            <button type="submit">添加</button>
          </div>
        </form>
      ) : null}
    </div>
  );
}

function notifyExpired(id, tabs) {
  const tab = tabs.find((item) => item.id === id);
  if (!tab || !("Notification" in window)) return;

  if (Notification.permission === "granted") {
    new Notification("Session Keeper", { body: `${tab.name} 可能已经跳转到登录或其他域名。` });
  } else if (Notification.permission !== "denied") {
    Notification.requestPermission().then((permission) => {
      if (permission === "granted") {
        new Notification("Session Keeper", { body: `${tab.name} 可能已经过期。` });
      }
    });
  }
}

ReactDOM.createRoot(document.getElementById("root")).render(<App />);
