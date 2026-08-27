import {
  createContext,
  type ReactNode,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";

export type UiLocale = "ja" | "en";

type UiPreferences = {
  chatFontScale: number;
  fontFamily: string;
  locale: UiLocale;
  participantListCollapsed: boolean;
  sidebarFontScale: number;
  sidebarWidth: number;
};

export const systemFontFamily = "__system__";
const storageKey = "moe-ui-preferences-v2";
const legacyStorageKey = "moe-ui-preferences-v1";
const defaultPreferences: UiPreferences = {
  chatFontScale: 1,
  fontFamily: systemFontFamily,
  locale: "ja",
  participantListCollapsed: false,
  sidebarFontScale: 1,
  sidebarWidth: 220,
};

const legacyFontFamilies: Record<string, string> = {
  system: systemFontFamily,
  "yu-gothic": "Yu Gothic UI",
  meiryo: "Meiryo",
  "biz-udp-gothic": "BIZ UDPGothic",
};
const systemFontStack = 'Inter, "Yu Gothic UI", "Hiragino Kaku Gothic ProN", system-ui, sans-serif';

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, value));
}

function normalizeFontFamily(value: unknown) {
  if (value === systemFontFamily) return systemFontFamily;
  if (typeof value !== "string" || value.length === 0 || value.length > 256 || /[\u0000-\u001f\u007f]/u.test(value)) {
    return systemFontFamily;
  }
  return value;
}

function fontStack(fontFamily: string) {
  return fontFamily === systemFontFamily ? systemFontStack : `${JSON.stringify(fontFamily)}, ${systemFontStack}`;
}

const ja = {
  roomConductor: "指揮者",
  roomConductorHelp: "このルームの依頼をまとめるAIを選びます",
  roomConductorSelection: "指揮者AI",
  noConductor: "指揮者なし",
  conductorModeChangeHelp: "送信欄で Direct / Conductor をいつでも切り替えられます",
  conductorBadge: "指揮者",
  sendMode: "送信モード",
  appLabel: "M.I.O. トークルーム",
  rooms: "トークルーム",
  roomList: "ルーム一覧",
  newRoom: "新しいルーム",
  aiCount: "AI {count}人",
  noConversation: "まだ会話なし",
  now: "いま",
  roomSettings: "ルーム設定",
  appearance: "背景を変更",
  preferences: "環境設定",
  close: "閉じる",
  minimize: "最小化",
  maximize: "最大化または元に戻す",
  participants: "参加AI",
  chooseRecipients: "クリックで今回の宛先を選択",
  collapseParticipants: "参加AI一覧を畳む",
  expandParticipants: "参加AI一覧を開く",
  checking: "確認中",
  checkingDetail: "接続状態を確認しています。",
  addAi: "AIを追加",
  addToRoom: "ルームに追加",
  addStatusNote: "追加後も接続状態は個別に表示されます",
  currentlyUnsupported: "現在未対応",
  allAiAdded: "追加できるAIはすべて参加中です。",
  conversation: "会話",
  viaCodex: "via Codex",
  quietRoom: "まだ静かなルームです",
  firstMessage: "宛先を選んで、最初のメッセージを送ってみましょう。",
  thinking: "{name} が考え中",
  removeRecipient: "{name}を宛先から外す",
  chooseRecipientFirst: "上の参加AIから宛先を選んでください",
  message: "メッセージ",
  messagePlaceholder: "M.I.O.のみんなに相談してみる…",
  saving: "保存中",
  waiting: "応答待ち",
  send: "送信",
  dismissSafetyWarning: "確認済みとして閉じる",
  appearanceTitle: "M.I.O.本体をきせかえ",
  appearanceSubtitle: "黄色い部分だけを変更します",
  backgroundColor: "背景色",
  colorPresets: "背景色プリセット",
  bodyBackground: "本体の背景画像",
  chooseBackground: "背景を選ぶ",
  clearBackground: "背景画像を外す",
  artwork: "M.I.O.全体の飾り絵",
  artworkHelp: "Chatツリーを含む黄色い部分全体へ、位置と倍率を指定します",
  chooseArtwork: "飾り絵を選ぶ",
  editPlacement: "配置を調整",
  clearArtwork: "飾り絵を外す",
  artworkNote: "M.I.O.のウインドウ全体に合わせた画像がおすすめです。",
  appearanceLoading: "保存済みのきせかえを読み込み中…",
  appearanceSaving: "この端末へきせかえを保存中…",
  appearanceSaved: "きせかえはこの端末へ自動保存されます。",
  appearanceSaveFailed: "きせかえを保存できませんでした。画像サイズをご確認ください。",
  preferencesTitle: "環境設定",
  preferencesSubtitle: "文字と表示言語を変更します",
  language: "言語",
  japanese: "日本語",
  english: "English",
  fontFamily: "フォント",
  fontSize: "チャット文字サイズ",
  fontSizeValue: "{value}%",
  systemFont: "システム標準",
  installedFontsLoading: "インストール済みフォントを読み込み中…",
  noInstalledFonts: "フォント一覧を取得できませんでした。システム標準を使用できます。",
  sidebarTextSmaller: "ツリーの文字を小さくする",
  sidebarTextLarger: "ツリーの文字を大きくする",
  resizeRoomTree: "トークルームツリーの幅を変更",
  preferenceSaved: "変更はこの端末に自動保存されます。",
  roomSettingsSubtitle: "名前と参加AIを管理します",
  participantProfiles: "参加者プロフィール",
  profileDeviceNote: "この端末で共通",
  editProfile: "編集",
  aiMembership: "参加AIの管理",
  aiContinuity: "AIの会話継続",
  aiContinuityHelp: "AI側の継続だけを解除します",
  roomHistoryKept: "Roomの会話履歴は残ります",
  resetContinuity: "継続をリセット",
  profileTitle: "参加者プロフィールを編集",
  profileSubtitle: "表示名と丸いアイコンを、この端末用に設定します",
  identityLocked: "正式な身元",
  displayName: "表示名",
  aiInstructions: "AIの基本設定",
  aiInstructionsHelp: "このAIの話し方・役割・あなたの呼び方を指定します。次の発言から反映されます。",
  aiInstructionsPlaceholder: "例：いつもノリノリで、親しみやすく返事をする。太郎さんと呼ぶ。",
  aiPermissions: "AIのアクセス権限",
  aiPermissionsHelp: "この端末でM.I.O.から呼ぶ時の上限です。実装済みの安全な権限だけ選べます。",
  permissionChatOnly: "会話のみ",
  permissionChatOnlyDetail: "Roomの会話だけを送り、ローカルファイルやコマンドを使いません。",
  permissionWorkspaceRead: "選択フォルダーを読む",
  permissionWorkspaceReadDetail: "nested junctionの読取り境界が未解決のため、Windows alpha.1では利用できません。",
  permissionWorkspaceWrite: "選択フォルダーを読取り・編集",
  permissionWorkspaceWriteDetail: "nested junctionの読取り境界が未解決のため、Windows alpha.1では利用できません。",
  permissionNotSupported: "このAIは未対応",
  permissionCommands: "コマンド",
  permissionWeb: "Web・ネットワーク",
  permissionOff: "禁止",
  permissionLocalOnly: "選択フォルダー内のみ",
  permissionNeedsWorkspace: "この権限を使うには、ルーム設定で作業フォルダーを選んでください。未選択時は会話のみになります。",
  avatarImage: "アイコン画像",
  avatarPositionHelp: "画像をドラッグして位置を調整できます。PNG・JPEG・WebPに対応。",
  chooseAvatar: "画像を選ぶ",
  changeAvatar: "画像を変更",
  removeAvatar: "画像を外す",
  avatarZoom: "拡大率",
  profilePreview: "見え方",
  invalidAvatarImage: "この画像は読み込めません。PNG・JPEG・WebPを選んでください。",
  avatarImageTooLarge: "画像が大きすぎます。5MB以下の画像を選んでください。",
  profileSaveFailed: "保存できませんでした。表示名と画像をご確認ください。",
  profileSaving: "保存中…",
  saveProfile: "プロフィールを保存",
  cancel: "キャンセル",
  roomName: "ルーム名",
  save: "保存",
  backupTitle: "全ルームのバックアップ",
  backupHelp: "Documents 内のM.I.O.バックアップ保存先へ安全なJSONを保存します（旧フォルダー名を互換利用）。",
  backup: "バックアップ",
  restore: "最新を復元…",
  reallyRestore: "本当に復元",
  codexMode: "Codex作業モード",
  workspaceAlphaUnavailable: "Windows alpha.1では安全確認未完了のため利用できません。会話のみを利用します。",
  workspaceActive: "{name} 内の読取り・編集を許可中です。",
  workspaceMissing: "{name} が見つかりません。",
  chatOnly: "現在は会話のみ。ローカルファイルを見ません。",
  changeFolder: "フォルダー変更",
  chooseFolder: "フォルダーを選ぶ",
  returnChatOnly: "会話のみに戻す",
  keepOneAi: "最低1人は残します",
  historyKeepsAi: "履歴に使われているため保持",
  cannotRemoveAi: "履歴または最低人数を守るため外せません",
  removeFromRoom: "ルームから外す",
  remove: "外す",
  deleteRoom: "このルームを削除",
  deleteRoomHelp: "会話履歴も一緒に削除され、元に戻せません。",
  delete: "削除…",
  reallyDelete: "本当に削除",
  protectedRoom: "標準ルームは削除から保護されています。",
  coreOffline: "Room offline",
  coreConnecting: "Room connecting",
  coreReady: "Core + Room ready",
  previewReady: "Preview ready",
  awaitingHint: "接続済みAIからの応答を待っています…",
  workspaceMissingHint: "Codex作業フォルダーが見つかりません · ルーム設定をご確認ください",
  unconnectedHint: "未接続AI宛はRoomへ保存のみ · 現在は返信がありません",
  workspaceHint: "Codex作業モード · {name}内を読取り・編集できます",
  sendHint: "Enter で送信 · Shift + Enter で改行 · Room保存後、接続済みAIへ配送します",
  roomUnavailableHint: "Rust Roomが利用できるまで送信できません",
  demoHint: "Enter で送信 · Shift + Enter で改行 · 現在はダミー応答です",
  selectedFolder: "選択フォルダー",
} as const;

type UiTextKey = keyof typeof ja;
const en: Record<UiTextKey, string> = {
  roomConductor: "Conductor",
  roomConductorHelp: "Choose the AI that coordinates requests in this room.",
  roomConductorSelection: "Conductor AI",
  noConductor: "No conductor",
  conductorModeChangeHelp: "You can switch between Direct and Conductor in the composer at any time.",
  conductorBadge: "Conductor",
  sendMode: "Send mode",
  appLabel: "M.I.O. Talk Rooms", rooms: "Talk rooms", roomList: "Room list", newRoom: "New room",
  aiCount: "{count} AI", noConversation: "No messages yet", now: "now", roomSettings: "Room settings",
  appearance: "Change appearance", preferences: "Preferences", close: "Close", minimize: "Minimize",
  maximize: "Maximize or restore", participants: "Participating AI", chooseRecipients: "Click to choose recipients for this message",
  collapseParticipants: "Collapse participating AI", expandParticipants: "Expand participating AI",
  checking: "Checking", checkingDetail: "Checking connection status.", addAi: "Add AI", addToRoom: "Add to room",
  addStatusNote: "Connection status remains visible after adding", currentlyUnsupported: "Currently unsupported", allAiAdded: "All available AI are already participating.",
  conversation: "Conversation", viaCodex: "via Codex", quietRoom: "This room is still quiet", firstMessage: "Choose recipients and send the first message.",
  thinking: "{name} is thinking", removeRecipient: "Remove {name} from recipients", chooseRecipientFirst: "Choose recipients from the participating AI above",
  message: "Message", messagePlaceholder: "Ask everyone in M.I.O.…", saving: "Saving", waiting: "Waiting", send: "Send",
  dismissSafetyWarning: "Acknowledge and close",
  appearanceTitle: "Dress up M.I.O.", appearanceSubtitle: "Changes only the colored shell", backgroundColor: "Background color",
  colorPresets: "Background color presets", bodyBackground: "Shell background image", chooseBackground: "Choose background",
  clearBackground: "Remove background image", artwork: "M.I.O. full-shell artwork", artworkHelp: "Position and scale artwork across the colored shell, including the Chat tree",
  chooseArtwork: "Choose artwork", editPlacement: "Adjust placement", clearArtwork: "Remove artwork",
  artworkNote: "An image matching the whole M.I.O. window works best.", preferencesTitle: "Preferences",
  appearanceLoading: "Loading saved appearance…", appearanceSaving: "Saving appearance to this device…",
  appearanceSaved: "Appearance is saved automatically on this device.",
  appearanceSaveFailed: "Appearance could not be saved. Check the image size.",
  preferencesSubtitle: "Change text and interface language", language: "Language", japanese: "日本語", english: "English",
  fontFamily: "Font", fontSize: "Chat text size", fontSizeValue: "{value}%", preferenceSaved: "Changes are saved automatically on this device.",
  systemFont: "System default", installedFontsLoading: "Loading installed fonts…",
  noInstalledFonts: "The installed font list could not be loaded. System default remains available.",
  sidebarTextSmaller: "Make room-tree text smaller", sidebarTextLarger: "Make room-tree text larger",
  resizeRoomTree: "Resize the talk-room tree",
  roomSettingsSubtitle: "Manage the name and participating AI", roomName: "Room name", save: "Save",
  participantProfiles: "Participant profiles", profileDeviceNote: "Shared on this device", editProfile: "Edit",
  aiMembership: "Room AI membership", profileTitle: "Edit participant profile",
  aiContinuity: "AI conversation continuity", aiContinuityHelp: "Clears only the AI-side continuation",
  roomHistoryKept: "Room conversation history is kept", resetContinuity: "Reset continuity",
  profileSubtitle: "Set a display name and circular avatar for this device", identityLocked: "Verified identity",
  displayName: "Display name", avatarImage: "Avatar image",
  aiInstructions: "AI defaults", aiInstructionsHelp: "Set this AI's tone, role, and how it addresses you. Applies from the next message.",
  aiInstructionsPlaceholder: "Example: Be energetic and friendly. Address me as John.",
  aiPermissions: "AI access permissions", aiPermissionsHelp: "This is the maximum access M.I.O. grants on this device. Only verified permissions can be selected.",
  permissionChatOnly: "Conversation only", permissionChatOnlyDetail: "Sends only the Room conversation. Local files and commands are unavailable.",
  permissionWorkspaceRead: "Read selected folder", permissionWorkspaceReadDetail: "Unavailable in Windows alpha.1 because the nested-junction read boundary is unresolved.",
  permissionWorkspaceWrite: "Read and edit selected folder", permissionWorkspaceWriteDetail: "Unavailable in Windows alpha.1 because the nested-junction read boundary is unresolved.",
  permissionNotSupported: "Not supported for this AI", permissionCommands: "Commands", permissionWeb: "Web and network",
  permissionOff: "Blocked", permissionLocalOnly: "Selected folder only",
  permissionNeedsWorkspace: "Choose a workspace folder in Room settings to use this permission. Without one, the AI remains conversation-only.",
  avatarPositionHelp: "Drag the image to reposition it. PNG, JPEG, and WebP are supported.",
  chooseAvatar: "Choose image", changeAvatar: "Change image", removeAvatar: "Remove image", avatarZoom: "Zoom",
  profilePreview: "Preview", invalidAvatarImage: "This image could not be read. Choose a PNG, JPEG, or WebP file.",
  avatarImageTooLarge: "The image is too large. Choose an image up to 5 MB.",
  profileSaveFailed: "The profile could not be saved. Check the display name and image.",
  profileSaving: "Saving…", saveProfile: "Save profile", cancel: "Cancel",
  backupTitle: "Back up all rooms", backupHelp: "Saves safe JSON to M.I.O.'s backup location in Documents (the legacy folder name is retained for compatibility).", backup: "Back up",
  restore: "Restore latest…", reallyRestore: "Confirm restore", codexMode: "Codex workspace mode",
  workspaceAlphaUnavailable: "Unavailable in Windows alpha.1 pending safety verification. Chat-only mode is used.",
  workspaceActive: "Reading and editing is allowed inside {name}.", workspaceMissing: "{name} could not be found.",
  chatOnly: "Chat only. Local files are not accessed.", changeFolder: "Change folder", chooseFolder: "Choose folder",
  returnChatOnly: "Return to chat only", keepOneAi: "Keep at least one", historyKeepsAi: "Kept because it appears in history",
  cannotRemoveAi: "Cannot remove because of history or the minimum participant count", removeFromRoom: "Remove from room", remove: "Remove",
  deleteRoom: "Delete this room", deleteRoomHelp: "Conversation history will also be deleted and cannot be restored.",
  delete: "Delete…", reallyDelete: "Confirm delete", protectedRoom: "Default rooms are protected from deletion.",
  coreOffline: "Room offline", coreConnecting: "Room connecting", coreReady: "Core + Room ready", previewReady: "Preview ready",
  awaitingHint: "Waiting for connected AI…", workspaceMissingHint: "Codex workspace was not found · Check Room settings",
  unconnectedHint: "Saved to the Room only for disconnected AI · No reply is available", workspaceHint: "Codex workspace · Can read and edit inside {name}",
  sendHint: "Enter to send · Shift + Enter for a new line · Delivered after Room save", roomUnavailableHint: "Sending is disabled until Rust Room is available",
  demoHint: "Enter to send · Shift + Enter for a new line · Demo replies are active", selectedFolder: "selected folder",
};

function loadPreferences(): UiPreferences {
  try {
    type StoredPreferences = Partial<UiPreferences> & { font?: string; fontScale?: number };
    const value = JSON.parse(localStorage.getItem(storageKey) ?? localStorage.getItem(legacyStorageKey) ?? "null") as StoredPreferences | null;
    const legacyFont = typeof value?.font === "string" ? legacyFontFamilies[value.font] : undefined;
    const fontFamily = normalizeFontFamily(value?.fontFamily ?? legacyFont);
    const locale = value?.locale === "en" || value?.locale === "ja" ? value.locale : defaultPreferences.locale;
    const rawChatScale = typeof value?.chatFontScale === "number"
      ? value.chatFontScale
      : typeof value?.fontScale === "number" ? value.fontScale : defaultPreferences.chatFontScale;
    const rawSidebarScale = typeof value?.sidebarFontScale === "number" ? value.sidebarFontScale : defaultPreferences.sidebarFontScale;
    const rawSidebarWidth = typeof value?.sidebarWidth === "number" ? value.sidebarWidth : defaultPreferences.sidebarWidth;
    return {
      chatFontScale: clamp(rawChatScale, 0.8, 1.5),
      fontFamily,
      locale,
      participantListCollapsed: typeof value?.participantListCollapsed === "boolean"
        ? value.participantListCollapsed
        : defaultPreferences.participantListCollapsed,
      sidebarFontScale: clamp(rawSidebarScale, 0.8, 1.3),
      sidebarWidth: clamp(rawSidebarWidth, 180, 420),
    };
  } catch {
    return defaultPreferences;
  }
}

type UiPreferencesContextValue = UiPreferences & {
  setChatFontScale: (scale: number) => void;
  setFontFamily: (fontFamily: string) => void;
  setLocale: (locale: UiLocale) => void;
  setParticipantListCollapsed: (collapsed: boolean) => void;
  setSidebarFontScale: (scale: number) => void;
  setSidebarWidth: (width: number) => void;
  t: (key: UiTextKey, values?: Record<string, string | number>) => string;
};

const UiPreferencesContext = createContext<UiPreferencesContextValue | null>(null);

export function UiPreferencesProvider({ children }: { children: ReactNode }) {
  const [preferences, setPreferences] = useState(loadPreferences);

  useEffect(() => {
    try {
      localStorage.setItem(storageKey, JSON.stringify(preferences));
    } catch {
      // Keep the preferences usable for this session when storage is unavailable.
    }
    document.documentElement.lang = preferences.locale;
    document.documentElement.style.setProperty("--moe-chat-font-scale", String(preferences.chatFontScale));
    document.documentElement.style.setProperty("--moe-font-family", fontStack(preferences.fontFamily));
    document.documentElement.style.setProperty("--moe-sidebar-font-scale", String(preferences.sidebarFontScale));
    document.documentElement.style.setProperty("--moe-sidebar-width", `${preferences.sidebarWidth}px`);
  }, [preferences]);

  const value = useMemo<UiPreferencesContextValue>(() => ({
    ...preferences,
    setChatFontScale: (chatFontScale) => setPreferences((current) => ({ ...current, chatFontScale: clamp(chatFontScale, 0.8, 1.5) })),
    setFontFamily: (fontFamily) => setPreferences((current) => ({ ...current, fontFamily: normalizeFontFamily(fontFamily) })),
    setLocale: (locale) => setPreferences((current) => ({ ...current, locale })),
    setParticipantListCollapsed: (participantListCollapsed) => setPreferences((current) => ({ ...current, participantListCollapsed })),
    setSidebarFontScale: (sidebarFontScale) => setPreferences((current) => ({ ...current, sidebarFontScale: clamp(sidebarFontScale, 0.8, 1.3) })),
    setSidebarWidth: (sidebarWidth) => setPreferences((current) => ({ ...current, sidebarWidth: clamp(sidebarWidth, 180, 420) })),
    t: (key, values = {}) => Object.entries(values).reduce(
      (text, [name, replacement]) => text.replaceAll(`{${name}}`, String(replacement)),
      (preferences.locale === "ja" ? ja[key] : en[key]) as string,
    ),
  }), [preferences]);

  return <UiPreferencesContext.Provider value={value}>{children}</UiPreferencesContext.Provider>;
}

export function useUiPreferences() {
  const value = useContext(UiPreferencesContext);
  if (!value) throw new Error("UiPreferencesProvider is missing");
  return value;
}
