(function () {
  const tauri = window.__TAURI__;
  const tauriInvoke = tauri?.core?.invoke;
  const tauriListen = tauri?.event?.listen;

  const fallbackConfig = {
    keep_alive_interval_secs: 600,
    tabs: [
      {
        id: "google-calendar",
        name: "Google Calendar",
        url: "https://calendar.google.com/calendar/u/0/r",
        expected_hostname: "calendar.google.com",
      },
      {
        id: "tencent-docs",
        name: "腾讯文档",
        url: "https://doc.weixin.qq.com/sheet/e3_AAkAnQYYAAobkO1wc4NRumAOQCo6a",
        expected_hostname: "doc.weixin.qq.com",
      },
      {
        id: "cloudflare",
        name: "Cloudflare",
        url: "https://dash.cloudflare.com/0aa088497f85e67a7eae3fbe77521797/",
        expected_hostname: "dash.cloudflare.com",
      },
      {
        id: "webstore-ratings",
        name: "Web Store 控制台",
        url: "https://chrome.google.com/webstore/devconsole",
        expected_hostname: "chrome.google.com",
      },
      {
        id: "extension-reviews",
        name: "扩展评价",
        url: "https://chromewebstore.google.com/detail/reviews?hl=zh-CN",
        expected_hostname: "chromewebstore.google.com",
      },
    ],
  };

  function invoke(command, args) {
    if (!tauriInvoke) {
      if (command === "load_config") return Promise.resolve(fallbackConfig);
      if (command === "save_config") return Promise.resolve();
      if (command === "get_site_webview_ids") return Promise.resolve([]);
      if (command === "ping_webview") return Promise.resolve("unknown");
      return Promise.resolve();
    }
    return tauriInvoke(command, args);
  }

  function listen(eventName, cb) {
    if (!tauriListen) return Promise.resolve(function noop() {});
    return tauriListen(eventName, (event) => cb(event.payload));
  }

  window.Bridge = {
    loadConfig: () => invoke("load_config"),
    saveConfig: (config) => invoke("save_config", { config }),
    getSiteWebviewIds: () => invoke("get_site_webview_ids"),
    createSiteWebview: (id, url) => invoke("create_site_webview", { id, url }),
    showSiteWebview: (id) => invoke("show_site_webview", { id }),
    closeSiteWebview: (id) => invoke("close_site_webview", { id }),
    startKeepAlive: (intervalSecs) => invoke("start_keep_alive", { intervalSecs }),
    stopKeepAlive: () => invoke("stop_keep_alive"),
    pingWebview: (id, expectedHostname) => invoke("ping_webview", { id, expectedHostname }),
    onSessionStatus: (cb) => listen("session-status", cb),
    onCycleDone: (cb) => listen("keep-alive-cycle-done", cb),
  };
})();
