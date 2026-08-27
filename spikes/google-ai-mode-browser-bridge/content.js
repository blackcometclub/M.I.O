(() => {
  "use strict";

  const HOST_ID = "moe-google-ai-bridge-poc";
  const MAX_CAPTURE_LENGTH = 100_000;
  const POLL_INTERVAL_MS = 2_000;

  if (document.getElementById(HOST_ID)) return;

  let lastSelection = "";
  let pendingDispatch = null;
  let feedbackTimer;
  let pollInProgress = false;

  const host = document.createElement("div");
  host.id = HOST_ID;
  host.setAttribute("data-moe-browser-bridge", "google-ai-mode-poc-v2");
  const shadow = host.attachShadow({ mode: "closed" });

  const style = document.createElement("style");
  style.textContent = `
    :host { all: initial; }
    .moe-bridge {
      position: fixed;
      right: 20px;
      bottom: 20px;
      z-index: 2147483647;
      display: grid;
      grid-template-columns: auto minmax(0, 1fr);
      gap: 8px 10px;
      width: min(370px, calc(100vw - 40px));
      padding: 12px;
      border: 1px solid rgba(123, 74, 0, 0.24);
      border-radius: 18px;
      background: rgba(255, 250, 237, 0.97);
      box-shadow: 0 10px 30px rgba(62, 38, 0, 0.2);
      color: #4b3100;
      font: 600 13px/1.35 system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      backdrop-filter: blur(8px);
    }
    .moe-mark {
      display: grid;
      grid-row: 1 / span 2;
      width: 32px;
      height: 32px;
      place-items: center;
      border-radius: 50%;
      background: #ffc126;
      color: #5b3600;
      font-size: 13px;
      font-weight: 800;
      letter-spacing: -0.04em;
    }
    .moe-status { min-width: 0; }
    .moe-status strong, .moe-status span { display: block; }
    .moe-status strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .moe-status span { margin-top: 2px; color: #785d2d; font-size: 11px; font-weight: 500; }
    .moe-actions {
      grid-column: 2;
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
    }
    .moe-action {
      appearance: none;
      padding: 7px 9px;
      border: 1px solid rgba(123, 74, 0, 0.2);
      border-radius: 9px;
      background: #fff9e9;
      color: inherit;
      cursor: pointer;
      font: inherit;
    }
    .moe-action:hover:not(:disabled) { background: #ffedb8; }
    .moe-action:focus-visible { outline: 2px solid #9a5b00; outline-offset: 2px; }
    .moe-action:disabled { cursor: default; opacity: 0.48; }
    .moe-action.is-primary { background: #ffc126; border-color: #d28b00; }
    .moe-bridge[data-state="success"] .moe-status strong { color: #176b43; }
    .moe-bridge[data-state="error"] .moe-status strong { color: #a23228; }
  `;

  const bridge = document.createElement("div");
  bridge.className = "moe-bridge";
  bridge.setAttribute("role", "region");
  bridge.setAttribute("aria-label", "M.O.E. Google AI Bridge");

  const mark = document.createElement("span");
  mark.className = "moe-mark";
  mark.textContent = "M";
  mark.setAttribute("aria-hidden", "true");

  const status = document.createElement("div");
  status.className = "moe-status";
  const statusTitle = document.createElement("strong");
  const statusDetail = document.createElement("span");
  status.append(statusTitle, statusDetail);

  const actions = document.createElement("div");
  actions.className = "moe-actions";
  const promptButton = document.createElement("button");
  promptButton.className = "moe-action is-primary";
  promptButton.type = "button";
  promptButton.textContent = "質問をGoogleへ入力";
  promptButton.disabled = true;
  const replyButton = document.createElement("button");
  replyButton.className = "moe-action";
  replyButton.type = "button";
  replyButton.textContent = "選択回答をM.O.E.へ返す";
  replyButton.disabled = true;
  actions.append(promptButton, replyButton);

  bridge.append(mark, status, actions);
  shadow.append(style, bridge);
  document.documentElement.append(host);

  const setStatus = (title, detail, state = "idle", temporary = false) => {
    window.clearTimeout(feedbackTimer);
    statusTitle.textContent = title;
    statusDetail.textContent = detail;
    bridge.dataset.state = state;
    if (temporary) {
      feedbackTimer = window.setTimeout(renderDispatchState, 2_800);
    }
  };

  const renderDispatchState = () => {
    promptButton.disabled = !pendingDispatch;
    replyButton.disabled = !pendingDispatch;
    if (pendingDispatch) {
      setStatus("M.O.E.から質問が届いています", "入力後、内容を確認してGoogle側で送信してください");
    } else {
      setStatus("M.O.E.と接続中", "Gemini Search宛の質問を待っています", "success");
    }
  };

  const bridgeRequest = (method, path, body) => new Promise((resolve) => {
    chrome.runtime.sendMessage(
      { type: "moe-browser-bridge-request", method, path, body },
      (response) => {
        if (chrome.runtime.lastError) {
          resolve({ ok: false, error: "extensionUnavailable" });
          return;
        }
        resolve(response ?? { ok: false, error: "emptyResponse" });
      },
    );
  });

  const validDispatch = (value) =>
    value &&
    typeof value.dispatchId === "string" &&
    typeof value.roomId === "string" &&
    typeof value.sourceMessageId === "string" &&
    typeof value.prompt === "string" &&
    typeof value.replyToken === "string";

  const poll = async () => {
    if (pollInProgress) return;
    pollInProgress = true;
    try {
      const response = await bridgeRequest("GET", "/v1/outbox/next");
      if (!response.ok || !response.value?.ok) {
        setStatus("M.O.E.お遊び版を起動してください", "通常版ではローカルBridgeは停止しています", "error");
        return;
      }
      const dispatch = response.value.dispatch;
      if (dispatch !== null && validDispatch(dispatch)) {
        if (pendingDispatch?.dispatchId !== dispatch.dispatchId) {
          lastSelection = "";
          pendingDispatch = dispatch;
        }
      } else if (dispatch !== null) {
        setStatus("質問データを確認できません", "安全のため受け取りませんでした", "error");
        return;
      }
      renderDispatchState();
    } finally {
      pollInProgress = false;
    }
  };

  const isVisible = (element) => {
    const rect = element.getBoundingClientRect();
    const style = window.getComputedStyle(element);
    return rect.width > 80 && rect.height > 20 && style.visibility !== "hidden" && style.display !== "none";
  };

  const promptEditors = () => Array.from(document.querySelectorAll(
    'textarea, input[type="text"], [contenteditable="true"]',
  )).filter(isVisible).sort((left, right) =>
    right.getBoundingClientRect().bottom - left.getBoundingClientRect().bottom,
  );

  const setNativeValue = (element, value) => {
    const prototype = element instanceof HTMLTextAreaElement
      ? HTMLTextAreaElement.prototype
      : HTMLInputElement.prototype;
    const setter = Object.getOwnPropertyDescriptor(prototype, "value")?.set;
    if (!setter) return false;
    setter.call(element, value);
    element.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: value }));
    element.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  };

  const insertPrompt = (editor, prompt) => {
    editor.focus();
    if (editor instanceof HTMLTextAreaElement || editor instanceof HTMLInputElement) {
      return setNativeValue(editor, prompt);
    }
    if (editor instanceof HTMLElement && editor.isContentEditable) {
      const selection = window.getSelection();
      const range = document.createRange();
      range.selectNodeContents(editor);
      selection?.removeAllRanges();
      selection?.addRange(range);
      const inserted = document.execCommand("insertText", false, prompt);
      if (!inserted) {
        editor.textContent = prompt;
        editor.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: prompt }));
      }
      return true;
    }
    return false;
  };

  const copyPromptFallback = async (prompt) => {
    await navigator.clipboard.writeText(prompt);
    setStatus("質問をコピーしました", "Googleの入力欄へ貼り付けてください", "success", true);
  };

  document.addEventListener("selectionchange", () => {
    const selectedText = window.getSelection()?.toString().trim() ?? "";
    if (selectedText) lastSelection = selectedText.slice(0, MAX_CAPTURE_LENGTH);
  });

  promptButton.addEventListener("click", async () => {
    if (!pendingDispatch) return;
    const editor = promptEditors()[0];
    if (editor && insertPrompt(editor, pendingDispatch.prompt)) {
      setStatus("質問を入力しました", "内容を確認してGoogle側で送信してください", "success", true);
      return;
    }
    try {
      await copyPromptFallback(pendingDispatch.prompt);
    } catch {
      setStatus("質問を入力できませんでした", "Googleの入力欄をクリックして再試行してください", "error", true);
    }
  });

  replyButton.addEventListener("pointerdown", (event) => event.preventDefault());
  replyButton.addEventListener("click", async () => {
    if (!pendingDispatch) return;
    const body = (window.getSelection()?.toString().trim() || lastSelection).slice(0, MAX_CAPTURE_LENGTH);
    if (!body) {
      setStatus("先にGeminiの回答を選択してください", "ドラッグ選択してから再試行します", "error", true);
      return;
    }
    replyButton.disabled = true;
    const response = await bridgeRequest("POST", "/v1/replies", {
      dispatchId: pendingDispatch.dispatchId,
      replyToken: pendingDispatch.replyToken,
      body,
      sourceUrl: `${window.location.origin}${window.location.pathname}`,
    });
    if (response.ok && response.value?.ok) {
      const truncated = response.value.truncated === true;
      pendingDispatch = null;
      lastSelection = "";
      setStatus(
        "Gemini SearchとしてM.O.E.へ返しました",
        truncated ? "長い回答の末尾はRoom上限に合わせて省略しました" : "M.O.E.の会話へ直接入りました",
        "success",
        true,
      );
      promptButton.disabled = true;
      return;
    }
    replyButton.disabled = false;
    setStatus("M.O.E.へ返せませんでした", "お遊び版を起動したまま再試行してください", "error", true);
  });

  setStatus("M.O.E.へ接続しています…", "少々お待ちください");
  void poll();
  window.setInterval(() => void poll(), POLL_INTERVAL_MS);
})();
